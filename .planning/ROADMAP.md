# Roadmap: ora UI Framework / muda Functional Editor

## Overview

**v1.0 (Complete):** Built ora as a standalone GPUI-inspired UI framework crate that owns the GPU rendering pipeline, provides reactive state management, and delivers a unified component library for the muda code editor. Phases 1-9 delivered app lifecycle, layout, reactive state, events, design system, editor chrome, advanced UI, GPU layered rendering, and transitions — all complete.

**v2.0 (Active):** Transform muda from a visual shell into a functional code editor. Fix foundation issues (event-driven redraw, selection rendering, command audit), build a Buffer Registry as the central data structure, then layer file operations, multi-tab editing, sidebar navigation, selection/clipboard, find/replace, and performance optimization. The result: a real code editor with Zed-level rendering performance.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

### v1.0 Phases (Complete)

- [x] **Phase 1: Foundation & View System** - App lifecycle, Entity/Model, View trait, Element trait basics
- [x] **Phase 2: Layout & Rendering Pipeline** - Simple stack/flex layout, GPU batching, text rendering via glyphon
- [x] **Phase 3: Reactive State System** - Observation, effect queue, Model change tracking
- [x] **Phase 4: Event System** - Mouse/keyboard routing, focus management, two-phase dispatch
- [x] **Phase 5: Element Library** - Styled primitives (Div, Text), builder API, layout containers
- [x] **Phase 6: Design System** - Color tokens, spacing scale, typography, theme switching
- [x] **Phase 7: Editor Chrome & Text Editing** - TabBar, StatusBar, Sidebar, Gutter, Dialog, TextArea, Caret, Selection
- [x] **Phase 8: Advanced UI & Widgets** - CommandPalette, FileTree, PanelManager, Button, Input, Checkbox, etc.
- [x] **Phase 8.1: GPU Layered Rendering Pipeline** - Fix overlay z-ordering so overlays correctly occlude lower-layer text (INSERTED)
- [x] **Phase 9: Transitions & Integration** - CSS-like animations, wgpu_client migration, pixel-based scroll, scissor clipping

### v2.0 Phases (Active)

- [x] **Phase 10: Foundation Fixes** - Event-driven redraw, command audit, EditorDataSource redesign, selection rendering fix
- [x] **Phase 11: Buffer Registry + Multi-Tab** - Central buffer deduplication, tab switching, Ctrl+W close, Ctrl+Tab cycle
- [x] **Phase 12: File Operations** - Ctrl+O open, Ctrl+S save, Ctrl+Shift+S Save As, Ctrl+N new, async I/O
- [ ] **Phase 13: Selection + Clipboard** - Click-to-position, drag selection, Shift+arrow, Ctrl+A, Ctrl+C/X/V
- [ ] **Phase 14: File Browser** - Real directory tree, click to open, expand/collapse state, file watcher
- [ ] **Phase 15: Find / Replace** - Inline find bar, match highlighting, next/prev, replace one/all, Go to line
- [ ] **Phase 16: Performance Refinement** - Incremental tree-sitter parsing, glyphon buffer caching, background parse thread

## Phase Details

### Phase 1: Foundation & View System
**Goal**: Establish ora crate with application lifecycle ownership, centralized entity storage, and declarative view/element model
**Depends on**: Nothing (first phase)
**Requirements**: CORE-01, CORE-02, CORE-05, VIEW-01, VIEW-02, ELEM-01, ELEM-04
**Success Criteria** (what must be TRUE):
  1. ora::run(app) entry point owns the winit event loop and creates a window with wgpu surface
  2. App context owns all entity data in centralized storage, Entity<T> handles provide typed access
  3. Views can define render() methods that return element trees
  4. Elements have three-phase lifecycle (request_layout, prepaint, paint) with composable children
  5. Window can register a root view and trigger render passes
**Plans**: 3 plans

Plans:
- [x] 01-01-PLAN.md -- App lifecycle + GPU pipeline + Entity storage (Wave 1)
- [x] 01-02-PLAN.md -- View/Element traits + Context types (Wave 2)
- [x] 01-03-PLAN.md -- Window integration + stub rendering + verification (Wave 3)

### Phase 2: Layout & Rendering Pipeline
**Goal**: Implement simple stack/flex layout engine, GPU rendering pipeline with batching, and glyphon text rendering
**Depends on**: Phase 1
**Requirements**: LAYOUT-01, LAYOUT-02, LAYOUT-03, LAYOUT-04, LAYOUT-05, REND-01, REND-02, REND-03, REND-04, REND-05, REND-06, ELEM-02, ELEM-03, ELEM-06
**Success Criteria** (what must be TRUE):
  1. ora uses simple stack/flex layout (row/column with alignment, gap, flex-grow/shrink) NOT taffy
  2. Elements can specify padding, margin, fixed sizing, and percentage sizing
  3. Div primitive renders GPU rectangles with background, border, border-radius
  4. Text primitive renders via wrapped glyphon with single-batch text rendering across all views
  5. Rendering uses three-phase pipeline (layout computation, prepaint hitbox registration, paint scene construction)
  6. GPU batching groups all rectangles into minimal draw calls using instanced rendering
  7. Automatic scissor clipping prevents children from rendering outside parent bounds
**Plans**: 6 plans

Plans:
- [x] 02-01-PLAN.md -- Style types & layout data structures (Wave 1)
- [x] 02-02-PLAN.md -- Flexbox layout algorithm (Wave 2)
- [x] 02-03-PLAN.md -- Rectangle GPU pipeline + shaders (Wave 2)
- [x] 02-04-PLAN.md -- Text rendering system + glyphon integration (Wave 2)
- [x] 02-05-PLAN.md -- Div + Text elements with layout integration (Wave 3)
- [x] 02-06-PLAN.md -- Scissor clipping, render pass integration, event loop updates, demo (Wave 4)

### Phase 3: Reactive State System
**Goal**: Add observation mechanism, effect queue, and Model change tracking for reactive UI updates
**Depends on**: Phase 2
**Requirements**: CORE-03, CORE-04, VIEW-03, VIEW-04
**Success Criteria** (what must be TRUE):
  1. State can live in Model<T>, views observe models and automatically re-render on change
  2. cx.notify() and cx.emit() queue effects, which flush after update completes (prevents reentrancy)
  3. Views can subscribe to typed events emitted by entities
  4. Multiple views can observe the same Model<T> and all re-render when it changes
**Plans**: 4 plans

Plans:
- [x] 03-01-PLAN.md -- Effect queue + Model<T> wrapper + ModelContext (Wave 1)
- [x] 03-02-PLAN.md -- Observer registry + notify-flush cycle + AppContext persistence (Wave 2)
- [x] 03-03-PLAN.md -- Typed event subscriptions + global event bus + Drop cleanup (Wave 3)
- [x] 03-04-PLAN.md -- cx.spawn() + async executor + dirty rendering + reactive demo (Wave 4)

### Phase 4: Event System
**Goal**: Implement mouse/keyboard event routing, focus management, and two-phase dispatch
**Depends on**: Phase 3
**Requirements**: EVT-01, EVT-02, EVT-03, EVT-04, EVT-05
**Success Criteria** (what must be TRUE):
  1. Mouse events route to correct element via hit testing against prepaint hitboxes
  2. Keyboard events translate to platform-agnostic actions via action registry
  3. Focus stack tracks focused element, keyboard events route to focused element
  4. Events dispatch in two phases (capture root->target, bubble target->root) with stop_propagation support
  5. Framework automatically tracks hover and active states, elements query is_hovered/is_active
**Plans**: 5 plans

Plans:
- [x] 04-01-PLAN.md -- Event types & hit testing infrastructure (Wave 1)
- [x] 04-02-PLAN.md -- Focus system with FocusHandle and tab navigation (Wave 1)
- [x] 04-03-PLAN.md -- Two-phase dispatch (capture/bubble) and event routing (Wave 2)
- [x] 04-04-PLAN.md -- Action registry and keyboard keybindings (Wave 2)
- [x] 04-05-PLAN.md -- Hover/active tracking and interactive demo (Wave 3)

### Phase 5: Element Library
**Goal**: Build reusable styled primitives with Tailwind-style builder API and layout containers
**Depends on**: Phase 4
**Requirements**: ELEM-05
**Success Criteria** (what must be TRUE):
  1. Builder API supports fluent styling: div().flex().gap(4).bg(color).padding(8)
  2. Basic elements (div, text, image) available with composable children
  3. Layout containers (row, column, stack) handle child positioning with alignment options
  4. Interactive elements (button, input) respond to hover and active states
**Plans**: 5 plans

Plans:
- [x] 05-01-PLAN.md -- Unit functions (px, pct) and enhanced Div builder (Wave 1)
- [x] 05-02-PLAN.md -- Stack container for z-layering (Wave 1)
- [x] 05-03-PLAN.md -- Button element with variants (Wave 2)
- [x] 05-04-PLAN.md -- Image element with texture cache (Wave 2)
- [x] 05-05-PLAN.md -- Element library demo and verification (Wave 3)

### Phase 6: Design System
**Goal**: Consolidate design tokens (colors, spacing, typography) as single source of truth with theme support
**Depends on**: Phase 5
**Requirements**: DS-01, DS-02, DS-03, DS-04, DS-05
**Success Criteria** (what must be TRUE):
  1. All color tokens use semantic roles (BgPrimary, FgSecondary, AccentPrimary, etc.) defined in ora crate
  2. Spacing tokens enforce named values (xs=4, sm=8, md=12, lg=16, xl=24) through API
  3. Typography scale defines semantic text sizes (body=14, small=12, code=14, heading=18) with line heights
  4. Theme system supports dark and light palettes, switchable at runtime via palette swap
  5. All design tokens live in ora crate only, no tokens remain in wgpu_client
**Plans**: 4 plans

Plans:
- [x] 06-01-PLAN.md -- Color token foundation (palette + ColorToken enum + Theme struct) (Wave 1)
- [x] 06-02-PLAN.md -- Spacing and typography tokens (sp() function + TextSize enum) (Wave 1)
- [x] 06-03-PLAN.md -- Theme context integration (AppContext storage + set_theme + ThemeChanged) (Wave 2)
- [x] 06-04-PLAN.md -- Element migration and demo (Button theme-aware + theme toggle demo) (Wave 3)

### Phase 7: Editor Chrome & Text Editing
**Goal**: Migrate editor chrome components (TabBar, StatusBar, Sidebar, Gutter, Dialog) and text editing core (TextArea, Caret, Selection) to ora Views
**Depends on**: Phase 6
**Requirements**: CHROME-01, CHROME-02, CHROME-03, CHROME-04, CHROME-05, EDIT-01, EDIT-02, EDIT-03
**Success Criteria** (what must be TRUE):
  1. TabBar shows tabs with close buttons, dirty indicators, and hover states using ora Views
  2. StatusBar displays cursor position, language, encoding, git branch using ora Views
  3. Sidebar renders collapsible file explorer panel with header and content area using ora Views
  4. Gutter displays line numbers with current line highlight using ora Views
  5. Dialog shows modal overlays for save/discard prompts using ora Views
  6. TextArea renders syntax-highlighted text with selection backgrounds using ora Views
  7. Caret displays blinking cursor with configurable blink rate using ora elements
  8. Selection renders background highlights for selected text ranges using ora elements
**Plans**: 5 plans

Plans:
- [x] 07-01-PLAN.md -- Views module + TabBarView + StatusBarView (Wave 1)
- [x] 07-02-PLAN.md -- SidebarView + GutterView (Wave 1)
- [x] 07-03-PLAN.md -- DialogView with modal overlay (Wave 1)
- [x] 07-04-PLAN.md -- TextAreaView + CaretElement + syntax highlighting (Wave 2)
- [x] 07-05-PLAN.md -- Integration demo and verification (Wave 3)

### Phase 8: Advanced UI & Widgets
**Goal**: Migrate advanced UI components (CommandPalette, FileTree, PanelManager, AppLayout) and all widget primitives to ora
**Depends on**: Phase 7
**Requirements**: UI-01, UI-02, UI-03, UI-04, WIDGET-01, WIDGET-02, WIDGET-03, WIDGET-04, WIDGET-05, WIDGET-06, WIDGET-07, WIDGET-08
**Success Criteria** (what must be TRUE):
  1. CommandPalette displays searchable command overlay with filtered results using ora Views
  2. FileTree renders hierarchical file navigation with expand/collapse and icons using ora Views
  3. PanelManager shows bottom panel system with tabs (output, problems) using ora Views
  4. AppLayout orchestrates main layout computing bounds for all regions using ora Views
  5. Button, Input, Checkbox, Toggle, ListItem, Tab, TreeItem, ContextMenu, Toast widgets work as ora components
  6. All widgets support size tiers, hover/active states, and follow design token system
**Plans**: 8 plans

Plans:
- [x] 08-01-PLAN.md -- WidgetSize foundation + Button size tiers + Input widget (Wave 1)
- [x] 08-02-PLAN.md -- Tab + ListItem + TreeItem navigation widgets (Wave 1)
- [x] 08-03-PLAN.md -- Checkbox + Toggle boolean controls (Wave 1)
- [x] 08-04-PLAN.md -- Fuzzy match algorithm + CommandPalette view (Wave 2)
- [x] 08-05-PLAN.md -- FileTree view + Sidebar integration (Wave 2)
- [x] 08-06-PLAN.md -- PanelManager view with resize handle (Wave 2)
- [x] 08-07-PLAN.md -- ContextMenu + Toast overlay widgets (Wave 2)
- [x] 08-08-PLAN.md -- AppLayout orchestrator + integration demo (Wave 3)

### Phase 8.1: GPU Layered Rendering Pipeline (INSERTED)
**Goal**: Fix the rendering pipeline so overlay elements (command palette, dialog, toast) correctly occlude content from lower z-layers, including text rendered by those layers
**Depends on**: Phase 8
**Requirements**: REND-07 (overlay z-ordering)
**Success Criteria** (what must be TRUE):
  1. Command palette overlay fully hides editor text behind its opaque background — no text bleed-through
  2. Dialog backdrop + dialog body fully occludes all content from lower layers
  3. Toast notifications render on top of all other content without artifacts
  4. Shadow rendering produces correct Gaussian falloff (strongest at box edge, fading with distance)
  5. Single-pass rendering performance maintained (no excessive render pass overhead)
  6. Existing sidebar, editor, tab bar, panel, status bar rendering unbroken
**Plans**: 3 plans

Plans:
- [x] 08.1-01-PLAN.md -- RectangleRenderer lifetime fix + draw range support (Wave 1)
- [x] 08.1-02-PLAN.md -- TextSystem multi-layer renderer support (Wave 1)
- [x] 08.1-03-PLAN.md -- Layered multi-pass render_frame() integration (Wave 2)

### Phase 9: Transitions & Integration
**Goal**: Add CSS-like transitions for polish and complete wgpu_client migration to thin app shell
**Depends on**: Phase 8.1
**Requirements**: TRANS-01, TRANS-02, TRANS-03, TRANS-04, INT-01, INT-02, INT-03, INT-04, INT-05
**Success Criteria** (what must be TRUE):
  1. Elements can animate opacity, color, background-color, position on state changes with easing functions
  2. Hover/active transitions smoothly fade between default, hovered, and pressed states
  3. wgpu_client is reduced to thin app shell that creates ora App, registers root View, calls ora::run()
  4. ora views consume RenderModel from core_editor's ViewModelBuilder
  5. Keyboard events translate to EditorCommand and dispatch to core_editor App
  6. All hardcoded values eliminated from wgpu_client (no raw float literals for heights, widths, font sizes, padding)
  7. All duplicated styling logic removed from wgpu_client (single layout calculation path per component)
**Plans**: 9 plans

Plans:
- [x] 09-01-PLAN.md -- Port animation primitives (Tween, Easing, CubicBezier) to ora (Wave 1)
- [x] 09-02-PLAN.md -- Adapter trait + mirror presentation types in ora (Wave 1)
- [x] 09-03-PLAN.md -- Migrate views to mirror types + remove core_editor dep (Wave 2)
- [x] 09-04-PLAN.md -- TransitionRegistry + TransitionSpec infrastructure (Wave 2)
- [x] 09-05-PLAN.md -- CoreEditorAdapter + AppLayout wiring + input translation (Wave 3)
- [x] 09-06-PLAN.md -- Div transition builders + paint-time interpolation (Wave 3)
- [x] 09-07-PLAN.md -- Hover/active transitions across all interactive elements (Wave 4)
- [x] 09-08-PLAN.md -- Strip wgpu_client to thin shell + cleanup (Wave 5)
- [x] 09-09-PLAN.md -- Final verification + scroll architecture + scissor clipping (Wave 6)

---

### Phase 10: Foundation Fixes
**Goal**: Eliminate idle GPU waste, audit all v2 commands, redesign EditorDataSource for v2 scope, and fix the selection rendering bug before feature work begins
**Depends on**: Phase 9 (v1.0 complete)
**Requirements**: FIX-01, FIX-02, FIX-03, FIX-04, FIX-05
**Success Criteria** (what must be TRUE):
  1. The editor is idle at rest — GPU usage drops to near-zero when no typing or animation occurs
  2. Caret blink animates correctly without continuous frame rendering between blinks
  3. Shift+arrow key selection renders visible highlights without text disappearing
  4. All v2 keyboard commands (SwitchTab, CloseTab, OpenFile, SaveAs, New, Find, Replace) are recognized without panicking or being silently dropped
  5. EditorDataSource is split into focused sub-traits covering workspace, search, and file operations — no single trait accumulating all v2 methods
**Plans**: 4 plans

Plans:
- [x] 10-01-PLAN.md -- Event-driven redraw + caret blink timer (Wave 1)
- [x] 10-02-PLAN.md -- v2 command variants + EditorDataSource trait split (Wave 1)
- [x] 10-03-PLAN.md -- Selection rendering fix (Wave 2)
- [x] 10-04-PLAN.md -- Integration verification checkpoint (Wave 2)

### Phase 11: Buffer Registry + Multi-Tab
**Goal**: Users can open multiple files in tabs and switch between them — each buffer is deduplicated and preserves its own scroll and cursor state
**Depends on**: Phase 10
**Requirements**: TAB-01, TAB-02, TAB-03, TAB-04, TAB-05
**Success Criteria** (what must be TRUE):
  1. Opening the same file path twice reuses the existing buffer — no duplicate document instances
  2. Clicking a tab switches the editor to that buffer with scroll position and cursor preserved from last visit
  3. Pressing Ctrl+W closes the active tab; if the buffer has unsaved changes, a save-before-close dialog appears
  4. Pressing Ctrl+Tab and Ctrl+Shift+Tab cycles through open tabs in order
  5. The tab dirty indicator (dot or asterisk) appears when a buffer has unsaved changes and clears after save
**Plans**: 4 plans

Plans:
- [x] 11-01-PLAN.md -- Buffer Registry + tab ordering + MRU stack in Workspace (Wave 1)
- [x] 11-02-PLAN.md -- SwitchTab + CloseTab handlers + adapter wiring + Ctrl+Shift+Tab (Wave 2)
- [x] 11-03-PLAN.md -- Tab bar click handlers + dot dirty indicator + accent border (Wave 3)
- [x] 11-04-PLAN.md -- Integration verification checkpoint (Wave 4)

### Phase 12: File Operations
**Goal**: Users can open, save, and create files using OS-native dialogs — disk I/O never blocks the UI thread
**Depends on**: Phase 11
**Requirements**: FILE-01, FILE-02, FILE-03, FILE-04, FILE-05, FILE-06
**Success Criteria** (what must be TRUE):
  1. Pressing Ctrl+O opens the OS native file picker; selecting a file loads it into a new tab (or switches to existing tab if already open)
  2. Pressing Ctrl+S on a named file saves it silently with no dialog; on an untitled buffer, it opens Save As dialog
  3. Pressing Ctrl+Shift+S opens Save As dialog for any buffer, allowing rename or path change
  4. Pressing Ctrl+N opens a new untitled buffer ready for editing
  5. Opening a non-UTF-8 file shows a user-visible error message instead of crashing or displaying garbled text
  6. The editor remains responsive during file load — UI does not freeze on large files
**Plans**: 4 plans

Plans:
- [x] PLAN-01.md -- rfd/dirs deps, untitled naming, PendingFileOp queue, Ctrl+N (Wave 1)
- [x] PLAN-02.md -- Ctrl+O native file dialog, async I/O, UTF-8 validation, BOM strip (Wave 2)
- [x] PLAN-03.md -- Ctrl+S save, Ctrl+Shift+S Save As, last-dir persistence (Wave 3)
- [ ] 12-04-PLAN.md -- Gap closure: dialog modal input suppression + timed status messages (Wave 4)

### Phase 13: Selection + Clipboard
**Goal**: Users can select text with mouse and keyboard, and copy/cut/paste through the OS clipboard
**Depends on**: Phase 10
**Requirements**: SEL-01, SEL-02, SEL-03, SEL-04, SEL-05, SEL-06, SEL-07
**Success Criteria** (what must be TRUE):
  1. Clicking in the text area positions the cursor at the correct line and column (including clicks in the middle of a word)
  2. Clicking and dragging selects a text range that highlights as the mouse moves
  3. Shift+arrow extends or shrinks the selection one character/line at a time; Ctrl+Shift+arrow extends by word
  4. Ctrl+A selects all text in the active buffer
  5. Ctrl+C copies the selected text to the OS clipboard; the selection remains visible after copy
  6. Ctrl+X cuts the selected text to the OS clipboard; the selection is deleted from the buffer
  7. Ctrl+V pastes clipboard text at the cursor position, replacing any active selection
**Plans**: TBD

### Phase 14: File Browser
**Goal**: Users can navigate the project directory tree in the sidebar and open files by clicking — the tree reflects real filesystem state
**Depends on**: Phase 11, Phase 12
**Requirements**: SIDE-01, SIDE-02, SIDE-03, SIDE-04, SIDE-05, SIDE-06
**Success Criteria** (what must be TRUE):
  1. The sidebar shows real files and folders from the workspace root — no mock data visible
  2. Clicking a file in the sidebar opens it in the editor (routing through the Buffer Registry for deduplication)
  3. Expanding and collapsing folders persists across renders — a folder does not snap closed on the next frame
  4. Using Open Folder dialog sets a new workspace root and refreshes the sidebar tree
  5. When a file is created, deleted, or renamed externally, the sidebar updates without requiring a restart
  6. Directories like target/, .git/, and node_modules/ are excluded from automatic expansion
**Plans**: TBD

### Phase 15: Find / Replace
**Goal**: Users can search for text within the active buffer, navigate matches, and replace occurrences — all without leaving the editor
**Depends on**: Phase 10, Phase 11
**Requirements**: FIND-01, FIND-02, FIND-03, FIND-04, FIND-05, FIND-06, FIND-07, FIND-08
**Success Criteria** (what must be TRUE):
  1. Pressing Ctrl+F opens the find bar as an inline overlay at the top of the editor area
  2. Typing in the find bar highlights all matches in the text area and shows a count like "3 of 12"
  3. Pressing Enter or clicking next/prev navigates between matches, wrapping at end of file
  4. Case-sensitive, whole-word, and regex toggles change which text is matched in real time
  5. Pressing Ctrl+H expands the find bar to show a replace row; Replace One replaces the current match and advances; Replace All replaces every match
  6. Replace All executes as a single undoable Transaction — one Ctrl+Z reverts all replacements
  7. Pressing Escape closes the find bar and returns keyboard focus to the editor
  8. Pressing Ctrl+G opens a Go to Line dialog; entering a number jumps the cursor to that line
**Plans**: TBD

### Phase 16: Performance Refinement
**Goal**: Editing large files stays responsive — syntax highlighting and text reshaping work are amortized across keystrokes
**Depends on**: Phase 10, Phase 11, Phase 12, Phase 13, Phase 14, Phase 15
**Requirements**: PERF-01, PERF-02, PERF-03
**Success Criteria** (what must be TRUE):
  1. Typing in a large file (10K+ lines) does not produce visible lag — incremental tree-sitter parsing updates only the changed region
  2. Scrolling through a file does not re-shape glyphon buffers for lines that have not changed — cached buffers are reused
  3. Opening a file larger than 100KB does not freeze the UI — tree-sitter parsing runs on a background thread while the file loads
**Plans**: TBD

## Progress

**Execution Order:**
v1.0: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 8.1 -> 9
v2.0: 10 -> 11 -> 12 -> 13 (can follow 10) -> 14 (needs 11+12) -> 15 (needs 10+11) -> 16 (always last)

### v1.0 Progress (Complete)

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & View System | 3/3 | Complete | 2026-01-28 |
| 2. Layout & Rendering Pipeline | 6/6 | Complete | 2026-01-29 |
| 3. Reactive State System | 4/4 | Complete | 2026-01-29 |
| 4. Event System | 5/5 | Complete | 2026-01-29 |
| 5. Element Library | 5/5 | Complete | 2026-01-30 |
| 6. Design System | 4/4 | Complete | 2026-01-30 |
| 7. Editor Chrome & Text Editing | 5/5 | Complete | 2026-01-30 |
| 8. Advanced UI & Widgets | 8/8 | Complete | 2026-03-02 |
| 8.1. GPU Layered Rendering | 3/3 | Complete | 2026-03-02 |
| 9. Transitions & Integration | 9/9 | Complete | 2026-03-26 |

### v2.0 Progress (Active)

| Phase | Plans Complete | Status | Started |
|-------|----------------|--------|---------|
| 10. Foundation Fixes | 4/4 | Complete | 2026-03-26 |
| 11. Buffer Registry + Multi-Tab | 4/4 | Complete | 2026-03-26 |
| 12. File Operations | 3/4 | Gap closure | 2026-03-26 |
| 13. Selection + Clipboard | 0/TBD | Pending | — |
| 14. File Browser | 0/TBD | Pending | — |
| 15. Find / Replace | 0/TBD | Pending | — |
| 16. Performance Refinement | 0/TBD | Pending | — |

---
*Roadmap created: 2026-01-28*
*v1.0 complete: 2026-03-26*
*v2.0 roadmap added: 2026-03-26*
