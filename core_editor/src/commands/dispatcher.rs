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
    /// Tracks whether the last clipboard write was a full-line copy.
    clipboard_is_line_copy: bool,
}

impl CommandDispatcher {
    /// Creates a new command dispatcher.
    pub fn new() -> Self {
        Self {
            clipboard: Clipboard::new().ok(),
            clipboard_is_line_copy: false,
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

            // === Mouse interaction ===
            EditorCommand::ClickAt { line, col, extend_selection, click_count } => {
                self.handle_click_at(ctx, line, col, extend_selection, click_count);
                DispatchResult::Executed
            }
            EditorCommand::DragTo { line, col, snap_mode } => {
                self.handle_drag_to(ctx, line, col, snap_mode);
                DispatchResult::Executed
            }

            // === View ===
            EditorCommand::ToggleLineNumbers => {
                ctx.view.toggle_line_numbers();
                DispatchResult::Executed
            }
            EditorCommand::Scroll { lines } => {
                self.handle_scroll(ctx, lines);
                DispatchResult::Executed
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

    fn handle_scroll(&self, ctx: &mut CommandContext, lines: i32) {
        let total_lines = ctx.document.len_lines();

        let current_scroll = ctx.view.viewport.scroll_y as i64;
        // Clamp so we can't scroll past the last line or before line 0.
        let max_scroll = total_lines.saturating_sub(1) as i64;
        let new_scroll = (current_scroll + lines as i64).clamp(0, max_scroll) as usize;

        ctx.view.viewport.scroll_y = new_scroll;

        // Do NOT move the caret — mouse scroll only changes the viewport.
        // The caret stays at its original document position. When the user
        // types or navigates with arrow keys, ensure_caret_visible (called
        // from App::dispatch for non-scroll commands) will snap the viewport
        // back to the caret.

        self.emit_viewport_changed(ctx);
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
    // Mouse Click Handlers
    // =========================================================================

    fn handle_click_at(
        &self,
        ctx: &mut CommandContext,
        line: usize,
        col: usize,
        extend_selection: bool,
        click_count: u32,
    ) {
        let max_line = ctx.document.len_lines().saturating_sub(1);
        let line = line.min(max_line);
        let max_col = self.line_len(ctx, line);
        let col = col.min(max_col);
        let offset = self.position_to_offset(ctx, TextPosition::new(line, col));

        match click_count {
            2 => {
                // Double-click: select word under cursor.
                let (word_start, word_end) = self.find_word_boundaries(ctx, offset);
                // Set anchor at word start, move caret to word end extending selection.
                ctx.view.move_caret_to(word_start, false);
                ctx.view.begin_selection();
                ctx.view.move_caret_to(word_end, true);
            }
            3 => {
                // Triple-click: select entire line.
                let line_start = self.position_to_offset(ctx, TextPosition::new(line, 0));
                let line_end = if line + 1 < ctx.document.len_lines() {
                    self.position_to_offset(ctx, TextPosition::new(line + 1, 0))
                } else {
                    ctx.document.len_chars()
                };
                ctx.view.move_caret_to(line_start, false);
                ctx.view.begin_selection();
                ctx.view.move_caret_to(line_end, true);
            }
            _ => {
                // Single click.
                if extend_selection {
                    // Shift+click: extend selection from current anchor.
                    if !ctx.view.has_selection() {
                        ctx.view.begin_selection();
                    }
                    ctx.view.move_caret_to(offset, true);
                } else {
                    ctx.view.move_caret_to(offset, false);
                    ctx.view.clear_selection();
                }
            }
        }

        self.emit_selection_changed(ctx);
    }

    fn handle_drag_to(&self, ctx: &mut CommandContext, line: usize, col: usize, snap_mode: u32) {
        let max_line = ctx.document.len_lines().saturating_sub(1);
        let line = line.min(max_line);
        let max_col = self.line_len(ctx, line);
        let col = col.min(max_col);
        let offset = self.position_to_offset(ctx, TextPosition::new(line, col));

        match snap_mode {
            1 => {
                // Word-snap: expand to word boundaries.
                let (word_start, word_end) = ctx.document.buffer().word_boundary_at(offset);
                let anchor = ctx.view.selection_anchor().unwrap_or(ctx.view.caret_offset());
                // Extend toward the boundary furthest from anchor.
                if offset >= anchor {
                    ctx.view.move_caret_to(word_end, true);
                } else {
                    ctx.view.move_caret_to(word_start, true);
                }
            }
            2 => {
                // Line-snap: extend to line boundaries.
                let anchor = ctx.view.selection_anchor().unwrap_or(ctx.view.caret_offset());
                if offset >= anchor {
                    // Dragging down/right: extend to end of line (or start of next line).
                    let target = if line + 1 < ctx.document.len_lines() {
                        self.position_to_offset(ctx, TextPosition::new(line + 1, 0))
                    } else {
                        ctx.document.len_chars()
                    };
                    ctx.view.move_caret_to(target, true);
                } else {
                    // Dragging up/left: extend to start of line.
                    let line_start = self.position_to_offset(ctx, TextPosition::new(line, 0));
                    ctx.view.move_caret_to(line_start, true);
                }
            }
            _ => {
                // Char mode: extend selection to exact offset.
                ctx.view.move_caret_to(offset, true);
            }
        }
        self.emit_selection_changed(ctx);
    }

    /// Find word boundaries around the given offset using VS Code-like rules.
    /// Delegates to `TextBuffer::word_boundary_at`.
    fn find_word_boundaries(&self, ctx: &CommandContext, offset: usize) -> (usize, usize) {
        ctx.document.buffer().word_boundary_at(offset)
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
            // Selection active: copy selected text
            let text = ctx.document.slice(TextRange::new(start, end));
            if let Some(ref mut cb) = self.clipboard {
                let _ = cb.set_text(text);
            }
            self.clipboard_is_line_copy = false;
        } else {
            // No selection: copy entire current line including newline
            let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
            let line_text = ctx.document.line(pos.line);
            let full_line = format!("{}\n", line_text);
            if let Some(ref mut cb) = self.clipboard {
                let _ = cb.set_text(full_line);
            }
            self.clipboard_is_line_copy = true;
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
            self.clipboard_is_line_copy = false;

            let op = EditOperation::delete(start, text);
            ctx.history.execute(op, ctx.document.buffer_mut());

            ctx.view.move_caret_to(start, false);
            ctx.view.clear_selection();

            // Use incremental parsing
            ctx.document.update_syntax_incremental(start, deleted_len, 0);

            let affected = TextRange::new(start, end);
            self.emit_document_changed(ctx, Some(affected));
            self.emit_selection_changed(ctx);
        } else {
            // No selection: cut entire current line
            let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
            let line_start = self.position_to_offset(ctx, TextPosition::new(pos.line, 0));

            // Calculate line end including newline
            let line_text = ctx.document.line(pos.line);
            let line_char_len = line_text.chars().count();
            let is_last_line = pos.line + 1 >= ctx.document.len_lines();
            let line_end = if is_last_line {
                // Last line: no trailing newline to consume
                line_start + line_char_len
            } else {
                // Include the newline character
                line_start + line_char_len + 1
            };

            let full_line = if is_last_line {
                format!("{}\n", line_text)
            } else {
                ctx.document.slice(TextRange::new(line_start, line_end))
            };

            if let Some(ref mut cb) = self.clipboard {
                let _ = cb.set_text(full_line.clone());
            }
            self.clipboard_is_line_copy = true;

            let deleted_len = line_end - line_start;
            if deleted_len > 0 {
                let deleted_text = ctx.document.slice(TextRange::new(line_start, line_end));
                let op = EditOperation::delete(line_start, deleted_text);
                ctx.history.execute(op, ctx.document.buffer_mut());

                // If we deleted the last line and there's a preceding newline, clean it up
                if is_last_line && line_start > 0 {
                    // Remove the trailing newline of the previous line
                    if let Some(ch) = ctx.document.buffer().char_at(line_start - 1) {
                        if ch == '\n' {
                            let nl_op = EditOperation::delete(line_start - 1, "\n".to_string());
                            ctx.history.execute(nl_op, ctx.document.buffer_mut());
                            ctx.view.move_caret_to(line_start - 1, false);
                            ctx.document.update_syntax_incremental(line_start - 1, deleted_len + 1, 0);
                            let affected = TextRange::new(line_start - 1, line_end);
                            ctx.view.clear_selection();
                            self.emit_document_changed(ctx, Some(affected));
                            self.emit_selection_changed(ctx);
                            return;
                        }
                    }
                }

                ctx.view.move_caret_to(line_start, false);
                ctx.view.clear_selection();

                ctx.document.update_syntax_incremental(line_start, deleted_len, 0);

                let affected = TextRange::new(line_start, line_end);
                self.emit_document_changed(ctx, Some(affected));
                self.emit_selection_changed(ctx);
            }
        }
    }

    fn handle_paste(&mut self, ctx: &mut CommandContext) {
        // Extract clipboard text before any &mut self calls (borrow conflict avoidance)
        let clipboard_text = self.clipboard.as_mut().and_then(|cb| cb.get_text().ok());
        let is_line_copy = self.clipboard_is_line_copy;

        if let Some(text) = clipboard_text {
            if is_line_copy && !ctx.view.has_selection() {
                // Line paste: insert above current line
                let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
                let line_start = self.position_to_offset(ctx, TextPosition::new(pos.line, 0));

                // Auto-indent the pasted line(s) to match the current line's indentation
                let pasted = self.auto_indent_paste(&text, ctx);

                let text_len = pasted.chars().count();
                let op = EditOperation::insert_string(line_start, pasted);
                ctx.history.execute(op, ctx.document.buffer_mut());

                // Position caret at the start of the inserted text
                ctx.view.move_caret_to(line_start, false);
                ctx.view.clear_selection();

                ctx.document.update_syntax_incremental(line_start, 0, text_len);

                let affected = TextRange::new(line_start, line_start + text_len);
                self.emit_document_changed(ctx, Some(affected));
                self.emit_selection_changed(ctx);
            } else {
                // Normal paste: delete selection first if any
                if ctx.view.has_selection() {
                    self.handle_delete_selection(ctx);
                }
                // Auto-indent multi-line paste
                let pasted = self.auto_indent_paste(&text, ctx);
                self.handle_insert_text(ctx, &pasted);
            }
        }
    }

    /// Auto-indents pasted text to match the indentation of the current line.
    ///
    /// For single-line paste: no adjustment.
    /// For multi-line paste: adjusts indentation of all lines relative to the
    /// first non-empty line, matching the current line's indentation.
    fn auto_indent_paste(&self, text: &str, ctx: &CommandContext) -> String {
        let lines: Vec<&str> = text.split('\n').collect();

        // Single-line paste: no adjustment needed
        if lines.len() <= 1 {
            return text.to_string();
        }

        // Find the indentation of the current line (where paste target is)
        let pos = self.offset_to_position(ctx, ctx.view.caret_offset());
        let current_line = ctx.document.line(pos.line);
        let target_indent: String = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        // Find the indentation of the first non-empty paste line
        let first_nonempty = lines.iter().find(|l| !l.trim().is_empty());
        let source_indent: String = match first_nonempty {
            Some(line) => line.chars().take_while(|c| *c == ' ' || *c == '\t').collect(),
            None => return text.to_string(),
        };

        // Re-indent: replace source indent with target indent on each line
        let mut result = Vec::with_capacity(lines.len());
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                // Preserve empty lines
                result.push(line.to_string());
            } else if line.starts_with(&source_indent) {
                // Replace source indentation prefix with target
                let remainder = &line[source_indent.len()..];
                result.push(format!("{}{}", target_indent, remainder));
            } else {
                // Line has less indent than source — keep as-is
                result.push(line.to_string());
            }

            // Avoid trailing newline duplication
            if i < lines.len() - 1 {
                // Will be joined with \n
            }
        }

        result.join("\n")
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

// Word boundary detection is now in TextBuffer::word_boundary_at.

#[cfg(test)]
impl CommandDispatcher {
    /// Sets the clipboard text and line-copy flag for testing.
    /// Bypasses system clipboard for deterministic test behavior.
    fn set_clipboard_for_test(&mut self, text: &str, is_line: bool) {
        if let Some(ref mut cb) = self.clipboard {
            let _ = cb.set_text(text.to_string());
        }
        self.clipboard_is_line_copy = is_line;
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

    fn create_multiline_context() -> (Document, EditorView, CommandHistory, EventBus) {
        let doc = Document::from_str("line one\nline two\nline three", None);
        let view = EditorView::new(doc.id());
        let history = CommandHistory::new();
        let event_bus = EventBus::new();
        (doc, view, history, event_bus)
    }

    #[test]
    fn test_copy_no_selection_copies_full_line() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Place caret on line 1 (middle of "line two")
        // "line one\n" = 9 chars, so offset 13 is inside "line two"
        ctx.view.move_caret_to(13, false);
        dispatcher.dispatch(EditorCommand::Copy, &mut ctx);

        // clipboard_is_line_copy should be true
        assert!(dispatcher.clipboard_is_line_copy);
        // Document unchanged
        assert_eq!(ctx.document.content(), "line one\nline two\nline three");
    }

    #[test]
    fn test_copy_with_selection_copies_selection_only() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Select "line" from "line one"
        ctx.view.begin_selection();
        ctx.view.move_caret_to(4, true);
        dispatcher.dispatch(EditorCommand::Copy, &mut ctx);

        // clipboard_is_line_copy should be false
        assert!(!dispatcher.clipboard_is_line_copy);
        // Document unchanged
        assert_eq!(ctx.document.content(), "line one\nline two\nline three");
    }

    #[test]
    fn test_cut_no_selection_removes_entire_line() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Place caret on line 1 ("line two")
        ctx.view.move_caret_to(13, false);
        dispatcher.dispatch(EditorCommand::Cut, &mut ctx);

        assert!(dispatcher.clipboard_is_line_copy);
        // Line "line two\n" should be removed
        assert_eq!(ctx.document.content(), "line one\nline three");
    }

    #[test]
    fn test_cut_no_selection_last_line() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Place caret on last line ("line three"), offset = 9 + 9 = 18
        ctx.view.move_caret_to(20, false);
        dispatcher.dispatch(EditorCommand::Cut, &mut ctx);

        assert!(dispatcher.clipboard_is_line_copy);
        // Last line removed, along with the preceding newline
        assert_eq!(ctx.document.content(), "line one\nline two");
    }

    #[test]
    fn test_cut_with_selection_cuts_selection() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Select "one" from "line one"
        ctx.view.begin_selection();
        ctx.view.move_caret_to(4, true);
        // Now select just chars 0..4 ("line")
        dispatcher.dispatch(EditorCommand::Cut, &mut ctx);

        assert!(!dispatcher.clipboard_is_line_copy);
        assert_eq!(ctx.document.content(), " one\nline two\nline three");
    }

    #[test]
    fn test_paste_line_copy_inserts_above_current_line() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        // Set clipboard directly to avoid system clipboard race conditions
        dispatcher.set_clipboard_for_test("line one\n", true);

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Move caret to line 2 ("line three")
        ctx.view.move_caret_to(20, false);
        dispatcher.dispatch(EditorCommand::Paste, &mut ctx);

        // "line one\n" should be inserted above "line three"
        assert_eq!(
            ctx.document.content(),
            "line one\nline two\nline one\nline three"
        );
    }

    #[test]
    fn test_paste_with_selection_replaces_selection() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        // Set clipboard directly to avoid system clipboard race conditions
        dispatcher.set_clipboard_for_test("line one", false);

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Select "line two"
        ctx.view.move_caret_to(9, false); // start of "line two"
        ctx.view.begin_selection();
        ctx.view.move_caret_to(17, true); // end of "line two"

        dispatcher.dispatch(EditorCommand::Paste, &mut ctx);
        assert_eq!(
            ctx.document.content(),
            "line one\nline one\nline three"
        );
    }

    #[test]
    fn test_auto_indent_multi_line_paste() {
        // Create a document with indented lines
        let doc = Document::from_str("    fn main() {\n        let x = 1;\n    }", None);
        let view = EditorView::new(doc.id());
        let history = CommandHistory::new();
        let event_bus = EventBus::new();
        let (mut doc, mut view, mut history, mut event_bus) = (doc, view, history, event_bus);
        let dispatcher = CommandDispatcher::new();

        let ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Test auto_indent_paste with a multi-line text pasted at line with 8-space indent
        // Caret at offset 16 = start of "        let x = 1;"
        ctx.view.move_caret_to(16, false);

        let paste_text = "if true {\n    println!(\"hello\");\n}";
        let result = dispatcher.auto_indent_paste(paste_text, &ctx);

        // The first non-empty line of paste_text has 0-space indent.
        // Current line has 8-space indent. So all lines get +8 spaces.
        assert_eq!(
            result,
            "        if true {\n            println!(\"hello\");\n        }"
        );
    }

    #[test]
    fn test_auto_indent_single_line_paste_no_change() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let dispatcher = CommandDispatcher::new();

        let ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        let paste_text = "hello world";
        let result = dispatcher.auto_indent_paste(paste_text, &ctx);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_clipboard_is_line_copy_flag_tracks_correctly() {
        let (mut doc, mut view, mut history, mut event_bus) = create_multiline_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Initially false
        assert!(!dispatcher.clipboard_is_line_copy);

        // Copy with no selection -> true
        ctx.view.move_caret_to(3, false);
        dispatcher.dispatch(EditorCommand::Copy, &mut ctx);
        assert!(dispatcher.clipboard_is_line_copy);

        // Copy with selection -> false
        ctx.view.begin_selection();
        ctx.view.move_caret_to(8, true);
        dispatcher.dispatch(EditorCommand::Copy, &mut ctx);
        assert!(!dispatcher.clipboard_is_line_copy);

        // Cut with no selection -> true
        ctx.view.clear_selection();
        ctx.view.move_caret_to(3, false);
        dispatcher.dispatch(EditorCommand::Cut, &mut ctx);
        assert!(dispatcher.clipboard_is_line_copy);
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

    #[test]
    fn test_click_at_positions_cursor() {
        // "Hello World" — clicking at (0, 5) should place caret at offset 5.
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        let result = dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 5, extend_selection: false, click_count: 1 },
            &mut ctx,
        );

        assert_eq!(result, DispatchResult::Executed);
        assert_eq!(ctx.view.caret_offset(), 5);
        assert!(!ctx.view.has_selection());
    }

    #[test]
    fn test_double_click_selects_word() {
        // "Hello World" — double-click at col 2 should select "Hello" (0..5).
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 2, extend_selection: false, click_count: 2 },
            &mut ctx,
        );

        assert!(ctx.view.has_selection());
        assert_eq!(ctx.view.selection_range(), Some((0, 5)));
    }

    #[test]
    fn test_shift_click_extends_selection() {
        // ClickAt (0, 2) without shift (sets caret at 2), then
        // ClickAt (0, 8) with shift -> selection range (2..8).
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // First click at col 2 (no shift).
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 2, extend_selection: false, click_count: 1 },
            &mut ctx,
        );
        assert_eq!(ctx.view.caret_offset(), 2);
        assert!(!ctx.view.has_selection());

        // Shift+click at col 8.
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 8, extend_selection: true, click_count: 1 },
            &mut ctx,
        );
        assert!(ctx.view.has_selection());
        assert_eq!(ctx.view.selection_range(), Some((2, 8)));
    }

    #[test]
    fn test_triple_click_selects_line() {
        // Multi-line document: triple-click on line 0 selects full line including newline.
        let doc = Document::from_str("Hello World\nSecond Line", None);
        let view = EditorView::new(doc.id());
        let history = CommandHistory::new();
        let event_bus = EventBus::new();
        let (mut doc, mut view, mut history, mut event_bus) = (doc, view, history, event_bus);
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 3, extend_selection: false, click_count: 3 },
            &mut ctx,
        );

        assert!(ctx.view.has_selection());
        // "Hello World\n" is 12 chars (0..12). Line 1 starts at offset 12.
        assert_eq!(ctx.view.selection_range(), Some((0, 12)));
    }

    #[test]
    fn test_click_past_end_of_line_snaps() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // "Hello World" has 11 chars on line 0. Click at col 100 should snap.
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 100, extend_selection: false, click_count: 1 },
            &mut ctx,
        );

        assert_eq!(ctx.view.caret_offset(), 11); // end of "Hello World"
    }

    #[test]
    fn test_click_below_last_line_snaps() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Only 1 line. Click at line 100 should snap to last line.
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 100, col: 5, extend_selection: false, click_count: 1 },
            &mut ctx,
        );

        assert_eq!(ctx.view.caret_offset(), 5);
    }

    #[test]
    fn test_drag_to_extends_selection() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Click at col 2 (starts potential drag).
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 2, extend_selection: false, click_count: 1 },
            &mut ctx,
        );

        // Now begin a drag — first we need to set anchor. The event loop would
        // have called ClickAt first, which positions caret. For DragTo to
        // extend selection, we need to begin selection at the click point.
        // Actually, re-reading the code: single click does move_caret_to(offset, false)
        // + clear_selection. DragTo calls move_caret_to(offset, true) which calls
        // extend_to. But extend_to on a collapsed selection should create a selection.
        // Let's verify by checking if DragTo produces a selection.
        dispatcher.dispatch(
            EditorCommand::DragTo { line: 0, col: 8, snap_mode: 0 },
            &mut ctx,
        );

        assert!(ctx.view.has_selection());
        assert_eq!(ctx.view.selection_range(), Some((2, 8)));
    }

    #[test]
    fn test_drag_to_word_snap_forward() {
        // "hello world_test foo"
        // Double-click on "world_test" at offset 6 selects the word [6..16].
        // Then drag forward to col 18 (in "foo") should snap to word end of "foo" -> [6..20].
        let text = "hello world_test foo";
        let mut doc = Document::from_str(text, None);
        let mut view = EditorView::new(doc.id());
        let mut history = CommandHistory::new();
        let mut event_bus = EventBus::new();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Double-click at col 8 (inside "world_test")
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 8, extend_selection: false, click_count: 2 },
            &mut ctx,
        );
        assert_eq!(ctx.view.selection_range(), Some((6, 16)));

        // Drag forward to col 18 with word-snap
        dispatcher.dispatch(
            EditorCommand::DragTo { line: 0, col: 18, snap_mode: 1 },
            &mut ctx,
        );
        assert!(ctx.view.has_selection());
        // Should snap to end of "foo" = offset 20
        let range = ctx.view.selection_range().unwrap();
        assert_eq!(range.1, 20);
    }

    #[test]
    fn test_drag_to_word_snap_backward() {
        // "hello world_test foo"
        // Double-click on "world_test" at col 8 selects [6..16].
        // Then drag backward to col 2 (in "hello") should snap to word start of "hello" -> [0..16].
        let text = "hello world_test foo";
        let mut doc = Document::from_str(text, None);
        let mut view = EditorView::new(doc.id());
        let mut history = CommandHistory::new();
        let mut event_bus = EventBus::new();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Double-click at col 8 (inside "world_test")
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 8, extend_selection: false, click_count: 2 },
            &mut ctx,
        );

        // Drag backward to col 2 with word-snap
        dispatcher.dispatch(
            EditorCommand::DragTo { line: 0, col: 2, snap_mode: 1 },
            &mut ctx,
        );
        assert!(ctx.view.has_selection());
        // Should snap to start of "hello" = offset 0
        let range = ctx.view.selection_range().unwrap();
        assert_eq!(range.0, 0);
    }

    #[test]
    fn test_drag_to_line_snap() {
        // "line one\nline two\nline three"
        let text = "line one\nline two\nline three";
        let mut doc = Document::from_str(text, None);
        let mut view = EditorView::new(doc.id());
        let mut history = CommandHistory::new();
        let mut event_bus = EventBus::new();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        // Triple-click on line 0 selects the entire line [0..9].
        dispatcher.dispatch(
            EditorCommand::ClickAt { line: 0, col: 3, extend_selection: false, click_count: 3 },
            &mut ctx,
        );
        assert!(ctx.view.has_selection());

        // Drag to line 1, col 5 with line-snap.
        // Since dragging forward, should extend to start of line 2 (offset 18).
        dispatcher.dispatch(
            EditorCommand::DragTo { line: 1, col: 5, snap_mode: 2 },
            &mut ctx,
        );
        assert!(ctx.view.has_selection());
        let range = ctx.view.selection_range().unwrap();
        // Line 0 start = 0, line 2 start = 18
        assert_eq!(range.0, 0);
        assert_eq!(range.1, 18);
    }
}
