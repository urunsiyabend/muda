# Project Research Summary

**Project:** muda — v2.0 Functional Editor
**Domain:** GPU-rendered code editor (wgpu + winit + glyphon + ora UI framework)
**Researched:** 2026-03-26
**Confidence:** HIGH

## Executive Summary

muda is a GPU-accelerated code editor built on the `ora` UI framework using wgpu 23, winit 0.30, and glyphon 0.7. The v1 shell renders correctly but is not yet a functional editor: file operations are stubs, clipboard is unwired, tabs do not switch buffers, and the file browser reads mock data. v2.0 makes it a real editor by wiring these systems to actual filesystem, OS clipboard, and buffer management. The overall stack requires only 3 new crates (`rfd`, `notify`, `notify-debouncer-full`) plus 2 promotions from transitive to explicit (`regex`, `pollster`). Everything else builds on what already exists.

The single highest-ROI change in the entire v2 scope is removing the unconditional `request_redraw()` call at line 549 of `ora/src/platform/event_loop.rs`. This one-line fix eliminates approximately 90% of idle GPU work and makes all subsequent profiling meaningful. It must happen before any performance-sensitive work begins. Beyond that, the feature work has a clear dependency ordering anchored by a new **Buffer Registry** — a central data structure that every other v2 feature routes through. FEATURES.md, ARCHITECTURE.md, and PITFALLS.md all independently identify this as the first thing to build. Nothing else can be built correctly until it exists.

The primary risks are not architectural — they are specific, named pitfalls in the existing codebase. Synchronous file I/O on the UI thread must be async from day one (retrofitting is expensive and touches every call site). The `can_switch_active()` guard in `workspace.rs` must not be called during normal tab switching or every click triggers a save dialog. The selection rendering bug is in span composition in the adapter, not in the GPU renderer — debugging in the wrong layer would waste significant time. All of these are preventable with upfront awareness, and all are grounded in specific file paths and line numbers.

---

## Key Findings

### Recommended Stack

The existing crate set is validated. New additions are minimal and purposeful. See [STACK.md](STACK.md) for full rationale.

**New dependencies:**

- `rfd = "0.17"` (wgpu_client): Native OS file dialogs — Windows IFileOpenDialog via COM, macOS NSOpenPanel. Use `AsyncFileDialog` + `pollster::block_on` pattern. Do not roll custom COM calls.
- `pollster = "0.4"` (wgpu_client): Promote to explicit dep for `pollster::block_on` calls in wgpu_client binary.
- `notify = "8"` (core_editor): Cross-platform filesystem watching via ReadDirectoryChangesW on Windows. De facto standard (used by Zed, rust-analyzer, cargo-watch).
- `notify-debouncer-full = "0.5"` (core_editor): Debounces burst events into single logical events; stitches rename FROM+TO pairs.
- `regex = "1"` (core_editor): Promote from transitive to explicit. Already present in Cargo.lock via tree-sitter chain — zero binary weight increase.

**Nothing else needed.** Clipboard is already covered by `arboard 3.6.1` (direct dep of core_editor). Performance optimization is architectural, not a library problem.

**Critical rope+regex pitfall:** Do not walk rope chunks for regex search. Matches spanning chunk boundaries are silently dropped. Always materialize the search range to a contiguous `String` first, then map byte offsets back via `rope.byte_to_char()`.

### Expected Features

See [FEATURES.md](FEATURES.md) for full behavior specs and complexity estimates.

**Must have (v2 table stakes):**

1. **Buffer Registry** — new central data structure; enables all other v2 features. Must be built first.
2. **File Open/Save** — `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`; real filesystem read/write with async I/O wrapper.
3. **Save-before-close dialog** — wired to existing `DialogView`; triggered on `Ctrl+W` and app exit only (not tab switch).
4. **Multi-tab switching** — click to switch, `Ctrl+W` close, `Ctrl+Tab` cycle; deduplication on open.
5. **Sidebar file browser (real data)** — real `std::fs::read_dir`, click to open file, expand/collapse persisted in view state.
6. **Selection rendering fix** — fix existing visual bug before building anything on top of selection.
7. **OS clipboard** — `Ctrl+C`, `Ctrl+X`, `Ctrl+V` via arboard at wgpu_client dispatch layer (not in core_editor or ora).
8. **Mouse click to position cursor** — pixel-to-line/col mapping; `floor((x - gutter_px) / char_width)` is sufficient for monospace.
9. **Mouse drag selection** — click and drag selects text.
10. **Find bar** (`Ctrl+F`) — inline overlay at top of editor area, next/prev navigation, match highlighting in TextAreaView.

**Should have (round out v2):**

- Regex find + Replace/Replace All (second row in find bar; include in same phase as find bar)
- Go to line (`Ctrl+G`)
- Open Folder dialog (sets workspace root for sidebar)
- Event-driven redraw (performance correctness — this is Phase 1 work, not an enhancement)

**Defer to post-v2:**

- Find across files (`Ctrl+Shift+F`), multi-cursor editing, file tree mutation (rename/delete/new), split panes, tab reordering via drag, all LSP features, bracket matching, minimap.

### Architecture Approach

The ora framework is already immediate-mode and structurally sound. The visual shell is correctly wired to `EditorDataSource`. What is missing is behavioral: commands that trigger real work, and data that flows from real sources. See [ARCHITECTURE.md](ARCHITECTURE.md) for full analysis including Zed/GPUI pattern research.

**Major components and v2 work:**

1. **Buffer Registry** (new, in core_editor/Workspace) — `HashMap<PathBuf, DocumentId>` index; deduplication on open; `SwitchTab`, `CloseTab`, `OpenFile` commands. Everything routes through this.
2. **Event-Driven Redraw** (fix in ora/event_loop.rs) — remove unconditional `request_redraw()` at line 549; add caret-blink timer in `about_to_wait()`. Prerequisite for all performance work.
3. **EditorDataSource Expansion** (design decision) — plan the full set of new trait methods across all v2 features before implementing any. Consider splitting into sub-traits (`EditorDataSource`, `WorkspaceDataSource`, `SearchDataSource`). This decision must be made upfront, not organically.
4. **FindReplaceView** (new, in ora/views) — stack overlay above TextAreaView; follows `CommandPaletteView` lifecycle pattern. Requires `TextStyle::SearchHighlight` tokens in the theme (no structural renderer change — two new color tokens).
5. **Async I/O wrapper** (new, at wgpu_client dispatch layer) — `Document::open/save` stay synchronous in core_editor; the async wrapper lives at the boundary.

**Performance optimizations ranked by impact (for Phase 7):**

1. Event-driven redraw — trivial effort, critical impact; do in Phase 1
2. Incremental tree-sitter parsing — `parser.parse(content, Some(old_tree))`; core_editor only
3. Glyphon Buffer caching — cache by `(text, font_size, line_height)`; eliminates reshaping for unchanged lines
4. Background parse thread — for files >100KB; high effort; do after (2) is stable
5. Layout caching — only after profiling confirms layout is a bottleneck

### Critical Pitfalls

See [PITFALLS.md](PITFALLS.md) for full details with specific file paths and line numbers.

1. **Synchronous file I/O blocks the UI thread** (Pitfall #1) — `Document::open/save` in `document.rs` line 132 runs on the UI thread. On Windows, antivirus hooks make even local SSDs unpredictable. Move to async wrapper from day one — retrofitting touches every call site.

2. **`can_switch_active()` blocks normal tab switching** (Pitfall #3) — `workspace.rs` line 460 returns `ProtectionError::UnsavedChanges` if the active document is dirty. Never call this during a tab click. Call `workspace.set_active_view(view_id)` directly. Reserve save prompts for close-tab, close-workspace, and quit.

3. **Opening the same file twice causes silent data loss** (Pitfall #2) — `open_document()` at `workspace.rs` line 92 creates a new `Document` every time with no path deduplication. Two independent instances diverge; saving one silently overwrites the other. Add `path_to_doc: HashMap<PathBuf, DocumentId>` before any file browser exists.

4. **Selection rendering bug is in span composition, not the renderer** (Pitfall #4) — "Shift+arrow causes text to disappear" is a bug in the adapter's `build_render_model` span generation. Gutter line numbers still render correctly during disappearance, which proves the GPU renderer is fine. Debugging in the renderer layer wastes time. Consider switching to a selection overlay (list of `(line, col_start, col_end)` rectangles drawn behind text) — the Zed/VS Code approach.

5. **Clipboard crosses the adapter boundary** (Pitfall #5) — `EditorDataSource` has no clipboard methods. Handle clipboard entirely at wgpu_client dispatch layer: on `Copy`, get selected text from workspace then call `arboard::Clipboard::set_text()`; on `Paste`, call `arboard::Clipboard::get_text()` then dispatch `InsertText`. Keep the clipboard handle alive across operations — repeated construction has overhead on Windows.

6. **`EditorDataSource` trait will accumulate method pressure** (Pitfall #7) — each new v2 feature wants new methods on a dyn trait. Plan the full expansion across all v2 features before implementing any. Design the sub-trait split or lazy sub-model approach upfront. Warning sign: `RenderModel` exceeding 15 fields, or unrelated methods like `find_highlights()` sharing a trait with `scroll_y()`.

---

## Implications for Roadmap

### Phase 1: Foundation Fixes

**Rationale:** Three bugs and one audit must land before feature work begins. The event-driven redraw fix is the highest-ROI change in v2 scope. The command audit prevents per-feature file churn later. The selection bug investigation must precede selection feature work.

**Delivers:** A codebase ready for features — correct idle GPU behavior, a complete command vocabulary, no inherited rendering bugs, consistent untitled document title.

**Work items:**
- Remove unconditional `request_redraw()` from `event_loop.rs` line 549; add caret-blink timer in `about_to_wait()`
- Audit and add all missing `ora::EditorCommand` variants at once: `SwitchTab`, `CloseTab`, `OpenFile`, `SaveAs`, `New`, `Find`, `Replace`, `ReplaceAll` — and update the `From<ora::EditorCommand>` conversion in wgpu_client
- Design `EditorDataSource` trait expansion strategy (sub-traits or sub-models) to cover all v2 features
- Investigate and fix selection span composition bug in adapter's `build_render_model` (consider overlay approach)
- Fix `Document::title()` line 279: change "Yeni Dosya" to "[New File]"

**Avoids:** Pitfall #4 (selection bug in wrong layer), Pitfall #16 (Turkish title), Pitfall #18 (missing command variants), Pitfall #7 (trait pressure — plan upfront)

**Research flag:** Standard patterns; no additional research needed.

---

### Phase 2: Buffer Registry + Multi-Tab

**Rationale:** The Buffer Registry is the central data structure for v2. FEATURES.md, ARCHITECTURE.md, and PITFALLS.md all converge on this as the prerequisite for every other feature. Nothing else can be built correctly until path-deduplication and tab-switching are in place.

**Delivers:** Functional multi-tab editing — click to switch buffers, `Ctrl+W` close, `Ctrl+Tab` cycle, deduplication on open, dirty indicator per tab.

**Work items:**
- Add `path_to_doc: HashMap<PathBuf, DocumentId>` index to `Workspace`
- Implement `SwitchTab` / `CloseTab` / `OpenFile` command handlers calling `set_active_view()` directly (not `can_switch_active()`)
- Wire `TabBarView` click events via `PrepaintContext::register_hitbox()` and `on_mouse_down()` (pattern already exists in codebase)
- Add per-tab scroll + cursor state preservation on switch (already stored per document in core_editor)
- Document `with_active_context` unsafe invariant before expanding its usage (Pitfall #14)

**Avoids:** Pitfall #2 (duplicate open), Pitfall #3 (blocked tab switch), Pitfall #8 (undo history — document limitation), Pitfall #14 (unsafe multi-document access)

**Stack additions:** None — pure architecture work.

**Research flag:** Standard patterns; no additional research needed.

---

### Phase 3: File Operations

**Rationale:** Depends on Buffer Registry. File open populates the registry; file save persists buffers from it. The async I/O requirement is non-negotiable from day one — it cannot be retrofitted later without touching every call site.

**Delivers:** Real filesystem read/write — `Ctrl+O` (native file picker), `Ctrl+S` (save silently if path known), `Ctrl+Shift+S` (Save As dialog), `Ctrl+N` (new untitled buffer), save-before-close dialog wired to `DialogView`.

**Work items:**
- Add async file load wrapper at wgpu_client dispatch layer (not in `Document::open` itself)
- Integrate `rfd::AsyncFileDialog` + `pollster::block_on` for Open and Save As
- Handle non-UTF-8 files: use `std::fs::read()` + `String::from_utf8()` with user-facing error message
- Strip UTF-8 BOM (`\xEF\xBB\xBF`) in `Document::from_str` preprocessing
- Wire save-before-close flow to `DialogView` overlay on `Ctrl+W` and app exit (not on tab switch)
- Document line-ending detection limitation (PITFALLS #17 — `contains("\r\n")` misclassifies mixed files)

**Avoids:** Pitfall #1 (sync I/O), Pitfall #2 (duplicate open — registry already handles this by Phase 2), Pitfall #15 (non-UTF-8 errors)

**Stack additions:** `rfd = "0.17"` and `pollster = "0.4"` to `wgpu_client/Cargo.toml`.

**Research flag:** Needs targeted research on how async file I/O return values thread back through the synchronous `dispatch_command(&mut self, cmd)` path. The current signature returns `()` — a result channel, callback, or new `query()` method is needed.

---

### Phase 4: Selection + Clipboard

**Rationale:** Depends on Phase 1 (selection rendering bug fixed). Building mouse selection on top of an unfixed rendering bug produces confusion about whether new bugs are regressions or the existing bug. The pixel-to-col mapping must be built once and reused by click, drag, and find/replace jump.

**Delivers:** Full selection model — click to position cursor, drag to select, double-click word, triple-click line, Shift+arrow, Shift+click, Ctrl+Shift+arrow. OS clipboard — `Ctrl+C`, `Ctrl+X`, `Ctrl+V` via arboard at wgpu_client dispatch layer.

**Work items:**
- Implement `hit_test(pixel_x, pixel_y) -> (line, col)` in `TextAreaView` using `floor((x - gutter_px) / char_width)` formula
- Wire mouse-down to cursor positioning; mouse-move during button-hold to drag selection
- Wire `Copy`/`Cut` to get selected text from workspace then call `arboard::Clipboard::set_text()` — all in wgpu_client dispatcher
- Wire `Paste` to call `arboard::Clipboard::get_text()` then dispatch `InsertText` — all in wgpu_client dispatcher
- Keep `arboard::Clipboard` alive (not re-created per operation)
- Verify `arboard` is called from `wgpu_client`, not added to `core_editor` or `ora`

**Avoids:** Pitfall #4 (selection rendering — fixed in Phase 1), Pitfall #5 (clipboard boundary), Pitfall #6 (mouse hit-testing offset)

**Stack additions:** None — arboard already in core_editor; invocation moves to wgpu_client dispatcher.

**Research flag:** Standard patterns; monospace hit-testing formula is documented in PITFALLS #6.

---

### Phase 5: File Browser

**Rationale:** Depends on Buffer Registry (Phase 2) and File Operations (Phase 3). Clicking a file in the browser calls `OpenFile` which routes through both. The sidebar shell already exists and renders visual structure; this phase wires it to real data and interactions.

**Delivers:** Functional file browser — reads real directory tree, folder expand/collapse (state persisted in view, not re-collapsed each frame), click to open file, current file highlighted, keyboard navigation (arrow keys, Enter), Open Folder dialog.

**Work items:**
- Replace mock data in `FileTreeView` with real `std::fs::read_dir` traversal
- Add full path to `FileEntryPresentation` (`path: String` field — currently missing)
- Load directory contents on a background thread; lazy-load on expand (do not read in `render()`)
- Exclude `target/`, `.git/`, `node_modules/`, `.venv/` from immediate expansion
- Wire click events to `OpenFile` command; highlight active file based on active buffer path
- Integrate `notify-debouncer-full` watcher for live tree updates; combine with mtime-check on focus gain as fallback for Windows reliability

**Avoids:** Pitfall #9 (sync directory reads in render path), Pitfall #10 (Windows file watcher unreliability)

**Stack additions:** `notify = "8"` and `notify-debouncer-full = "0.5"` to `core_editor/Cargo.toml`.

**Research flag:** The file watcher integration with the existing workspace model may need targeted research on how change events propagate to trigger sidebar re-renders without rebuilding the full tree. Standard file tree UI patterns otherwise apply.

---

### Phase 6: Find / Replace

**Rationale:** Most new code in v2. Architecturally self-contained — adds a new view, new commands, and new fields to `RenderModel`. Depends on Phase 1 (selection span rendering provides the path for match highlight rendering). Can be built after Phase 2; does not require Phases 3-5.

**Delivers:** Inline find bar (`Ctrl+F`) — all matches highlighted, match count "N of M", next/prev navigation with wrap, case-sensitive/whole-word/regex toggles, Escape to close. Replace row (`Ctrl+H`) — Replace One (advances to next match), Replace All (batched as single `Transaction` for one-step undo).

**Work items:**
- Add `FindReplaceView` as Stack overlay above TextAreaView (follow `CommandPaletteView` lifecycle pattern)
- Add `EditorCommand::Find { query, options }` / `FindNext` / `FindPrev` / `Replace` / `ReplaceAll` command handlers
- Add `TextStyle::SearchHighlight` and `TextStyle::SearchCurrent` tokens (two new theme color entries — no structural renderer change)
- Add `search_results: Option<SearchResultPresentation>` to `RenderModel`
- Run search on a background thread; debounce 100-150ms after last keystroke before triggering
- Use `regex` crate (bounded execution time — no catastrophic backtracking)
- Materialize rope range to `String` for search; do not chunk-walk (see STACK.md rope+regex pitfall)
- Batch Replace All into a single `Transaction` (type exists at `core_editor/src/commands/transaction.rs`); save/restore caret and scroll position before/after

**Avoids:** Pitfall #11 (regex stalls UI — background thread + debounce), Pitfall #12 (Replace All cursor loss — Transaction + position restore)

**Stack additions:** `regex = "1"` promoted to explicit dep in `core_editor/Cargo.toml`.

**Research flag:** Needs research on focus/keyboard routing lifecycle. The `CommandPaletteView` pattern (Escape to close) is the right starting point, but the find bar stays open while the user types in the editor — the focus model differs from the palette.

---

### Phase 7: Performance Refinement

**Rationale:** Always last. The Phase 1 event-driven redraw fix makes profiling accurate. Only after all features are working and real usage patterns are visible should further performance work begin. Profile first; optimize second. Premature caching creates cache invalidation bugs that are harder to fix than the performance problem (PITFALLS #13).

**Delivers:** Responsive editing on large files — incremental tree-sitter parsing (~30ms to ~0.5ms per keystroke on 10K-line files), glyphon buffer caching (~97.5% reshaping reduction for unchanged lines), background parse thread for files >100KB.

**Work items:**
- Measure `build_render_model` time using `std::time::Instant`; use `document.revision()` as cheap staleness check before optimizing
- Implement incremental tree-sitter parsing: `parser.parse(content, Some(old_tree))` with `TextEdit` structs from buffer mutations
- Cache `glyphon::Buffer` objects keyed by `(text, font_size, line_height)` with LRU eviction in `TextSystem`
- Move tree-sitter parsing to background thread (std::thread + mpsc channel) after incremental parsing is stable
- Do not cache individual `LinePresentation` objects — complexity is not worth it for v2

**Avoids:** Pitfall #13 (premature caching — measure first, use `document.revision()`)

**Stack additions:** None.

**Research flag:** Standard patterns, thoroughly documented in ARCHITECTURE.md. Primary constraint is "measure first" — do not begin until profiling data is available after Phase 1 fix.

---

### Phase Ordering Rationale

- Phase 1 is not optional: the event-driven redraw fix must precede profiling; the command audit must precede feature implementation; the selection bug investigation must precede selection features.
- Phase 2 must precede everything else: three independent research files converge on Buffer Registry as the root dependency.
- Phase 3 follows Phase 2: file open/save creates and populates registry entries.
- Phase 4 follows Phase 1: selection features require the rendering bug to be understood and fixed first.
- Phase 5 follows Phases 2-3: file browser click-to-open requires both the registry and file operations.
- Phase 6 can proceed after Phases 1 and 2; it does not depend on Phases 3-5 and can run in parallel with them if desired.
- Phase 7 is always last: profiling only becomes meaningful after the event-driven redraw fix in Phase 1.

### Research Flags

Phases needing deeper research during planning:
- **Phase 3 (File Operations):** How async file I/O return values thread back through the synchronous `dispatch_command(&mut self, cmd) -> ()` signature. A result channel, callback, or new query method is needed; the design must be settled before implementation begins.
- **Phase 6 (Find/Replace):** Focus and keyboard routing lifecycle when find bar stays open while editing in the text area — this is a different focus model than the `CommandPaletteView` pattern.

Phases with standard, well-documented patterns (skip research-phase):
- **Phase 1 (Foundation Fixes):** The event loop fix is one line; command audit is mechanical; bug investigation is localized.
- **Phase 2 (Buffer Registry):** HashMap index + hitbox registration — established patterns already present in the codebase.
- **Phase 4 (Selection/Clipboard):** Monospace hit-testing formula documented in PITFALLS #6; arboard API is well-understood.
- **Phase 5 (File Browser):** Lazy-load tree pattern is standard; file watcher integration is the only uncertainty.
- **Phase 7 (Performance):** Patterns fully documented in ARCHITECTURE.md; constraint is "measure first."

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crate versions confirmed on crates.io (March 2026); arboard already in project at 3.6.1; no speculative additions |
| Features | HIGH | Derived from direct codebase analysis of core_editor commands and ora views; complexity estimates are MEDIUM |
| Architecture | HIGH (ora) / MEDIUM (Zed) | ora findings from direct source reading; Zed patterns from DeepWiki + WebSearch summaries (WebFetch blocked during research) |
| Pitfalls | HIGH | Every pitfall traceable to a specific file path, struct name, and line number; no pitfall based solely on generic advice |

**Overall confidence:** HIGH

### Gaps to Address

- **Async dispatch return path:** The `dispatch_command(&mut self, cmd)` signature returns `()`. File open/save results (success, error, loaded content) need a mechanism to return to the UI. Options: a result channel, a callback, or a new `query()` method on `EditorDataSource`. This design decision must be resolved during Phase 3 planning before implementation begins.

- **EditorDataSource sub-trait strategy:** The research recommends splitting the trait but does not prescribe the exact split. The Phase 1 design work must produce a concrete proposal (which methods on which trait) before Phase 2 begins adding new commands.

- **Zed source verification:** WebFetch was blocked during research. All Zed architecture findings are from WebSearch summaries and DeepWiki, cross-validated against ora source code. The findings are internally consistent but are not primary-source verified. Verify against current GPUI README before closely following any GPUI-specific pattern.

---

## Sources

### Primary — direct codebase analysis (HIGH confidence)
- `ora/src/platform/event_loop.rs` — continuous redraw bug at line 549, render pipeline
- `ora/src/rendering/text.rs` — atlas persistence, `clear()` semantics
- `ora/src/views/*.rs` — existing view coverage, integration gaps
- `core_editor/src/domain/document.rs` — `open()`, `save()`, `title()`, `LineEnding::detect()`
- `core_editor/src/domain/workspace.rs` — `open_document()`, `can_switch_active()`, `with_active_context()`
- `ora/src/editor_adapter/types.rs` + `mod.rs` — `EditorDataSource` trait, `RenderModel`, `EditorCommand`

### Secondary — external crate documentation (HIGH confidence)
- [rfd 0.17.2 crates.io](https://crates.io/crates/rfd) — confirmed version, Windows COM backend
- [notify 8.2.0 crates.io](https://crates.io/crates/notify) — confirmed version, Windows RDCW backend
- [notify-debouncer-full 0.5.0 docs.rs](https://docs.rs/notify-debouncer-full) — debouncer API
- [arboard GitHub (1Password)](https://github.com/1Password/arboard) — 3.6.1, already in project
- [regex 1.12.2 docs.rs](https://docs.rs/regex) — confirmed version, bounded execution guarantee

### Secondary — Zed architecture research (MEDIUM confidence)
- DeepWiki (zed-industries/zed) — EditorElement, DisplayMap, ScrollManager, snapshot pattern
- Zed blog: "Rope & SumTree", "Syntax-Aware Editing", "videogame", "120fps" (via WebSearch summaries)
- Zed blog "Quality Week Dec 2025" — idle GPU reduction via conditional frame presentation confirms event-driven redraw pattern
- GitHub PR #25009 summary — `AnyView::cached`; scroll/mousemove no longer trigger full render
- Zed PR #8919 — Windows IFileOpenDialog (reference for `rfd` choice over custom COM)
- Zed issues #29657, #51278 — Windows clipboard soundness (reference for arboard preference)

---
*Research completed: 2026-03-26*
*Ready for roadmap: yes*
