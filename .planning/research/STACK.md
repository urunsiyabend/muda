# Technology Stack: v2.0 Functional Editor Additions

**Project:** muda — Functional editor capabilities on top of existing ora UI framework
**Researched:** 2026-03-26
**Scope:** NEW dependencies only. Existing stack (wgpu 23, winit 0.30, glyphon 0.7, ora, core_editor) is
validated and not re-researched here.
**Overall Confidence:** HIGH

---

## Baseline Stack (Do Not Change)

The existing workspace already has these locked in Cargo.lock:

| Crate | Locked Version | Where |
|-------|----------------|-------|
| arboard | 3.6.1 | core_editor (direct dep) |
| regex | 1.12.2 | transitive (tree-sitter chain) |
| aho-corasick | 1.1.4 | transitive (via regex) |
| ropey | 1.6.1 | core_editor (direct dep) |
| tree-sitter | 0.26.3 | core_editor (direct dep) |

The key insight: **arboard is already a direct dependency of core_editor** (verified in Cargo.toml).
Clipboard for text selection and copy/cut/paste requires zero new dependencies.

---

## New Dependencies Required

### 1. Native File Dialogs — `rfd`

**Add to:** `wgpu_client/Cargo.toml`

```toml
rfd = "0.17"
pollster = "0.4"   # pollster lives in ora; needs explicit dep in wgpu_client binary too
```

**Current version:** 0.17.2 (released March 2026, confirmed via crates.io search)
**Confidence:** HIGH — confirmed via multiple sources including crates.io version listing

**Why rfd:**
- Cross-platform native file dialogs: uses Windows Common Item Dialog (IFileOpenDialog/IFileSaveDialog
  via COM), macOS NSSavePanel/NSOpenPanel, and GTK/XDG Portal on Linux.
- Zed does NOT use rfd. Zed implemented its own COM-based Windows dialog directly in GPUI
  (PR #8919: `windows: Add file dialog using IFileOpenDialog`). That approach required significant
  Windows-specific COM plumbing. rfd wraps exactly that COM API, already maintained upstream.
- Both sync (`FileDialog`) and async (`AsyncFileDialog`) APIs are provided.
- Supports filter lists, default directories, multiple selection.

**Why wgpu_client, not ora:**
ora is a framework crate. It has no business knowing about OS file dialogs. wgpu_client is the binary
that owns the winit event loop and the top-level application state. Dialog invocation lives there,
feeding results back to core_editor's workspace model.

**Critical Windows integration note:**
The sync `FileDialog::pick_file()` call blocks the calling thread. On Windows, COM pumps its own
message loop internally, so the application window freezes while the dialog is open. This is
technically acceptable for modal file open dialogs. However, the preferred approach is
`AsyncFileDialog` which spawns a COM thread internally.

Note on pollster: `pollster` is declared as a dep of `ora`, not `wgpu_client`. Since wgpu_client
depends on ora the binary already links it, but to call `pollster::block_on()` from wgpu_client code
you need it as an explicit direct dependency of wgpu_client. Add `pollster = "0.4"` to wgpu_client's
Cargo.toml alongside rfd. Do NOT add tokio for this purpose — it is overkill.

```rust
// wgpu_client: async approach using pollster (explicit dep of wgpu_client)
use rfd::AsyncFileDialog;

let future = AsyncFileDialog::new()
    .add_filter("All files", &["*"])
    .add_filter("Rust", &["rs"])
    .set_directory("/")
    .pick_file();

// pollster::block_on is fine here: rfd spawns its own COM thread,
// so this blocks only while waiting for the user to close the dialog.
let result = pollster::block_on(future);
if let Some(file) = result {
    let path = file.path().to_owned();
    // Send path to core_editor workspace via event/command
}
```

**What NOT to use:**
- `nfd` / `nfd2` — older alternatives, less maintained, Windows behavior is less reliable
- `egui_file` / `dear-file-browser` — custom UI file pickers, not native dialogs
- Rolling your own COM calls — Zed's approach is correct for a framework but wrong for an app
  that can just pull rfd

---

### 2. File System Watching — `notify` + `notify-debouncer-full`

**Add to:** `core_editor/Cargo.toml`

```toml
notify = "8"
notify-debouncer-full = "0.5"
```

**Current versions:** notify 8.2.0, notify-debouncer-full 0.5.0 (confirmed via search, March 2026)
**Confidence:** HIGH — confirmed used by Zed, rust-analyzer, deno, cargo-watch

**Why notify:**
- Cross-platform filesystem notification library, the de facto standard in the Rust ecosystem.
- Used by Zed, rust-analyzer, deno, alacritty, mdBook, watchexec.
- On Windows uses ReadDirectoryChangesW (RDCW), the correct kernel API for directory watching.
- `notify-debouncer-full` wraps the raw event stream to: (a) deduplicate rapid-fire events into
  single logical events, (b) stitch together rename FROM+TO pairs, (c) suppress redundant Remove
  events for directory trees.

**Why core_editor, not ora:**
File watching is workspace/document-model state. `core_editor/src/domain/workspace.rs` already owns
workspace lifecycle. The watcher belongs beside the workspace, not in the UI framework.

**Why debouncer, not raw notify:**
The sidebar file browser and buffer invalidation don't need every individual inotify/RDCW event.
They need "this file changed once." Debouncing with a 100-200ms timeout collapses burst writes
(save operations that write multiple times) into one event, preventing spurious reloads.

```rust
// core_editor: attach watcher to workspace
use notify_debouncer_full::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;

let (tx, rx) = std::sync::mpsc::channel();
let mut debouncer = new_debouncer(Duration::from_millis(200), None, move |res| {
    let _ = tx.send(res);
})?;
debouncer.watch(&workspace_root, RecursiveMode::Recursive)?;
```

**What NOT to use:**
- `hotwatch` — thin wrapper around notify with simpler API, but less control and less maintained
- Manual polling — do not poll directory mtimes on a timer; RDCW events are zero-latency

---

### 3. Clipboard — No New Dependency

**Existing:** `arboard = "3.6.1"` already in `core_editor/Cargo.toml`

arboard is a direct dependency, already integrated in `core_editor/src/commands/dispatcher.rs`:
- `Clipboard::new()` creates the system clipboard handle
- `get_text()` / `set_text()` for copy/cut/paste
- Version 3.6.1 added PNG preference on Windows and file list pasting

**Text selection (mouse drag, shift+arrow) is a UI concern, not a clipboard concern.** Selection state
is tracked in core_editor's text buffer / editor model. The clipboard is only touched at the moment of
copy/cut/paste. No new library needed.

**What NOT to add:**
- `clipboard` crate — older, less maintained, Windows behavior issues
- `clipboard-rs` — niche, less adoption
- GPUI-style custom clipboard: Zed found soundness issues in their Windows clipboard implementation
  (issue #29657, #51278). arboard handles this correctly.

---

### 4. Text Search / Regex — No New Dependency

**Existing in Cargo.lock:** `regex = "1.12.2"` (transitive), `aho-corasick = "1.1.4"` (transitive)

**Promote regex to explicit dependency in core_editor:**

```toml
# core_editor/Cargo.toml
regex = "1"
```

This makes the dependency intentional and version-tracked, but adds zero binary weight since it is
already present transitively through the tree-sitter dependency chain.

**Why regex is sufficient for find/replace:**
- Literal string search: `regex::Regex::new(&regex::escape(query))` — no aho-corasick needed
- Regex search: `Regex::new(pattern)` — direct
- Case-insensitive: `(?i)` prefix flag
- Incremental search (as-you-type): compile once per pattern change, search buffer on each keystroke

**Do NOT add aho-corasick as a direct dependency.**
aho-corasick provides multi-pattern search (finding many needles at once). For editor find/replace,
you have one pattern at a time. aho-corasick would only be justified for "find-in-files across all
open buffers simultaneously with many active patterns" — not a v2.0 requirement, and even then only
after profiling proves regex is the bottleneck.

**On ropey integration — chunk boundary pitfall:**
regex does not natively iterate `ropey::Rope`. A naive chunk-by-chunk iteration silently drops matches
that span chunk boundaries (a match starting in one chunk, ending in the next). This is a correctness
bug, not a performance issue.

The correct approach (what most production editors including Zed use) is to materialize the search
range to a contiguous `String`, search that, then map byte offsets back to rope positions:

```rust
// Correct: materialize the search range, search a contiguous string.
// Do NOT walk rope chunks: matches spanning chunk boundaries are silently dropped.
fn find_all(rope: &ropey::Rope, pattern: &regex::Regex) -> Vec<(usize, usize)> {
    // rope.to_string() for full buffer is safe: typical code files are well under 1MB.
    // For very large files, materialize only the visible range.
    let text = rope.to_string();
    pattern.find_iter(&text)
        .map(|m| (m.start(), m.end()))   // byte offsets in the materialized string
        .collect()
}

// Map byte offsets back to rope char positions when needed
let char_start = rope.byte_to_char(byte_start);
```

Chunk-walking is a future optimization only if profiling proves materialization is a bottleneck for
multi-megabyte files. For an editor with typical source files, it will not be.

---

### 5. Performance — No New Libraries

Dirty tracking, layout cache, and incremental rendering are architectural patterns over existing
primitives, not library problems.

- **Dirty tracking:** Add a `dirty: bool` flag to element/view state; skip layout recomputation
  when nothing changed. This is a pattern in the core rendering loop, not a crate.
- **Layout cache:** Cache computed `LayoutResult` per view behind a generation counter.
  Compare generation to invalidate. This is a data structure decision, not a library.
- **Incremental rendering:** Track which lines are visible (scroll offset + viewport height),
  render only those. ropey already supports `byte_to_line()` and `line_to_byte()` for constant-time
  line range queries.

**Do NOT add:**
- Any "reactive diffing" crate — they bring VDOM patterns inappropriate for immediate-mode GPU UI
- An async task scheduler — the existing `async-executor` in ora is sufficient for background tasks

---

## Summary: What Changes Per Crate

### wgpu_client/Cargo.toml — Add

```toml
rfd = "0.17"
pollster = "0.4"   # needed directly to call pollster::block_on in wgpu_client code
```

### core_editor/Cargo.toml — Add

```toml
notify = "8"
notify-debouncer-full = "0.5"
regex = "1"          # Promote from transitive to explicit
```

### ora/Cargo.toml — No changes

### Nothing Else

Total new crates: **3** (rfd, notify, notify-debouncer-full).
Total promoted to explicit: **2** (regex in core_editor; pollster in wgpu_client).
Clipboard: already done (arboard 3.6.1 in core_editor).
Performance: no library needed.

---

## Alternatives Considered

| Feature | Recommended | Alternative | Why Not |
|---------|-------------|-------------|---------|
| File dialogs | rfd 0.17 | nfd2 | nfd2 less maintained, Windows edge cases |
| File dialogs | rfd 0.17 | Custom COM (Zed approach) | Correct for a framework, wrong for an app binary |
| File watching | notify 8 | hotwatch | hotwatch is a thin wrapper, less control, less maintained |
| File watching | notify 8 | Manual mtime polling | Inferior latency and CPU cost vs kernel events |
| Clipboard | arboard (existing) | clipboard-rs | Less adoption, Windows issues reported |
| Text search | regex (existing) | aho-corasick (direct dep) | Overkill; single-pattern search doesn't need multi-pattern automaton |
| Rope search | Materialize to String | Chunk-walk with regex | Chunk-walking silently drops matches spanning chunk boundaries |
| Performance | Architectural patterns | External caches/diff libs | Patterns over data structures are sufficient; libs add complexity |

---

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| arboard (clipboard) | HIGH | Already in project, verified Cargo.toml, 3.6.1 locked |
| regex (text search) | HIGH | Already in Cargo.lock at 1.12.2, latest confirmed as 1.12.2 |
| rfd (file dialogs) | HIGH | crates.io confirmed 0.17.2 (March 2026); Windows COM backend verified via Zed PR #8919 context |
| notify (file watching) | HIGH | 8.2.0 confirmed, debouncer-full 0.5.0 confirmed, used by Zed and rust-analyzer |
| Performance (no libs) | HIGH | Pattern-based, verified against existing ora/core_editor architecture |
| Rope+regex integration | HIGH | Chunk boundary pitfall documented; String materialization is the standard approach used in production editors |

---

## Sources

- [rfd crates.io](https://crates.io/crates/rfd) — version 0.17.2, March 2026
- [rfd GitHub](https://github.com/PolyMeilex/rfd) — Windows COM backend details
- [Zed PR #8919: Windows IFileOpenDialog](https://github.com/zed-industries/zed/pull/8919) — Zed's custom COM approach (reference for why rfd is the right choice for an app)
- [notify crates.io](https://crates.io/crates/notify) — version 8.2.0 confirmed
- [notify GitHub](https://github.com/notify-rs/notify) — Windows RDCW backend, used by Zed/rust-analyzer
- [notify-debouncer-full docs.rs](https://docs.rs/notify-debouncer-full/latest/notify_debouncer_full/) — 0.5.0 API
- [arboard GitHub (1Password)](https://github.com/1Password/arboard) — 3.6.1 changelog (file list, Windows PNG preference)
- [arboard in project Cargo.toml](/c/Users/uruns/RustroverProjects/muda/core_editor/Cargo.toml) — confirmed direct dependency at 3.6.1
- [regex docs.rs](https://docs.rs/regex/latest/regex/) — 1.12.2 (2025-10-13 release), latest confirmed
- [aho-corasick 1.1.4 docs.rs](https://docs.rs/crate/aho-corasick/latest) — already transitive, confirmed not needed as direct dep
- [Zed clipboard soundness issue #29657](https://github.com/zed-industries/zed/issues/29657) — reason to prefer arboard over custom implementation
- [Zed clipboard issue #51278](https://github.com/zed-industries/zed/issues/51278) — Windows paste failures in custom clipboard
