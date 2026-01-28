# Coding Conventions

**Analysis Date:** 2026-01-28

## Naming Patterns

**Files:**
- Snake case for all module files: `app.rs`, `text_area.rs`, `styled_rect.rs`
- Descriptive names matching the primary type/functionality in the file
- Example: `ui_text_renderer.rs` contains `UITextRenderer` struct

**Functions:**
- Snake case: `translate_key()`, `build_render_model()`, `request_redraw()`
- Constructor functions use `new()` pattern: `Rect::new()`, `Theme::new()`
- Getter methods use `get_()` or direct noun: `theme()`, `char_width()`, `scale_factor()`
- Boolean checks use `is_()` prefix: `is_dragging`, `is_double_click()`, `is_srgb()`
- Setter/mutation methods use verb form: `request_redraw()`, `set_cursor()`, `update()`

**Variables:**
- Snake case for all bindings: `scale_factor`, `sidebar_width`, `char_width`
- Boolean variables use `is_` or `has_` prefix: `is_dragging`, `has_pending_action()`, `redraw_pending`
- Cached/temporary values use descriptive names: `cached_viewport`, `current_cursor`

**Types:**
- Pascal case (PascalCase) for structs, enums, traits: `WgpuApp`, `GpuRenderer`, `AppAction`
- Pascal case for type aliases and newtypes
- Const generics use UPPER_CASE: `MAX_RECTS` (see `const MAX_RECTS: usize = 4096`)

**Constants:**
- UPPER_CASE with underscores: `DOUBLE_CLICK_TIME_MS = 500`, `DOUBLE_CLICK_DISTANCE = 5.0`
- Grouped logically with comment headers in enums and const definitions

## Code Style

**Formatting:**
- Standard Rust formatting (implied rustfmt usage via Cargo defaults)
- 4-space indentation (consistent with Rust conventions)
- Line lengths typically under 120 characters, occasionally up to ~140 for GPU resource definitions
- Single line `let` statements for simple initialization
- Block statements for complex logic with explicit braces

**Linting:**
- No explicit linter configuration file found
- Code follows Rust 2024 edition conventions (see `Cargo.toml: edition = "2024"`)
- Error types use standard Rust patterns: `anyhow::Result`, `std::io::Result`
- All public items appear to be intentionally exposed

**Module Structure:**
- Typical mod.rs barrel export pattern: internal modules declared in `mod.rs`, then re-exported publicly
- Example from `design_system/mod.rs`: declares modules, then re-exports key types
- Private items use `mod` in module files; public APIs use `pub use` in `mod.rs`

## Import Organization

**Order:**
1. Standard library imports (`std::`)
2. External crate imports (winit, wgpu, glyphon, etc.)
3. Local crate imports (`use crate::`)
4. Type-specific imports (for trait usage, re-exported enums)

**Pattern from app.rs:**
```rust
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use core_editor::app::App as EditorApp;

use crate::input::{translate_app_action, translate_key, AppAction};
use crate::renderer::GpuRenderer;
```

**Path Aliases:**
- Not heavily used; direct imports preferred
- `use X as Y` only when disambiguation needed (rarely)

## Error Handling

**Patterns:**
- Result-based error handling using `anyhow::Result<T>` for fallible operations
- `?` operator propagation in functions returning `Result`
- `unwrap_or_else()` for graceful recovery with default behavior (see `main.rs` line 44)
- `match` on error types for specific handling (see `app.rs` lines 140-148):
  ```rust
  match renderer.render(&model, scale_factor) {
      Ok(()) => {}
      Err(wgpu::SurfaceError::Lost) => {
          renderer.resize(window.inner_size());
      }
      Err(wgpu::SurfaceError::OutOfMemory) => {
          log::error!("Out of GPU memory");
      }
      Err(e) => {
          log::warn!("Render error: {:?}", e);
      }
  }
  ```
- Log errors when they occur but continue execution where safe (defensive programming)
- Panics via `expect()` only in initialization code where failure is unrecoverable

## Logging

**Framework:** `log` crate with `simplelog` backend

**Patterns:**
- Initialized in `main.rs` using `simplelog::WriteLogger` to file (`editor.log`)
- Log levels: `debug!()`, `info!()`, `warn!()`, `error!()` used appropriately
- Debug logs for state changes and entry/exit events
- Error logs for unrecoverable failures: `log::error!("Save failed: {}", e)`
- Warn logs for unexpected but recoverable states: `log::warn!("Render error: {:?}", e)`
- Example from `app.rs` line 251: `log::debug!("Executing command: {}", command_id);`

## Comments

**When to Comment:**
- Module-level documentation using `//!` describing purpose and architecture
- Complex algorithms or GPU operations with explanation of approach
- Invariants that must be maintained (e.g., "Invalidate cached viewport after view change")
- Section headers with comment dividers for organization

**JSDoc/TSDoc:**
- Rust uses `///` for item documentation (not heavily used in this codebase)
- Example from `app.rs`: `/// Main application state for the GPU client.` above struct
- Parameters and return types documented in prose, not with markdown tags

**Example from components/mod.rs:**
```rust
/// Rendering context passed to all components.
pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    // ...
}

impl<'a> RenderContext<'a> {
    /// Physical width in pixels.
    pub fn physical_width(&self) -> f32 {
        self.width as f32
    }
}
```

## Function Design

**Size:**
- Functions range 10-50 lines typically; large handlers like `handle_mouse_click()` reach 60+ lines
- Complex event handlers kept in single function with internal helper methods
- GPU initialization and rendering functions are methods, not standalone

**Parameters:**
- Self or mutable self for stateful operations
- References (`&T`, `&mut T`) prefer over owned values where applicable
- Generic lifetimes used in rendering contexts: `RenderContext<'a>`
- Named parameters in match statements for clarity

**Return Values:**
- `Option<T>` for operations that may not produce a value (e.g., `translate_key()` returns `Option<EditorCommand>`)
- `Result<T, E>` for fallible I/O operations
- Unit `()` for mutations/state changes
- Early returns with `?` operator in Result-returning functions

**Example from input.rs:**
```rust
pub fn translate_key(event: &KeyEvent, modifiers: &Modifiers) -> Option<EditorCommand> {
    // Only handle key presses, not releases
    if event.state != ElementState::Pressed {
        return None;
    }
    // ... match logic ...
}
```

## Module Design

**Exports:**
- Public modules re-export key types in `mod.rs` for clean API surface
- Example from `components/mod.rs`: internal modules marked private (no `pub mod`), types re-exported via `pub use`
- Design pattern keeps implementation details internal

**Barrel Files:**
- Used extensively: `components/mod.rs`, `design_system/mod.rs`, `widgets/mod.rs` all aggregate and re-export
- Centralizes API, makes imports cleaner for consumers
- Each barrel organizes logically: design_system exports tokens, primitives, interaction, layout, animation

**Trait Implementation:**
- Standard traits implemented where needed: `Default`, `Clone`, `Copy`, `Debug`
- Custom derive macros used: `#[derive(Clone, Copy, Debug)]` is common
- `impl Default for WgpuApp` implemented explicitly with meaningful defaults

## Code Organization & Patterns

**Builder Pattern:**
- Methods chain with parameter helpers: `GpuStyle::new().with_bg().bold().italic()`
- Example from `theme/mod.rs` lines 43-61: chaining const methods for style configuration

**Guard Clauses:**
- Early returns for validation: `if event.state != ElementState::Pressed { return None; }`
- Common in event handlers to short-circuit unnecessary processing

**Option/Result Propagation:**
- `let Some(var) = option_val else { return }` used frequently
- Example from `app.rs` line 110: `let Some(window) = &self.window else { return };`

**Struct Initialization:**
- Multi-field initialization spelled out explicitly (no builder pattern used)
- Example from `app.rs` lines 48-62: `Self { field: value, ... }` with proper initialization of all fields

---

*Convention analysis: 2026-01-28*
