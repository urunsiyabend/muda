//! EditOperation: Low-level reversible text operations.
//!
//! These are the primitive operations that modify the text buffer.
//! Each operation can be executed and undone.

use crate::domain::{TextBuffer, TextRange};

/// A low-level reversible edit operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOperation {
    /// Insert a single character at a position.
    InsertChar { pos: usize, ch: char },
    /// Insert a string at a position.
    InsertString { pos: usize, text: String },
    /// Delete text at a position (stores deleted text for undo).
    Delete { pos: usize, deleted_text: String },
}

impl EditOperation {
    /// Creates an insert character operation.
    pub fn insert_char(pos: usize, ch: char) -> Self {
        Self::InsertChar { pos, ch }
    }

    /// Creates an insert string operation.
    pub fn insert_string(pos: usize, text: impl Into<String>) -> Self {
        Self::InsertString {
            pos,
            text: text.into(),
        }
    }

    /// Creates a delete operation.
    pub fn delete(pos: usize, deleted_text: impl Into<String>) -> Self {
        Self::Delete {
            pos,
            deleted_text: deleted_text.into(),
        }
    }

    /// Executes the operation on the given buffer.
    pub fn execute(&self, buffer: &mut TextBuffer) {
        match self {
            EditOperation::InsertChar { pos, ch } => {
                buffer.insert_char(*pos, *ch);
            }
            EditOperation::InsertString { pos, text } => {
                buffer.insert(*pos, text);
            }
            EditOperation::Delete { pos, deleted_text } => {
                let range = TextRange::new(*pos, *pos + deleted_text.chars().count());
                buffer.delete(range);
            }
        }
    }

    /// Undoes the operation on the given buffer.
    pub fn undo(&self, buffer: &mut TextBuffer) {
        match self {
            EditOperation::InsertChar { pos, .. } => {
                let range = TextRange::new(*pos, *pos + 1);
                buffer.delete(range);
            }
            EditOperation::InsertString { pos, text } => {
                let range = TextRange::new(*pos, *pos + text.chars().count());
                buffer.delete(range);
            }
            EditOperation::Delete { pos, deleted_text } => {
                buffer.insert(*pos, deleted_text);
            }
        }
    }

    /// Returns the cursor position after executing this operation.
    pub fn cursor_after_execute(&self) -> usize {
        match self {
            EditOperation::InsertChar { pos, .. } => *pos + 1,
            EditOperation::InsertString { pos, text } => *pos + text.chars().count(),
            EditOperation::Delete { pos, .. } => *pos,
        }
    }

    /// Returns the cursor position after undoing this operation.
    pub fn cursor_after_undo(&self) -> usize {
        match self {
            EditOperation::InsertChar { pos, .. } => *pos,
            EditOperation::InsertString { pos, .. } => *pos,
            EditOperation::Delete { pos, deleted_text } => *pos + deleted_text.chars().count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_char() {
        let mut buffer = TextBuffer::from_str("Hello");
        let op = EditOperation::insert_char(5, '!');
        op.execute(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello!");
        assert_eq!(op.cursor_after_execute(), 6);
    }

    #[test]
    fn test_insert_string() {
        let mut buffer = TextBuffer::from_str("Hello");
        let op = EditOperation::insert_string(5, " World");
        op.execute(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello World");
        assert_eq!(op.cursor_after_execute(), 11);
    }

    #[test]
    fn test_delete() {
        let mut buffer = TextBuffer::from_str("Hello World");
        let op = EditOperation::delete(5, " World");
        op.execute(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello");
        assert_eq!(op.cursor_after_execute(), 5);
    }

    #[test]
    fn test_undo_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        let op = EditOperation::insert_char(5, '!');
        op.execute(&mut buffer);
        op.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello");
        assert_eq!(op.cursor_after_undo(), 5);
    }

    #[test]
    fn test_undo_delete() {
        let mut buffer = TextBuffer::from_str("Hello World");
        let op = EditOperation::delete(5, " World");
        op.execute(&mut buffer);
        op.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello World");
        assert_eq!(op.cursor_after_undo(), 11);
    }
}
