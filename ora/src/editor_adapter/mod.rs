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

// =============================================================================
// Sub-traits (FIX-04): split the monolithic EditorDataSource into three
// focused traits. This allows future phases to add methods to the correct
// sub-trait and narrows method signatures for views that only need rendering.
// =============================================================================

/// Read-only buffer/viewport data for rendering.
///
/// Views that only read data (e.g. text area, gutter) can accept
/// `&dyn BufferDataSource` instead of the full `&dyn EditorDataSource`.
pub trait BufferDataSource {
    /// Build the complete render model for the current frame.
    ///
    /// `viewport_lines` is the number of text lines currently visible
    /// in the editor viewport (used to compute `visible_lines` in the model).
    fn build_render_model(&self, viewport_lines: usize) -> RenderModel;

    /// Resize the editor viewport (called on window resize).
    ///
    /// `width_chars` is the viewport width in characters.
    /// `height_lines` is the viewport height in text lines.
    fn resize_viewport(&mut self, width_chars: usize, height_lines: usize);

    /// Returns the current viewport height in lines.
    fn viewport_lines(&self) -> usize;

    /// Returns the current vertical scroll offset in lines.
    ///
    /// Used by the smooth-scroll accumulator to detect document boundaries
    /// without building a full RenderModel.
    fn scroll_y(&self) -> usize;

    /// Returns the total number of lines in the current document.
    ///
    /// Used to compute the maximum scroll position for clamping:
    /// `max_scroll_px = (total_lines - 1) * LINE_HEIGHT`
    fn total_lines(&self) -> usize;
}

/// Command dispatch to the editor backend.
pub trait CommandDispatcher {
    /// Dispatch an editor command (from keyboard input or UI action).
    fn dispatch_command(&mut self, cmd: EditorCommand);
}

/// Window/chrome metadata.
pub trait WindowDataSource {
    /// Get the window title (for title bar updates).
    fn window_title(&self) -> String;
}

// =============================================================================
// EditorDataSource — convenience super-trait combining all sub-traits.
//
// The blanket impl means any type implementing all three sub-traits
// automatically implements EditorDataSource, preserving full backward
// compatibility. Existing Box<dyn EditorDataSource> usage is unchanged.
// =============================================================================

/// Trait abstracting over an editor backend.
///
/// `wgpu_client` implements this for `core_editor::app::App`.
/// ora views receive `&dyn EditorDataSource` to query presentation data.
///
/// This is a convenience super-trait combining `BufferDataSource`,
/// `CommandDispatcher`, and `WindowDataSource`. Implement those three
/// sub-traits and this trait is satisfied automatically via blanket impl.
pub trait EditorDataSource: BufferDataSource + CommandDispatcher + WindowDataSource {}

impl<T> EditorDataSource for T where T: BufferDataSource + CommandDispatcher + WindowDataSource {}
