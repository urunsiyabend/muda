//! CommandDispatcher: Routes EditorCommands to the appropriate handlers.
//!
//! The dispatcher orchestrates command execution by:
//! 1. Receiving high-level EditorCommands
//! 2. Translating them to EditOperations
//! 3. Executing operations on the document
//! 4. Updating view state (caret, selection)
//! 5. Emitting domain events
//!
//! This extracts all editing logic from the App into a reusable component.

use arboard::Clipboard;
use log::debug;

use crate::commands::{CommandHistory, EditOperation, EditorCommand};
use crate::commands::editor_command::{Direction, MoveScope};
use crate::domain::{Document, TextPosition, TextRange};
use crate::events::EventBus;
use crate::view::EditorView;

/// Context for command execution.
///
/// Provides mutable access to all state needed for command handling.
pub struct CommandContext<'a> {
    /// The view being operated on.
    pub view: &'a mut EditorView,
    /// The document being edited.
    pub document: &'a mut Document,
    /// The undo/redo history.
    pub history: &'a mut CommandHistory,
    /// The event bus for emitting events.
    pub event_bus: &'a mut EventBus,
}

/// Result of command dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    /// Command executed successfully.
    Executed,
    /// Command requires application-level handling (Quit, dialogs, etc.).
    RequiresAppHandling,
    /// No operation was performed.
    NoOp,
}

/// Dispatches EditorCommands to their handlers.
///
/// Encapsulates all editing logic, making it reusable across different
/// UI frontends (TUI, GUI, tests).
pub struct CommandDispatcher {
    /// System clipboard for copy/cut/paste.
    clipboard: Option<Clipboard>,
}

impl CommandDispatcher {
    /// Creates a new command dispatcher.
    pub fn new() -> Self {
        Self {
            clipboard: Clipboard::new().ok(),
        }
    }

    /// Dispatches a command to the appropriate handler.
    ///
    /// Returns the dispatch result indicating what action was taken.
    pub fn dispatch(&mut self, cmd: EditorCommand, ctx: &mut CommandContext) -> DispatchResult {
        match cmd {
            // === Navigation ===
            EditorCommand::MoveCursor {
                direction,
                scope,
                extend_selection,
            } => {
                self.handle_move_cursor(ctx, direction, scope, extend_selection);
                DispatchResult::Executed
            }
            EditorCommand::GotoLine(line) => {
                self.handle_goto_line(ctx, line);
                DispatchResult::Executed
            }

            // === Editing ===
            EditorCommand::InsertChar(ch) => {
                self.handle_insert_char(ctx, ch);
                DispatchResult::Executed
            }
            EditorCommand::InsertText(text) => {
                self.handle_insert_text(ctx, &text);
                DispatchResult::Executed
            }
            EditorCommand::InsertNewline => {
                self.handle_insert_newline(ctx);
                DispatchResult::Executed
            }
            EditorCommand::Backspace => {
                self.handle_backspace(ctx);
                DispatchResult::Executed
            }
            EditorCommand::Delete => {
                self.handle_delete(ctx);
                DispatchResult::Executed
            }
            EditorCommand::DeleteSelection => {
                self.handle_delete_selection(ctx);
                DispatchResult::Executed
            }

            // === Selection ===
            EditorCommand::SelectAll => {
                self.handle_select_all(ctx);
                DispatchResult::Executed
            }
            EditorCommand::ClearSelection => {
                ctx.view.clear_selection();
                self.emit_selection_changed(ctx);
                DispatchResult::Executed
            }

            // === Clipboard ===
            EditorCommand::Copy => {
                self.handle_copy(ctx);
                DispatchResult::Executed
            }
            EditorCommand::Cut => {
                self.handle_cut(ctx);
                DispatchResult::Executed
            }
            EditorCommand::Paste => {
                self.handle_paste(ctx);
                DispatchResult::Executed
            }

            // === Undo/Redo ===
            EditorCommand::Undo => {
                self.handle_undo(ctx);
                DispatchResult::Executed
            }
            EditorCommand::Redo => {
                self.handle_redo(ctx);
                DispatchResult::Executed
            }

            // === View ===
            EditorCommand::ToggleLineNumbers => {
                ctx.view.toggle_line_numbers();
                DispatchResult::Executed
            }
            EditorCommand::Scroll { lines: _ } => {
                // Scroll is typically handled by the UI layer
                DispatchResult::NoOp
            }

            // === Application-level (require App handling) ===
            EditorCommand::Save
            | EditorCommand::SaveAs(_)
            | EditorCommand::Open(_)
            | EditorCommand::New
            | EditorCommand::Quit
            | EditorCommand::ForceQuit
            | EditorCommand::Cancel => DispatchResult::RequiresAppHandling,
        }
    }

    // =========================================================================
    // Event Emission Helpers
    // =========================================================================

    fn emit_document_changed(&self, ctx: &mut CommandContext, affected_range: Option<TextRange>) {
        let event = ctx.document.event_changed(affected_range);
        ctx.event_bus.publish(event);

        let syntax_event = ctx.document.event_syntax_updated();
        ctx.event_bus.publish(syntax_event);
    }

    fn emit_selection_changed(&self, ctx: &mut CommandContext) {
        let event = ctx.view.event_selection_changed();
        ctx.event_bus.publish(event);
    }

    fn emit_viewport_changed(&self, ctx: &mut CommandContext) {
        let event = ctx.view.event_viewport_changed();
        ctx.event_bus.publish(event);
    }

    // =========================================================================
    // Coordinate Helpers
    // =========================================================================

    fn offset_to_position(&self, ctx: &CommandContext, offset: usize) -> TextPosition {
        ctx.document.offset_to_position(offset)
    }

    fn position_to_offset(&self, ctx: &CommandContext, pos: TextPosition) -> usize {
        ctx.document.position_to_offset(pos)
    }

    fn line_len(&self, ctx: &CommandContext, line: usize) -> usize {
        ctx.document.line_len(line)
    }

    // =========================================================================
    // Navigation Handlers
    // =========================================================================

    fn handle_move_cursor(
        &self,
        ctx: &mut CommandContext,
        direction: Direction,
        scope: MoveScope,
        extend_selection: bool,
    ) {
        // Start selection if extending from no selection
        if extend_selection && !ctx.view.has_selection() {
            ctx.view.begin_selection();
        }

        let new_offset = match (direction, scope) {
            (Direction::Left, MoveScope::Char) => self.move_left_char(ctx),
            (Direction::Right, MoveScope::Char) => self.move_right_char(ctx),
            (Direction::Up, MoveScope::Char) => self.move_up(ctx),
            (Direction::Down, MoveScope::Char) => self.move_down(ctx),
            (Direction::Left, MoveScope::Word) => self.move_left_word(ctx),
            (Direction::Right, MoveScope::Word) => self.move_right_word(ctx),
            (Direction::Left, MoveScope::Line) => self.move_line_start(ctx),
            (Direction::Right, MoveScope::Line) => self.move_line_end(ctx),
            (Direction::Up, MoveScope::Page) | (Direction::Left, MoveScope::Page) => {
                self.move_page_up(ctx)
            }
            (Direction::Down, MoveScope::Page) | (Direction::Right, MoveScope::Page) => {
                self.move_page_down(ctx)
            }
            (Direction::Left, MoveScope::Document) | (Direction::Up, MoveScope::Document) => 0,
            (Direction::Right, MoveScope::Document) | (Direction::Down, MoveScope::Document) => {
                ctx.document.len_chars()
            }
            _ => ctx.view.caret_offset(), // No change for unhandled combinations
        };

        ctx.view.move_caret_to(new_offset, extend_selection);

        if !extend_selection {
            ctx.view.clear_selection();
        }

        self.emit_selection_changed(ctx);
    }

    fn move_left_char(&self, ctx: &CommandContext) -> usize {
        let offset = ctx.view.caret_offset();
        if offset > 0 { offset - 1 } else { 0 }
    }

    fn move_right_char(&self, ctx: &CommandContext) -> usize {
        let offset = ctx.view.caret_offset();
        let max = ctx.document.len_chars();
        if offset < max { offset + 1 } else { max }
    }

    fn move_up(&self, ctx: &CommandContext) -> usize {
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        if pos.line > 0 {
            let new_line = pos.line - 1;
            let new_col = pos.column.min(self.line_len(ctx, new_line));
            self.position_to_offset(ctx, TextPosition::new(new_line, new_col))
        } else {
            ctx.view.caret_offset()
        }
    }

    fn move_down(&self, ctx: &CommandContext) -> usize {
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        if pos.line + 1 < ctx.document.len_lines() {
            let new_line = pos.line + 1;
            let new_col = pos.column.min(self.line_len(ctx, new_line));
            self.position_to_offset(ctx, TextPosition::new(new_line, new_col))
        } else {
            ctx.view.caret_offset()
        }
    }

    fn move_left_word(&self, ctx: &CommandContext) -> usize {
        let mut offset = ctx.view.caret_offset();

        // Skip whitespace backwards
        while offset > 0 {
            if let Some(ch) = ctx.document.buffer().char_at(offset - 1) {
                if !ch.is_whitespace() && ch != '\n' {
                    break;
                }
            }
            offset -= 1;
        }

        // Skip word characters backwards
        while offset > 0 {
            if let Some(ch) = ctx.document.buffer().char_at(offset - 1) {
                if ch.is_whitespace() || ch == '\n' {
                    break;
                }
            }
            offset -= 1;
        }

        offset
    }

    fn move_right_word(&self, ctx: &CommandContext) -> usize {
        let total_chars = ctx.document.len_chars();
        let mut offset = ctx.view.caret_offset();

        // Skip word characters forwards
        while offset < total_chars {
            if let Some(ch) = ctx.document.buffer().char_at(offset) {
                if ch.is_whitespace() || ch == '\n' {
                    break;
                }
            }
            offset += 1;
        }

        // Skip whitespace forwards
        while offset < total_chars {
            if let Some(ch) = ctx.document.buffer().char_at(offset) {
                if !ch.is_whitespace() && ch != '\n' {
                    break;
                }
            }
            offset += 1;
        }

        offset
    }

    fn move_line_start(&self, ctx: &CommandContext) -> usize {
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        self.position_to_offset(ctx, TextPosition::new(pos.line, 0))
    }

    fn move_line_end(&self, ctx: &CommandContext) -> usize {
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        let line_len = self.line_len(ctx, pos.line);
        self.position_to_offset(ctx, TextPosition::new(pos.line, line_len))
    }

    fn move_page_up(&self, ctx: &CommandContext) -> usize {
        let page_height = ctx.view.viewport.height.max(1);
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        let new_line = pos.line.saturating_sub(page_height);
        let new_col = pos.column.min(self.line_len(ctx, new_line));
        self.position_to_offset(ctx, TextPosition::new(new_line, new_col))
    }

    fn move_page_down(&self, ctx: &CommandContext) -> usize {
        let page_height = ctx.view.viewport.height.max(1);
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        let max_line = ctx.document.len_lines().saturating_sub(1);
        let new_line = (pos.line + page_height).min(max_line);
        let new_col = pos.column.min(self.line_len(ctx, new_line));
        self.position_to_offset(ctx, TextPosition::new(new_line, new_col))
    }

    fn handle_goto_line(&self, ctx: &mut CommandContext, line: usize) {
        // Convert 1-indexed line to 0-indexed
        let line_idx = line.saturating_sub(1).min(ctx.document.len_lines().saturating_sub(1));
        let offset = self.position_to_offset(ctx, TextPosition::new(line_idx, 0));
        ctx.view.move_caret_to(offset, false);
        ctx.view.clear_selection();
        self.emit_selection_changed(ctx);
    }

    // =========================================================================
    // Editing Handlers
    // =========================================================================

    fn handle_insert_char(&self, ctx: &mut CommandContext, ch: char) {
        let offset = ctx.view.caret_offset();
        let op = EditOperation::insert_char(offset, ch);
        ctx.history.execute(op, ctx.document.buffer_mut());
        ctx.view.move_caret_to(offset + 1, false);
        ctx.view.clear_selection();

        // Use incremental parsing for single character inserts
        ctx.document.update_syntax_incremental(offset, 0, 1);

        let affected = TextRange::new(offset, offset + 1);
        self.emit_document_changed(ctx, Some(affected));
        self.emit_selection_changed(ctx);
    }

    fn handle_insert_text(&self, ctx: &mut CommandContext, text: &str) {
        let offset = ctx.view.caret_offset();
        let text_len = text.chars().count();
        let op = EditOperation::insert_string(offset, text);
        ctx.history.execute(op, ctx.document.buffer_mut());

        let new_offset = offset + text_len;
        ctx.view.move_caret_to(new_offset, false);
        ctx.view.clear_selection();

        // Use incremental parsing for text inserts
        ctx.document.update_syntax_incremental(offset, 0, text_len);

        let affected = TextRange::new(offset, new_offset);
        self.emit_document_changed(ctx, Some(affected));
        self.emit_selection_changed(ctx);
    }

    fn handle_insert_newline(&self, ctx: &mut CommandContext) {
        let offset = ctx.view.caret_offset();
        let pos = self.offset_to_position(ctx, offset);

        // Get current line indentation
        let current_line = ctx.document.line(pos.line);
        let indent: String = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        // Insert newline
        let newline_op = EditOperation::insert_char(offset, '\n');
        ctx.history.execute(newline_op, ctx.document.buffer_mut());

        let mut new_offset = offset + 1;
        let mut total_inserted = 1;

        // Insert indentation if any
        if !indent.is_empty() {
            let indent_len = indent.chars().count();
            let indent_op = EditOperation::insert_string(offset + 1, indent.clone());
            ctx.history.execute(indent_op, ctx.document.buffer_mut());
            new_offset += indent_len;
            total_inserted += indent_len;
        }

        ctx.view.move_caret_to(new_offset, false);
        ctx.view.clear_selection();

        // Use incremental parsing
        ctx.document.update_syntax_incremental(offset, 0, total_inserted);

        let affected = TextRange::new(offset, new_offset);
        self.emit_document_changed(ctx, Some(affected));
        self.emit_selection_changed(ctx);
    }

    fn handle_backspace(&self, ctx: &mut CommandContext) {
        if ctx.view.has_selection() {
            self.handle_delete_selection(ctx);
            return;
        }

        let offset = ctx.view.caret_offset();
        if offset > 0 {
            let deleted_char = ctx.document.buffer().char_at(offset - 1).unwrap_or(' ');
            let op = EditOperation::delete(offset - 1, deleted_char.to_string());
            ctx.history.execute(op, ctx.document.buffer_mut());
            ctx.view.move_caret_to(offset - 1, false);
            ctx.view.clear_selection();

            // Use incremental parsing for single character delete
            ctx.document.update_syntax_incremental(offset - 1, 1, 0);

            let affected = TextRange::new(offset - 1, offset);
            self.emit_document_changed(ctx, Some(affected));
            self.emit_selection_changed(ctx);
        }
    }

    fn handle_delete(&self, ctx: &mut CommandContext) {
        if ctx.view.has_selection() {
            self.handle_delete_selection(ctx);
            return;
        }

        let offset = ctx.view.caret_offset();
        if offset < ctx.document.len_chars() {
            let deleted_char = ctx.document.buffer().char_at(offset).unwrap_or(' ');
            let op = EditOperation::delete(offset, deleted_char.to_string());
            ctx.history.execute(op, ctx.document.buffer_mut());

            // Use incremental parsing for single character delete
            ctx.document.update_syntax_incremental(offset, 1, 0);

            let affected = TextRange::new(offset, offset + 1);
            self.emit_document_changed(ctx, Some(affected));
        }
    }

    fn handle_delete_selection(&self, ctx: &mut CommandContext) {
        if let Some((start, end)) = ctx.view.selection_range() {
            let deleted_len = end - start;
            let text = ctx.document.slice(TextRange::new(start, end));
            let op = EditOperation::delete(start, text);
            ctx.history.execute(op, ctx.document.buffer_mut());

            ctx.view.move_caret_to(start, false);
            ctx.view.clear_selection();

            // Use incremental parsing for selection delete
            ctx.document.update_syntax_incremental(start, deleted_len, 0);

            let affected = TextRange::new(start, end);
            self.emit_document_changed(ctx, Some(affected));
            self.emit_selection_changed(ctx);
        }
    }

    // =========================================================================
    // Selection Handlers
    // =========================================================================

    fn handle_select_all(&self, ctx: &mut CommandContext) {
        let total_chars = ctx.document.len_chars();
        ctx.view.select_all(total_chars);
        self.emit_selection_changed(ctx);
    }

    // =========================================================================
    // Clipboard Handlers
    // =========================================================================

    fn handle_copy(&mut self, ctx: &CommandContext) {
        if let Some((start, end)) = ctx.view.selection_range() {
            let text = ctx.document.slice(TextRange::new(start, end));
            if let Some(ref mut cb) = self.clipboard {
                let _ = cb.set_text(text);
            }
        }
    }

    fn handle_cut(&mut self, ctx: &mut CommandContext) {
        if let Some((start, end)) = ctx.view.selection_range() {
            debug!("=== CUT SELECTION ===");
            debug!("Selection range: start={}, end={}", start, end);

            let deleted_len = end - start;
            let text = ctx.document.slice(TextRange::new(start, end));

            if let Some(ref mut cb) = self.clipboard {
                let _ = cb.set_text(text.clone());
            }

            let op = EditOperation::delete(start, text);
            ctx.history.execute(op, ctx.document.buffer_mut());

            ctx.view.move_caret_to(start, false);
            ctx.view.clear_selection();

            // Use incremental parsing
            ctx.document.update_syntax_incremental(start, deleted_len, 0);

            let affected = TextRange::new(start, end);
            self.emit_document_changed(ctx, Some(affected));
            self.emit_selection_changed(ctx);
        }
    }

    fn handle_paste(&mut self, ctx: &mut CommandContext) {
        if let Some(ref mut cb) = self.clipboard {
            if let Ok(text) = cb.get_text() {
                // Delete selection first if any
                if ctx.view.has_selection() {
                    self.handle_delete_selection(ctx);
                }
                self.handle_insert_text(ctx, &text);
            }
        }
    }

    // =========================================================================
    // Undo/Redo Handlers
    // =========================================================================

    fn handle_undo(&self, ctx: &mut CommandContext) {
        debug!("=== UNDO ===");
        if let Some(pos) = ctx.history.undo(ctx.document.buffer_mut()) {
            ctx.view.move_caret_to(pos, false);
            ctx.view.clear_selection();
            ctx.document.update_syntax();
            let cursor = self.offset_to_position(ctx, pos);
            debug!("Undo complete, cursor at ({}, {})", cursor.column, cursor.line);

            self.emit_document_changed(ctx, None);
            self.emit_selection_changed(ctx);
        }
    }

    fn handle_redo(&self, ctx: &mut CommandContext) {
        debug!("=== REDO ===");
        if let Some(pos) = ctx.history.redo(ctx.document.buffer_mut()) {
            ctx.view.move_caret_to(pos, false);
            ctx.view.clear_selection();
            ctx.document.update_syntax();
            let cursor = self.offset_to_position(ctx, pos);
            debug!("Redo complete, cursor at ({}, {})", cursor.column, cursor.line);

            self.emit_document_changed(ctx, None);
            self.emit_selection_changed(ctx);
        }
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> (Document, EditorView, CommandHistory, EventBus) {
        let doc = Document::from_str("Hello World", None);
        let view = EditorView::new(doc.id());
        let history = CommandHistory::new();
        let event_bus = EventBus::new();
        (doc, view, history, event_bus)
    }

    #[test]
    fn test_insert_char() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Move to end and insert
        ctx.view.move_caret_to(11, false);
        let result = dispatcher.dispatch(EditorCommand::InsertChar('!'), &mut ctx);

        assert_eq!(result, DispatchResult::Executed);
        assert_eq!(ctx.document.content(), "Hello World!");
        assert_eq!(ctx.view.caret_offset(), 12);
    }

    #[test]
    fn test_backspace() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        ctx.view.move_caret_to(5, false);
        let result = dispatcher.dispatch(EditorCommand::Backspace, &mut ctx);

        assert_eq!(result, DispatchResult::Executed);
        assert_eq!(ctx.document.content(), "Hell World");
        assert_eq!(ctx.view.caret_offset(), 4);
    }

    #[test]
    fn test_move_cursor() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        ctx.view.move_caret_to(5, false);

        let cmd = EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: MoveScope::Char,
            extend_selection: false,
        };
        dispatcher.dispatch(cmd, &mut ctx);

        assert_eq!(ctx.view.caret_offset(), 6);
        assert!(!ctx.view.has_selection());
    }

    #[test]
    fn test_select_all() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        dispatcher.dispatch(EditorCommand::SelectAll, &mut ctx);

        assert!(ctx.view.has_selection());
        assert_eq!(ctx.view.selection_range(), Some((0, 11)));
    }

    #[test]
    fn test_undo_redo() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Insert a character
        ctx.view.move_caret_to(5, false);
        dispatcher.dispatch(EditorCommand::InsertChar('X'), &mut ctx);
        assert_eq!(ctx.document.content(), "HelloX World");

        // Undo
        dispatcher.dispatch(EditorCommand::Undo, &mut ctx);
        assert_eq!(ctx.document.content(), "Hello World");

        // Redo
        dispatcher.dispatch(EditorCommand::Redo, &mut ctx);
        assert_eq!(ctx.document.content(), "HelloX World");
    }

    #[test]
    fn test_requires_app_handling() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        assert_eq!(
            dispatcher.dispatch(EditorCommand::Quit, &mut ctx),
            DispatchResult::RequiresAppHandling
        );
        assert_eq!(
            dispatcher.dispatch(EditorCommand::Save, &mut ctx),
            DispatchResult::RequiresAppHandling
        );
    }
}
