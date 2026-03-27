# Domain Pitfalls: v2.0 Functional Editor Features

**Domain:** Adding file ops, file browser, multi-tab, find/replace, selection, clipboard, and
performance optimization to an existing GPU editor
**Researched:** 2026-03-26
**Confidence:** HIGH (grounded in actual codebase analysis — specific types, methods, and
architectural decisions are cited throughout)

---

## Preface: How to Read This Document

Each pitfall points to specific code in this codebase: file paths, struct names, method names. The
goal is to prevent specific bugs, not recite generic advice. Where a pitfall says "see
`workspace.rs` line 92", that line actually exists and contains the named risk.

---

## Critical Pitfalls

Mistakes that cause data loss, rewrites, or irreparable user-facing bugs.

---

### Pitfall 1: Synchronous File I/O Blocking the UI Thread

**What goes wrong:** `Document::open()` calls `std::fs::read_to_string(path)` synchronously. `Document::save()` calls `self.buffer.to_string()` (materializing the full rope into a heap string) then `std::fs::write()`. Both run on the UI thread inside an `EditorCommand` dispatch cycle. Opening a 5 MB file on a slow disk or network share freezes the window for hundreds of milliseconds. On Windows, antivirus scanning adds unpredictable latency to every file open.

**Why it happens:** The current single-document design never exposed this problem. `Document::open` in `core_editor/src/domain/document.rs` line 132 was written for correctness, not async.

**Consequences:**
- UI thread hangs — winit stops processing events, the window goes white on Windows
- No way to cancel an in-progress open/save
- Network-mounted directories (WSL, UNC paths) can stall for 30+ seconds
- Antivirus hooks make "fast" local SSDs behave unpredictably

**Prevention:**
- Move file I/O to a background thread using `std::thread::spawn` or `tokio::spawn`
- Dispatch `EditorCommand::Open(path)` to trigger async load; when complete, send result back via channel
- Show a "loading..." indicator while load is in flight
- Accept that `Document::open()` in core_editor stays synchronous — the async wrapper lives in wgpu_client or a new workspace service layer

**Warning signs:**
- Opening any file visibly lags the editor
- The OS "not responding" indicator appears on the title bar
- Files on mapped network drives freeze the process

**Phase:** File Operations phase. Must be async from day one — retrofitting sync to async touches every call site.

---

### Pitfall 2: Opening the Same File Twice Creates Silent Data Loss

**What goes wrong:** `Workspace::open_document()` (workspace.rs line 92) creates a new Document every time it is called, with a new `DocumentId`. If the user opens `main.rs` from the file browser while a tab for `main.rs` already exists, two independent `Document` instances exist in the workspace. The user edits in one tab. Saving the other tab (which has stale content from disk) overwrites the live edits. Data is lost with no warning.

**Why it happens:** `open_document` has no path-deduplication check. There is no index from `PathBuf` to `DocumentId`.

**Consequences:**
- Silent data loss when two tabs for the same file diverge and one is saved
- `undo_history` for each copy is independent, so undo after the overwrite produces unexpected results
- The file browser has no way to highlight that a file is "already open"

**Prevention:**
- Add a `path_to_doc: HashMap<PathBuf, DocumentId>` index to `Workspace`
- In `open_document`, check the index first: if found, activate the existing view instead of creating a new document
- Canonicalize paths before indexing (`std::fs::canonicalize`) to handle `./foo.rs` vs `foo.rs` vs symlinks
- When activating an existing tab, consider a "file may have changed on disk" check if time elapsed

**Warning signs:**
- Opening a file that is already in a tab creates a second tab with the same name
- Saving one instance silently overwrites what you typed in the other

**Phase:** File Operations phase. Must be in the first implementation before any file browser exists.

---

### Pitfall 3: `can_switch_active()` Blocks Tab Switching

**What goes wrong:** `Workspace::can_switch_active()` (workspace.rs line 460) returns a `ProtectionError::UnsavedChanges` if the active document is dirty. If the tab bar calls this before switching, every tab click is blocked when the user has unsaved edits. VS Code and Zed allow free switching between tabs regardless of dirty state — the dirty indicator in the tab title is sufficient.

**Why it happens:** `can_switch_active` was designed for a flow where switching documents requires confirmation, not for a tab bar model where dirty tabs are a normal, expected state.

**Consequences:**
- Tab clicks trigger an "unsaved changes" dialog instead of just switching
- Users cannot look at another file without saving or discarding edits
- The entire multi-tab UX is broken

**Prevention:**
- Do not call `can_switch_active()` during normal tab switching
- Reserve unsaved-change prompts for: close tab, close workspace, quit application
- In the ora tab bar click handler, call `workspace.set_active_view(view_id)` directly
- `can_switch_active()` may be removed or repurposed; document its intended use clearly

**Warning signs:**
- Every tab click shows "save or discard" dialog
- Users cannot switch between open files without saving

**Phase:** Multi-tab phase. Must be checked before implementing tab click handlers.

---

### Pitfall 4: Selection Rendering Bug Is in Span Generation, Not the Renderer

**What goes wrong:** The known bug — "Shift+arrow selection causes text to disappear" — is almost certainly not in the GPU renderer. Selection is represented as `TextStyle::Selection` on `StyledSpan` objects in `LinePresentation` (see `types.rs` line 25). The renderer simply colors those spans. When selection spans appear where visible text should be, the text "disappears" because it renders in selection-background color. The bug is in the `build_render_model` code that generates spans (in wgpu_client's adapter implementation), not in ora's rendering.

**Why it happens:** When selection overlaps a line, the code composing `TextStyle::Selection` spans may be replacing content spans instead of layering them, or the selection range is not correctly translated to the `VisualPosition` system used by `LinePresentation`.

**Consequences:**
- Debugging in the wrong layer (renderer) while the actual bug is in span composition
- Weeks lost if investigation starts at the GPU/rendering layer
- Mouse selection implementation will reproduce the same bug if the root cause is not understood

**Prevention:**
- Investigate by printing `visible_lines[i].spans` to stdout during a selection event — if the text content is missing from the spans, the bug is in the adapter
- The fix is in `wgpu_client`'s `build_render_model` implementation, specifically in how selection ranges intersect styled spans
- Consider switching from `TextStyle::Selection` in spans to a separate selection overlay (list of `(line, col_start, col_end)` rectangles drawn behind text). This is the Zed/VS Code approach — selection rendering is decoupled from text spans

**Warning signs:**
- Text disappears during selection but comes back when selection is cleared
- The gutter line numbers still show correctly during the "disappearance" (proves the renderer is fine)

**Phase:** Selection/Clipboard phase, but the existing bug should be fixed before that phase begins.

---

### Pitfall 5: Clipboard Requires Platform Access, Which Crosses the Adapter Boundary

**What goes wrong:** `EditorCommand::Copy/Cut/Paste` exist in both `core_editor` and the ora mirror types (`types.rs` line 432). The clipboard operation itself requires a platform API call — on Windows, `arboard` or `clipboard-win` needs a window handle or COM initialization. The `EditorDataSource` trait has no clipboard methods. The ora view dispatches `EditorCommand::Copy`, which flows to wgpu_client's dispatcher, which calls `workspace.dispatch_command(Copy)` — but at that point the clipboard content needs to come from the platform, not from core_editor.

**Why it happens:** The adapter boundary (ora views never import core_editor) was designed for rendering/commands, not bidirectional platform I/O. Clipboard is both output (write on copy/cut) and input (read on paste).

**Consequences:**
- Paste inserts nothing because no clipboard content was ever fetched
- Copy/Cut silently fail because nowhere in the pipeline initiates a platform clipboard write
- `arboard` initialization failures on Windows (COM not initialized) crash silently

**Prevention:**
- Handle clipboard at the wgpu_client dispatch layer, not inside core_editor
- On `Copy`: get text from `workspace.active_document()` using selection range, then call `arboard::Clipboard::new().set_text(text)` — all in wgpu_client
- On `Paste`: call `arboard::Clipboard::new().get_text()` first, then dispatch `EditorCommand::InsertText(text)`
- On Windows, call `arboard::Clipboard::new()` once and keep it alive — repeated construction has overhead
- Add `arboard` to `wgpu_client/Cargo.toml`, not to `core_editor` or `ora`

**Warning signs:**
- Ctrl+C appears to do nothing
- Ctrl+V inserts nothing or inserts from a previous session
- COM-related panic on Windows clipboard construction

**Phase:** Selection/Clipboard phase.

---

### Pitfall 6: Mouse Hit-Testing Requires Character Width Knowledge That Lives Behind the GPU

**What goes wrong:** Mouse text selection requires mapping pixel coordinates `(x, y)` to a text position `(line, column)`. Column calculation requires knowing the rendered width of each character, which depends on the font metrics managed by the `TextSystem` inside `GpuState`. The `EditorDataSource` trait has no hit-test method. `ora::views::TextAreaView` receives pixel events from winit but has no way to convert them to document offsets without touching the text system.

**Why it happens:** The current design is render-only. Events flow: `winit -> ora -> EditorCommand dispatch`. No return path for coordinate mapping exists.

**Consequences:**
- Mouse click-to-position works only for monospace text at fixed `char_width`
- Click position is off by several characters for non-monospace fonts or varying glyph widths
- Mouse drag selection accumulates position errors with each event

**Prevention:**
- For a monospace editor with fixed `char_width` (currently `CHAR_WIDTH` constant in `TextAreaView`), hit-testing is `column = floor((mouse_x - gutter_width) / char_width)`, `line = floor((mouse_y + scroll_offset_px) / LINE_HEIGHT)`. This is sufficient for v2.
- Add `hit_test(pixel_x: f32, pixel_y: f32) -> (line: usize, col: usize)` to `EditorDataSource` or implement it directly in `TextAreaView` using `char_width` and `LINE_HEIGHT`
- For the gutter width offset: `GutterModel::width` is already in the `RenderModel`
- Do not attempt sub-character pixel-perfect hit testing — column-granular is correct for a code editor

**Warning signs:**
- Clicking at end of a line positions cursor one character left of where you clicked
- Click accuracy degrades as `char_width` estimate diverges from actual rendered width

**Phase:** Selection/Clipboard phase.

---

## Moderate Pitfalls

Mistakes that cause notable technical debt, delayed bugs, or UX failures.

---

### Pitfall 7: `EditorDataSource` Trait Pressure — Every Feature Adds Methods

**What goes wrong:** Each new v2.0 feature (find/replace state, multi-tab commands, file browser interaction) requires new data to flow through the adapter boundary. Currently `EditorDataSource` has 7 methods (`build_render_model`, `dispatch_command`, `resize_viewport`, `viewport_lines`, `scroll_y`, `total_lines`, `window_title`). Find/replace needs highlight ranges. The file browser needs directory entries and expansion state. Clipboard needs read/write. Each addition is a dyn trait method, which means every implementor (wgpu_client's adapter) must implement it, and the trait becomes a grab-bag.

**Why it happens:** The trait was designed for the current single-document render model. It was not designed for extension.

**Consequences:**
- Trait implementations become large monoliths
- Adding a method requires touching the trait definition, wgpu_client implementation, and any test stubs
- `RenderModel` grows to include find highlight state, sidebar state, file browser state — one large struct for everything
- Performance: building the full RenderModel every frame even when only tab title changed

**Prevention:**
- Plan the full set of methods needed across all v2.0 features before starting implementation
- Consider splitting: `EditorDataSource` (text content), `WorkspaceDataSource` (multi-tab, file state), `SearchDataSource` (find/replace)
- Or add sub-models to `RenderModel` lazily (only populate when the relevant UI is visible)
- The `RenderModel` could gain a revision/generation counter so `TextAreaView` can skip re-rendering when only status bar data changed

**Warning signs:**
- Adding a new feature requires touching `EditorDataSource` in 3+ files
- `RenderModel` struct has more than 15 fields
- Methods like `find_highlights()` sit on the same trait as `scroll_y()`

**Phase:** All v2.0 phases. Plan the trait expansion strategy during the first feature phase.

---

### Pitfall 8: Undo History Destroyed When Tab Is Closed and Reopened

**What goes wrong:** `Workspace::close_document_force()` (workspace.rs line 109) calls `self.histories.remove(&doc_id)`. When the user closes a tab and reopens the same file, a new `DocumentId` is created and a fresh `CommandHistory` is inserted. The entire undo history from the previous session is gone. There is no "I just closed and reopened this file" distinction from "fresh open."

**Why it happens:** `CommandHistory` is indexed by `DocumentId`, which is ephemeral. There is no persistence across close/reopen.

**Consequences:**
- Ctrl+Z after accidentally closing and reopening a tab does nothing useful
- User mental model: "I can undo recent accidental edits" — violated
- Cannot be fixed without either persisting history to disk or keeping it in memory after close

**Prevention:**
- For v2: do not try to persist history across close/reopen — document the limitation
- Instead, add "close tab" confirmation when a dirty document has meaningful history (document not saved since last meaningful edit)
- Consider keeping recently-closed document histories in a small LRU cache indexed by path — if the same path is reopened within the same session, restore the history
- Do not index by `DocumentId` for the LRU — use `PathBuf` as key

**Warning signs:**
- User closes a tab by accident, reopens the file, and Ctrl+Z produces nothing

**Phase:** Multi-tab phase.

---

### Pitfall 9: File Browser Reads Directory Synchronously During Render

**What goes wrong:** If the file browser's `render()` function calls `std::fs::read_dir()` to populate the tree, it runs on the UI thread during the render pass. Large directories (node_modules, Rust target/) contain thousands of entries. Even on SSDs, iterating 50,000 entries synchronously drops frames.

**Why it happens:** The simplest implementation of a file tree is to read the directory when the tree node is expanded. `FileTreeNode` in `types.rs` line 312 holds `children: Vec<FileTreeNode>` in-memory — easy to populate synchronously.

**Consequences:**
- Expanding `target/` in a Rust project freezes the UI for 1-3 seconds
- Windows: antivirus may scan each accessed directory entry, multiplying latency
- Frame budget exceeded, input events queue up, UI appears stuck

**Prevention:**
- Read directory contents on a background thread; populate `FileTreeNode.children` asynchronously
- Show a spinner or "loading..." placeholder in the tree while loading
- Lazy-load: only read children when a directory node is expanded, not eagerly on workspace open
- Cache directory contents; invalidate when file system watcher fires (or on manual refresh)
- Exclude known large directories from immediate expansion: `.git`, `target`, `node_modules`, `.venv`

**Warning signs:**
- Expanding any directory node causes a visible pause
- The editor becomes unresponsive for 1+ seconds after clicking a folder arrow

**Phase:** File browser phase.

---

### Pitfall 10: File System Watching Is Unreliable on Windows

**What goes wrong:** `ReadDirectoryChangesW` (the Windows API underlying `notify` crate) has well-documented limitations: it misses changes in deeply nested directories when the buffer overflows, drops events during high I/O activity, and does not fire for changes made via certain backup or antivirus processes. The `notify` crate wraps this API.

**Why it happens:** Windows file system watching requires polling as a fallback for reliability, not just event-driven watching.

**Consequences:**
- File tree does not update when an external tool creates or deletes files
- "File changed on disk" detection fails silently for some save patterns
- Buffer overflow causes entire subtree to stop being watched until restart

**Prevention:**
- Do not rely solely on file system events for freshness
- Combine events with periodic polling (every 5-10 seconds) as fallback
- On external change event: reload only the affected entry, not the entire tree
- For "file changed on disk" detection: check mtime on focus gain as a reliable fallback
- Use `notify` crate with `RecommendedWatcher` (it uses `ReadDirectoryChangesW` on Windows) but treat events as hints, not guarantees

**Warning signs:**
- Renaming a file externally does not update the file tree
- "This file has been changed on disk" prompt fails to appear after external save
- File tree becomes stale after high I/O activity (build, `cargo check`)

**Phase:** File browser phase. Note Windows-specific reliability as a known limitation in user documentation.

---

### Pitfall 11: Find/Replace Regex Stalls the UI Thread on Large Files

**What goes wrong:** Running a regex search over the entire rope on the UI thread blocks rendering. A poorly written regex (catastrophic backtracking) or a large file (200K lines) can stall the thread for seconds. The `ropey::Rope` does not have built-in regex search — the naive implementation materializes the rope to a `String` and searches that, which is an O(n) allocation plus O(n*m) search.

**Why it happens:** `EditorCommand` dispatch is synchronous (see `EditorDataSource::dispatch_command` — it takes `&mut self`). There is no async path for long-running commands.

**Consequences:**
- Typing in the search box causes visible lag on large files as regex is re-run on each keystroke
- Catastrophic backtracking on user-entered regex hangs the process entirely
- UI thread stalls = dropped input events = missed keystrokes

**Prevention:**
- Run find/replace search on a background thread; send results back to UI via channel
- Debounce search: wait 100-150ms after the last keystroke before starting search
- Use the `regex` crate which has a bounded execution time guarantee (no catastrophic backtracking)
- Limit search to visible range first, then expand to full document asynchronously
- Add a timeout: if search takes > 200ms, show "searching..." instead of blocking

**Warning signs:**
- Typing in search box causes stutter
- Opening find on a 50K+ line file freezes the editor
- CPU spikes to 100% during search

**Phase:** Find/Replace phase.

---

### Pitfall 12: Replace-All Does Not Preserve Cursor or Scroll Position

**What goes wrong:** A naive replace-all implementation replaces text positions from end to start (to preserve offsets) but then resets the cursor to position 0 and scroll offset to 0. The user's view context is lost. In files with many replacements, this is disorienting.

**Why it happens:** `Replace All` is often implemented as "clear document, insert new content" or as a loop of `EditOperation::Replace` calls that each update the caret.

**Consequences:**
- Cursor jumps to line 1 after every Replace All
- User must manually find their place again
- Undo creates N individual undo steps (one per replacement) instead of one atomic "replace all" step

**Prevention:**
- Batch all replacements into a single `Transaction` so Ctrl+Z undoes the entire Replace All in one step (the `Transaction` type exists in `core_editor/src/commands/transaction.rs`)
- Save caret offset and scroll position before Replace All; restore after
- If the caret was within a replaced range, move it to end of that replacement; otherwise keep the relative position
- Process replacements in reverse offset order to avoid invalidating earlier offsets

**Warning signs:**
- After Replace All, cursor is at line 1
- Ctrl+Z requires N presses to undo all replacements
- Editor scrolls back to top after Replace All

**Phase:** Find/Replace phase.

---

### Pitfall 13: Premature Performance Optimization via Cache Invalidation Complexity

**What goes wrong:** The current architecture rebuilds `RenderModel` every frame including `Vec<LinePresentation>` with `Vec<StyledSpan>` per line. The first performance instinct is to add caching everywhere. But complex cache invalidation introduces bugs that are harder to fix than the performance problem: stale line presentations, incorrect gutter numbers after insertion, syntax highlighting that doesn't update after edits.

**Why it happens:** Performance anxiety drives premature optimization. The real bottleneck is often not where developers expect.

**Consequences:**
- Stale render model causes visual artifacts (old line shown after deletion)
- Cache invalidation logic becomes the most bug-prone part of the codebase
- Dirty tracking bugs cause "why didn't this update?" UI regression reports

**Prevention:**
- Measure first. Use `std::time::Instant` around `build_render_model` calls. If it's under 1ms, it's not the bottleneck.
- The correct first optimization: only call `build_render_model` when `document.revision()` or view state has changed (store last-seen revision in the adapter)
- The `Document` already has `revision()` and `cached_content_revision` for exactly this pattern
- Second optimization: only regenerate syntax spans for lines whose byte range was touched by the last edit (the `update_syntax_incremental` method already exists)
- Avoid caching individual line presentations — the complexity is not worth it for v2

**Warning signs:**
- Visual glitch where old content shows after an edit
- Gutter shows wrong line numbers after line insertion/deletion
- Syntax highlighting doesn't update after file open

**Phase:** Performance optimization phase. Do not start optimizing before profiling.

---

### Pitfall 14: `with_active_context` Unsafe Block Will Not Extend to Multi-Buffer Operations

**What goes wrong:** `Workspace::with_active_context` (workspace.rs line 319) uses raw pointer casts to obtain simultaneous mutable access to view, document, history, and event bus. This works for single-document operations. Any feature that needs to access two documents simultaneously (move text between tabs, compare files, "paste into new file") cannot use this pattern safely.

**Why it happens:** Rust's borrow checker prevents multiple `&mut` references to fields of the same struct. The raw pointer workaround is sound for the current use case because the fields are truly disjoint.

**Consequences:**
- The unsafe block is correct today but becomes a trap when the API expands
- Copy-paste of the pattern for non-disjoint accesses causes undefined behavior
- Developers unfamiliar with the safety contract will misuse it

**Prevention:**
- Add a comment at the unsafe block documenting the invariant: "safe only because views, documents, and histories are separate HashMaps with different key types"
- Do not use this pattern for any operation involving two documents at once
- For multi-document operations, fetch what you need from each, then apply changes separately
- Consider refactoring to a method-based API: `workspace.with_document_and_view(doc_id, view_id, |doc, view| ...)` that statically prevents the dangerous cases

**Warning signs:**
- Someone adds a case to `with_active_context` that accesses `self.documents` twice (different keys)
- A multi-buffer operation is implemented by nesting `with_active_context` calls

**Phase:** Multi-tab phase. Add the documentation comment before implementing multi-tab.

---

### Pitfall 15: Non-UTF-8 Files Produce Unhelpful Errors

**What goes wrong:** `Document::open()` calls `std::fs::read_to_string()`, which returns `Err(InvalidData)` for any file that is not valid UTF-8. This includes Latin-1 encoded source files (common in older codebases), binary files the user accidentally opens, and files with a UTF-8 BOM that `read_to_string` handles inconsistently. The error propagates up as `std::io::Error` with no explanation to the user.

**Why it happens:** The current implementation is correct Rust (UTF-8 is the string type), but provides no user-facing feedback.

**Consequences:**
- User tries to open a file, nothing happens (or generic "could not open file" error)
- UTF-8 BOM (common in Windows tools like Notepad) can cause issues if not stripped
- Old `.cpp` files with Latin-1 encoded comments fail to open entirely

**Prevention:**
- Detect non-UTF-8 and show a user-facing message: "This file contains characters that cannot be displayed (not UTF-8 encoded)"
- Strip UTF-8 BOM (`\xEF\xBB\xBF`) before creating the buffer (add to `Document::from_str` preprocessing)
- For v2: do not implement full encoding detection/conversion. Document the UTF-8 limitation.
- Use `std::fs::read()` (returns `Vec<u8>`), attempt UTF-8 via `String::from_utf8()`, handle the error with a helpful message

**Warning signs:**
- Opening any file from a Windows-generated project silently fails
- Old C++ files with non-ASCII comments in strings produce errors

**Phase:** File Operations phase.

---

## Minor Pitfalls

Mistakes that cause annoyance but are straightforward to fix.

---

### Pitfall 16: Tab Title Shows "[Yeni Dosya]" (Turkish) Instead of "[New File]"

**What goes wrong:** `Document::title()` (document.rs line 279) returns `"[Yeni Dosya]"` (Turkish for "New File") when no path is set. `Workspace::document_title()` (workspace.rs line 409) returns `"[New File]"` (English). Two different fallback strings for the same concept in two different places.

**Why it happens:** Copy-paste from a test with a Turkish string was never updated. The workspace method uses English.

**Consequences:**
- Tab bar shows Turkish text for untitled documents
- Inconsistent: status bar shows "New File", tab bar shows "Yeni Dosya"
- Search for "[New File]" in codebase misses the Turkish variant

**Prevention:**
- Standardize on `"[New File]"` in `Document::title()` line 279
- Add a test asserting the English string
- Consider a dedicated constant `UNTITLED_DOCUMENT_TITLE` shared between both

**Warning signs:**
- New tabs show "[Yeni Dosya]" in the title

**Phase:** Fix before multi-tab (will be visible in every untitled document tab).

---

### Pitfall 17: `LineEnding::detect()` Detects CRLF Even in Mixed Files

**What goes wrong:** `LineEnding::detect()` (document.rs line 43) uses `content.contains("\r\n")`. A file with a single `\r\n` line followed by 999 `\n` lines is classified as CRLF. On save, the file is written back without normalization — it still has mixed endings. But the metadata says CRLF, so the status bar shows "CRLF" which is misleading.

**Why it happens:** Line ending detection as a fast `contains` check is a simplification.

**Consequences:**
- Status bar shows wrong line ending mode for mixed files
- Users cannot trust the displayed line ending indicator
- No option to normalize line endings (convert CRLF to LF or vice versa)

**Prevention:**
- For v2: accept the limitation and document it — mixed line ending handling is complex
- Consider majority-rules detection: if > 50% of lines are CRLF, report CRLF
- Add a "Normalize Line Endings" command to the command palette for v2.1

**Warning signs:**
- Status bar shows CRLF for a file with a single Windows newline

**Phase:** File Operations phase. Low priority, but note as a known limitation.

---

### Pitfall 18: `ora::EditorCommand` Enum Missing File and Search Variants

**What goes wrong:** The `EditorCommand` enum in `ora/src/editor_adapter/types.rs` (line 431) does not have variants for: `Open(PathBuf)`, `SaveAs(PathBuf)`, `New`, `CloseTab(u64)`, `SwitchTab(u64)`, `Find { query: String }`, `Replace { query: String, replacement: String }`. The core_editor `EditorCommand` (`core_editor/src/commands/editor_command.rs`) has `Open`, `SaveAs`, and `New`, but these are not mirrored in the ora adapter. Any UI action that needs these commands has no channel to dispatch them.

**Why it happens:** The ora adapter types were created for the v1 rendering-only use case. File and search commands were deferred.

**Consequences:**
- Implementing "Open File" button or file browser click requires adding to both enums and the wgpu_client conversion
- Find bar keyboard shortcut has no command to dispatch
- Tab close button click has no command to send

**Prevention:**
- Before implementing any v2.0 feature, audit the full set of commands needed and add them to `ora::EditorCommand` all at once
- Update the `From<ora::EditorCommand> for core_editor::EditorCommand` conversion in wgpu_client in the same PR
- Do not add commands one at a time as each feature is built — the churn is wasteful

**Warning signs:**
- Implementing a UI button requires touching 3+ files just to add the command variant

**Phase:** Audit needed before any v2.0 feature phase begins.

---

## Phase-Specific Warnings

| Phase | Likely Pitfall | Mitigation |
|-------|---------------|------------|
| File Operations | Synchronous I/O blocks UI | Async wrapper from day one; never `read_to_string` on UI thread |
| File Operations | Same file opened twice | Path deduplication index in Workspace before file browser exists |
| File Operations | Non-UTF-8 errors | Use `read()` + `from_utf8()` with user-facing error message |
| File Operations | Line ending display wrong | Document limitation; fix `contains` detection |
| Multi-tab | Tab switch blocked by `can_switch_active` | Call `set_active_view` directly; reserve protection for close/quit |
| Multi-tab | Undo history lost on close | LRU cache indexed by PathBuf for recent close/reopen; document limitation |
| Multi-tab | Missing `EditorCommand` variants | Audit and add all needed variants before implementing any feature |
| Multi-tab | Title shows "Yeni Dosya" | Fix before multi-tab is visible to users |
| Selection/Clipboard | Disappearing text bug | Fix in span composition (adapter), not renderer; consider overlay approach |
| Selection/Clipboard | Clipboard platform access | Handle in wgpu_client dispatch layer; use `arboard` only there |
| Selection/Clipboard | Mouse hit-testing offset | Implement `floor((mouse_x - gutter_px) / char_width)` in TextAreaView |
| Find/Replace | Regex stalls UI thread | Background thread + debounce + `regex` crate (bounded execution) |
| Find/Replace | Replace All wrecks cursor | Batch as Transaction; save/restore position before/after |
| File Browser | Directory read blocks frame | Background thread; lazy-load on expand; exclude `target/`, `.git/` |
| File Browser | File watcher misses events | Combine events with mtime polling on focus gain |
| Performance | Cache invalidation bugs | Measure first; use `document.revision()` as cheap staleness check |
| Integration | `with_active_context` unsafe | Add invariant comment; do not use for multi-document operations |
| Integration | Trait pressure on `EditorDataSource` | Plan trait expansion upfront; consider splitting into sub-traits |

---

## Integration Pitfalls: Preserving Existing Functionality

These pitfalls are specific to adding new features without breaking what already works.

**Pixel-level scroll accumulator.** The smooth scrolling system in ora uses a sub-line pixel accumulator that drives `scroll_y_offset_px` in `RenderModel`. Any change to `EditorDataSource::scroll_y()` or `total_lines()` semantics (e.g., to support multi-tab scroll state) must preserve this accumulator's behavior. Resetting the accumulator when switching tabs is correct; leaking state between tabs is not.

**Scissor clipping.** The text area uses scissor rects to clip text to the viewport. Adding the find bar overlay, selection highlight, or file browser panel must not interfere with existing scissor rect regions. Each UI region has its own clip rect; overlapping them without proper z-ordering causes visual artifacts.

**Frame-rebuild assumption.** The entire view tree is rebuilt every frame. New stateful features (find bar open/close state, file browser expansion state) must store their state in the model (core_editor Workspace or a new state struct), not in local variables within a `render()` call. A `render()` method has no memory between frames.

**`EditorDataSource` is `&mut self` for `dispatch_command`.** Adding new commands that need to return data (e.g., clipboard read, search result count) cannot use the current `dispatch_command(&mut self, cmd)` signature. The adapter may need a `query` or `request` method for operations that require a return value.

---

## Sources

All findings are grounded in direct codebase analysis. File paths and line numbers reference the state of the codebase as of 2026-03-26.

- `core_editor/src/domain/document.rs` — `open()`, `save()`, `title()`, `LineEnding::detect()`
- `core_editor/src/domain/workspace.rs` — `open_document()`, `close_document_force()`, `can_switch_active()`, `with_active_context()`
- `core_editor/src/view/selection.rs` — `Selection`, `SelectionSet`
- `core_editor/src/commands/editor_command.rs` — full command enum
- `ora/src/editor_adapter/types.rs` — `EditorCommand`, `RenderModel`, `TextStyle::Selection`
- `ora/src/editor_adapter/mod.rs` — `EditorDataSource` trait
- `ora/src/views/text_area.rs` — `LINE_HEIGHT`, `TextAreaView` stateless render
- `.planning/PROJECT.md` — v2.0 milestone goals, known bugs, key decisions

**Confidence:** HIGH. Every pitfall is traceable to a specific named type or method in the codebase. No pitfall is based solely on generic industry knowledge.
