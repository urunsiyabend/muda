//! Application state and editor logic.
//!
//! App is a thin UI-facing shell that:
//! - Holds Workspace + CommandDispatcher
//! - Manages UI-only state (should_quit, dialogs)
//! - Dispatches EditorCommands to the workspace
//! - Handles application-level commands (Save, Quit, etc.)

use std::path::PathBuf;

use crate::commands::editor_command::{Direction, MoveScope};
use crate::commands::{CommandContext, CommandDispatcher, DispatchResult, EditorCommand};
use crate::domain::{DiscardAcknowledgment, ProtectionError, Workspace};
use crate::view::{FocusState, Sidebar};
use crate::view_model::{PendingAction, RenderModel, builder::ViewModelBuilder};

/// The main application state.
///
/// A thin shell over Workspace that handles UI-specific concerns.
pub struct App {
    /// The workspace containing documents and views.
    pub workspace: Workspace,

    /// Command dispatcher for executing commands.
    pub dispatcher: CommandDispatcher,

    /// Whether a re-render is needed.
    pub needs_render: bool,

    /// Whether the app should quit.
    pub should_quit: bool,

    /// Pending action that requires user confirmation (e.g., exit with unsaved changes).
    pub pending_action: Option<PendingAction>,

    /// Protection error associated with the pending action.
    pub pending_error: Option<ProtectionError>,

    /// File explorer sidebar state.
    pub sidebar: Sidebar,

    /// Current focus state (Editor or Sidebar).
    pub focus: FocusState,

    /// Optional status message for user feedback (errors, confirmations, etc.).
    pub status_message: Option<String>,
}

impl App {
    /// Creates a new App with an empty document.
    pub fn new() -> Self {
        let workspace = Workspace::with_new_document();
        Self {
            workspace,
            dispatcher: CommandDispatcher::new(),
            needs_render: true,
            should_quit: false,
            pending_action: None,
            pending_error: None,
            sidebar: Sidebar::default(),
            focus: FocusState::Editor,
            status_message: None,
        }
    }

    /// Creates an App by opening a file.
    pub fn open_file(path: &str) -> std::io::Result<Self> {
        let path_buf = PathBuf::from(path);
        let mut workspace = Workspace::new();

        match workspace.open_document(&path_buf) {
            Ok(doc_id) => {
                workspace.create_view(doc_id);
                log::debug!("Opened file: {}", path);
            }
            Err(e) => {
                log::debug!("Could not open file: {}, creating new document", e);
                let doc_id = workspace.create_document();
                if let Some(doc) = workspace.document_mut(doc_id) {
                    doc.set_file_path(path_buf);
                }
                workspace.create_view(doc_id);
            }
        }

        Ok(Self {
            workspace,
            dispatcher: CommandDispatcher::new(),
            needs_render: true,
            should_quit: false,
            pending_action: None,
            pending_error: None,
            sidebar: Sidebar::default(),
            focus: FocusState::Editor,
            status_message: None,
        })
    }

    /// Creates an App by opening a directory (shows sidebar).
    pub fn open_directory(path: &str) -> std::io::Result<Self> {
        let path_buf = PathBuf::from(path);
        let canonical_path = std::fs::canonicalize(&path_buf)?;

        let workspace = Workspace::with_new_document();
        let sidebar = Sidebar::new(Some(canonical_path));

        Ok(Self {
            workspace,
            dispatcher: CommandDispatcher::new(),
            needs_render: true,
            should_quit: false,
            pending_action: None,
            pending_error: None,
            sidebar,
            focus: FocusState::Sidebar,
            status_message: None,
        })
    }

    // =========================================================================
    // Command Dispatch
    // =========================================================================

    /// Dispatches an EditorCommand through the command system.
    ///
    /// Returns true if the command was handled, false if it requires
    /// additional application-level handling.
    pub fn dispatch(&mut self, cmd: EditorCommand) -> bool {
        // Scroll commands manage their own viewport positioning — calling
        // ensure_caret_visible afterwards would drag the viewport back to
        // the caret, making mouse-wheel scrolling impossible.
        let is_scroll = matches!(cmd, EditorCommand::Scroll { .. });

        let dispatcher = &mut self.dispatcher;

        let result = self
            .workspace
            .with_active_context(|view, document, history, event_bus| {
                let mut ctx = CommandContext {
                    view,
                    document,
                    history,
                    event_bus,
                };
                dispatcher.dispatch(cmd, &mut ctx)
            });

        match result {
            Some(DispatchResult::Executed) => {
                self.needs_render = true;
                // After non-scroll commands, ensure the caret is visible by
                // updating the viewport scroll position. Without this, typing
                // or moving the cursor past the viewport boundary would never
                // scroll.
                if !is_scroll {
                    let cursor_pos = self.workspace.active_cursor_position();
                    if let (Some(pos), Some(view)) =
                        (cursor_pos, self.workspace.active_view_mut())
                    {
                        view.ensure_caret_visible(pos.line, pos.column);
                    }
                }
                true
            }
            Some(DispatchResult::RequiresAppHandling) => false,
            Some(DispatchResult::NoOp) => true,
            None => false,
        }
    }

    // =========================================================================
    // Application-Level Command Handlers
    // =========================================================================

    /// Saves the active document, updating status message on success or failure.
    pub fn save(&mut self) -> std::io::Result<bool> {
        match self.workspace.save_active_document() {
            Ok(true) => {
                self.set_status_message("File saved successfully");
                self.needs_render = true;
                Ok(true)
            }
            Ok(false) => {
                self.set_status_message("No file path set - use Save As");
                self.needs_render = true;
                Ok(false)
            }
            Err(e) => {
                self.set_status_message(format!("Save failed: {}", e));
                self.needs_render = true;
                Err(e)
            }
        }
    }

    // =========================================================================
    // Status Message Management
    // =========================================================================

    /// Sets a status message to display to the user.
    pub fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Clears the current status message.
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// Saves the active document to a new path.
    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        if let Some(doc) = self.workspace.active_document_mut() {
            doc.save_as(std::path::Path::new(path))
        } else {
            Ok(())
        }
    }

    /// Returns whether any document in the workspace has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.workspace.has_unsaved_documents()
    }

    /// Returns the file path of the active document.
    #[allow(dead_code)]
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.workspace.active_document().and_then(|d| d.file_path())
    }

    /// Sets the file path for the active document.
    ///
    /// This is typically used when creating a new document that will be
    /// saved to a specific path.
    pub fn set_file_path(&mut self, path: PathBuf) {
        if let Some(doc) = self.workspace.active_document_mut() {
            doc.set_file_path(path);
        }
    }

    // =========================================================================
    // Protected Operations (Unsaved File Protection)
    // =========================================================================

    /// Requests application exit.
    ///
    /// If there are unsaved changes, sets `pending_action` to `Exit` and
    /// returns the protection error. Otherwise, sets `should_quit` to true.
    pub fn request_exit(&mut self) -> Result<(), ProtectionError> {
        match self.workspace.can_exit() {
            Ok(()) => {
                self.should_quit = true;
                Ok(())
            }
            Err(error) => {
                self.pending_action = Some(PendingAction::Exit);
                self.pending_error = Some(error.clone());
                self.needs_render = true;
                Err(error)
            }
        }
    }

    /// Requests opening a file from the sidebar.
    ///
    /// If the current active document has unsaved changes, sets `pending_action`
    /// to `OpenFile` and returns the protection error. Otherwise, opens the file.
    pub fn request_open_file(&mut self, path: PathBuf) -> Result<(), ProtectionError> {
        match self.workspace.can_switch_active() {
            Ok(()) => {
                self.open_file_from_sidebar(&path);
                self.focus_editor();
                Ok(())
            }
            Err(error) => {
                self.pending_action = Some(PendingAction::OpenFile(path));
                self.pending_error = Some(error.clone());
                self.needs_render = true;
                Err(error)
            }
        }
    }

    /// Confirms the pending action, discarding unsaved changes.
    ///
    /// This should only be called after the user has explicitly acknowledged
    /// the potential loss of data.
    pub fn confirm_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            self.pending_error = None;
            let _ack = DiscardAcknowledgment::confirmed();

            match action {
                PendingAction::Exit => {
                    self.should_quit = true;
                }
                PendingAction::CloseDocument(doc_id) => {
                    self.workspace.close_document_acknowledged(doc_id, _ack);
                }
                PendingAction::OpenFile(path) => {
                    self.open_file_from_sidebar(&path);
                    self.focus_editor();
                }
            }
            self.needs_render = true;
        }
    }

    /// Cancels the pending action.
    pub fn cancel_pending_action(&mut self) {
        self.pending_action = None;
        self.pending_error = None;
        self.needs_render = true;
    }

    /// Returns whether there is a pending action requiring confirmation.
    pub fn has_pending_action(&self) -> bool {
        self.pending_action.is_some()
    }

    // =========================================================================
    // Convenience Methods (delegate to dispatch)
    // =========================================================================

    pub fn move_cursor_left(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: MoveScope::Char,
            extend_selection: false,
        });
    }

    pub fn move_cursor_right(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: MoveScope::Char,
            extend_selection: false,
        });
    }

    pub fn move_cursor_up(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Up,
            scope: MoveScope::Char,
            extend_selection: false,
        });
    }

    pub fn move_cursor_down(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Down,
            scope: MoveScope::Char,
            extend_selection: false,
        });
    }

    pub fn move_cursor_word_left(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: MoveScope::Word,
            extend_selection: false,
        });
    }

    pub fn move_cursor_word_right(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: MoveScope::Word,
            extend_selection: false,
        });
    }

    pub fn move_cursor_home(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: MoveScope::Line,
            extend_selection: false,
        });
    }

    pub fn move_cursor_end(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: MoveScope::Line,
            extend_selection: false,
        });
    }

    pub fn move_cursor_file_start(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: MoveScope::Document,
            extend_selection: false,
        });
    }

    pub fn move_cursor_file_end(&mut self) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: MoveScope::Document,
            extend_selection: false,
        });
    }

    pub fn page_up(&mut self, _page_height: usize) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Up,
            scope: MoveScope::Page,
            extend_selection: false,
        });
    }

    pub fn page_down(&mut self, _page_height: usize) {
        self.dispatch(EditorCommand::MoveCursor {
            direction: Direction::Down,
            scope: MoveScope::Page,
            extend_selection: false,
        });
    }

    pub fn insert_char(&mut self, ch: char) {
        self.dispatch(EditorCommand::InsertChar(ch));
    }

    pub fn insert_newline(&mut self) {
        self.dispatch(EditorCommand::InsertNewline);
    }

    pub fn insert_string_at_cursor(&mut self, text: &str) {
        self.dispatch(EditorCommand::InsertText(text.to_string()));
    }

    pub fn backspace(&mut self) {
        self.dispatch(EditorCommand::Backspace);
    }

    pub fn delete_at_cursor(&mut self) {
        self.dispatch(EditorCommand::Delete);
    }

    pub fn delete_selection(&mut self) {
        self.dispatch(EditorCommand::DeleteSelection);
    }

    pub fn select_all(&mut self) {
        self.dispatch(EditorCommand::SelectAll);
    }

    pub fn copy_selection(&mut self) {
        self.dispatch(EditorCommand::Copy);
    }

    pub fn cut_selection(&mut self) {
        self.dispatch(EditorCommand::Cut);
    }

    pub fn paste(&mut self) {
        self.dispatch(EditorCommand::Paste);
    }

    pub fn undo(&mut self) {
        self.dispatch(EditorCommand::Undo);
    }

    pub fn redo(&mut self) {
        self.dispatch(EditorCommand::Redo);
    }

    pub fn toggle_line_numbers(&mut self) {
        self.dispatch(EditorCommand::ToggleLineNumbers);
        self.needs_render = true;
    }

    // =========================================================================
    // Selection State (delegates to view)
    // =========================================================================

    pub fn begin_selection(&mut self) {
        if let Some(view) = self.workspace.active_view_mut() {
            view.begin_selection();
        }
    }

    pub fn extend_selection_to(&mut self, offset: usize) {
        if let Some(view) = self.workspace.active_view_mut() {
            view.move_caret_to(offset, true);
        }
    }

    pub fn clear_selection(&mut self) {
        if let Some(view) = self.workspace.active_view_mut() {
            view.clear_selection();
        }
    }

    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        self.workspace
            .active_view()
            .and_then(|v| v.selection_range())
    }

    // =========================================================================
    // View State Accessors
    // =========================================================================

    /// Returns the current caret offset.
    pub fn cursor_to_char_idx(&self) -> usize {
        self.workspace
            .active_view()
            .map(|v| v.caret_offset())
            .unwrap_or(0)
    }

    /// Returns whether the active view has a selection.
    pub fn has_selection(&self) -> bool {
        self.workspace
            .active_view()
            .map(|v| v.has_selection())
            .unwrap_or(false)
    }

    /// Returns whether line numbers are shown.
    pub fn show_line_numbers(&self) -> bool {
        self.workspace
            .active_view()
            .map(|v| v.show_line_numbers())
            .unwrap_or(true)
    }

    /// Calculates the width needed for line numbers.
    pub fn line_number_width(&self) -> usize {
        if !self.show_line_numbers() {
            return 0;
        }
        let line_count = self
            .workspace
            .active_document()
            .map(|d| d.len_lines())
            .unwrap_or(1);
        let digits = line_count.to_string().len();
        digits + 2
    }

    // =========================================================================
    // Viewport Management
    // =========================================================================

    /// Updates viewport and ensures cursor is visible.
    pub fn check_scrolling(&mut self, width: usize, height: usize) {
        let cursor_pos = self.workspace.active_cursor_position();

        if let Some(view) = self.workspace.active_view_mut() {
            view.viewport.resize(width, height);

            if let Some(pos) = cursor_pos {
                view.ensure_caret_visible(pos.line, pos.column);
            }
        }
    }

    // =========================================================================
    // Sidebar Methods
    // =========================================================================

    /// Toggles sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar.toggle();
        self.needs_render = true;
    }

    /// Focuses the sidebar (only if visible).
    pub fn focus_sidebar(&mut self) {
        if self.sidebar.visible {
            self.focus = FocusState::Sidebar;
            self.needs_render = true;
        }
    }

    /// Focuses the editor.
    pub fn focus_editor(&mut self) {
        self.focus = FocusState::Editor;
        self.needs_render = true;
    }

    /// Returns whether the sidebar is focused.
    pub fn is_sidebar_focused(&self) -> bool {
        self.focus == FocusState::Sidebar
    }

    /// Moves sidebar selection up.
    pub fn sidebar_move_up(&mut self) {
        self.sidebar.move_up();
        self.needs_render = true;
    }

    /// Moves sidebar selection down.
    pub fn sidebar_move_down(&mut self) {
        self.sidebar.move_down();
        self.needs_render = true;
    }

    /// Opens the selected file from the sidebar.
    pub fn sidebar_open_selected(&mut self) {
        if let Some(entry) = self.sidebar.selected_entry() {
            if entry.is_dir {
                // Navigate into directory
                let new_path = entry.path.clone();
                self.sidebar.set_base_directory(new_path);
            } else {
                // Open the file
                let path = entry.path.clone();
                self.open_file_from_sidebar(&path);
                self.focus_editor();
            }
            self.needs_render = true;
        }
    }

    /// Opens a file from the sidebar into the workspace.
    fn open_file_from_sidebar(&mut self, path: &PathBuf) {
        match self.workspace.open_document(path) {
            Ok(doc_id) => {
                self.workspace.create_view(doc_id);
                log::debug!("Opened file from sidebar: {:?}", path);
            }
            Err(e) => {
                log::debug!("Could not open file from sidebar: {:?}, error: {}", path, e);
            }
        }
    }

    /// Navigates the sidebar to the parent directory.
    pub fn sidebar_go_back(&mut self) {
        self.sidebar.go_to_parent_directory();
        self.needs_render = true;
    }

    /// Adjusts sidebar scroll for viewport height.
    pub fn adjust_sidebar_scroll(&mut self, viewport_height: usize) {
        self.sidebar.adjust_scroll_for_height(viewport_height);
    }

    // =========================================================================
    // Render Model
    // =========================================================================

    /// Returns information about all open views for the tab bar.
    /// Returns a list of (view_id, title, is_active, is_dirty) tuples.
    fn get_open_views_info(&self) -> Vec<(u64, String, bool, bool)> {
        let active_view_id = self.workspace.active_view_id();

        self.workspace.views()
            .filter_map(|(view_id, view)| {
                // Get the document for this view
                let doc = self.workspace.document(view.document_id())?;
                let title = doc.title();
                let is_active = Some(*view_id) == active_view_id;
                let is_dirty = doc.is_dirty();
                Some((view_id.as_u64(), title, is_active, is_dirty))
            })
            .collect()
    }

    /// Builds a render-ready model for the UI.
    pub fn build_render_model(&mut self, viewport_height: usize) -> RenderModel {
        // Ensure the active view's viewport is properly sized.
        // The GUI renderer passes viewport_height (in lines); for width we use a
        // generous default (in characters) since the GPU renderer is not
        // constrained by terminal columns. The old TUI code called
        // check_scrolling() before rendering, but the new adapter path did not.
        if let Some(view) = self.workspace.active_view_mut() {
            let vp = &view.viewport;
            if vp.width == 0 || vp.height == 0 {
                // Use 200 char width as default; real width doesn't matter much
                // for GPU rendering since lines are not clipped by character count
                // in the display pipeline, only in the view-model projection.
                view.viewport.resize(200, viewport_height);
            } else if vp.height != viewport_height {
                view.viewport.resize(vp.width, viewport_height);
            }
        }

        let status_message = self.status_message.as_deref();
        let open_views = self.get_open_views_info();

        let model = match (
            self.workspace.active_document(),
            self.workspace.active_view(),
        ) {
            (Some(doc), Some(view)) => ViewModelBuilder::build(
                doc,
                view,
                self.pending_action.as_ref(),
                self.pending_error.as_ref(),
                &self.sidebar,
                self.focus,
                viewport_height,
                status_message,
                &open_views,
            ),
            _ => {
                // Even without a document, we might want to show the sidebar
                ViewModelBuilder::build_sidebar_only(&self.sidebar, self.focus, viewport_height)
            }
        };

        // Clear the status message after building the model (one-shot display)
        self.status_message = None;

        model
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod app_render_tests {
    use super::*;

    #[test]
    fn test_new_app_render_model_for_gui() {
        let mut app = App::new();
        let model = app.build_render_model(40);

        // Should have at least 1 visible line (empty document has 1 line)
        assert!(!model.visible_lines.is_empty(),
            "visible_lines should not be empty for new doc, got {} lines", model.visible_lines.len());

        // Gutter should be visible with 1 line
        assert!(model.gutter.visible, "gutter should be visible");
        assert_eq!(model.gutter.total_lines, 1, "new doc should have 1 line");

        // Caret should be visible at (0,0)
        assert!(model.caret.visible, "caret should be visible");
        assert_eq!(model.caret.position.row, 0);
        assert_eq!(model.caret.position.column, 0);

        // Tab bar should be visible with 1 tab
        assert!(model.tab_bar.visible, "tab bar should be visible");
        assert!(!model.tab_bar.tabs.is_empty(), "should have at least 1 tab");

        // Status should have valid cursor position
        assert_eq!(model.status.cursor_line, 1); // 1-indexed
        assert_eq!(model.status.cursor_column, 1);

        // Sidebar should NOT be visible (no directory opened)
        assert!(!model.sidebar.visible, "sidebar should not be visible for new app");
    }
}

// =========================================================================
// Legacy accessors - DEPRECATED
// =========================================================================
//
// These methods provide direct mutable access to domain objects, bypassing
// the command system. They are retained for backwards compatibility but
// should NOT be used for new code.
//
// Instead, use:
// - EditorCommand + dispatch() for all editing operations
// - Higher-level App methods (insert_char, move_cursor_*, etc.)
//
// Direct mutations bypass undo/redo history and can leak domain logic
// into UI code. Plan to remove these once all usages are migrated.

impl App {
    /// Returns a reference to the active view.
    ///
    /// Prefer using App's higher-level methods for read access.
    #[allow(dead_code)]
    pub(crate) fn view(&self) -> Option<&crate::view::EditorView> {
        self.workspace.active_view()
    }

    /// Returns a mutable reference to the active view.
    ///
    /// **DEPRECATED**: Direct view mutations bypass the command system.
    /// Use EditorCommand + dispatch() instead for operations that modify state.
    #[allow(dead_code)]
    pub(crate) fn view_mut(&mut self) -> Option<&mut crate::view::EditorView> {
        self.workspace.active_view_mut()
    }

    /// Returns a reference to the active document.
    ///
    /// Prefer using App's higher-level methods for read access.
    #[allow(dead_code)]
    pub(crate) fn document(&self) -> Option<&crate::domain::Document> {
        self.workspace.active_document()
    }

    /// Returns a mutable reference to the active document.
    ///
    /// **DEPRECATED**: Direct document mutations bypass the command system
    /// and undo/redo history. Use EditorCommand + dispatch() instead.
    #[allow(dead_code)]
    pub(crate) fn document_mut(&mut self) -> Option<&mut crate::domain::Document> {
        self.workspace.active_document_mut()
    }
}
