//! Editor adapter boundary between ora and any editor backend.
//!
//! ora views must NOT import core_editor types directly (locked decision).
//! Instead, views receive a `&dyn EditorDataSource` and use the mirror types
//! defined in this module.
//!
//! # Architecture
//!
//! ```text
//! wgpu_client (implements EditorDataSource for core_editor::app::App)
//!      │
//!      │  &dyn EditorDataSource
//!      ▼
//! ora::views  (consume RenderModel / dispatch EditorCommand)
//! ```
//!
//! The `From` conversions between core_editor types and these mirror types
//! are implemented in `wgpu_client`, keeping this module free of core_editor.

pub mod types;
pub use types::*;

/// Trait abstracting over an editor backend.
///
/// `wgpu_client` implements this for `core_editor::app::App`.
/// ora views receive `&dyn EditorDataSource` to query presentation data.
pub trait EditorDataSource {
    /// Build the complete render model for the current frame.
    ///
    /// `viewport_lines` is the number of text lines currently visible
    /// in the editor viewport (used to compute `visible_lines` in the model).
    fn build_render_model(&self, viewport_lines: usize) -> RenderModel;

    /// Dispatch an editor command (from keyboard input or UI action).
    fn dispatch_command(&mut self, cmd: EditorCommand);

    /// Get the window title (for title bar updates).
    fn window_title(&self) -> String;
}
