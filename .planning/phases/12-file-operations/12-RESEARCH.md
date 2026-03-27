# Phase 12: File Operations - Research

**Researched:** 2026-03-26
**Domain:** Native file dialogs (rfd), async I/O, toast notifications, Untitled buffer naming, UTF-8 validation
**Confidence:** HIGH

---

## Summary

Phase 12 wires the file operation stubs already present in the codebase to real implementations.
`EditorCommand::OpenFile`, `SaveAs`, `New`, and `Save` already exist as variants in
`ora/src/editor_adapter/types.rs` and are already mapped to keystrokes in
`ora/src/events/editor_input.rs`. The adapter in `wgpu_client/src/adapter.rs` currently returns
stub messages for all of them. This phase replaces those stubs with real behavior.

The key architectural challenge is that `dispatch_command(&mut self, cmd)` returns `()` — it is
synchronous and has no result channel. A native file-picker dialog is a blocking OS call that
must not run on the event thread. The solution is ora's existing
`AppContext::spawn(future)` / `LocalExecutor` mechanism (already used for transitions and
effects). The event loop calls `app_context.tick_executor()` on every event and on
`about_to_wait`, so futures spawned there are polled continuously without any external runtime.

The file dialog library is **rfd 0.15.x** (`AsyncFileDialog`). On all platforms it is
truly non-blocking when called inside a windowed app (winit is already running). The dialog
future resolves with `Option<FileHandle>` (single) or `Option<Vec<FileHandle>>` (multi). On
native, `FileHandle::path()` returns `&Path` directly — no async read needed for the path.

The async I/O plan for FILE-05: spawn a background `std::thread` to read the file bytes, send
the content back through a `std::sync::mpsc::channel`, and poll the receiver inside a
`async_executor` future that lives on the event thread. This keeps the UI thread free while
reading large files.

**Primary recommendation:** Use `rfd 0.15` with `AsyncFileDialog`, run inside `cx.spawn()` on
the `LocalExecutor`, deliver results back via `std::sync::mpsc::channel` polled in the same
future. Add rfd to `wgpu_client/Cargo.toml` only (not ora or core_editor).

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rfd | 0.15 | Native OS file picker dialogs | De-facto standard for Rust; pure Rust on Windows/macOS, XDG Portal on Linux; zero C deps on Linux default |
| async-executor | 1 (already in ora) | Run dialog futures on main thread | Already present; `LocalExecutor::try_tick()` is called each event loop cycle |
| std::sync::mpsc | stdlib | Channel from read thread → main thread | Zero-dep; non-blocking `try_recv` can be polled inside executor futures |
| std::thread | stdlib | Background file read thread | No async runtime needed; spawn once per open |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dirs | 5.x | Platform config/data directories | Persisting last-used directory across sessions |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rfd 0.15 | rfd 0.17 | 0.17 dropped tokio/async-std features, uses pollster directly; 0.15 is simpler and stable |
| std::sync::mpsc | tokio::sync::oneshot | tokio is not in the project; adding it would be heavyweight |
| dirs crate | hard-coded path | dirs gives correct platform paths (AppData on Windows, ~/.config on Linux) |
| async file read | synchronous read with rayon | Simpler but adds rayon dep; std::thread is sufficient |

### Installation

```toml
# wgpu_client/Cargo.toml — add to [dependencies]
rfd = { version = "0.15", default-features = false, features = ["xdg-portal"] }
dirs = "5"
```

Note: `default-features = false` drops async-std dependency; `xdg-portal` is the pure-Rust
Linux backend. On Windows and macOS no extra features are needed.

---

## Architecture Patterns

### Recommended Project Structure

No new files required for a minimal phase. Changes are concentrated in:

```
wgpu_client/src/
├── adapter.rs        # Replace stubs: handle OpenFile, SaveAs, New in dispatch_command
└── Cargo.toml        # Add rfd + dirs

core_editor/src/
├── app.rs            # new_untitled() method + open_file_path() for async result delivery
├── domain/document.rs # Document::open_async() — reads via thread, validates UTF-8
└── domain/workspace.rs # create_untitled_document() with naming counter
```

### Pattern 1: Async Dialog via Local Executor

**What:** Spawn a `Future` on `AppContext`'s `LocalExecutor`. The future opens the dialog (which
is itself async via `AsyncFileDialog`), then when a path is returned, spawns a background thread
to read the file, and delivers the result by mutating the adapter through a shared `Rc<RefCell<>>`.

**When to use:** All file dialog operations (OpenFile, SaveAs). Must stay on main thread for
macOS compatibility.

**The problem:** `dispatch_command` is called with `&mut self` on the adapter; it cannot capture
`self` in a future. The solution is to use the same `Rc<RefCell<Box<dyn EditorDataSource>>>`
(`SharedAdapter`) that `OraApp` already holds. The event loop already borrows it to call
`dispatch_command`; the future can also borrow it (since futures only run between events, not
during event processing).

**Concrete approach:**

The event loop needs a way to pass `SharedAdapter` and a request-redraw callback into futures.
Two options:

A. **Pass SharedAdapter into dispatch_command result** — change the return type. Locked decision
   says dispatch returns `()` — do NOT do this.

B. **Add a `PendingFileOp` queue to the adapter** — `dispatch_command` sets a flag/queue entry;
   the event loop polls the queue each tick and spawns the future with access to `SharedAdapter`.
   This is clean and requires no trait changes.

**Recommended approach (B):**

```rust
// In CoreEditorAdapter:
pub enum PendingFileOp {
    Open,
    SaveAs,
    New,
}
pub pending_file_op: Option<PendingFileOp>,

// In dispatch_command:
EditorCommand::OpenFile => {
    self.pending_file_op = Some(PendingFileOp::Open);
}

// In event loop (after adapter.borrow_mut().dispatch_command(cmd)):
if let Some(op) = adapter.borrow_mut().take_pending_file_op() {
    let adapter_clone = adapter.clone();  // Rc clone
    let window_handle = gpu_state.window.clone();
    cx.spawn(async move {
        match op {
            PendingFileOp::Open => {
                let files = AsyncFileDialog::new()
                    .set_directory(last_dir())
                    .pick_files()
                    .await;
                if let Some(handles) = files {
                    for handle in handles {
                        let path = handle.path().to_path_buf();
                        // Spawn thread to read
                        let (tx, rx) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let result = std::fs::read(&path)
                                .and_then(|bytes| String::from_utf8(bytes)
                                    .map_err(|_| std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "file is not valid UTF-8"
                                    )));
                            let _ = tx.send((path, result));
                        });
                        // Poll until result arrives
                        loop {
                            if let Ok((path, result)) = rx.try_recv() {
                                adapter_clone.borrow_mut().handle_file_loaded(path, result);
                                window_handle.request_redraw();
                                break;
                            }
                            // Yield to event loop
                            futures_lite::future::yield_now().await;
                        }
                    }
                }
            }
            // ... SaveAs, New
        }
    });
}
```

Note: `futures-lite` is a dependency of `async-executor` and is likely already available.
Alternatively, yield with a custom future that wraps `Poll::Pending` once.

### Pattern 2: UTF-8 Detection

**What:** `std::fs::read_to_string()` returns `Err` with `kind: InvalidData` for non-UTF-8 files.
Alternatively, `std::fs::read()` (bytes) then `String::from_utf8()` gives explicit `FromUtf8Error`.
The latter is preferred for clear error discrimination.

**UTF-8 BOM stripping:**
```rust
fn strip_utf8_bom(s: String) -> String {
    if s.starts_with('\u{FEFF}') {
        s[3..].to_string()  // BOM is 3 bytes (EF BB BF)
    } else {
        s
    }
}
```

**Non-UTF-8 error message (FILE-06):** "Cannot open: file is not valid UTF-8" — shown as
`ToastSeverity::Error` (red, persistent). Do NOT create a tab.

### Pattern 3: Untitled Buffer Naming

The current `Document::title()` returns `"[New File]"` for untitled documents. Phase 12 changes
this to `"Untitled"`, `"Untitled (2)"`, `"Untitled (3)"`, etc.

The naming counter should live in `Workspace` (or `App`), not `Document`, because documents
don't know about other documents. Simplest approach:

```rust
// Workspace:
untitled_counter: u32,  // increments on each new untitled doc

pub fn create_untitled_document(&mut self) -> DocumentId {
    self.untitled_counter += 1;
    let name = if self.untitled_counter == 1 {
        "Untitled".to_string()
    } else {
        format!("Untitled ({})", self.untitled_counter)
    };
    let document = Document::new_untitled(name);
    // ... rest same as create_document
}
```

`Document` needs a `display_name: Option<String>` field, or the name is embedded in `title()`.
The simplest approach: add `untitled_name: Option<String>` to `DocumentMetadata`; when
`file_path` is None, use `untitled_name` for display. When saved (Save As), `file_path` is set
and `untitled_name` is cleared.

### Pattern 4: Last-Used Directory Persistence

Store in a plain JSON/TOML file in the platform config directory:

```rust
// wgpu_client/src/adapter.rs (or a new config.rs):
fn last_dir_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("muda")
        .join("state.json")
}

fn load_last_dir() -> PathBuf {
    // Read JSON file, parse "last_dir" key
    // Fall back to dirs::home_dir() or "."
}

fn save_last_dir(path: &Path) {
    // Write {"last_dir": path.to_string_lossy()} to state.json
    // Create parent dir if missing
    // Ignore write errors silently
}
```

No extra library needed — `serde_json` is the obvious choice but a simple hand-rolled
`format!(r#"{{"last_dir": "{}"}}"#, path)` works for a single key. Alternatively, use `serde`
+ `serde_json` which is already common in Rust ecosystems.

### Pattern 5: Toast Notification Integration

The `Toast` infrastructure already exists in `ora/src/elements/toast.rs`:
- `Toast::show(message, ToastSeverity::Error)` — red, persistent (requires dismiss)
- `Toast::show(message, ToastSeverity::Warning)` — yellow, persistent
- `AUTO_DISMISS_DURATION = 4 seconds` for Info/Success

For Phase 12 file errors, use `ToastSeverity::Error` (red). The `Toast` is already wired into
`EditorRootView` via the render loop. The adapter needs a way to enqueue toast messages that the
render layer picks up. One approach: add `pending_toasts: Vec<(String, ToastSeverity)>` to
`CoreEditorAdapter`, and expose a `drain_toasts()` method called by the event loop or
`build_render_model`. Or attach it to `StatusPresentation.message` for now and upgrade later.

Check if `EditorRootView` already renders a `Toast` element and has a mechanism to push
notifications.

### Pattern 6: Deleted File Tracking

When a file is deleted externally while open, its `Document` remains valid (buffer is in memory).
The tab title should show `"(deleted)"` suffix.

Implementation: Add `file_deleted: bool` flag to `DocumentMetadata`. `title()` appends
`" (deleted)"` when set. Detection: when `Ctrl+S` is attempted on a file with a path, call
`std::fs::metadata(path)` first; if `NotFound`, set `file_deleted = true` and show toast
"File was deleted. Use Save As to save to a new location." Do not auto-detect on every render —
only check on explicit save.

### Anti-Patterns to Avoid

- **Running dialogs synchronously in `dispatch_command`**: This blocks the winit event loop for
  the duration of the dialog. The OS may mark the window as unresponsive.
- **Blocking file read on main thread**: Even a 1 MB file can take tens of milliseconds.
  Always use a background thread.
- **Calling `rfd` before the winit window exists**: On macOS, `AsyncFileDialog` requires
  `NSApplication`, which winit creates in `resumed()`. The adapter is created before the window;
  dialogs should only be opened from futures spawned after `resumed()`.
- **Using `rfd` from a non-main-thread on macOS**: The macOS backend requires the dialog to run
  on the main thread. The `LocalExecutor` pattern keeps everything on the main thread.
- **Multiple simultaneous dialogs**: Guard with a `dialog_open: bool` flag so rapid Ctrl+O
  presses don't open multiple dialogs.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Native file picker UI | Custom dialog widget | rfd | OS dialog handles accessibility, theming, keyboard nav, Recent files, network drives, etc. |
| Platform config dir | Hardcoded `~/.muda/` | dirs crate | Windows AppData, macOS Application Support, XDG on Linux |
| UTF-8 BOM detection | Custom byte inspection | stdlib `starts_with('\u{FEFF}')` | Trivial, no dep needed |
| Async runtime for dialogs | tokio, smol | async-executor (already in ora) | Already present; adding tokio adds ~10 deps |

**Key insight:** The rfd dialog is OS-native. No Rust-side file browser, filter rendering, or
keyboard event handling is needed. Just call the API and handle the result.

---

## Common Pitfalls

### Pitfall 1: macOS Dialog on Non-Main Thread

**What goes wrong:** On macOS, opening a file dialog from any thread other than the main thread
causes a crash or hang. If future work moves the executor off-main-thread, dialogs break silently
on macOS and work fine on Windows/Linux, leading to hard-to-reproduce bugs.

**Why it happens:** macOS AppKit requires UI operations on the main thread.

**How to avoid:** Always spawn file dialog futures via `cx.spawn()` (which uses the
`LocalExecutor` — main-thread only). Never `std::thread::spawn` the dialog itself.

**Warning signs:** macOS-only crashes or beach-balling when file operations are triggered.

### Pitfall 2: Dialog Blocks Event Loop on Windows/Linux if Called Synchronously

**What goes wrong:** `FileDialog::pick_files()` (the sync API) blocks the calling thread for the
duration of the dialog. If called on the event thread, the window freezes.

**Why it happens:** The sync API is designed for apps without event loops.

**How to avoid:** Always use `AsyncFileDialog` (the async API). It opens the dialog in a way
that doesn't block the calling task.

### Pitfall 3: Rapid Ctrl+O Opens Multiple Dialogs

**What goes wrong:** If the user presses Ctrl+O twice quickly, two dialogs open simultaneously.
The second dialog's result races with the first.

**How to avoid:** Set `dialog_open: bool = true` in `CoreEditorAdapter` when a dialog is
spawned; clear it when the future completes. In `dispatch_command`, skip OpenFile/SaveAs if
`dialog_open` is true.

### Pitfall 4: UTF-8 Error Message Leaks Technical Detail

**What goes wrong:** Showing `"Custom { kind: InvalidData, error: ... }"` to the user.

**Why it happens:** Formatting `io::Error` directly.

**How to avoid:** Match on `err.kind() == io::ErrorKind::InvalidData` and show the human-readable
message `"Cannot open: file is not valid UTF-8"`.

### Pitfall 5: Untitled Counter Not Reset Correctly

**What goes wrong:** Opening app, creating 3 untitled docs, closing them all, creating a new one
— it shows "Untitled (4)" instead of "Untitled".

**Decision:** The counter is monotonically increasing within a session. Do NOT reset it when
untitled docs are closed. This is simpler and avoids confusion (VS Code and most editors behave
this way).

### Pitfall 6: Save Failure Leaves Document Marked Clean

**What goes wrong:** `doc.save()` fails (disk full, permission denied), but dirty flag was
cleared before the write.

**How to avoid:** The current `Document::save()` implementation only sets `dirty = false` after
`std::fs::write()` succeeds. This is already correct. The adapter just needs to show a toast on
`Err`.

### Pitfall 7: rfd 0.15 Feature Flags

**What goes wrong:** Using `default-features = true` pulls in `async-std`, which conflicts with
existing build or adds unnecessary weight.

**How to avoid:** Use `default-features = false, features = ["xdg-portal"]` on Linux. Windows
and macOS backends are always compiled in on their respective platforms regardless of features.

---

## Code Examples

Verified patterns from official sources and codebase inspection:

### rfd AsyncFileDialog — Open Multiple Files (no filter)

```rust
// Source: https://docs.rs/rfd/latest/rfd/struct.AsyncFileDialog.html
use rfd::AsyncFileDialog;

let handles = AsyncFileDialog::new()
    .set_directory(&last_dir)  // PathBuf
    .pick_files()              // multi-select; None if cancelled
    .await;

if let Some(handles) = handles {
    for handle in handles {
        let path: &std::path::Path = handle.path();
        // path is the full absolute path
    }
}
```

### rfd AsyncFileDialog — Save As

```rust
// Source: https://docs.rs/rfd/latest/rfd/struct.AsyncFileDialog.html
let handle = AsyncFileDialog::new()
    .set_directory(&last_dir)
    .set_file_name("")         // empty = user types name from scratch
    .save_file()               // None if cancelled
    .await;

if let Some(handle) = handle {
    let path = handle.path().to_path_buf();
    // write to path
}
```

### FileHandle::path() — Native Path Access

```rust
// Source: https://docs.rs/rfd/latest/rfd/struct.FileHandle.html
// path() is NOT available on WASM32, but this project only targets native.
let path: &std::path::Path = file_handle.path();
```

### UTF-8 Validation and BOM Strip

```rust
// Source: stdlib + codebase pattern
fn read_text_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;
    // Strip UTF-8 BOM (EF BB BF = U+FEFF)
    if text.starts_with('\u{FEFF}') {
        Ok(text['\u{FEFF}'.len_utf8()..].to_string())
    } else {
        Ok(text)
    }
}
```

### Untitled Naming Counter

```rust
// Source: codebase pattern — new field on Workspace
pub fn create_untitled_document(&mut self) -> DocumentId {
    self.untitled_counter += 1;
    let name = if self.untitled_counter == 1 {
        "Untitled".to_string()
    } else {
        format!("Untitled ({})", self.untitled_counter)
    };
    let doc = Document::new_with_name(name);
    let doc_id = doc.id();
    self.documents.insert(doc_id, doc);
    self.histories.insert(doc_id, CommandHistory::new());
    doc_id
}
```

### Persisting Last Directory (simple approach)

```rust
// Source: dirs crate docs + stdlib
fn config_state_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("muda")
        .join("state.json")
}

fn load_last_dir() -> std::path::PathBuf {
    let path = config_state_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| {
            // Simple JSON extraction: find "last_dir": "..."
            let key = r#""last_dir": ""#;
            let start = s.find(key)? + key.len();
            let end = s[start..].find('"')? + start;
            Some(std::path::PathBuf::from(&s[start..end]))
        })
        .filter(|p| p.exists())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
}

fn save_last_dir(dir: &std::path::Path) {
    let path = config_state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = format!(r#"{{"last_dir": "{}"}}"#,
        dir.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\""));
    let _ = std::fs::write(&path, content);
}
```

### PendingFileOp Queue in Adapter

```rust
// Source: codebase pattern
// In CoreEditorAdapter:
#[derive(Debug)]
pub enum PendingFileOp {
    Open,
    SaveAs,
    New,
}

pub pending_file_op: Option<PendingFileOp>,

pub fn take_pending_file_op(&mut self) -> Option<PendingFileOp> {
    self.pending_file_op.take()
}

// In dispatch_command:
EditorCommand::OpenFile => {
    if !self.dialog_open {
        self.pending_file_op = Some(PendingFileOp::Open);
    }
}
```

### Event Loop Spawning Dialog Future

```rust
// Source: codebase pattern (event_loop.rs + context.rs)
// After dispatch_command returns, in the event loop:
if let Some(op) = adapter.borrow_mut().take_pending_file_op() {
    let adapter_clone = Rc::clone(&self.editor_adapter.as_ref().unwrap());
    let window = self.gpu_state.as_ref().unwrap().window.clone();
    self.app_context.spawn(async move {
        // ... dialog + file read logic
        adapter_clone.borrow_mut().handle_file_loaded(path, content);
        window.request_redraw();
    });
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| rfd with tokio/async-std features | rfd 0.15+ uses xdg-portal + pollster, no runtime dep | rfd 0.15 (2023) | Simpler feature flags; default is xdg-portal on Linux |
| Blocking file dialog on main thread | AsyncFileDialog with executor | Modern pattern | Non-blocking UI |
| Single-threaded file I/O inline | Background thread + channel | Best practice | UI stays responsive |

**Deprecated/outdated:**
- `FileDialog::pick_file()` (sync): Blocks the event loop. Use `AsyncFileDialog::pick_files().await`.
- rfd 0.14 tokio feature: rfd 0.15 removed runtime coupling; use `default-features = false`.

---

## Open Questions

1. **Where does `AppContext::spawn()` get called from in the event loop?**
   - What we know: `OraApp` has `app_context: AppContext` and calls `tick_executor()` after every
     event. The future needs access to `SharedAdapter` and `Arc<Window>`.
   - What's unclear: `app_context.spawn()` requires `&self` (takes a `Future`). The future
     captures `Rc<RefCell<...>>` which is `!Send`. This is fine for `LocalExecutor` but must
     be verified — `async_executor::LocalExecutor::spawn` likely requires `'static` but NOT
     `Send`.
   - Recommendation: Verify `async_executor::Task` bounds before writing the spawn code.
     `LocalExecutor::spawn` signature: `fn spawn<F>(&self, future: F) -> Task<F::Output> where F: Future + 'static`.
     `Rc` is `'static` as long as it doesn't hold references with shorter lifetimes. ✓

2. **Does EditorRootView already integrate the `Toast` element for display?**
   - What we know: `Toast` struct exists in `ora/src/elements/toast.rs` and is fully
     implemented. `ToastSeverity::Warning`/`Error` are persistent.
   - What's unclear: Whether `EditorRootView` renders a `Toast` instance and exposes a way
     to push notifications from outside the view.
   - Recommendation: Read `ora/src/views/editor_root.rs` before planning task that adds
     toast display. If Toast is not yet integrated, add it as a field on `EditorRootView`
     with a method `push_toast(msg, severity)`.

3. **Empty untitled buffer close — how to detect "no changes"?**
   - What we know: `Document::is_dirty()` returns `false` for a freshly-created document.
   - What's unclear: Whether a newly-created untitled document with no typing should close
     silently. The context says "Closing an empty untitled buffer (no changes) closes silently
     with no prompt."
   - The dirty flag starts false on `Document::new()`. A new empty Untitled doc is not dirty.
   - Recommendation: In `close_document_protected`, add check: if doc has no path AND is not
     dirty AND has zero chars, close silently (bypass dirty dialog). This handles the empty
     untitled case without changing the general dirty-protection logic.

---

## What Already Exists (Do Not Rebuild)

| Component | Location | Status |
|-----------|----------|--------|
| `EditorCommand::OpenFile/SaveAs/New/Save` variants | `ora/src/editor_adapter/types.rs` | Complete |
| Ctrl+O/S/Shift+S/N key bindings | `ora/src/events/editor_input.rs` | Complete |
| Stub handlers returning "not yet available" | `wgpu_client/src/adapter.rs:383-397` | Stubs |
| `Document::save()` / `Document::save_as()` | `core_editor/src/domain/document.rs` | Complete |
| `App::save()` / `App::save_as()` | `core_editor/src/app.rs` | Complete |
| `Workspace::open_document()` (with Buffer Registry dedup) | `core_editor/src/domain/workspace.rs` | Complete |
| `Toast` / `ToastSeverity` element | `ora/src/elements/toast.rs` | Complete |
| `AppContext::spawn()` / `tick_executor()` | `ora/src/context.rs` | Complete |
| `SharedAdapter` (`Rc<RefCell<Box<dyn EditorDataSource>>>`) | `ora/src/app.rs` | Complete |
| `DocumentMetadata.dirty` tracking | `core_editor/src/domain/document.rs` | Complete |
| Dirty tab indicator (`is_dirty` in `TabPresentation`) | `ora/src/editor_adapter/types.rs` | Complete |
| `PendingAction` / dirty-close dialog | `core_editor/src/app.rs` | Complete |

---

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection: `wgpu_client/src/adapter.rs`, `ora/src/platform/event_loop.rs`,
  `ora/src/context.rs`, `core_editor/src/app.rs`, `core_editor/src/domain/workspace.rs`,
  `core_editor/src/domain/document.rs`, `ora/src/elements/toast.rs`,
  `ora/src/editor_adapter/mod.rs`, `ora/src/editor_adapter/types.rs`
- https://docs.rs/rfd/latest/rfd/struct.AsyncFileDialog.html — API verified
- https://docs.rs/rfd/latest/rfd/struct.FileHandle.html — path(), file_name(), read() verified
- https://docs.rs/crate/rfd/latest/source/Cargo.toml.orig — confirmed rfd 0.17.2 exists (use 0.15 for stable feature flags)

### Secondary (MEDIUM confidence)
- https://github.com/rust-windowing/winit/issues/3179 — macOS rfd freeze issue; fixed in
  winit 0.29.3+. This project uses winit 0.30 (confirmed in ora/Cargo.toml) — fix is included.
- https://lib.rs/crates/rfd — platform support table and feature flags summary

### Tertiary (LOW confidence)
- WebSearch results confirming `std::sync::mpsc` + winit event loop pattern for async dialog
  result delivery

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — rfd docs verified, async-executor confirmed in codebase
- Architecture: HIGH — all touch points directly inspected in source
- Pitfalls: HIGH — macOS issue documented in winit tracker; others from source analysis

**Research date:** 2026-03-26
**Valid until:** 2026-06-26 (rfd is stable; winit 0.30 is current)
