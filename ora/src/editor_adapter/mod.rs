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

use std::path::PathBuf;

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

/// File-operation callbacks invoked by the event loop's async dialog futures.
///
/// The event loop spawns async tasks that open native file dialogs, read files,
/// and then call these methods to deliver results back to the adapter.
pub trait FileOpDataSource {
    /// Take and clear any pending file operation queued by `dispatch_command`.
    ///
    /// Called every frame by the event loop; returns `None` most frames.
    fn take_pending_file_op(&mut self) -> Option<PendingFileOp>;

    /// Called when a file has been successfully read and validated.
    ///
    /// `path` is the canonical path of the file; `content` is the UTF-8 text
    /// (BOM already stripped by the caller). The adapter must create or
    /// activate the appropriate tab and trigger a re-render.
    fn handle_file_loaded(&mut self, path: PathBuf, content: String);

    /// Called when a file could not be opened (I/O error or non-UTF-8 data).
    ///
    /// `message` is a user-visible error string to display in the status bar.
    fn handle_file_error(&mut self, message: String);

    /// Returns `true` if a native file dialog is currently open.
    ///
    /// Used to guard against concurrent dialogs (e.g. rapid Ctrl+O presses).
    fn is_dialog_open(&self) -> bool;

    /// Set the dialog-open guard flag.
    ///
    /// Called by the event loop before spawning a dialog task and cleared
    /// when the dialog completes.
    fn set_dialog_open(&mut self, open: bool);
}

// =============================================================================
// EditorDataSource — convenience super-trait combining all sub-traits.
//
// The blanket impl means any type implementing all four sub-traits
// automatically implements EditorDataSource, preserving full backward
// compatibility. Existing Box<dyn EditorDataSource> usage is unchanged.
// =============================================================================

/// Trait abstracting over an editor backend.
///
/// `wgpu_client` implements this for `core_editor::app::App`.
/// ora views receive `&dyn EditorDataSource` to query presentation data.
///
/// This is a convenience super-trait combining `BufferDataSource`,
/// `CommandDispatcher`, `WindowDataSource`, and `FileOpDataSource`.
/// Implement those four sub-traits and this trait is satisfied automatically
/// via blanket impl.
pub trait EditorDataSource:
    BufferDataSource + CommandDispatcher + WindowDataSource + FileOpDataSource
{
}

impl<T> EditorDataSource for T where
    T: BufferDataSource + CommandDispatcher + WindowDataSource + FileOpDataSource
{
}
