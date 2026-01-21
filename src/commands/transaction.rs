//! UndoTransaction: Groups of operations for atomic undo/redo.
//!
//! A transaction groups multiple edit operations together so they
//! can be undone/redone as a single unit.

use crate::commands::EditOperation;
use crate::domain::TextBuffer;

/// A transaction grouping multiple operations for atomic undo/redo.
#[derive(Clone, Debug, Default)]
pub struct UndoTransaction {
    /// The operations in this transaction (in execution order).
    operations: Vec<EditOperation>,
    /// Cursor position before the transaction.
    cursor_before: usize,
    /// Cursor position after the transaction.
    cursor_after: usize,
    /// Selection anchor before (if any).
    selection_before: Option<usize>,
}

impl UndoTransaction {
    /// Creates a new empty transaction.
    pub fn new(cursor_before: usize, selection_before: Option<usize>) -> Self {
        Self {
            operations: Vec::new(),
            cursor_before,
            cursor_after: cursor_before,
            selection_before,
        }
    }

    /// Adds an operation to the transaction.
    pub fn push(&mut self, op: EditOperation) {
        self.cursor_after = op.cursor_after_execute();
        self.operations.push(op);
    }

    /// Returns true if the transaction is empty.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Returns the number of operations in the transaction.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Returns the cursor position before the transaction.
    pub fn cursor_before(&self) -> usize {
        self.cursor_before
    }

    /// Returns the cursor position after the transaction.
    pub fn cursor_after(&self) -> usize {
        self.cursor_after
    }

    /// Returns the selection anchor before the transaction.
    pub fn selection_before(&self) -> Option<usize> {
        self.selection_before
    }

    /// Executes all operations in the transaction.
    pub fn execute(&self, buffer: &mut TextBuffer) {
        for op in &self.operations {
            op.execute(buffer);
        }
    }

    /// Undoes all operations in the transaction (in reverse order).
    pub fn undo(&self, buffer: &mut TextBuffer) {
        for op in self.operations.iter().rev() {
            op.undo(buffer);
        }
    }

    /// Merges another transaction into this one.
    /// Used for combining small operations (e.g., individual keystrokes).
    pub fn merge(&mut self, other: UndoTransaction) {
        self.operations.extend(other.operations);
        self.cursor_after = other.cursor_after;
    }

    /// Returns true if this transaction can be merged with a new character insert.
    /// Used for grouping consecutive character typing.
    pub fn can_merge_char_insert(&self, new_pos: usize) -> bool {
        if self.operations.is_empty() {
            return false;
        }

        // Can merge if the last operation was a character insert at the adjacent position
        match self.operations.last() {
            Some(EditOperation::InsertChar { pos, .. }) => *pos + 1 == new_pos,
            Some(EditOperation::InsertString { pos, text }) => {
                *pos + text.chars().count() == new_pos
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_new() {
        let tx = UndoTransaction::new(10, None);
        assert!(tx.is_empty());
        assert_eq!(tx.cursor_before(), 10);
        assert_eq!(tx.cursor_after(), 10);
    }

    #[test]
    fn test_transaction_push() {
        let mut tx = UndoTransaction::new(0, None);
        tx.push(EditOperation::insert_char(0, 'H'));
        tx.push(EditOperation::insert_char(1, 'i'));

        assert_eq!(tx.len(), 2);
        assert_eq!(tx.cursor_before(), 0);
        assert_eq!(tx.cursor_after(), 2);
    }

    #[test]
    fn test_transaction_execute_undo() {
        let mut buffer = TextBuffer::from_str("World");
        let mut tx = UndoTransaction::new(0, None);
        tx.push(EditOperation::insert_string(0, "Hello "));

        tx.execute(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello World");

        tx.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "World");
    }

    #[test]
    fn test_transaction_undo_multiple_ops() {
        let mut buffer = TextBuffer::new();
        let mut tx = UndoTransaction::new(0, None);
        tx.push(EditOperation::insert_char(0, 'a'));
        tx.push(EditOperation::insert_char(1, 'b'));
        tx.push(EditOperation::insert_char(2, 'c'));

        tx.execute(&mut buffer);
        assert_eq!(buffer.to_string(), "abc");

        tx.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");
    }

    #[test]
    fn test_can_merge_char_insert() {
        let mut tx = UndoTransaction::new(0, None);
        tx.push(EditOperation::insert_char(0, 'H'));

        assert!(tx.can_merge_char_insert(1));
        assert!(!tx.can_merge_char_insert(0));
        assert!(!tx.can_merge_char_insert(2));
    }
}
