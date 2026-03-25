# Milestones

## v1.0 — ora UI Framework (Complete)

**Completed:** 2026-03-26
**Phases:** 1-9 (plus 8.1 inserted)
**Plans executed:** 40
**Total execution time:** ~5 hours 34 minutes

**What shipped:**
- ora crate with GPUI-style application lifecycle
- GPU pipeline ownership (wgpu Instance/Device/Queue/Surface)
- Text rendering encapsulation (glyphon wrapped internally)
- Simple stack/flex layout engine
- Trait-based View system with element tree diffing
- Entity/Model reactive state system
- Consolidated design tokens with dark/light theme
- CSS-like property transitions
- Styled primitives: Div, Text, Image
- Event handling (mouse, keyboard, focus, hover)
- Layered multi-pass GPU rendering for correct z-ordering
- Full component library: TabBar, StatusBar, Sidebar, Gutter, Dialog, TextArea, Caret
- Advanced UI: CommandPalette, FileTree, PanelManager, AppLayout
- Widget primitives: Button, Checkbox, Toggle, Input, ListItem, Tab, TreeItem, ContextMenu, Toast
- wgpu_client reduced to thin app shell
- EditorDataSource adapter boundary (ora views never import core_editor)
- Pixel-based scroll with scissor clipping

**Key decisions:** See PROJECT.md Key Decisions table.

---
*Last updated: 2026-03-26*
