# Codebase Structure

**Analysis Date:** 2026-01-28

## Directory Layout

```
wgpu_client/
├── src/
│   ├── main.rs                 # Binary entry point: logging, CLI args, event loop
│   ├── app.rs                  # WgpuApp: ApplicationHandler implementation, event routing
│   ├── input.rs                # Input translation: keys → EditorCommand/AppAction
│   ├── renderer/
│   │   └── mod.rs              # GpuRenderer: render orchestration, component lifecycle
│   ├── components/             # GPU-rendered UI components (lower-level, legacy)
│   │   ├── mod.rs              # Bounds, RenderContext, component exports
│   │   ├── text_area.rs        # Text rendering with glyphon
│   │   ├── gutter.rs           # Line numbers
│   │   ├── caret.rs            # Blinking cursor (animation state)
│   │   ├── status_bar.rs       # Bottom status bar
│   │   ├── sidebar.rs          # File sidebar component
│   │   ├── tab_bar.rs          # Editor tabs (legacy, replaced by EditorTabs)
│   │   ├── dialog.rs           # Modal dialog overlay
│   │   ├── rect.rs             # Rectangle rendering primitive
│   │   ├── ui_text_renderer.rs # UI-level text rendering with glyphon
│   │   └── shaders/            # Glsl shader files
│   ├── design_system/          # Design token system and rendering primitives
│   │   ├── mod.rs              # Module structure, re-exports
│   │   ├── tokens/             # Design tokens (not runtime colors)
│   │   │   ├── mod.rs
│   │   │   ├── color.rs        # ColorRole enum, ColorPalette
│   │   │   ├── spacing.rs      # Space enum (xs, sm, md, lg, xl)
│   │   │   ├── sizing.rs       # ComponentSize enum
│   │   │   ├── radius.rs       # Border radius and corner radii
│   │   │   ├── elevation.rs    # Elevation/shadow system
│   │   │   ├── motion.rs       # Animation duration/easing presets
│   │   │   └── theme.rs        # Theme struct combining all tokens
│   │   ├── primitives/         # GPU-renderable primitives
│   │   │   ├── mod.rs
│   │   │   ├── styled_rect.rs  # StyledRect with color, border, shadow
│   │   │   ├── styled_rect_renderer.rs  # GPU rendering for StyledRect
│   │   │   └── text_block.rs   # Text layout primitive
│   │   ├── interaction/        # Interaction state management
│   │   │   ├── mod.rs
│   │   │   ├── state.rs        # InteractionState (hover, focus, active)
│   │   │   └── focus.rs        # Focus tracking
│   │   ├── layout/             # Layout algorithms
│   │   │   ├── mod.rs
│   │   │   └── flex.rs         # Flexbox-like layout system
│   │   └── animation/          # Animation primitives
│   │       ├── mod.rs
│   │       ├── tween.rs        # Tween<T> animation type
│   │       └── easing.rs       # Easing functions (linear, ease-in, etc.)
│   ├── theme/                  # Theme mapping and colors
│   │   ├── mod.rs              # Style mapping: TextStyle → GpuStyle
│   │   └── colors.rs           # Color definitions and conversions
│   ├── ui/                     # High-level modern UI components (new)
│   │   ├── mod.rs              # Module structure, re-exports
│   │   ├── app_layout.rs       # AppLayout: overall screen region calculation
│   │   ├── editor_tabs.rs      # EditorTabs: modern tab bar component
│   │   ├── file_tree.rs        # FileTree: sidebar file explorer
│   │   ├── command_palette.rs  # CommandPalette: Ctrl+Shift+P search
│   │   ├── panels.rs           # PanelManager: bottom panel (terminal/output)
│   │   ├── status_line.rs      # StatusLine: cursor position, language info
│   │   ├── toolbar.rs          # Toolbar (emerging, minimal)
│   │   └── split_view.rs       # SplitView: split pane layout
│   ├── widgets/                # Emerging widget system
│   │   ├── mod.rs              # Widget trait definitions
│   │   ├── traits.rs           # Widget, WidgetRenderer traits
│   │   ├── widget_renderer.rs  # Base rendering for widgets
│   │   ├── core/               # Basic widgets
│   │   │   ├── mod.rs
│   │   │   ├── button.rs
│   │   │   ├── checkbox.rs
│   │   │   ├── input.rs        # Text input widget
│   │   │   └── toggle.rs
│   │   ├── navigation/         # Navigation widgets
│   │   │   ├── mod.rs
│   │   │   ├── tab.rs
│   │   │   ├── list_item.rs
│   │   │   └── tree_item.rs
│   │   └── composites/         # Complex widgets
│   │       ├── mod.rs
│   │       ├── context_menu.rs
│   │       └── toast.rs
│   └── Cargo.toml              # Workspace root dependencies
├── .planning/
│   └── codebase/
│       ├── ARCHITECTURE.md     # Architecture overview
│       └── STRUCTURE.md        # This file
└── Cargo.lock                  # Lockfile
```

## Directory Purposes

**src/**
- Purpose: All Rust source code for GPU client
- Contains: Application logic, rendering components, design system, input handling
- Key files: `main.rs` (entry), `app.rs` (event handling), `renderer/mod.rs` (orchestration)

**src/components/**
- Purpose: GPU-rendered UI components - lower level, direct wgpu usage
- Contains: TextArea, Gutter, Caret, StatusBar, TabBar, Sidebar, Dialog components
- Pattern: Each component manages its own GPU buffers and render passes
- Key files: `text_area.rs` (core editor text), `caret.rs` (cursor with animation)

**src/design_system/**
- Purpose: Reusable design system with tokens, primitives, interaction, layout
- Contains: Color roles, spacing/sizing enums, StyledRect primitive, animation tweens, focus tracking
- Pattern: Tokens define semantic roles, primitives render to GPU, interaction tracks state
- Key files: `tokens/mod.rs` (ColorRole, ComponentSize), `primitives/styled_rect.rs` (GPU rendering)

**src/design_system/tokens/**
- Purpose: Design token definitions (not runtime colors, but semantic roles)
- Contains: ColorRole enum with semantic names (FgPrimary, Selection, SyntaxKeyword, etc.)
- Key files: `color.rs` (ColorRole, ColorPalette), `theme.rs` (Theme combining all tokens)

**src/design_system/primitives/**
- Purpose: GPU-renderable building blocks
- Contains: StyledRect (rectangle with styling), TextBlock (text layout), renderers for GPU
- Key files: `styled_rect_renderer.rs` (wgpu pipeline for rectangles)

**src/design_system/interaction/**
- Purpose: Track component interaction states (hover, focus, active)
- Contains: InteractionState enum, InteractionTracker for managing state
- Used by: UI components to apply hover/active styles

**src/design_system/layout/**
- Purpose: Layout algorithm (flexbox-like)
- Contains: FlexLayout, FlexDirection, AlignItems, JustifyContent
- Status: Emerging, not yet widely used

**src/design_system/animation/**
- Purpose: Animation primitives (tweens, easing)
- Contains: Tween<T> generic animation, easing functions
- Used by: Caret blinking, command palette animations

**src/theme/**
- Purpose: Map semantic TextStyle to concrete GPU rendering style
- Contains: `map_style()` function, Color struct, color conversions
- Depends on: Design system tokens, TextStyle from core_editor

**src/ui/**
- Purpose: High-level modern UI components built on design system
- Contains: EditorTabs, FileTree, CommandPalette, StatusLine, PanelManager
- Pattern: Each component builds StyledRect + UIText primitives, handles interactions
- Key files: `command_palette.rs` (Ctrl+Shift+P overlay), `editor_tabs.rs` (tab management)

**src/widgets/**
- Purpose: Emerging reusable widget library (similar to UI toolkit)
- Contains: Button, Checkbox, Input, Tab, ListItem, ContextMenu, Toast
- Status: Early stage, not yet integrated into main UI
- Key files: `traits.rs` (Widget trait), `core/button.rs` (example widget)

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry, logging initialization, event loop creation
- `src/app.rs`: ApplicationHandler for winit, main event dispatch
- `src/renderer/mod.rs`: GpuRenderer frame rendering orchestration

**Configuration:**
- `Cargo.toml`: Dependencies (wgpu, winit, glyphon, log, simplelog)
- `src/design_system/tokens/theme.rs`: Theme configuration (colors, sizing)

**Core Logic:**
- `src/app.rs`: Event handling (keyboard, mouse), viewport calculation, frame request
- `src/renderer/mod.rs`: Component preparation, layout calculation, render pass sequencing
- `src/components/text_area.rs`: Text rendering with syntax highlighting
- `src/input.rs`: Key translation to commands and actions

**Design System:**
- `src/design_system/tokens/color.rs`: ColorRole definitions and palette
- `src/design_system/primitives/styled_rect.rs`: GPU-renderable rectangle
- `src/design_system/tokens/theme.rs`: Theme struct combining tokens

**UI Components:**
- `src/ui/command_palette.rs`: Command search palette (Ctrl+Shift+P)
- `src/ui/editor_tabs.rs`: Tab bar with close buttons
- `src/ui/file_tree.rs`: File explorer in sidebar
- `src/ui/panels.rs`: Bottom panel (terminal/output)
- `src/ui/status_line.rs`: Cursor position and language info

**Testing:**
- Not detected (no test directory found)

## Naming Conventions

**Files:**
- Component files use snake_case: `text_area.rs`, `status_bar.rs`, `command_palette.rs`
- Module files: `mod.rs` for directory modules
- Shader files: `.glsl` in `shaders/` subdirectories

**Directories:**
- Functional modules: snake_case (`design_system`, `ui`, `components`)
- Nested modules group related functionality (`tokens`, `primitives`, `interaction`)

**Structs:**
- PascalCase for all public types: `WgpuApp`, `GpuRenderer`, `TextArea`, `Bounds`
- Component structs match their module: `TextArea` in `text_area.rs`, `CommandPalette` in `command_palette.rs`

**Methods:**
- snake_case: `new()`, `render()`, `prepare()`, `build_rects()`, `on_pointer_move()`
- Builder methods return `Self` for chaining: `.with_shortcut()`, `.with_bg()`
- State query methods use simple names: `is_visible()`, `height()`, `has_selection()`

**Enums:**
- PascalCase variants: `ColorRole::FgPrimary`, `LayoutRegion::Editor`
- Use full paths in match statements (no wildcards for exhaustive checking)

## Where to Add New Code

**New Feature (editing/behavior):**
- Primary code: `src/app.rs` - add event handler or action variant in `AppAction`
- Command dispatch: `src/input.rs` - add translation rule if keyboard-driven
- Core logic: Dispatch to `core_editor::App` for document operations
- Tests: Not currently present, but would go in `src/app.rs` (unit tests at file bottom)

**New UI Component:**
- If low-level GPU rendering: `src/components/new_component.rs`
  - Implement prepare/render methods using wgpu pipeline
  - Export from `src/components/mod.rs`
  - Add to `GpuRenderer` fields
  - Call prepare() and render() in `GpuRenderer::render()`
- If high-level UI: `src/ui/new_component.rs`
  - Build StyledRect and UIText primitives
  - Handle interactions through `on_pointer_move()`, `on_click()`
  - Export from `src/ui/mod.rs`
  - Integrate in `GpuRenderer` render pass

**New Design Token/Primitive:**
- Token definition: `src/design_system/tokens/` - add variant to enum (e.g., ColorRole)
- Primitive rendering: `src/design_system/primitives/` - add new primitive type
- Renderer: `src/design_system/primitives/` - add renderer using wgpu

**New Widget (for widget system):**
- Widget definition: `src/widgets/core/new_widget.rs`
  - Implement `Widget` trait from `src/widgets/traits.rs`
- Widget renderer: Implement renderer logic in widget
- Export: `src/widgets/core/mod.rs`

**Utilities/Helpers:**
- Shared math functions: `src/design_system/` (layout calculations)
- Input helpers: `src/input.rs` (key translation logic)
- Color conversions: `src/theme/colors.rs`
- GPU utilities: In component file or new `src/gpu_utils.rs`

## Special Directories

**src/components/shaders/**
- Purpose: GLSL shader source files for GPU rendering
- Generated: No, hand-written
- Committed: Yes
- Files: Vertex/fragment shaders for text rendering (glyphon), rectangles, etc.
- Pattern: Loaded and compiled by components using wgpu's `include_wgsl!()` macro

**.planning/codebase/**
- Purpose: Architecture and structure documentation for GSD commands
- Generated: Yes, by `gsd map-codebase` command
- Committed: Yes (versioned documentation)
- Files: ARCHITECTURE.md, STRUCTURE.md (this file), plus CONVENTIONS.md, TESTING.md as project matures

---

*Structure analysis: 2026-01-28*
