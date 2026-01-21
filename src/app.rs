//! Application state and editor logic.
//!
//! App is a thin UI-facing shell that:
//! - Holds Workspace + CommandDispatcher
//! - Manages UI-only state (should_quit, dialogs)
//! - Dispatches EditorCommands to the workspace
//! - Handles application-level commands (Save, Quit, etc.)

use std::path::PathBuf;

use mudatexteditor::commands::{CommandContext, CommandDispatcher, DispatchResult, EditorCommand};
use mudatexteditor::commands::editor_command::{Direction, MoveScope};
use mudatexteditor::domain::Workspace;
use mudatexteditor::view_model::{builder::ViewModelBuilder, RenderModel};

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

    /// Whether to show the exit confirmation dialog.
    pub show_exit_dialog: bool,
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
            show_exit_dialog: false,
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
            show_exit_dialog: false,
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
        let dispatcher = &mut self.dispatcher;

        let result = self.workspace.with_active_context(|view, document, history, event_bus| {
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

    /// Saves the active document.
    pub fn save(&mut self) -> std::io::Result<bool> {
        self.workspace.save_active_document()
    }

    /// Saves the active document to a new path.
    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        if let Some(doc) = self.workspace.active_document_mut() {
            doc.save_as(std::path::Path::new(path))
        } else {
            Ok(())
        }
    }

    /// Returns whether the active document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.workspace.active_document_dirty()
    }

    /// Returns the file path of the active document.
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.workspace.active_document().and_then(|d| d.file_path())
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
        self.workspace.active_view().and_then(|v| v.selection_range())
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
    // Render Model
    // =========================================================================

    /// Builds a render-ready model for the UI.
    pub fn build_render_model(&self) -> RenderModel {
        match (self.workspace.active_document(), self.workspace.active_view()) {
            (Some(doc), Some(view)) => {
                ViewModelBuilder::build(doc, view, self.show_exit_dialog)
            }
            _ => RenderModel::default(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// Expose view for direct access in main.rs (backwards compatibility)
// =========================================================================

impl App {
    /// Returns a reference to the active view.
    ///
    /// This is provided for backwards compatibility with main.rs.
    pub fn view(&self) -> Option<&mudatexteditor::view::EditorView> {
        self.workspace.active_view()
    }

    /// Returns a mutable reference to the active view.
    pub fn view_mut(&mut self) -> Option<&mut mudatexteditor::view::EditorView> {
        self.workspace.active_view_mut()
    }

    /// Returns a reference to the active document.
    pub fn document(&self) -> Option<&mudatexteditor::domain::Document> {
        self.workspace.active_document()
    }

    /// Returns a mutable reference to the active document.
    pub fn document_mut(&mut self) -> Option<&mut mudatexteditor::domain::Document> {
        self.workspace.active_document_mut()
    }
}
