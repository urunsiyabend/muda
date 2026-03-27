# Requirements: muda v2.0 Functional Editor

**Defined:** 2026-03-26
**Core Value:** A functional, performant code editor built on ora's GPU-accelerated UI framework — real file operations, multi-tab editing, selection, clipboard, find/replace, and Zed-level rendering performance.

## v1.0 Requirements (Validated)

All v1.0 ora UI framework requirements shipped and validated. See `.planning/MILESTONES.md` for full list.

Summary: 59 requirements across Core Framework, View System, Element System, Layout, Rendering, Events, Design System, Transitions, Component Migration (Chrome, Text Editing, Advanced UI, Widgets), and Integration — all complete.

## v2.0 Requirements

Requirements for v2.0 Functional Editor milestone. Each maps to roadmap phases.

### Foundation & Bug Fixes

- [x] **FIX-01**: Event loop uses event-driven redraw — no unconditional `request_redraw()`, idle GPU work eliminated
- [x] **FIX-02**: Selection rendering works correctly — Shift+arrow no longer causes text to disappear
- [x] **FIX-03**: All v2 EditorCommand variants defined at once (SwitchTab, CloseTab, OpenFile, SaveAs, New, Find, Replace, ReplaceAll, GoToLine)
- [x] **FIX-04**: EditorDataSource trait split into focused sub-traits covering all v2 methods (workspace, search, file operations)
- [x] **FIX-05**: Caret blink timer drives redraw in `about_to_wait()` instead of continuous rendering

### Buffer Registry & Multi-Tab

- [ ] **TAB-01**: Buffer Registry (`HashMap<PathBuf, DocumentId>`) deduplicates files — opening same path reuses existing buffer
- [ ] **TAB-02**: User can click a tab to switch active buffer with scroll/cursor state preserved per tab
- [ ] **TAB-03**: User can close a tab with Ctrl+W — dirty buffers trigger save-before-close dialog
- [ ] **TAB-04**: User can cycle through open tabs with Ctrl+Tab / Ctrl+Shift+Tab
- [ ] **TAB-05**: Tab dirty indicator reflects actual buffer modified state

### File Operations

- [x] **FILE-01**: User can open a file via Ctrl+O with native OS file picker (rfd)
- [x] **FILE-02**: User can save the active file via Ctrl+S (silently if path known, Save As dialog if untitled)
- [x] **FILE-03**: User can Save As via Ctrl+Shift+S to choose a new file path
- [x] **FILE-04**: User can create a new untitled buffer via Ctrl+N
- [x] **FILE-05**: File I/O uses async wrapper at wgpu_client dispatch layer — UI thread never blocks on disk
- [x] **FILE-06**: Non-UTF-8 files show user-facing error message; UTF-8 BOM stripped on load

### Sidebar File Browser

- [ ] **SIDE-01**: Sidebar reads real directory tree via `std::fs::read_dir` (replaces mock data)
- [ ] **SIDE-02**: User can click a file in sidebar to open it in editor (routes through Buffer Registry)
- [ ] **SIDE-03**: Folder expand/collapse state persisted in view (not re-collapsed each frame)
- [ ] **SIDE-04**: User can open a folder via dialog to set workspace root
- [ ] **SIDE-05**: File watcher (`notify`) updates sidebar when external changes occur
- [ ] **SIDE-06**: Common directories excluded from expansion (target/, .git/, node_modules/)

### Selection & Clipboard

- [x] **SEL-01**: User can click in text area to position cursor at that location (pixel-to-line/col mapping)
- [x] **SEL-02**: User can click and drag to select text ranges
- [x] **SEL-03**: User can select text with Shift+arrow, Shift+click, Ctrl+Shift+arrow
- [x] **SEL-04**: User can select all text with Ctrl+A
- [x] **SEL-05**: User can copy selection to OS clipboard with Ctrl+C (via arboard)
- [x] **SEL-06**: User can cut selection to OS clipboard with Ctrl+X
- [x] **SEL-07**: User can paste from OS clipboard with Ctrl+V

### Find & Replace

- [ ] **FIND-01**: User can open find bar with Ctrl+F — inline overlay at top of editor area
- [ ] **FIND-02**: All matches highlighted in text area with match count "N of M" displayed
- [ ] **FIND-03**: User can navigate matches with next/prev (Enter/Shift+Enter or arrow buttons)
- [ ] **FIND-04**: User can toggle case-sensitive, whole-word, and regex search modes
- [ ] **FIND-05**: User can open replace row with Ctrl+H — Replace One and Replace All buttons
- [ ] **FIND-06**: Replace All executes as single Transaction for one-step undo
- [ ] **FIND-07**: User can close find bar with Escape, returning focus to editor
- [ ] **FIND-08**: User can jump to a line number with Ctrl+G dialog

### Performance

- [ ] **PERF-01**: Incremental tree-sitter parsing — `parser.parse(content, Some(old_tree))` reuses previous parse tree
- [ ] **PERF-02**: Glyphon Buffer objects cached by (text, font_size, line_height) with LRU eviction
- [ ] **PERF-03**: Tree-sitter parsing runs on background thread for files >100KB

## Future Requirements (Post-v2)

### Multi-File Search
- **SEARCH-01**: Find across files (Ctrl+Shift+F) with results panel

### Advanced Editing
- **AEDIT-01**: Multi-cursor editing
- **AEDIT-02**: Bracket matching and auto-close
- **AEDIT-03**: Minimap sidebar

### File Tree Mutation
- **FTREE-01**: Create new file/folder from sidebar
- **FTREE-02**: Rename file/folder inline
- **FTREE-03**: Delete file/folder with confirmation

### Editor Layout
- **LAYOUT-01**: Split panes (vertical/horizontal)
- **LAYOUT-02**: Tab reordering via drag and drop

### Language Intelligence
- **LSP-01**: LSP client integration
- **LSP-02**: Diagnostics display (errors, warnings)
- **LSP-03**: Go to definition, find references
- **LSP-04**: Autocomplete

### Terminal
- **TERM-01**: Integrated terminal panel

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Find across files (Ctrl+Shift+F) | High complexity, defer to v3 |
| Multi-cursor editing | Complex selection model, not needed for functional editor |
| File tree mutation (rename/delete/new) | v2 is read-only file browsing; mutation in v3 |
| Split panes | Single editor pane sufficient for v2 |
| Tab reordering via drag | Click switching sufficient for v2 |
| LSP integration | v2 focuses on basic editing; language intelligence in v3 |
| Bracket matching / minimap | Polish features, not core functionality |
| Terminal panel | Separate concern, defer to v3 |
| Hot-reload themes at runtime | Compile-time token system sufficient |
| Plugin/extension API | Internal framework only |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FIX-01 | Phase 10 | Complete |
| FIX-02 | Phase 10 | Complete |
| FIX-03 | Phase 10 | Complete |
| FIX-04 | Phase 10 | Complete |
| FIX-05 | Phase 10 | Complete |
| TAB-01 | Phase 11 | Pending |
| TAB-02 | Phase 11 | Pending |
| TAB-03 | Phase 11 | Pending |
| TAB-04 | Phase 11 | Pending |
| TAB-05 | Phase 11 | Pending |
| FILE-01 | Phase 12 | Complete |
| FILE-02 | Phase 12 | Complete |
| FILE-03 | Phase 12 | Complete |
| FILE-04 | Phase 12 | Complete |
| FILE-05 | Phase 12 | Complete |
| FILE-06 | Phase 12 | Complete |
| SIDE-01 | Phase 14 | Pending |
| SIDE-02 | Phase 14 | Pending |
| SIDE-03 | Phase 14 | Pending |
| SIDE-04 | Phase 14 | Pending |
| SIDE-05 | Phase 14 | Pending |
| SIDE-06 | Phase 14 | Pending |
| SEL-01 | Phase 13 | Complete |
| SEL-02 | Phase 13 | Complete |
| SEL-03 | Phase 13 | Complete |
| SEL-04 | Phase 13 | Complete |
| SEL-05 | Phase 13 | Complete |
| SEL-06 | Phase 13 | Complete |
| SEL-07 | Phase 13 | Complete |
| FIND-01 | Phase 15 | Pending |
| FIND-02 | Phase 15 | Pending |
| FIND-03 | Phase 15 | Pending |
| FIND-04 | Phase 15 | Pending |
| FIND-05 | Phase 15 | Pending |
| FIND-06 | Phase 15 | Pending |
| FIND-07 | Phase 15 | Pending |
| FIND-08 | Phase 15 | Pending |
| PERF-01 | Phase 16 | Pending |
| PERF-02 | Phase 16 | Pending |
| PERF-03 | Phase 16 | Pending |

**Coverage:**
- v2.0 requirements: 40 total (note: source material stated 34; actual count is FIX(5)+TAB(5)+FILE(6)+SIDE(6)+SEL(7)+FIND(8)+PERF(3)=40)
- Mapped to phases: 40/40
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after v2.0 roadmap creation — all 40 requirements mapped*
