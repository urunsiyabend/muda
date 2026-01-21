//! CommandHistory: Transaction-based undo/redo stack.
//!
//! Manages undo/redo with transaction grouping support.

use crate::commands::{EditOperation, UndoTransaction};
use crate::domain::TextBuffer;

/// History of transactions for undo/redo support.
#[derive(Default)]
pub struct CommandHistory {
    /// Stack of completed transactions for undo.
    undo_stack: Vec<UndoTransaction>,
    /// Stack of undone transactions for redo.
    redo_stack: Vec<UndoTransaction>,
    /// Current transaction being built (for grouping operations).
    current_transaction: Option<UndoTransaction>,
}

impl CommandHistory {
    /// Creates a new empty command history.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: None,
        }
    }

    /// Begins a new transaction.
    pub fn begin_transaction(&mut self, cursor_pos: usize, selection: Option<usize>) {
        self.commit_transaction();
        self.current_transaction = Some(UndoTransaction::new(cursor_pos, selection));
    }

    /// Adds an operation to the current transaction.
    /// Creates a new transaction if none exists.
    pub fn push_operation(
        &mut self,
        op: EditOperation,
        buffer: &mut TextBuffer,
        cursor_pos: usize,
    ) {
        self.redo_stack.clear();

        if self.current_transaction.is_none() {
            self.current_transaction = Some(UndoTransaction::new(cursor_pos, None));
        }

        op.execute(buffer);

        if let Some(tx) = &mut self.current_transaction {
            tx.push(op);
        }
    }

    /// Commits the current transaction to the undo stack.
    pub fn commit_transaction(&mut self) {
        if let Some(tx) = self.current_transaction.take() {
            if !tx.is_empty() {
                self.undo_stack.push(tx);
            }
        }
    }

    /// Executes a single operation as its own transaction.
    /// This is a convenience method for simple operations.
    pub fn execute(&mut self, op: EditOperation, buffer: &mut TextBuffer) {
        self.commit_transaction();
        let cursor_before = match &op {
            EditOperation::InsertChar { pos, .. } => *pos,
            EditOperation::InsertString { pos, .. } => *pos,
            EditOperation::Delete { pos, .. } => *pos,
        };

        let mut tx = UndoTransaction::new(cursor_before, None);
        op.execute(buffer);
        tx.push(op);

        self.undo_stack.push(tx);
        self.redo_stack.clear();
    }

    /// Undoes the last transaction.
    /// Returns the cursor position after undo.
    pub fn undo(&mut self, buffer: &mut TextBuffer) -> Option<usize> {
        self.commit_transaction();

        if let Some(tx) = self.undo_stack.pop() {
            let cursor_pos = tx.cursor_before();
            tx.undo(buffer);
            self.redo_stack.push(tx);
            Some(cursor_pos)
        } else {
            None
        }
    }

    /// Redoes the last undone transaction.
    /// Returns the cursor position after redo.
    pub fn redo(&mut self, buffer: &mut TextBuffer) -> Option<usize> {
        self.commit_transaction();

        if let Some(tx) = self.redo_stack.pop() {
            tx.execute(buffer);
            let cursor_pos = tx.cursor_after();
            self.undo_stack.push(tx);
            Some(cursor_pos)
        } else {
            None
        }
    }

    /// Clears all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_transaction = None;
    }

    /// Returns true if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
            || self
                .current_transaction
                .as_ref()
                .map_or(false, |t| !t.is_empty())
    }

    /// Returns true if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_new() {
        let history = CommandHistory::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_execute_undo_redo() {
        let mut buffer = TextBuffer::from_str("Hello");
        let mut history = CommandHistory::new();

        history.execute(EditOperation::insert_char(5, '!'), &mut buffer);
        assert_eq!(buffer.to_string(), "Hello!");
        assert!(history.can_undo());

        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello");
        assert!(history.can_redo());

        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello!");
    }

    #[test]
    fn test_transaction_grouping() {
        let mut buffer = TextBuffer::new();
        let mut history = CommandHistory::new();

        history.begin_transaction(0, None);
        history.push_operation(EditOperation::insert_char(0, 'H'), &mut buffer, 0);
        history.push_operation(EditOperation::insert_char(1, 'i'), &mut buffer, 1);
        history.commit_transaction();

        assert_eq!(buffer.to_string(), "Hi");

        // Single undo should undo both characters
        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");
    }

    #[test]
    fn test_redo_cleared_on_new_edit() {
        let mut buffer = TextBuffer::from_str("Hello");
        let mut history = CommandHistory::new();

        history.execute(EditOperation::insert_char(5, '!'), &mut buffer);
        history.undo(&mut buffer);
        assert!(history.can_redo());

        history.execute(EditOperation::insert_char(5, '?'), &mut buffer);
        assert!(!history.can_redo());
    }
}
