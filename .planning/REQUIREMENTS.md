# Requirements: ora UI Framework

**Defined:** 2026-01-28
**Core Value:** A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Core Framework

- [x] **CORE-01**: ora crate provides `ora::run(app)` entry point that owns the winit event loop and application lifecycle
- [x] **CORE-02**: Single window creation with wgpu surface setup, resize handling, and vsync
- [ ] **CORE-03**: Entity/Model reactive state system — state lives in `Model<T>`, views observe models and re-render on change
- [ ] **CORE-04**: Effect queue system — `cx.notify()` and `cx.emit()` queue effects, flushed after update completes (prevents reentrancy)
- [x] **CORE-05**: Context types provide scoped access to application state (`AppContext`, `ViewContext`, `WindowContext`)

### View System

- [x] **VIEW-01**: `View` trait with `render(&mut self, cx: &mut ViewContext) -> impl Element` method
- [x] **VIEW-02**: Framework manages element tree lifecycle — reconstructs tree on state change, diffs for efficient re-rendering
- [ ] **VIEW-03**: Views can observe `Model<T>` entities and automatically re-render when observed state changes
- [ ] **VIEW-04**: Views can subscribe to typed events emitted by entities

### Element System

- [x] **ELEM-01**: `Element` trait with three-phase lifecycle: `request_layout()`, `prepaint()`, `paint()`
- [ ] **ELEM-02**: Styled `Div` primitive — GPU-rendered rectangle with background, border, padding, margin, border-radius
- [ ] **ELEM-03**: Styled `Text` primitive — `Text::new("hello").size(14)` API, framework manages glyphon internally
- [x] **ELEM-04**: Elements can have children (composable tree structure)
- [ ] **ELEM-05**: Tailwind-style builder API — `div().flex().gap(4).bg(color).padding(8)` fluent syntax
- [ ] **ELEM-06**: Automatic scissor clipping — framework manages clip rects per element, children cannot render outside parent bounds

### Layout

- [ ] **LAYOUT-01**: Simple stack/flex layout engine — row and column containers with basic alignment (start, center, end, stretch)
- [ ] **LAYOUT-02**: Gap support — spacing between children in row/column containers
- [ ] **LAYOUT-03**: Flex-grow and flex-shrink — proportional space distribution among children
- [ ] **LAYOUT-04**: Fixed and percentage sizing — elements can specify width/height as fixed pixels or percentage of parent
- [ ] **LAYOUT-05**: Padding and margin — per-element spacing that affects layout calculations

### Rendering

- [ ] **REND-01**: ora owns wgpu Instance, Device, Queue, Surface — creates and manages GPU resources
- [ ] **REND-02**: Three-phase rendering pipeline — layout computation, prepaint (hitbox registration), paint (scene construction)
- [ ] **REND-03**: Text rendering via wrapped glyphon — single FontSystem, TextAtlas, SwashCache managed by framework
- [ ] **REND-04**: Single-batch text rendering — all text from all views collected and rendered in one glyphon prepare/render pass
- [ ] **REND-05**: Draw call batching — all rectangles batched into minimal draw calls using instanced rendering
- [ ] **REND-06**: Render pass structure — UI backgrounds, editor content, UI text, overlays as ordered phases

### Events

- [ ] **EVT-01**: Mouse event routing — hit testing against element tree, dispatch click/hover/scroll to correct element
- [ ] **EVT-02**: Keyboard event system — platform-agnostic key translation, action registry for shortcuts
- [ ] **EVT-03**: Focus management — focus stack with enter/exit callbacks, keyboard events routed to focused element
- [ ] **EVT-04**: Two-phase event dispatch — capture (root→target) then bubble (target→root), with stop_propagation
- [ ] **EVT-05**: Automatic hover/active state tracking — framework tracks mouse position, elements query `is_hovered`, `is_active`

### Design System

- [ ] **DS-01**: Consolidated color token system — semantic roles (BgPrimary, FgSecondary, AccentPrimary, etc.) as single source of truth
- [ ] **DS-02**: Spacing token system — named spacing values (xs=4, sm=8, md=12, lg=16, xl=24) enforced through API
- [ ] **DS-03**: Typography scale — semantic text sizes (body=14, small=12, code=14, heading=18) with line height multipliers
- [ ] **DS-04**: Theme system — dark and light palettes, switchable at runtime via token palette swap
- [ ] **DS-05**: All design tokens defined in ora crate — no tokens in wgpu_client, single import for all styling

### Transitions

- [ ] **TRANS-01**: CSS-like property transitions — animate opacity, color, background-color, position on state changes
- [ ] **TRANS-02**: Easing functions — linear, ease-in, ease-out, ease-in-out, custom cubic-bezier
- [ ] **TRANS-03**: Duration specification — transitions have configurable duration in milliseconds
- [ ] **TRANS-04**: Hover/active transitions — elements smoothly transition between default, hovered, and pressed states

### Component Migration — Editor Chrome

- [ ] **CHROME-01**: TabBar/EditorTabs migrated to ora View — tab strip with close buttons, dirty indicators, hover states
- [ ] **CHROME-02**: StatusBar/StatusLine migrated to ora View — cursor position, language, encoding, git branch display
- [ ] **CHROME-03**: Sidebar migrated to ora View — collapsible file explorer panel with header and content area
- [ ] **CHROME-04**: Gutter migrated to ora View — line numbers with current line highlight
- [ ] **CHROME-05**: Dialog migrated to ora View — modal overlay for save/discard prompts

### Component Migration — Text Editing Core

- [ ] **EDIT-01**: TextArea migrated to ora View — syntax-highlighted text rendering with selection backgrounds
- [ ] **EDIT-02**: Caret migrated to ora element — blinking cursor with configurable blink rate
- [ ] **EDIT-03**: Selection rendering migrated to ora — background highlights for selected text ranges

### Component Migration — Advanced UI

- [ ] **UI-01**: CommandPalette migrated to ora View — searchable command overlay with filtered results
- [ ] **UI-02**: FileTree migrated to ora View — hierarchical file navigation with expand/collapse, icons
- [ ] **UI-03**: PanelManager migrated to ora View — bottom panel system with tabs (output, problems)
- [ ] **UI-04**: AppLayout migrated to ora — main layout orchestrator that computes bounds for all regions

### Component Migration — Widgets

- [ ] **WIDGET-01**: Button widget implemented in ora — with size tiers, hover/active states, label + optional icon
- [ ] **WIDGET-02**: Input widget implemented in ora — text input with placeholder, focus state, selection
- [ ] **WIDGET-03**: Checkbox and Toggle widgets in ora — boolean controls with checked/unchecked states
- [ ] **WIDGET-04**: ListItem widget in ora — selectable list row with label, optional icon, hover state
- [ ] **WIDGET-05**: Tab widget in ora — individual tab with active/inactive states, close button
- [ ] **WIDGET-06**: TreeItem widget in ora — tree node with expand/collapse chevron, indentation
- [ ] **WIDGET-07**: ContextMenu widget in ora — popup menu with items, separators, keyboard navigation
- [ ] **WIDGET-08**: Toast widget in ora — temporary notification with auto-dismiss

### Integration

- [ ] **INT-01**: wgpu_client reduced to thin app shell — creates ora App, registers root View, calls `ora::run()`
- [ ] **INT-02**: core_editor integration — ora views consume RenderModel from core_editor's ViewModelBuilder
- [ ] **INT-03**: EditorCommand dispatch — keyboard events translated to EditorCommand, dispatched to core_editor App
- [ ] **INT-04**: All hardcoded values eliminated — no raw float literals for heights, widths, font sizes, padding in wgpu_client
- [ ] **INT-05**: All duplicated styling logic removed — single layout calculation path per component

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Layout Engine

- **LAYOUT-V2-01**: Full CSS flexbox via taffy — flex-wrap, align-content, order, flex-basis
- **LAYOUT-V2-02**: CSS Grid layout support via taffy — grid-template, grid-area, named lines

### Animation

- **ANIM-V2-01**: Keyframe animation system — multi-step animations with per-keyframe easing
- **ANIM-V2-02**: Spring physics animations — natural motion for panel slides, scroll deceleration

### Advanced Features

- **ADV-V2-01**: Multi-window support — secondary windows for split views, settings
- **ADV-V2-02**: Drag-and-drop API — tab reordering, file tree drag, cross-component drops
- **ADV-V2-03**: Virtualized scrolling — framework-level list virtualization for large datasets
- **ADV-V2-04**: Accessibility/screen reader — platform-specific APIs (VoiceOver, NVDA)
- **ADV-V2-05**: Hot-reload themes — runtime theme switching without recompile
- **ADV-V2-06**: Layout debug overlay — browser-like element inspector with bounds visualization

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Web rendering target | wgpu-web requires WASM, different event model; entirely different ecosystem |
| Custom shader API | Exposes wgpu internals, breaks abstraction; styled primitives cover 99% of editor UI |
| TUI/ratatui backend | ora is GPU-only; ratatui_client has fundamentally different rendering semantics |
| Component marketplace/plugins | ora is internal to muda; no external consumers; premature abstraction |
| Responsive breakpoints | Editor layout is manual (user drags dividers); no automatic "mobile view" |
| Rich animation timeline | Editors need transitions, not After Effects; keyframes deferred to v2 |
| Undo/redo for UI state | Document undo lives in core_editor; UI state doesn't need undo |
| Gesture recognition | Editors are keyboard-first; basic mouse events sufficient |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 1 | Complete |
| CORE-02 | Phase 1 | Complete |
| CORE-03 | Phase 3 | Pending |
| CORE-04 | Phase 3 | Pending |
| CORE-05 | Phase 1 | Complete |
| VIEW-01 | Phase 1 | Complete |
| VIEW-02 | Phase 1 | Complete |
| VIEW-03 | Phase 3 | Pending |
| VIEW-04 | Phase 3 | Pending |
| ELEM-01 | Phase 1 | Complete |
| ELEM-02 | Phase 2 | Pending |
| ELEM-03 | Phase 2 | Pending |
| ELEM-04 | Phase 1 | Complete |
| ELEM-05 | Phase 5 | Pending |
| ELEM-06 | Phase 2 | Pending |
| LAYOUT-01 | Phase 2 | Pending |
| LAYOUT-02 | Phase 2 | Pending |
| LAYOUT-03 | Phase 2 | Pending |
| LAYOUT-04 | Phase 2 | Pending |
| LAYOUT-05 | Phase 2 | Pending |
| REND-01 | Phase 2 | Pending |
| REND-02 | Phase 2 | Pending |
| REND-03 | Phase 2 | Pending |
| REND-04 | Phase 2 | Pending |
| REND-05 | Phase 2 | Pending |
| REND-06 | Phase 2 | Pending |
| EVT-01 | Phase 4 | Pending |
| EVT-02 | Phase 4 | Pending |
| EVT-03 | Phase 4 | Pending |
| EVT-04 | Phase 4 | Pending |
| EVT-05 | Phase 4 | Pending |
| DS-01 | Phase 6 | Pending |
| DS-02 | Phase 6 | Pending |
| DS-03 | Phase 6 | Pending |
| DS-04 | Phase 6 | Pending |
| DS-05 | Phase 6 | Pending |
| TRANS-01 | Phase 9 | Pending |
| TRANS-02 | Phase 9 | Pending |
| TRANS-03 | Phase 9 | Pending |
| TRANS-04 | Phase 9 | Pending |
| CHROME-01 | Phase 7 | Pending |
| CHROME-02 | Phase 7 | Pending |
| CHROME-03 | Phase 7 | Pending |
| CHROME-04 | Phase 7 | Pending |
| CHROME-05 | Phase 7 | Pending |
| EDIT-01 | Phase 7 | Pending |
| EDIT-02 | Phase 7 | Pending |
| EDIT-03 | Phase 7 | Pending |
| UI-01 | Phase 8 | Pending |
| UI-02 | Phase 8 | Pending |
| UI-03 | Phase 8 | Pending |
| UI-04 | Phase 8 | Pending |
| WIDGET-01 | Phase 8 | Pending |
| WIDGET-02 | Phase 8 | Pending |
| WIDGET-03 | Phase 8 | Pending |
| WIDGET-04 | Phase 8 | Pending |
| WIDGET-05 | Phase 8 | Pending |
| WIDGET-06 | Phase 8 | Pending |
| WIDGET-07 | Phase 8 | Pending |
| WIDGET-08 | Phase 8 | Pending |
| INT-01 | Phase 9 | Pending |
| INT-02 | Phase 9 | Pending |
| INT-03 | Phase 9 | Pending |
| INT-04 | Phase 9 | Pending |
| INT-05 | Phase 9 | Pending |

**Coverage:**
- v1 requirements: 58 total
- Mapped to phases: 58
- Unmapped: 0 (100% coverage)

---
*Requirements defined: 2026-01-28*
*Last updated: 2026-01-28 after roadmap creation with phase mappings*
