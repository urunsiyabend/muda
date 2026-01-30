# Roadmap: ora UI Framework

## Overview

Build ora as a standalone GPUI-inspired UI framework crate that owns the GPU rendering pipeline, provides reactive state management, and delivers a unified component library for the muda code editor. Starting with core framework primitives (App lifecycle, Entity/Model system), we'll layer on rendering capabilities (simple stack/flex layout, GPU batching, text rendering), add reactivity and event handling, build the design system, and systematically migrate all existing wgpu_client UI components into the framework. The result: a single authoritative UI toolkit that eliminates duplicated styling and enforces consistent design tokens.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation & View System** - App lifecycle, Entity/Model, View trait, Element trait basics
- [x] **Phase 2: Layout & Rendering Pipeline** - Simple stack/flex layout, GPU batching, text rendering via glyphon
- [x] **Phase 3: Reactive State System** - Observation, effect queue, Model change tracking
- [x] **Phase 4: Event System** - Mouse/keyboard routing, focus management, two-phase dispatch
- [x] **Phase 5: Element Library** - Styled primitives (Div, Text), builder API, layout containers
- [x] **Phase 6: Design System** - Color tokens, spacing scale, typography, theme switching
- [ ] **Phase 7: Editor Chrome & Text Editing** - TabBar, StatusBar, Sidebar, Gutter, Dialog, TextArea, Caret, Selection
- [ ] **Phase 8: Advanced UI & Widgets** - CommandPalette, FileTree, PanelManager, Button, Input, Checkbox, etc.
- [ ] **Phase 9: Transitions & Integration** - CSS-like animations, wgpu_client migration, cleanup

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
  4. Events dispatch in two phases (capture root→target, bubble target→root) with stop_propagation support
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
- [ ] 07-01-PLAN.md -- Views module + TabBarView + StatusBarView (Wave 1)
- [ ] 07-02-PLAN.md -- SidebarView + GutterView (Wave 1)
- [ ] 07-03-PLAN.md -- DialogView with modal overlay (Wave 1)
- [ ] 07-04-PLAN.md -- TextAreaView + CaretElement + syntax highlighting (Wave 2)
- [ ] 07-05-PLAN.md -- Integration demo and verification (Wave 3)

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
**Plans**: TBD

Plans:
- (Plans will be created during /gsd:plan-phase 8)

### Phase 9: Transitions & Integration
**Goal**: Add CSS-like transitions for polish and complete wgpu_client migration to thin app shell
**Depends on**: Phase 8
**Requirements**: TRANS-01, TRANS-02, TRANS-03, TRANS-04, INT-01, INT-02, INT-03, INT-04, INT-05
**Success Criteria** (what must be TRUE):
  1. Elements can animate opacity, color, background-color, position on state changes with easing functions
  2. Hover/active transitions smoothly fade between default, hovered, and pressed states
  3. wgpu_client is reduced to thin app shell that creates ora App, registers root View, calls ora::run()
  4. ora views consume RenderModel from core_editor's ViewModelBuilder
  5. Keyboard events translate to EditorCommand and dispatch to core_editor App
  6. All hardcoded values eliminated from wgpu_client (no raw float literals for heights, widths, font sizes, padding)
  7. All duplicated styling logic removed from wgpu_client (single layout calculation path per component)
**Plans**: TBD

Plans:
- (Plans will be created during /gsd:plan-phase 9)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & View System | 3/3 | Complete | 2026-01-28 |
| 2. Layout & Rendering Pipeline | 6/6 | Complete | 2026-01-29 |
| 3. Reactive State System | 4/4 | Complete | 2026-01-29 |
| 4. Event System | 5/5 | Complete | 2026-01-29 |
| 5. Element Library | 5/5 | Complete | 2026-01-30 |
| 6. Design System | 4/4 | Complete | 2026-01-30 |
| 7. Editor Chrome & Text Editing | 0/5 | Not started | - |
| 8. Advanced UI & Widgets | 0/TBD | Not started | - |
| 9. Transitions & Integration | 0/TBD | Not started | - |

---
*Roadmap created: 2026-01-28*
*Last updated: 2026-01-30 after Phase 7 planning complete (5 plans in 3 waves)*
