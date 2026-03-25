# Feature Landscape: v2.0 Functional Editor

**Domain:** Code editor features — file operations, navigation, editing
**Researched:** 2026-03-26
**Milestone:** v2.0 Functional Editor (what exists is a visual shell; this makes it a real editor)
**Confidence:** HIGH (editor conventions are stable, well-established; verified against existing codebase)

---

## What Already Exists

Before mapping features, it's essential to know what the codebase already has. The roadmap
consumer should build on these foundations, not around them.

### Existing in core_editor (commands)

| Command | Status |
|---------|--------|
| `Save`, `SaveAs(PathBuf)`, `Open(PathBuf)`, `New` | Defined, NOT wired to real file I/O |
| `MoveCursor { extend_selection }` (char/word/line/page) | Defined and dispatched |
| `SelectAll`, `ClearSelection`, `DeleteSelection` | Defined |
| `Copy`, `Cut`, `Paste` | Defined, NOT wired to OS clipboard |
| `Undo`, `Redo` | Defined and working |
| `InsertChar`, `InsertText`, `InsertNewline`, `Backspace`, `Delete` | Working |
| `Scroll { lines }` | Working (pixel-level smooth scroll) |

### Existing in ora views

| View | Status |
|------|--------|
| `TabBarView` | Renders tabs visually; no buffer switch logic |
| `SidebarView` / `FileTreeView` | Shell exists; reads no real directory |
| `GutterView` | Working (line numbers) |
| `TextAreaView` | Renders text + caret; selection rendering has visual bug |
| `StatusBarView` | Shows position, language, line count |
| `DialogView` | Modal overlay exists; no "save before close?" wiring |
| `CommandPaletteView` | Fuzzy search overlay exists |

---

## Table Stakes

Features users expect from any code editor. Missing these = editor is not functional.

### 1. File Open / Save

**What users take for granted:**
- `Ctrl+S` saves silently with no dialog if the file has an existing path. No prompt, no confirmation, just saves. The dirty indicator on the tab disappears.
- `Ctrl+Shift+S` opens Save As dialog (native OS file picker).
- `Ctrl+O` opens an OS-native file picker dialog.
- When opening a file, a new tab appears and the file renders immediately.
- If the user closes a tab with unsaved changes, a "Save / Don't Save / Cancel" dialog appears — not before, only on close.
- A dot or bullet in the tab title signals unsaved changes (VS Code: circle, Zed: dot).
- `Ctrl+N` creates a new untitled buffer with a placeholder title like "Untitled" or "untitled-1".
- On exit with dirty buffers, the app prompts to save all / discard all / cancel — not one by one.

**What exists vs. what is new work:**
- `Save`, `SaveAs`, `Open`, `New` commands exist but are stubs — no filesystem read/write
- `DialogView` overlay exists but is not wired to save-before-close logic
- TabBarView dirty indicator needs data from buffer state (exists in view model)
- Actual file I/O requires async disk read/write (tokio or std::fs)
- OS file picker requires a native dialog crate (e.g., `rfd` — Rusty File Dialogs)

| Behavior | Complexity | Dependency |
|----------|------------|------------|
| Ctrl+S write file to disk | Low | std::fs or tokio::fs |
| OS file picker (Open, Save As) | Low | `rfd` crate |
| New buffer with untitled placeholder | Low | Tab + buffer management |
| Dirty indicator per tab | Low | TabBarView + buffer dirty state |
| Save-before-close dialog | Medium | DialogView wiring + close flow |
| Exit prompt for all dirty buffers | Medium | App shutdown flow |

**Overall complexity: Medium**

---

### 2. Sidebar File Browser / File Tree

**What users take for granted:**
- Clicking a folder in the sidebar shows a file tree rooted at that folder.
- Files in the tree are grouped: folders first (expanded), then files, alphabetically.
- Click a file opens it in a new tab (or switches to existing tab if already open).
- Arrow keys navigate the tree; Enter opens the selected item; right arrow expands a folder.
- The currently open file is highlighted in the tree.
- Folder expand/collapse is persisted for the session (collapsed stays collapsed on re-render).
- Nested folders indent visually; icons distinguish file types by extension.
- A "Open Folder" button at the top (or drag-and-drop) sets the workspace root.

**What exists vs. what is new work:**
- `FileTreeView` and `SidebarView` exist as visual shells
- `FileTreeView` renders static mock data — needs real `std::fs::read_dir` traversal
- No "current file highlight" logic wired to active tab
- No keyboard navigation in file tree (arrow keys, Enter)
- Tree expand/collapse state needs to persist in view state (not re-collapsed every render)

| Behavior | Complexity | Dependency |
|----------|------------|------------|
| Read real directory with `std::fs::read_dir` | Low | Filesystem |
| Folder expand/collapse (click + persist) | Low | FileTreeView state |
| Click to open file in tab | Medium | Tab + buffer management |
| Current file highlighting | Low | Active buffer state |
| Arrow key navigation | Medium | Focus + keyboard routing in FileTreeView |
| File type icons by extension | Low | Icon set or text fallback |
| Open Folder dialog | Low | `rfd` crate |

**Overall complexity: Medium**

---

### 3. Multi-Tab Editing with Buffer Management

**What users take for granted:**
- Each open file is a separate tab with its own scroll position, cursor position, and undo history.
- Clicking a tab switches the editor area to that buffer — instantly.
- `Ctrl+W` closes the current tab. If dirty, triggers save prompt.
- Middle-click closes a tab directly.
- `Ctrl+Tab` cycles forward through open tabs; `Ctrl+Shift+Tab` cycles backward.
- `Ctrl+1` through `Ctrl+9` jump to the nth tab.
- Opening a file that's already in a tab switches to that tab rather than opening a duplicate.
- Closing the last tab shows a blank editor or an untitled buffer, not a crash.
- Tabs overflow gracefully: if more tabs than fit, the tab bar scrolls or shows a dropdown.
- Each tab shows the filename (not full path) with a tooltip showing the full path on hover.

**What exists vs. what is new work:**
- `TabBarView` renders tabs but no buffer switch logic is wired
- `TextAreaView` renders one fixed document — needs to render the active buffer
- No buffer registry / tab manager (who owns the list of open buffers?)
- No `Ctrl+W` close handling
- No deduplication check (opening same file twice)

| Behavior | Complexity | Dependency |
|----------|------------|------------|
| Buffer registry (list of open tabs + active index) | Medium | New state in app |
| Switch active buffer on tab click | Medium | TabBarView to buffer registry |
| `Ctrl+W` close tab | Medium | Close flow + DialogView |
| Middle-click close | Low | Mouse button event in TabBarView |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle | Low | Keyboard shortcut + buffer registry |
| Deduplication on open | Low | Path comparison in buffer registry |
| Per-tab scroll + cursor position | Low | Already stored per document in core_editor |
| Tab overflow (scrollable tab bar) | Medium | TabBarView layout |

**Overall complexity: Medium-High**
**Key insight: A buffer registry is the central new data structure for v2. Everything else depends on it.**

---

### 4. Find / Replace

**What users take for granted:**
- `Ctrl+F` opens a find bar **inline at the top of the editor area** (not a modal dialog, not a floating window). The find bar overlays the top of the text area.
- Typing immediately starts searching; all matches in the document are highlighted.
- The match count is shown: "3 of 47 matches".
- `Enter` or `F3` jumps to the next match. `Shift+Enter` or `Shift+F3` jumps to the previous.
- `Escape` closes the find bar and removes highlights; cursor stays at the last match position.
- Toggle buttons in the find bar: case-sensitive (Aa), whole word (W), regex (.*).
- `Ctrl+H` opens the replace bar — same as find bar but with a second row for replace text.
- "Replace" replaces the current match and advances. "Replace All" replaces every match.
- Find wraps around: going past the last match goes back to the first (with a "Wrapped" indicator).
- Matches visible in the scrollable viewport are highlighted even when not the active match.

**What exists vs. what is new work:**
- Nothing. Find/Replace is entirely new.
- No search state in core_editor, no search UI, no match highlighting in TextAreaView.
- Requires: search engine (plain text + regex), match positions, overlay UI, keyboard routing.
- Reuse pattern: `CommandPaletteView` already demonstrates the overlay + text input + Escape-to-close + focus-routing pattern. `FindBarView` should follow that same pattern, not reinvent it.

| Behavior | Complexity | Dependency |
|----------|------------|------------|
| Find bar UI (inline overlay at top of TextArea) | Medium | New FindBarView |
| Plain-text search in buffer | Low | str::find or similar |
| Regex search | Medium | `regex` crate |
| Match highlighting in TextAreaView | Medium | Render pass for match spans |
| Match count + navigation (next/prev) | Medium | Search state + cursor jump |
| Wrap-around with indicator | Low | Search state |
| Case-sensitive, whole-word toggles | Low | Search options flags |
| Replace current + Replace All | Medium | Buffer mutation via EditorCommand |
| Keyboard routing (Ctrl+F, Escape, Enter) | Low | Existing action registry |

**Overall complexity: High** (most new code, touches TextAreaView rendering, needs new state)

---

### 5. Text Selection + Clipboard

**What users take for granted:**
- Click positions the cursor (no selection).
- Click and drag selects text. Selection renders as a colored background behind the text.
- Double-click selects the word under cursor.
- Triple-click selects the entire line.
- `Shift+Click` extends the selection from the current anchor point.
- `Shift+Arrow` extends selection one character in the direction pressed.
- `Ctrl+Shift+Right` / `Ctrl+Shift+Left` extends selection by word.
- `Shift+Home` / `Shift+End` extends selection to start/end of line.
- `Shift+Ctrl+Home` / `Shift+Ctrl+End` extends to start/end of document.
- `Ctrl+A` selects all text.
- `Ctrl+C` copies selected text to the OS clipboard.
- `Ctrl+X` cuts selected text (copies + deletes).
- `Ctrl+V` pastes at the cursor position, replacing any selection.
- Typing while text is selected replaces the selection.
- Pressing Delete or Backspace with selection active deletes the selection.
- When the selection spans multiple lines, the selection background wraps correctly across lines.

**Known bug:** Shift+Arrow causes text to visually disappear. This is the first thing to fix.

**What exists vs. what is new work:**
- `MoveCursor { extend_selection: true }` command exists
- `SelectAll`, `ClearSelection`, `DeleteSelection` commands exist
- `Copy`, `Cut`, `Paste` commands exist but are NOT wired to OS clipboard
- Selection range is stored in core_editor view state
- Selection rendering in `TextAreaView` has a visual bug (text disappears on selection)
- Mouse drag selection requires tracking mouse-down anchor + mouse-move positions
- OS clipboard requires a platform integration crate (e.g., `arboard`)

| Behavior | Complexity | Dependency |
|----------|------------|------------|
| Fix shift+arrow visual bug | Medium | TextAreaView selection rendering |
| Mouse click to position cursor | Low | Requires pixel-to-line/col mapping |
| Mouse drag to extend selection | Medium | Mouse event tracking + pixel-to-col mapping |
| Double-click word select | Low | Word boundary logic |
| Triple-click line select | Low | Line boundary |
| Shift+click extend | Low | Anchor tracking |
| Multi-line selection rendering | Medium | TextAreaView painting fix |
| OS clipboard (Ctrl+C/X/V) | Medium | `arboard` crate integration |
| Replace selection on type | Low | Already handled by InsertChar if selection is active |

**Overall complexity: Medium**
**Pixel-to-line/col mapping is a shared dependency for both mouse click and mouse drag — must be built once and reused.**

---

### 6. Performance Patterns

**What users take for granted (they don't think about this, they just expect it to feel fast):**
- Opening a 50,000-line file is instant (under 1 second).
- Scrolling never stutters. Frame rate is stable even in large files.
- Typing has zero perceived latency (keypress to character on screen in the same frame).
- Syntax highlighting doesn't cause visible lag when typing.
- Switching tabs is instant.
- The editor does not increase memory usage indefinitely as files are edited.

**What the current architecture needs to maintain this:**
- Viewport-only rendering: only render lines visible in the scroll window. The current `TextAreaView` should only pass visible lines to the rendering pipeline — not all 50K.
- Dirty tracking: when only the cursor moves, don't re-run syntax highlighting. When only one line changes, only re-parse that region.
- Layout caching: tab bar height, gutter width, status bar height do not change between frames. Cache the layout result.
- Text atlas management: glyphon's atlas must not thrash. Avoid re-creating the font system on each frame.
- Background syntax highlighting: for large files, tree-sitter parsing should happen off the UI thread (incremental re-parse only).

**What exists vs. what is new work:**
- Viewport culling: partially done (scroll offset applied), but confirmation needed that only visible lines are rendered
- Dirty tracking: not implemented — every frame re-renders everything
- Background tree-sitter: not implemented — runs synchronously on the UI thread

| Behavior | Complexity | Notes |
|----------|------------|-------|
| Viewport-only line rendering | Medium | Verify + enforce visible range only |
| Frame-level dirty tracking | Medium | Skip re-render if nothing changed |
| Layout result caching | Low | Cache heights/widths that don't change |
| Background tree-sitter parsing | High | Requires threading + incremental parsing |
| Memory cap on text atlas | Medium | glyphon atlas eviction policy |

**Overall complexity: Medium-High (ongoing concern, not a single feature)**

---

## Differentiators

Features that are not expected by default but add meaningful value. These are "nice to have" for v2.

| Feature | Value Proposition | Complexity | When to Build |
|---------|-------------------|------------|---------------|
| Find across files (Ctrl+Shift+F) | Search all open files or workspace directory | High | Post-v2 |
| Regex find/replace | Power user feature; VS Code has it, users expect it | Medium | Include in Find/Replace phase |
| Multi-cursor editing | Ctrl+D to add cursor at next match; column selection | High | Post-v2 |
| File tree right-click context menu | Rename, delete, new file, new folder | Medium | Post-v2 |
| Drag to reorder tabs | Click and drag tab to reorder; common in all editors | Medium | Post-v2 |
| Recently opened files list | Ctrl+R or similar; fast re-open without file picker | Low | v2 stretch goal |
| Diff between unsaved + disk (dirty indicator tooltip) | Hover dirty indicator shows "N unsaved changes" | Low | v2 stretch goal |
| Go to line (Ctrl+G) | Jump to line N; standard in all editors | Low | Include in v2 |
| Go to definition stub (F12) | Shows "not yet supported" message; reserves the shortcut | Low | v2 with LSP prep |

---

## Anti-Features for v2

Features to explicitly NOT build in v2. These are common scope creep traps.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Find across files (Ctrl+Shift+F) | Requires directory indexing, result panel UI, progress indication — separate project | Defer to v3 with LSP work |
| Multi-cursor editing | Complex interaction model; own selection state machine; not a v2 blocker | Single cursor only in v2 |
| File tree rename/delete/new | Context menus + confirmation dialogs + undo for FS ops | Defer; sidebar is read-navigate-open in v2 |
| Split panes / editor groups | Separate layout system, separate scroll/focus state per pane | Single editor pane in v2 |
| Tab reordering via drag | Drag-and-drop API is complex; not blocking for v2 | Fixed tab order in v2 |
| Bracket matching / auto-close | Useful but not blocking; can be added incrementally | Post-v2 |
| Auto-indent (smart indentation) | Requires language-aware logic; tree-sitter needed | Post-v2 |
| Minimap / code outline | Nice but not a v2 blocker; expensive to render accurately | Post-v2 |
| LSP integration (completions, hover docs) | Entire separate milestone; protocol + server management | v3 |
| Git blame / diff in gutter | Requires git process management + diff parsing | v3 |
| Terminal panel integration | Separate PTY management; unrelated to editor functionality | v3 |

---

## Feature Dependencies

```
Buffer Registry (new, central data structure)
  -> Multi-Tab Editing (switch active buffer)
  -> File Open/Save (open = add to registry, save = persist buffer)
  -> Sidebar File Tree (click = add to registry or switch to existing)
  -> Save-Before-Close (check registry for dirty buffers)

Pixel-to-Line/Col Mapping (new utility)
  -> Mouse Click to Position Cursor
  -> Mouse Drag to Select
  -> Find/Replace (click match to jump)

OS Clipboard Integration (arboard crate)
  -> Copy (Ctrl+C)
  -> Cut (Ctrl+X)
  -> Paste (Ctrl+V)

Selection Visual Rendering Fix (TextAreaView)
  -> All selection-dependent features (clipboard, replace-on-type)
  -> Find/Replace match highlighting (same rendering path)

OS File Dialog (rfd crate)
  -> File Open (Ctrl+O)
  -> Save As (Ctrl+Shift+S)
  -> Open Folder (sidebar)

Find/Replace (standalone new subsystem)
  -> FindBarView (UI)
  -> Search engine (text/regex matching)
  -> Match highlight rendering in TextAreaView
```

**Critical Path for v2:**
Buffer Registry must be the first new thing built. Everything else in v2 routes through it.

**Quickest win:**
File save (`Ctrl+S` to write to disk) is one function call once a buffer has a path. Do this early to give the editor real utility fast.

---

## MVP Recommendation for v2

### Must Have (core functionality)

1. **Buffer Registry** — the new central data structure; enables tabs, file ops, tree navigation
2. **File Open/Save** — `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`; real filesystem read/write
3. **Save-Before-Close dialog** — wired to existing `DialogView`; triggered on `Ctrl+W` and exit
4. **Multi-Tab switching** — click to switch, `Ctrl+W` close, `Ctrl+Tab` cycle
5. **Sidebar file tree (real data)** — read real directory, click to open file
6. **Selection rendering fix** — fix visual bug first before building anything on top of selections
7. **OS clipboard** — `Ctrl+C`, `Ctrl+X`, `Ctrl+V` working via `arboard`
8. **Mouse click to position cursor** — pixel-to-line/col mapping
9. **Mouse drag selection** — click + drag selects text
10. **Find bar (Ctrl+F)** — inline find bar, next/prev, match highlighting

### Should Have (round out the experience)

11. **Regex find** — toggle in find bar
12. **Replace / Replace All** — second row in find bar
13. **Go to line (Ctrl+G)** — via command palette or dedicated shortcut
14. **Open Folder** — sets workspace root for sidebar
15. **Dirty tracking (performance)** — skip re-render when nothing changed

### Defer to Post-v2

- Find across files
- Multi-cursor editing
- File tree file management (rename, delete, new)
- Split panes
- All LSP features

---

## Sources

All findings derived from direct codebase analysis (`core_editor` commands, `ora` views, existing `EditorCommand` enum) plus established editor conventions (VS Code, Zed, Sublime Text). Editor UX conventions in this space are stable and well-documented in the editors themselves; no web search required. Confidence is HIGH.

| Area | Confidence | Basis |
|------|------------|-------|
| What commands exist | HIGH | Read `editor_command.rs` directly |
| What views exist | HIGH | Read `ora/src/views/` directly |
| Table stakes UX expectations | HIGH | Stable conventions across VS Code / Zed / Sublime Text |
| Complexity estimates | MEDIUM | Judgment based on what exists vs. what is new code |
| Anti-features list | HIGH | Clear scope boundaries from existing v1 decisions |
