# ora — GPUI-Inspired UI Framework for muda

## What This Is

ora is a standalone Rust crate that provides a GPUI-style UI framework for the muda code editor. It owns the full GPU rendering pipeline (wgpu), the winit event loop, text rendering (glyphon), flexbox layout (taffy), and a reactive entity/model state system. Views are trait-based declarative render functions, and the framework manages the element tree and layout diffing behind the scenes. All existing wgpu_client UI code will be migrated to use ora exclusively, making wgpu_client a thin application shell.

## Core Value

A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.

## Requirements

### Validated

- ✓ Core editor domain logic (text buffers, documents, commands, undo/redo) — existing (`core_editor`)
- ✓ View layer (cursor, selection, viewport, sidebar, focus state) — existing (`core_editor`)
- ✓ ViewModel projection (RenderModel, TextStyle tokens, StyledSpan) — existing (`core_editor`)
- ✓ GPU rendering backend with wgpu + winit + glyphon — existing (`wgpu_client`)
- ✓ Design token foundation (color roles, spacing scale, sizing tiers, elevation) — existing (`wgpu_client/design_system`)
- ✓ Component hierarchy (TextArea, Gutter, StatusBar, TabBar, Sidebar, Dialog) — existing (`wgpu_client/components`)
- ✓ UI components (EditorTabs, FileTree, CommandPalette, StatusLine, PanelManager) — existing (`wgpu_client/ui`)
- ✓ Widget primitives (Button, Checkbox, Toggle, Input, ListItem, Tab, TreeItem, ContextMenu, Toast) — existing (`wgpu_client/widgets`)
- ✓ Syntax highlighting via tree-sitter — existing (`core_editor`)
- ✓ TUI rendering backend — existing (`ratatui_client`, untouched by this work)

### Active

- [ ] ora crate with GPUI-style application lifecycle (`ora::run(app)` owns winit event loop)
- [ ] GPU pipeline ownership (wgpu Instance/Device/Queue/Surface managed by ora)
- [ ] Text rendering encapsulation (glyphon wrapped internally — views use `Text::new("hello").size(14)`)
- [ ] Flexbox layout engine via taffy (row/column, flex-grow, align, justify, gap)
- [ ] Trait-based View system (`View` trait with `render()` returning element tree)
- [ ] Element tree with framework-managed diffing and efficient re-rendering
- [ ] Entity/Model reactive state system (state in `Model<T>`, views observe and re-render on change)
- [ ] Consolidated design tokens (colors, spacing, sizing, elevation, typography) — single source of truth
- [ ] Theme system (dark/light) driven by token palette
- [ ] CSS-like property transitions (opacity, color, position) for hover fades, tab slides, state changes
- [ ] Styled primitives: Div, Text, Image (GPU-rendered rectangles, text spans, textures)
- [ ] Event handling system (mouse, keyboard, focus, hover) routed through element tree
- [ ] Scissor clipping built into layout (components don't manage their own clip rects)
- [ ] Component library migrated to ora: TextArea, Gutter, StatusBar, TabBar, Sidebar, Dialog
- [ ] UI components migrated to ora: EditorTabs, FileTree, CommandPalette, StatusLine, PanelManager, AppLayout
- [ ] Widget primitives migrated to ora: Button, Checkbox, Toggle, Input, ListItem, Tab, TreeItem, ContextMenu, Toast
- [ ] wgpu_client reduced to thin app shell (creates ora App, registers root view, hands off control)
- [ ] All hardcoded values eliminated (heights, font sizes, padding use token system)
- [ ] All duplicated styling logic removed (single layout calculation path per component)

### Out of Scope

- TUI/ratatui backend support — ora is GPU-only; ratatui_client remains independent
- Keyframe/spring/staggered animation system — only CSS-like transitions for v1
- Web rendering target — desktop native only
- Custom shader API — ora handles rendering internally, no user-facing shader hooks
- Multi-window support — single window for v1
- Accessibility/screen reader integration — deferred to future milestone
- Hot-reload of themes/tokens at runtime — compile-time token system for v1

## Context

**Existing codebase:** muda is a Rust code editor with a Cargo workspace containing `core_editor` (domain logic), `wgpu_client` (GPU rendering), and `ratatui_client` (TUI rendering). The GPU client already has a component hierarchy, design token system, and widget library, but these are tightly coupled to wgpu_client with significant duplication and inconsistency.

**Pain points ora addresses:**
- Status bar height (24.0) hardcoded in 4 places in app.rs instead of referencing the constant
- Tab bar height mismatched between old (28px) and new (35px) systems
- Font metrics (size, line height) redefined per component instead of using theme
- ComponentSize enum defined but unused — widgets define their own const heights
- Spacing tokens (Space enum) defined but components use raw float literals
- Gutter width calculated in two separate places
- No centralized typography scale (12px, 14px, font_size * 0.9 scattered)
- Color lookups done inline with no accessor helpers
- Scissor rect calculations duplicated across components

**Inspiration:** GPUI from the Zed editor — views as functions of state, framework-managed element tree, entity/model reactive state, GPU-driven rendering.

**Tech stack for ora:**
- Rust (Edition 2024)
- wgpu (GPU abstraction)
- winit (windowing + event loop)
- glyphon (text rendering)
- taffy (flexbox layout)
- Existing design tokens migrated and consolidated

## Constraints

- **Tech stack**: Rust only — ora is a native Rust crate in the existing Cargo workspace
- **Rendering**: wgpu — must work with Vulkan/Metal/DirectX backends already supported
- **Text**: glyphon — must maintain current text rendering quality and performance
- **Layout**: taffy — flexbox model, no custom layout engine
- **Compatibility**: core_editor API is stable — ora consumes RenderModel/EditorCommand, does not modify core_editor
- **Performance**: Must maintain or improve current frame rendering performance (existing scissor clipping, single-pass batching optimizations)
- **Migration**: wgpu_client components must be migrated incrementally — editor must remain functional throughout

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GPUI-style views (trait-based, render() returns element tree) | Matches Zed's proven model for editor UIs; declarative, composable, testable | — Pending |
| Entity/Model reactive state | Decouples state ownership from views; enables multiple views of same state | — Pending |
| Ora owns winit event loop | Simplifies app lifecycle; framework controls render timing and event dispatch | — Pending |
| Ora wraps glyphon internally | Views shouldn't manage text atlases; Text::new() API is cleaner | — Pending |
| Flexbox via taffy | Industry-standard layout model; no need to invent custom constraints | — Pending |
| GPU-only (no TUI backend) | Keeps ora focused; ratatui_client has different rendering semantics entirely | — Pending |
| CSS-like transitions only (no keyframes) | Sufficient for editor UI; hover fades, slide transitions cover all current needs | — Pending |

---
*Last updated: 2026-01-28 after initialization*
