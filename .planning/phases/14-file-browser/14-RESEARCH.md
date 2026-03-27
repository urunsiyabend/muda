# Phase 14: File Browser - Research

**Researched:** 2026-03-27
**Domain:** Rust filesystem I/O, filesystem watching, state persistence, winit cross-thread wakeup
**Confidence:** HIGH

## Summary

Phase 14 wires real filesystem data into an already-built UI shell. The `FileTreeView`, `SidebarView`, and `FileTreeNode` presentation types are complete from Phase 8/13.1. The `Sidebar` struct in `core_editor::view::sidebar` already reads real directories via `std::fs::read_dir` and implements `toggle_dir`, `build_tree`, and `set_base_directory`. The `CoreEditorAdapter` already handles `OpenSidebarFile` and `ToggleSidebarDir` commands. What is NOT yet done: (1) dotfile filtering is broken — `.git/` is currently filtered with ALL dotfiles, but the decision says only `.git/` hidden; (2) no filesystem watcher; (3) no Open Folder dialog command; (4) no `ignored_patterns` in state.json; (5) no workspace-path persistence; (6) no first-level auto-expand on open; (7) no deleted-file tab marker; (8) no external-modification reload.

The key new dependency is `notify` (v8.2.0) with `notify-debouncer-full` (v0.7.0) for filesystem watching. The watcher runs on a background thread and sends events to a `std::sync::mpsc::channel`. The event loop polls this channel via `mpsc::Receiver::try_recv()` in `about_to_wait`, and calls `window.request_redraw()` after marking `tree_dirty = true` on the sidebar.

**Primary recommendation:** Add `notify-debouncer-full = "0.7"` to `core_editor/Cargo.toml`, hold the `Debouncer` in `CoreEditorAdapter`, poll the mpsc channel in `about_to_wait`, and call `app.sidebar.mark_tree_dirty()` + `request_redraw()` when events arrive.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| notify | 8.2.0 | Cross-platform filesystem events | Used by cargo-watch, rust-analyzer, deno, alacritty |
| notify-debouncer-full | 0.7.0 | Debounced + deduped events with rename matching | Handles rapid-fire saves, avoids duplicate events |
| std::fs::read_dir | stdlib | Directory listing | Already used in `Sidebar::read_dir_sorted` |
| std::sync::mpsc | stdlib | Channel from watcher thread to main thread | Already used in Phase 12 file loading pattern |
| rfd 0.15 | already in deps | Folder picker dialog | `AsyncFileDialog::pick_folder()` supported |
| dirs 5 | already in deps | Platform config path | Already used in `config_state_path()` |

### Already In Place (No New Deps Needed)
| Component | Location | Status |
|-----------|----------|--------|
| `FileTreeView` | `ora/src/views/file_tree.rs` | Complete, virtualized |
| `SidebarView` | `ora/src/views/sidebar.rs` | Complete |
| `FileTreeNode` / `FileTreePresentation` | `ora/src/editor_adapter/types.rs` | Complete |
| `Sidebar` struct | `core_editor/src/view/sidebar.rs` | Complete — `read_dir_sorted`, `build_tree`, `toggle_dir`, `set_base_directory` |
| `OpenSidebarFile` command | `wgpu_client/src/adapter.rs:643` | Wired |
| `ToggleSidebarDir` command | `wgpu_client/src/adapter.rs:652` | Wired |
| `config_state_path()` + `load_last_dir()` + `save_last_dir()` | `wgpu_client/src/adapter.rs` | Exists but only saves `last_dir` |
| `rfd::AsyncFileDialog::pick_folder()` | rfd 0.15 | Available |

### New Dependencies Required
```toml
# core_editor/Cargo.toml (for the watcher — lives closest to Sidebar)
# OR wgpu_client/Cargo.toml (for the adapter wrapper)
notify-debouncer-full = "0.7"
```

The watcher should live in `CoreEditorAdapter` (wgpu_client) because:
- It needs access to `Arc<Window>` for `request_redraw()`
- It owns the mpsc receiver used for polling
- core_editor should stay free of platform/OS dependencies

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| notify-debouncer-full | notify-debouncer-mini | mini doesn't deduplicate events or handle renames; full is worth the minor complexity |
| notify-debouncer-full | notify (raw) | Raw requires manual debouncing — don't hand-roll |
| mpsc polling in about_to_wait | EventLoopProxy UserEvent | Proxy approach is more architecturally clean but requires threading winit EventLoop type through more code. mpsc polling matches the existing PendingFileOp pattern exactly |

## Architecture Patterns

### Current Execution Flow (what already works)
```
wgpu_client/main.rs
  → CoreEditorAdapter::new() / open_directory()
  → ora::run_with_editor(adapter)
    → OraApp event loop
      → WindowEvent::KeyboardInput → dispatch_command(ToggleSidebarDir / OpenSidebarFile)
        → adapter.rs: app.sidebar.toggle_dir(path)
        → adapter.rs: app.request_open_file(path)
      → WindowEvent::RedrawRequested
        → EditorRootView::render() → SidebarView.render() → FileTreeView.render()
        → FileTreeView reads FileTreePresentation from build_render_model()
        → build_render_model() calls sidebar.build_tree() (returns cached tree)
```

### New Execution Flow (what Phase 14 adds)
```
CoreEditorAdapter::new_with_watcher(workspace_path)
  → notify-debouncer-full watcher on background thread
  → sends events to mpsc::Sender<WatcherEvent>
  → CoreEditorAdapter holds mpsc::Receiver<WatcherEvent>

OraApp::about_to_wait()  [modified]
  → poll adapter.take_watcher_events()  (try_recv loop)
  → if events: adapter.app.sidebar.mark_tree_dirty()
  → request_redraw()

Open Folder dialog (new):
  spawn_open_folder_dialog()  [new in OraApp]
  → rfd::AsyncFileDialog::pick_folder().await
  → adapter.handle_folder_opened(path)
    → app.sidebar.set_base_directory(path)
    → save workspace_path to state.json
    → close dirty tabs (prompt)
    → tree_dirty = true

External file deleted:
  watcher fires Remove event for path P
  → adapter.handle_file_removed(P)
    → mark tab with path P as "deleted" (new TabPresentation.is_deleted field)
    → sidebar.mark_tree_dirty()

External file modified:
  watcher fires Modify event for path P
  → adapter.handle_file_modified(P)
    → if buffer for P is not dirty: reload content
    → if buffer for P is dirty: set pending_reload_prompt flag
```

### State.json Schema Extension
Current state.json only holds `last_dir`. Phase 14 needs:
```json
{
  "last_dir": "/path/to/last/open/save/dir",
  "workspace_path": "/path/to/last/opened/folder",
  "ignored_patterns": [".git"]
}
```
- `workspace_path`: restored on startup to reopen last workspace
- `ignored_patterns`: array of directory names to hide (default `[".git"]`)

### Filtering Logic Fix
Current `read_dir_sorted` in `core_editor/src/view/sidebar.rs:127` filters ALL dotfiles:
```rust
if name.starts_with('.') {
    return None;  // BUG: hides .env, .rustfmt.toml, etc.
}
```
Decision says only `.git/` is hidden. Fix:
```rust
// Only hide .git/ by default; other dotfiles visible
if name == ".git" {
    return None;
}
// Check ignored_patterns from config
if ignored_patterns.contains(&name.as_str()) {
    return None;
}
```
`ignored_patterns` is loaded from state.json and passed into `Sidebar` at creation/refresh time.

### First-Level Auto-Expand
When `set_base_directory` is called (either at startup or via Open Folder), expand root children:
```rust
pub fn auto_expand_first_level(&mut self) {
    // Read root entries, add each directory to expanded_dirs
    if let Some(ref base) = self.base_directory {
        let entries = Self::read_dir_sorted(base, &self.ignored_patterns);
        for entry in &entries {
            if entry.is_dir {
                self.expanded_dirs.insert(entry.path.clone());
            }
        }
        self.tree_dirty = true;
    }
}
```

### Filesystem Watcher Integration Pattern
```rust
// In CoreEditorAdapter — init
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::sync::mpsc;

let (tx, rx) = mpsc::channel::<DebounceEventResult>();
let mut debouncer = new_debouncer(
    Duration::from_millis(300), // 300ms debounce — within 200-500ms range
    None,                        // no tick_rate override
    move |res: DebounceEventResult| {
        let _ = tx.send(res);
    },
).expect("watcher init failed");

// Watch the workspace root recursively
debouncer.watcher().watch(&workspace_path, RecursiveMode::Recursive)?;
// Store debouncer (drops = stops watching)
self.watcher = Some(debouncer);
self.watcher_rx = Some(rx);
```

```rust
// In OraApp::about_to_wait — poll the channel
fn poll_watcher_events(&mut self) {
    let adapter = match &self.editor_adapter { Some(a) => a, None => return };
    // Non-blocking drain of all pending events
    while let Some(result) = adapter.borrow_mut().take_watcher_event() {
        match result {
            Ok(events) => {
                // Mark sidebar tree as needing rebuild
                adapter.borrow_mut().handle_watcher_events(events);
                self.needs_layout = true;
                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
                }
            }
            Err(e) => log::warn!("Watcher error: {:?}", e),
        }
    }
}
```

### Open Folder Dialog Pattern (new PendingFileOp variant)
```rust
// In types.rs
pub enum PendingFileOp {
    Open,
    SaveAs,
    Save,
    OpenFolder,  // NEW
}

// In OraApp::poll_pending_file_ops():
PendingFileOp::OpenFolder => self.spawn_open_folder_dialog(),

// spawn_open_folder_dialog():
self.app_context.spawn(async move {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Open Folder")
        .pick_folder()
        .await;
    if let Some(handle) = handle {
        adapter.borrow_mut().handle_folder_opened(handle.path().to_path_buf());
    }
    window.request_redraw();
}).detach();
```

### Anti-Patterns to Avoid
- **Calling build_tree() in the render path**: Already avoided — `Sidebar` has a `tree_dirty` flag and `build_tree()` only rebuilds when dirty. The render model builder calls `sidebar.build_tree()` once per frame but it's a cache.
- **Watching before workspace opens**: Only start watcher when `set_base_directory` is called. Don't watch when `base_directory` is `None`.
- **Blocking the main thread**: All file I/O in the watcher runs on its own thread. The main thread only calls `try_recv()` which never blocks.
- **Dropping the Debouncer**: The `Debouncer` guard must be stored (in `CoreEditorAdapter`) for the lifetime of the watch. Dropping it stops watching.
- **Re-creating watcher on every tree refresh**: Create once, re-use. Only recreate when workspace root changes.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Filesystem event debouncing | Custom timer + batching | notify-debouncer-full | cargo build fires hundreds of Create/Modify/Remove events in milliseconds; hand-rolled debouncing will miss events or coalesce wrong ones |
| Cross-platform FS events | Custom inotify/FSEvents/ReadDirectoryChangesW | notify 8.2.0 | Each OS API has edge cases (inotify fd limits, FSEvents security model, NTFS change journal); notify handles all platforms |
| Folder picker dialog | Custom dialog widget | rfd 0.15 `pick_folder()` | Already a dependency; pick_folder() is one line |
| JSON parsing for state.json | Full serde dependency | Extend existing hand-rolled parser | state.json is a flat 3-key object; adding two fields to the existing text parser is simpler than adding serde as a dep to wgpu_client |

**Key insight:** The filesystem watching domain has significant per-platform complexity. notify's debouncer-full also handles rename matching (From+To event correlation) and duplicate suppression that are non-trivial to replicate.

## Common Pitfalls

### Pitfall 1: Dotfile Filtering Bug
**What goes wrong:** Current `read_dir_sorted` hides ALL filenames starting with `.`. This means `.env`, `.rustfmt.toml`, `.gitignore`, etc. are invisible. Decision says only `.git/` hidden.
**Why it happens:** Phase 8 used a simple `starts_with('.')` guard.
**How to avoid:** Replace with explicit `.git` check + `ignored_patterns` lookup. Apply filter in `read_dir_sorted` which receives `ignored_patterns: &[&str]` parameter.
**Warning signs:** `.env` file not showing in sidebar when workspace is open.

### Pitfall 2: Watcher Thread Races on Windows
**What goes wrong:** On Windows, `notify` uses ReadDirectoryChangesW which can emit events for files still being written (e.g., during `cargo build`). Auto-reloading a file that's mid-write corrupts the buffer.
**Why it happens:** Write events fire before the file handle is released.
**How to avoid:** `notify-debouncer-full` with 300ms debounce handles this — by the time the event arrives, the write is complete. Do not use the raw `notify` crate for auto-reload.
**Warning signs:** Garbled file content in editor after cargo build.

### Pitfall 3: `expanded_dirs` Uses PathBuf Equality
**What goes wrong:** `expanded_dirs` is a `HashSet<PathBuf>`. On case-insensitive filesystems (Windows, macOS), paths `/Foo/Bar` and `/foo/bar` are the same directory but different `PathBuf`s and therefore different keys.
**Why it happens:** `PathBuf` equality is byte-level, not filesystem-level.
**How to avoid:** Always canonicalize paths before inserting into `expanded_dirs`. `std::fs::canonicalize` normalizes case on Windows/macOS. The existing `App::open_directory` already canonicalizes.
**Warning signs:** Expanding a directory on Windows doesn't toggle, or shows it as both expanded and collapsed.

### Pitfall 4: Watcher Invalidated After Workspace Change
**What goes wrong:** If the user opens a new folder via Open Folder dialog, the old watcher is still watching the old path and the new watcher isn't started.
**Why it happens:** Watcher is set up once in `new()` pointing at initial path.
**How to avoid:** In `handle_folder_opened(path)`, stop the old watcher (drop it) and create a new one watching `path`. Pattern: `self.watcher = None; self.watcher = Some(new_debouncer(...))`.
**Warning signs:** After opening a second folder, sidebar stops reflecting external changes.

### Pitfall 5: Collapsed-dir Children Not Rebuilt After External Change
**What goes wrong:** User expands folder A, external process creates file in A, watcher fires, tree rebuilt — but the new file appears because `build_tree_recursive` re-reads disk. This is correct behavior. However, if the user then collapses A and a file is deleted inside A, the deletion event marks tree_dirty=true but the deleted file won't be visible anyway (collapsed). No action needed for invisible paths — this is correct.
**Why it happens:** Not a pitfall, but a common confusion point during implementation.
**How to avoid:** Just mark `tree_dirty = true` for all events; let `build_tree_recursive` re-read the directory. The collapsed/expanded state is in `expanded_dirs` which is not affected by file changes.
**Warning signs:** None — this works correctly by design.

### Pitfall 6: state.json Write Breaks Existing `last_dir` Key
**What goes wrong:** Current `save_last_dir()` writes `{"last_dir": "..."}` as the entire JSON. If `save_workspace_path()` is written separately, the two calls will overwrite each other.
**Why it happens:** Each function writes the entire JSON blob.
**How to avoid:** Create a single `save_state(last_dir, workspace_path, ignored_patterns)` function that writes all fields atomically. Replace the existing `save_last_dir` call sites.
**Warning signs:** `last_dir` or `workspace_path` not persisting between sessions.

### Pitfall 7: `build_tree` Called on Every Frame Touching filesystem
**What goes wrong:** `build_tree_recursive` calls `read_dir_sorted` recursively for every expanded directory on every frame, causing hundreds of `fs::read_dir` calls per second.
**Why it happens:** If `tree_dirty` is incorrectly set to true by something other than actual changes.
**How to avoid:** Only set `tree_dirty = true` in `toggle_dir`, `set_base_directory`, `refresh_entries`, and the new watcher event handler. Never set it in the render path or `build_render_model`. The existing implementation already follows this pattern.
**Warning signs:** Disk I/O visible in task manager when the editor is idle.

## Code Examples

Verified patterns from project source and official docs:

### Notify-Debouncer-Full: Create and Poll
```rust
// Source: https://docs.rs/notify-debouncer-full/0.7.0/notify_debouncer_full/
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use notify::RecursiveMode;
use std::sync::mpsc;
use std::time::Duration;

let (tx, rx) = mpsc::channel::<DebounceEventResult>();
let mut debouncer = new_debouncer(
    Duration::from_millis(300),
    None, // tick_rate — None = use default (50ms)
    move |res: DebounceEventResult| {
        let _ = tx.send(res);
    },
).expect("failed to create watcher");

debouncer.watcher().watch(
    std::path::Path::new("/path/to/watch"),
    RecursiveMode::Recursive,
).expect("failed to watch path");

// Poll in the event loop (non-blocking):
while let Ok(result) = rx.try_recv() {
    match result {
        Ok(events) => { /* events: Vec<DebouncedEvent> */ }
        Err(errors) => { /* Vec<notify::Error> */ }
    }
}
```

### rfd Async Folder Picker
```rust
// Source: https://docs.rs/rfd/0.15.1/rfd/struct.AsyncFileDialog.html
// Already available in ora's rfd 0.15 dependency
let handle = rfd::AsyncFileDialog::new()
    .set_title("Open Folder")
    .pick_folder()
    .await;
if let Some(handle) = handle {
    let path = handle.path().to_path_buf();
    // handle.path() returns &Path
}
```

### Path Canonicalization for expanded_dirs
```rust
// Source: Rust stdlib std::fs::canonicalize
// Already used in App::open_directory (core_editor/src/app.rs:104)
let canonical_path = std::fs::canonicalize(&path_buf)?;
// canonical_path is absolute and resolves symlinks
// On Windows: uses UNC paths (\\?\C:\...) — PathBuf handles this
```

### State.json Extension (no new deps)
```rust
// Extend existing pattern in wgpu_client/src/adapter.rs
// load_state() replaces load_last_dir():
fn load_state() -> AppState {
    let state_path = config_state_path();
    let text = std::fs::read_to_string(&state_path).unwrap_or_default();
    AppState {
        last_dir: extract_string_field(&text, "last_dir")
            .and_then(|p| { let pb = PathBuf::from(&p); pb.exists().then_some(pb) })
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default()),
        workspace_path: extract_string_field(&text, "workspace_path")
            .map(PathBuf::from)
            .filter(|p| p.exists()),
        ignored_patterns: extract_array_field(&text, "ignored_patterns")
            .unwrap_or_else(|| vec![".git".to_string()]),
    }
}

// save_state() replaces save_last_dir():
fn save_state(state: &AppState) {
    let escaped_last = escape_json_string(&state.last_dir.to_string_lossy());
    let escaped_ws = state.workspace_path.as_ref()
        .map(|p| format!("\"{}\"", escape_json_string(&p.to_string_lossy())))
        .unwrap_or_else(|| "null".to_string());
    let patterns_json = state.ignored_patterns.iter()
        .map(|p| format!("\"{}\"", escape_json_string(p)))
        .collect::<Vec<_>>()
        .join(", ");
    let json = format!(
        "{{\"last_dir\": \"{}\", \"workspace_path\": {}, \"ignored_patterns\": [{}]}}",
        escaped_last, escaped_ws, patterns_json
    );
    let _ = std::fs::write(config_state_path(), json);
}
```

### Sidebar Header: Workspace Root Name
```rust
// SidebarView.render_header() needs workspace name
// Currently uses SidebarPresentation.directory_name
// Already set in ViewModelBuilder::build_sidebar_presentation:
let directory_name = sidebar.base_directory
    .as_ref()
    .and_then(|p| p.file_name())
    .and_then(|n| n.to_str())
    .unwrap_or("Files")
    .to_string();
// This is already correct — just needs to be rendered in the header
// (currently renders "EXPLORER"; should render directory_name)
```

### "Open Folder" Button in Sidebar Empty State
```rust
// SidebarView::render_content() empty state needs a button
// Currently renders: TextElement::new("No folder open")
// Phase 14 should add a button that dispatches EditorCommand::OpenFolder
// The dispatch callback already exists in SidebarView.dispatch
```

### Tab "Deleted" Indicator
```rust
// TabPresentation needs a new field
pub struct TabPresentation {
    pub view_id: u64,
    pub title: String,
    pub is_active: bool,
    pub is_dirty: bool,
    pub is_deleted: bool,  // NEW: file deleted externally
}
// Rendered in tab_bar.rs with a strikethrough or color indicator
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual FS polling (while true + sleep) | Event-based notify watcher | notify crate adoption | No CPU overhead when filesystem is idle |
| notify 5.x raw API | notify 8.x + debouncer crates | ~2023 | Debouncing is now a separate crate; simpler API |
| Monolithic `EditorDataSource` | Sub-traits (BufferDataSource, etc.) | Phase 10 | File watcher callbacks go in `FileOpDataSource` or a new `WorkspaceDataSource` |

**Current state of dotfile handling:** `Sidebar::read_dir_sorted` at line 127 hides ALL dotfiles. This must be fixed in Phase 14 to match the decision (only `.git/` hidden).

**Current state of first-level expansion:** `Sidebar::new` calls `refresh_entries()` but never calls `auto_expand_first_level`. The first level is always collapsed on startup.

**Current state of ignore list:** The `is_generated` field in `FileTreeNode` marks `target`, `node_modules`, etc. but this does not hide them from display — it only signals to the UI that they could be styled differently. Phase 14 changes this: `ignored_patterns` from state.json controls what is hidden entirely (default: only `.git`).

## Open Questions

1. **Where to hold the Debouncer**
   - What we know: `Debouncer` is a `!Send` struct (notify-debouncer-full's Debouncer holds Rc internals in some configurations), or it may be `Send`. Need to verify.
   - What's unclear: If `Debouncer` is `!Send`, it cannot be stored in `CoreEditorAdapter` which is behind `Rc<RefCell<>>` on the main thread. But since everything is main-thread only, this should be fine.
   - Recommendation: Store in `CoreEditorAdapter`. The mpsc channel is `Send`; only the receiver lives on the main thread.

2. **Reload prompt dialog for dirty buffers**
   - What we know: `DialogPresentation` exists with `UnsavedChangesConfirmation`. A new variant `ExternalModificationPrompt` is needed.
   - What's unclear: Whether to reuse the existing dialog component or add a new variant.
   - Recommendation: Add `ExternalModificationPrompt { path: String }` to `DialogPresentation`. Reuse the existing dialog view.

3. **notify-debouncer-full Send-ness**
   - What we know: The Debouncer guards the watcher thread. The mpsc::Receiver lives on main thread.
   - What's unclear: Whether `Debouncer<FileIdMap>` is `Send` or not in v0.7.0.
   - Recommendation: If not Send, keep the Debouncer in a thread_local or just in OraApp (not in CoreEditorAdapter). The rx (receiver) stays in CoreEditorAdapter.

## Sources

### Primary (HIGH confidence)
- Project source: `core_editor/src/view/sidebar.rs` — current Sidebar implementation (read fully)
- Project source: `wgpu_client/src/adapter.rs` — state.json pattern, PendingFileOp pattern (read fully)
- Project source: `ora/src/platform/event_loop.rs` — about_to_wait, spawn pattern, poll_pending_file_ops (read fully)
- Project source: `ora/src/views/file_tree.rs` — FileTreeView, virtualization (read fully)
- Project source: `ora/src/views/sidebar.rs` — SidebarView, dispatch pattern (read fully)
- https://docs.rs/notify/8.2.0/notify/ — notify 8.2.0 API
- https://docs.rs/notify-debouncer-full/0.7.0/notify_debouncer_full/ — debouncer-full 0.7.0 API
- https://docs.rs/rfd/0.15.1/rfd/struct.AsyncFileDialog.html — pick_folder() confirmed

### Secondary (MEDIUM confidence)
- https://docs.rs/notify-debouncer-mini/0.7.0/notify_debouncer_mini/ — mini vs full comparison
- WebSearch: winit 0.30 EventLoopProxy cross-thread wakeup pattern

### Tertiary (LOW confidence)
- None applicable

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies verified against official docs; existing deps confirmed in Cargo.toml
- Architecture: HIGH — based on direct code reading of the full stack; patterns are extensions of existing working code
- Pitfalls: HIGH — identified from direct code reading (dotfile bug confirmed at sidebar.rs:127); watcher thread issues from notify docs
- state.json extension: HIGH — direct code reading of existing hand-rolled JSON parser pattern

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (notify and rfd are stable; notify-debouncer-full 0.7.0 is recent)
