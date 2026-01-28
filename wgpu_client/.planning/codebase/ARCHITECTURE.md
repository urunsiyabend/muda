# Architecture

**Analysis Date:** 2026-01-28

## Pattern Overview

**Overall:** GPU-accelerated event-driven architecture with layered rendering pipeline.

**Key Characteristics:**
- Event-driven application using `winit` for windowing and input
- Immediate-mode GPU rendering using `wgpu` for all UI elements
- Separation of application state (core editor) from presentation layer
- Component-based architecture where each UI element owns its GPU resources
- Design token system (colors, spacing, sizing) for consistent theming across components

## Layers

**Application Layer (Event Handling):**
- Purpose: Receive windowed events, translate to semantic actions, coordinate app state
- Location: `src/app.rs`
- Contains: `WgpuApp` struct implementing `ApplicationHandler`
- Depends on: `core_editor::App`, `GpuRenderer`, `winit` event system
- Used by: winit event loop

**State Layer (Core Editor):**
- Purpose: Maintain document state, cursor position, viewport, file management
- Location: imported from `core_editor` crate (sibling project)
- Contains: `EditorApp` managing workspace, documents, and editor state
- Depends on: Nothing from this crate
- Used by: `WgpuApp` for all editing operations

**Renderer Orchestration Layer:**
- Purpose: Coordinate all GPU rendering, manage component lifecycle, calculate layout
- Location: `src/renderer/mod.rs`
- Contains: `GpuRenderer` struct managing wgpu device/queue, all rendering components
- Depends on: All component modules, theme system, design system
- Used by: `WgpuApp` to render frames and handle UI interactions

**Component Rendering Layer:**
- Purpose: Prepare GPU resources per frame, render individual UI elements
- Location: `src/components/` (legacy components) and `src/ui/` (new components)
- Contains: TextArea, Gutter, StatusBar, TabBar, Dialog, Sidebar, Caret (legacy); EditorTabs, FileTree, CommandPalette, StatusLine, PanelManager (new)
- Depends on: Theme, design system primitives, wgpu
- Used by: `GpuRenderer` for rendering and interaction handling

**Design System Layer:**
- Purpose: Provide reusable design tokens, primitives, and rendering abstractions
- Location: `src/design_system/`
- Contains:
  - `tokens/`: Color roles, spacing, sizing, elevation, motion, theme
  - `primitives/`: StyledRect, TextBlock rendering primitives
  - `interaction/`: Focus tracking, interaction state management
  - `layout/`: Flexbox-like layout system
  - `animation/`: Tween animation system
- Depends on: wgpu, glyphon (text)
- Used by: All UI components, theme mapping

**Theme/Styling Layer:**
- Purpose: Map semantic TextStyle to concrete GPU rendering attributes
- Location: `src/theme/`
- Contains: Color definitions, style mapping function `map_style()`
- Depends on: Design system tokens, TextStyle from core_editor
- Used by: Component rendering, text styling

**Input Translation Layer:**
- Purpose: Convert winit events to semantic commands and actions
- Location: `src/input.rs`
- Contains: `translate_key()` → EditorCommand, `translate_app_action()` → AppAction
- Depends on: core_editor command definitions, winit
- Used by: `WgpuApp` for keyboard handling

**Widget System (Emerging):**
- Purpose: Provide reusable stateful UI widgets (button, checkbox, input, etc.)
- Location: `src/widgets/`
- Contains:
  - `core/`: Basic widgets (Button, Checkbox, Input, Toggle)
  - `navigation/`: Navigation widgets (Tab, ListItem, TreeItem)
  - `composites/`: Composite widgets (ContextMenu, Toast)
- Depends on: Design system, interaction state
- Used by: UI components as building blocks

## Data Flow

**Frame Rendering Pipeline:**

1. **Event Reception** (`winit` event loop)
2. **Event Translation** (`app.rs`/`input.rs`)
   - Winit event → EditorCommand (keyboard) or AppAction (app-level)
3. **State Update** (`WgpuApp`)
   - Dispatch command to core_editor or handle app action
   - Request redraw if state changed
4. **Render Model Preparation** (`WgpuApp`)
   - Calculate viewport dimensions in characters/lines (account for sidebar, gutter, status bar)
   - Call `editor.build_render_model(height)` to get structured presentation data
5. **Layout Calculation** (`GpuRenderer::render()`)
   - `app_layout.calculate()` for overall regions
   - `calculate_layout()` splits screen into bounds for each component
6. **Component Preparation** (each component)
   - Update GPU buffers from render model
   - Calculate text glyphon cache for text components
7. **GPU Rendering** (command encoding)
   - Clear screen with background color
   - Render sidebar/file tree (styled rects + text)
   - Render tab bar (styled rects + text)
   - Render gutter background
   - Render selection backgrounds
   - Render text area
   - Render caret (blinking animation)
   - Render status line
   - Render command palette (if visible)
   - Render panel (if visible)
   - Render dialog (if visible, topmost)
8. **Frame Submission**
   - `queue.submit()` command encoder
   - `output.present()` to display

**State Management:**

- **Document State**: Owned by `core_editor::App` (external)
- **View State**: Viewport scroll position, line numbers visibility in `core_editor::View`
- **Renderer State**: Animation timers (caret blink, palette), component hover states
- **Selection State**: Anchor point and extent tracked in editor, extended via drag
- **UI State**: Command palette search input, panel visibility, file tree expansion (in component)

**Click Handling Flow:**

1. `MouseInput` event with position
2. Hit test to determine region (editor, sidebar, tab bar, status bar)
3. Calculate local coordinates and map to logical pixels
4. Determine row/column accounting for sidebar, gutter, scroll offset
5. Convert to document offset via `TextPosition` and position math
6. Update cursor position or perform action (open file, close tab, etc.)

## Key Abstractions

**RenderModel (from core_editor):**
- Purpose: Structured presentation model of editor state for rendering
- Examples: Contains sidebar entries, tab bar state, caret position, text lines with styles
- Pattern: Built from editor state, consumed by components without modifying state

**Bounds (Component Layout):**
- Purpose: Rectangular viewport for rendering components
- Examples: `src/components/mod.rs` defines `Bounds` struct with split operations
- Pattern: Hierarchical layout through recursive `split_horizontal()` and `split_vertical()`

**StyledRect (Design System):**
- Purpose: GPU-renderable rectangle with styling (color, border, shadow, corner radius)
- Examples: `src/design_system/primitives/styled_rect.rs`
- Pattern: Accumulate rects into buffer, render with `StyledRectRenderer`

**UITextRenderer (Design System):**
- Purpose: Render styled text using glyphon glyph cache
- Examples: Used for tabs, file tree, command palette, status line
- Pattern: Prepare text data, upload to GPU, render with glyph cache

**InteractionState (Design System):**
- Purpose: Track hover/focus/active state for components
- Examples: `src/design_system/interaction/state.rs`
- Pattern: Checked by component rendering to display hover effects

**AppAction (Input Translation):**
- Purpose: Application-level semantic action from keyboard input
- Examples: RequestQuit, ToggleSidebar, CommandPaletteUp, etc.
- Pattern: Enumeration of high-level actions, handled by `WgpuApp::handle_app_action()`

## Entry Points

**Main Binary Entry:**
- Location: `src/main.rs`
- Triggers: Application startup
- Responsibilities: Initialize logging, parse command-line args, create WgpuApp, run event loop

**Event Loop:**
- Location: `src/app.rs` - `ApplicationHandler` trait implementation
- Triggers: Each window event (keyboard, mouse, resize, etc.)
- Responsibilities: Route events to appropriate handlers, request redraws, exit on quit

**Frame Rendering:**
- Location: `src/renderer/mod.rs` - `GpuRenderer::render()`
- Triggers: `RedrawRequested` event or animation timeout
- Responsibilities: Calculate layout, update all components, encode and submit GPU commands

**Input Handler:**
- Location: `src/app.rs` - `window_event()` keyboard handling
- Triggers: `KeyboardInput` event
- Responsibilities: Translate to EditorCommand or AppAction, dispatch to core editor

## Error Handling

**Strategy:** Result-based error handling with logging at application layer.

**Patterns:**

- **File Operations**: `std::io::Result` propagated up to WgpuApp initialization
- **GPU Operations**: `wgpu::SurfaceError` caught in render loop, logged and handled gracefully
- **Editor Operations**: Errors logged with `log::error!()`, application continues
- **Command Execution**: Command dispatch returns success/failure, UI updated accordingly

**Error Recovery:**

- Surface lost during render: Call `renderer.resize()` to reconfigure
- GPU out of memory: Log error, continue (may drop frames)
- File open failure: Log error, create new document
- Save failure: Log error, user can retry

## Cross-Cutting Concerns

**Logging:** Uses `simplelog` crate with file output to `editor.log`, debug level by default. All major state changes and errors logged.

**Validation:** Input validation at multiple levels:
- Keyboard input char bounds checking
- Click position bounds checking against viewport
- Document offset bounds checking (saturating math)
- Cursor clamping to document bounds

**Authentication:** Not applicable (single-user desktop app)

**Scaling:**
- Viewport caching to avoid recalculation on every frame
- Glyph cache shared across frame renders
- GPU buffer reuse (prepare/render cycle)
- Font measurement cached per theme change

**Performance:**
- Frame skipping on slow machines (VSyncAdaptive present mode)
- Caret blinking uses timer instead of redrawing every frame
- Drag selection updates cursor on pointer move (debounced by frame rate)
- Hit testing early exit on region misses

---

*Architecture analysis: 2026-01-28*
