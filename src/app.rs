//! Application state and editor logic.
//!
//! This represents the current editor session, combining document state
//! with view state via EditorView.

use arboard::Clipboard;
use log::debug;
use std::path::PathBuf;

use mudatexteditor::commands::{CommandHistory, EditOperation};
use mudatexteditor::domain::{Document, TextPosition, TextRange};
use mudatexteditor::syntax::SyntaxHighlighter;
use mudatexteditor::view::EditorView;

/// The main application state.
///
/// Combines a Document with an EditorView for editing.
pub struct App {
    /// The document being edited.
    pub document: Document,

    /// The editor view (cursor, selection, viewport, options).
    pub view: EditorView,

    /// Command history for undo/redo.
    pub history: CommandHistory,

    /// Whether the app should quit.
    pub should_quit: bool,

    /// Whether to show the exit confirmation dialog.
    pub show_exit_dialog: bool,
}

// Backwards compatibility shims - these will be removed after draw.rs and main.rs are updated
impl App {
    /// Backwards compat: get cursor_x
    pub fn cursor_x(&self) -> usize {
        let pos = self.document.offset_to_position(self.view.caret_offset());
        pos.column
    }

    /// Backwards compat: get cursor_y
    pub fn cursor_y(&self) -> usize {
        let pos = self.document.offset_to_position(self.view.caret_offset());
        pos.line
    }

    /// Backwards compat: get scroll_x
    pub fn scroll_x(&self) -> usize {
        self.view.viewport.scroll_x
    }

    /// Backwards compat: get scroll_y
    pub fn scroll_y(&self) -> usize {
        self.view.viewport.scroll_y
    }

    /// Backwards compat: check if selection is active
    pub fn selection_anchor(&self) -> Option<usize> {
        self.view.selection_range().map(|(start, _)| start)
    }

    /// Backwards compat: show line numbers
    pub fn show_line_numbers(&self) -> bool {
        self.view.show_line_numbers()
    }
}

impl App {
    pub fn new() -> Self {
        let document = Document::new();
        let view = EditorView::new(document.id());

        Self {
            document,
            view,
            history: CommandHistory::new(),
            should_quit: false,
            show_exit_dialog: false,
        }
    }

    pub fn open_file(path: &str) -> std::io::Result<Self> {
        let path_buf = PathBuf::from(path);
        let document = Document::open(&path_buf)?;
        log::debug!("Opened file: {}", path);

        let view = EditorView::new(document.id());

        Ok(Self {
            document,
            view,
            history: CommandHistory::new(),
            should_quit: false,
            show_exit_dialog: false,
        })
    }

    pub fn save(&mut self) -> std::io::Result<bool> {
        self.document.save()
    }

    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        self.document.save_as(std::path::Path::new(path))
    }

    pub fn get_title(&self) -> String {
        self.document.title()
    }

    /// Returns whether the document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.document.is_dirty()
    }

    pub fn mark_dirty(&mut self) {
        self.document.mark_dirty();
    }

    /// Returns the file path if the document is associated with a file.
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.document.file_path()
    }

    /// Returns the highlighter for syntax highlighting.
    pub fn highlighter(&self) -> &SyntaxHighlighter {
        self.document.highlighter()
    }

    /// Returns the content as a Rope (for draw.rs compatibility).
    pub fn content(&self) -> &ropey::Rope {
        self.document.buffer().rope()
    }

    // =========================================================================
    // Cursor and coordinate helpers
    // =========================================================================

    fn get_line_len(&self, line_idx: usize) -> usize {
        self.document.line_len(line_idx)
    }

    /// Returns the current cursor position as a character offset.
    pub fn cursor_to_char_idx(&self) -> usize {
        self.view.caret_offset()
    }

    fn char_idx_to_position(&self, char_idx: usize) -> TextPosition {
        self.document.offset_to_position(char_idx)
    }

    fn set_cursor_from_char_idx(&mut self, char_idx: usize, extend_selection: bool) {
        let clamped = char_idx.min(self.document.len_chars());
        self.view.move_caret_to(clamped, extend_selection);
    }

    // =========================================================================
    // Cursor movement
    // =========================================================================

    pub fn move_cursor_left(&mut self) {
        let offset = self.view.caret_offset();
        if offset > 0 {
            self.view.move_caret_to(offset - 1, false);
            self.view.clear_selection();
        }
    }

    pub fn move_cursor_right(&mut self) {
        let offset = self.view.caret_offset();
        if offset < self.document.len_chars() {
            self.view.move_caret_to(offset + 1, false);
            self.view.clear_selection();
        }
    }

    pub fn move_cursor_up(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        if pos.line > 0 {
            let new_line = pos.line - 1;
            let new_col = pos.column.min(self.get_line_len(new_line));
            let new_offset = self
                .document
                .position_to_offset(TextPosition::new(new_line, new_col));
            self.view.move_caret_to(new_offset, false);
            self.view.clear_selection();
        }
    }

    pub fn move_cursor_down(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        if pos.line + 1 < self.document.len_lines() {
            let new_line = pos.line + 1;
            let new_col = pos.column.min(self.get_line_len(new_line));
            let new_offset = self
                .document
                .position_to_offset(TextPosition::new(new_line, new_col));
            self.view.move_caret_to(new_offset, false);
            self.view.clear_selection();
        }
    }

    pub fn move_cursor_word_left(&mut self) {
        let mut offset = self.view.caret_offset();

        // Skip whitespace
        while offset > 0 {
            if let Some(ch) = self.document.buffer().char_at(offset - 1) {
                if !ch.is_whitespace() && ch != '\n' {
                    break;
                }
            }
            offset -= 1;
        }

        // Skip word characters
        while offset > 0 {
            if let Some(ch) = self.document.buffer().char_at(offset - 1) {
                if ch.is_whitespace() || ch == '\n' {
                    break;
                }
            }
            offset -= 1;
        }

        self.view.move_caret_to(offset, false);
        self.view.clear_selection();
    }

    pub fn move_cursor_word_right(&mut self) {
        let total_chars = self.document.len_chars();
        let mut offset = self.view.caret_offset();

        // Skip word characters
        while offset < total_chars {
            if let Some(ch) = self.document.buffer().char_at(offset) {
                if ch.is_whitespace() || ch == '\n' {
                    break;
                }
            }
            offset += 1;
        }

        // Skip whitespace
        while offset < total_chars {
            if let Some(ch) = self.document.buffer().char_at(offset) {
                if !ch.is_whitespace() && ch != '\n' {
                    break;
                }
            }
            offset += 1;
        }

        self.view.move_caret_to(offset, false);
        self.view.clear_selection();
    }

    pub fn check_scrolling(&mut self, width: usize, height: usize) {
        self.view.viewport.resize(width, height);
        let pos = self.char_idx_to_position(self.view.caret_offset());
        self.view.ensure_caret_visible(pos.line, pos.column);
    }

    // =========================================================================
    // Editing operations
    // =========================================================================

    pub fn insert_char(&mut self, ch: char) {
        let char_idx = self.view.caret_offset();
        let op = EditOperation::insert_char(char_idx, ch);
        self.history.execute(op, self.document.buffer_mut());
        self.view.move_caret_to(char_idx + 1, false);
        self.view.clear_selection();
        self.document.update_syntax();
    }

    pub fn insert_newline(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let current_line = self.document.line(pos.line);
        let indent: String = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        let char_idx = self.view.caret_offset();

        let newline_op = EditOperation::insert_char(char_idx, '\n');
        self.history.execute(newline_op, self.document.buffer_mut());

        let mut new_offset = char_idx + 1;

        if !indent.is_empty() {
            let indent_op = EditOperation::insert_string(char_idx + 1, indent.clone());
            self.history.execute(indent_op, self.document.buffer_mut());
            new_offset += indent.len();
        }

        self.view.move_caret_to(new_offset, false);
        self.view.clear_selection();
        self.document.update_syntax();
    }

    pub fn clamp_cursor(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let line_len = self.get_line_len(pos.line);
        if pos.column > line_len {
            let new_offset = self
                .document
                .position_to_offset(TextPosition::new(pos.line, line_len));
            self.view.move_caret_to(new_offset, false);
        }
    }

    pub fn backspace(&mut self) {
        let char_idx = self.view.caret_offset();
        if char_idx > 0 {
            let deleted_char = self.document.buffer().char_at(char_idx - 1).unwrap_or(' ');

            let op = EditOperation::delete(char_idx - 1, deleted_char.to_string());
            self.history.execute(op, self.document.buffer_mut());
            self.view.move_caret_to(char_idx - 1, false);
            self.view.clear_selection();
            self.document.update_syntax();
        }
    }

    pub fn delete_at_cursor(&mut self) {
        let char_idx = self.view.caret_offset();
        if char_idx < self.document.len_chars() {
            let deleted_char = self.document.buffer().char_at(char_idx).unwrap_or(' ');
            let op = EditOperation::delete(char_idx, deleted_char.to_string());
            self.history.execute(op, self.document.buffer_mut());
            self.document.update_syntax();
        }
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Start selection from current position (for shift+arrow keys)
    pub fn begin_selection(&mut self) {
        self.view.begin_selection();
    }

    /// Extend selection to given offset
    pub fn extend_selection_to(&mut self, offset: usize) {
        self.view.move_caret_to(offset, true);
    }

    /// Clear any active selection
    pub fn clear_selection(&mut self) {
        self.view.clear_selection();
    }

    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        self.view.selection_range()
    }

    pub fn copy_selection(&self) {
        if let Some((start, end)) = self.get_selection_range() {
            let text = self.document.slice(TextRange::new(start, end));
            if let Ok(mut cb) = Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }
    }

    pub fn cut_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            debug!("=== CUT SELECTION ===");
            debug!("Selection range: start={}, end={}", start, end);

            let text = self.document.slice(TextRange::new(start, end));

            if let Ok(mut cb) = Clipboard::new() {
                let _ = cb.set_text(text.clone());
            }

            let op = EditOperation::delete(start, text);
            self.history.execute(op, self.document.buffer_mut());

            self.view.move_caret_to(start, false);
            self.view.clear_selection();
            self.document.update_syntax();
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let text = self.document.slice(TextRange::new(start, end));

            let op = EditOperation::delete(start, text);
            self.history.execute(op, self.document.buffer_mut());

            self.view.move_caret_to(start, false);
            self.view.clear_selection();
            self.document.update_syntax();
        }
    }

    pub fn insert_string_at_cursor(&mut self, text: &str) {
        let char_idx = self.view.caret_offset();
        let op = EditOperation::insert_string(char_idx, text);
        self.history.execute(op, self.document.buffer_mut());

        let new_char_idx = char_idx + text.chars().count();
        self.view.move_caret_to(new_char_idx, false);
        self.view.clear_selection();
        self.document.update_syntax();
    }

    pub fn paste(&mut self) {
        if let Ok(mut cb) = Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                self.insert_string_at_cursor(&text);
            }
        }
    }

    // =========================================================================
    // Undo/Redo
    // =========================================================================

    pub fn undo(&mut self) {
        debug!("=== UNDO ===");
        if let Some(pos) = self.history.undo(self.document.buffer_mut()) {
            self.view.move_caret_to(pos, false);
            self.view.clear_selection();
            self.document.update_syntax();
            let cursor = self.char_idx_to_position(pos);
            debug!(
                "Undo complete, cursor at ({}, {})",
                cursor.column, cursor.line
            );
        }
    }

    pub fn redo(&mut self) {
        debug!("=== REDO ===");
        if let Some(pos) = self.history.redo(self.document.buffer_mut()) {
            self.view.move_caret_to(pos, false);
            self.view.clear_selection();
            self.document.update_syntax();
            let cursor = self.char_idx_to_position(pos);
            debug!(
                "Redo complete, cursor at ({}, {})",
                cursor.column, cursor.line
            );
        }
    }

    // =========================================================================
    // Other operations
    // =========================================================================

    pub fn select_all(&mut self) {
        let total_chars = self.document.len_chars();
        self.view.select_all(total_chars);
    }

    pub fn toggle_line_numbers(&mut self) {
        self.view.toggle_line_numbers();
    }

    pub fn line_number_width(&self) -> usize {
        if self.view.show_line_numbers() {
            let line_count = self.document.len_lines();
            let digits = line_count.to_string().len();
            digits + 2
        } else {
            0
        }
    }

    pub fn move_cursor_home(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let new_offset = self
            .document
            .position_to_offset(TextPosition::new(pos.line, 0));
        self.view.move_caret_to(new_offset, false);
        self.view.clear_selection();
    }

    pub fn move_cursor_end(&mut self) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let line_len = self.get_line_len(pos.line);
        let new_offset = self
            .document
            .position_to_offset(TextPosition::new(pos.line, line_len));
        self.view.move_caret_to(new_offset, false);
        self.view.clear_selection();
    }

    pub fn move_cursor_file_start(&mut self) {
        self.view.move_caret_to(0, false);
        self.view.clear_selection();
    }

    pub fn move_cursor_file_end(&mut self) {
        let total = self.document.len_chars();
        self.view.move_caret_to(total, false);
        self.view.clear_selection();
    }

    pub fn page_up(&mut self, page_height: usize) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let new_line = pos.line.saturating_sub(page_height);
        let new_col = pos.column.min(self.get_line_len(new_line));
        let new_offset = self
            .document
            .position_to_offset(TextPosition::new(new_line, new_col));
        self.view.move_caret_to(new_offset, false);
        self.view.clear_selection();
    }

    pub fn page_down(&mut self, page_height: usize) {
        let pos = self.char_idx_to_position(self.view.caret_offset());
        let new_line = (pos.line + page_height).min(self.document.len_lines().saturating_sub(1));
        let new_col = pos.column.min(self.get_line_len(new_line));
        let new_offset = self
            .document
            .position_to_offset(TextPosition::new(new_line, new_col));
        self.view.move_caret_to(new_offset, false);
        self.view.clear_selection();
    }

    pub fn update_syntax(&mut self) {
        self.document.update_syntax();
    }
}
