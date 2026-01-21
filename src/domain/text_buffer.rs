//! TextBuffer: The authoritative source of truth for document text.
//!
//! Provides efficient editing operations and coordinate mappings between
//! offsets and line/column positions.

use ropey::Rope;

/// A scalar position in the buffer (character index).
pub type TextOffset = usize;

/// A monotonically increasing version number representing document state evolution.
pub type DocumentRevision = u64;

/// Human-facing location in (line, column) terms.
/// Both line and column are 0-indexed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

impl TextPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// A half-open interval `[start, end)` in TextOffset coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TextRange {
    pub start: TextOffset,
    pub end: TextOffset,
}

impl TextRange {
    pub fn new(start: TextOffset, end: TextOffset) -> Self {
        debug_assert!(start <= end, "TextRange start must be <= end");
        Self { start, end }
    }

    pub fn empty(offset: TextOffset) -> Self {
        Self { start: offset, end: offset }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns a normalized range where start <= end.
    pub fn normalized(a: TextOffset, b: TextOffset) -> Self {
        if a <= b {
            Self { start: a, end: b }
        } else {
            Self { start: b, end: a }
        }
    }
}

/// The source of truth for the document's textual content.
/// 
/// Provides efficient editing operations and coordinate mappings.
/// Implementation uses a Rope data structure for efficient large-file editing.
pub struct TextBuffer {
    rope: Rope,
    revision: DocumentRevision,
}

impl TextBuffer {
    /// Creates a new empty TextBuffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            revision: 0,
        }
    }

    /// Creates a TextBuffer from a string.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            revision: 0,
        }
    }

    /// Returns the current revision number.
    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    /// Returns the total number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Returns the total number of lines in the buffer.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns the character at the given offset.
    pub fn char_at(&self, offset: TextOffset) -> Option<char> {
        if offset < self.len_chars() {
            Some(self.rope.char(offset))
        } else {
            None
        }
    }

    /// Returns the text of a specific line (without trailing newline).
    pub fn line(&self, line_idx: usize) -> String {
        if line_idx >= self.len_lines() {
            return String::new();
        }
        let line = self.rope.line(line_idx);
        let mut s = line.to_string();
        // Remove trailing newline characters
        if s.ends_with('\n') {
            s.pop();
        }
        if s.ends_with('\r') {
            s.pop();
        }
        s
    }

    /// Returns the length of a line in characters (excluding newline).
    pub fn line_len(&self, line_idx: usize) -> usize {
        if line_idx >= self.len_lines() {
            return 0;
        }
        let line = self.rope.line(line_idx);
        let len = line.len_chars();
        // Subtract newline if present
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    /// Converts a character offset to a line index.
    pub fn offset_to_line(&self, offset: TextOffset) -> usize {
        let clamped = offset.min(self.len_chars());
        self.rope.char_to_line(clamped)
    }

    /// Returns the character offset of the start of a line.
    pub fn line_to_offset(&self, line_idx: usize) -> TextOffset {
        if line_idx >= self.len_lines() {
            return self.len_chars();
        }
        self.rope.line_to_char(line_idx)
    }

    /// Converts a character offset to a TextPosition (line, column).
    pub fn offset_to_position(&self, offset: TextOffset) -> TextPosition {
        let clamped = offset.min(self.len_chars());
        let line = self.rope.char_to_line(clamped);
        let line_start = self.rope.line_to_char(line);
        let column = clamped - line_start;
        TextPosition { line, column }
    }

    /// Converts a TextPosition (line, column) to a character offset.
    pub fn position_to_offset(&self, pos: TextPosition) -> TextOffset {
        if pos.line >= self.len_lines() {
            return self.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.line_len(pos.line);
        let column = pos.column.min(line_len);
        line_start + column
    }

    /// Converts a character offset to a byte offset.
    pub fn char_to_byte(&self, offset: TextOffset) -> usize {
        let clamped = offset.min(self.len_chars());
        self.rope.char_to_byte(clamped)
    }

    /// Inserts text at the given offset.
    /// Returns the range of the inserted text.
    pub fn insert(&mut self, offset: TextOffset, text: &str) -> TextRange {
        let clamped = offset.min(self.len_chars());
        self.rope.insert(clamped, text);
        self.revision += 1;
        TextRange::new(clamped, clamped + text.chars().count())
    }

    /// Inserts a single character at the given offset.
    pub fn insert_char(&mut self, offset: TextOffset, ch: char) -> TextRange {
        let clamped = offset.min(self.len_chars());
        self.rope.insert_char(clamped, ch);
        self.revision += 1;
        TextRange::new(clamped, clamped + 1)
    }

    /// Deletes text in the given range.
    /// Returns the deleted text.
    pub fn delete(&mut self, range: TextRange) -> String {
        let start = range.start.min(self.len_chars());
        let end = range.end.min(self.len_chars());
        if start >= end {
            return String::new();
        }
        let deleted = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);
        self.revision += 1;
        deleted
    }

    /// Replaces text in the given range with new text.
    /// Returns the deleted text.
    pub fn replace(&mut self, range: TextRange, text: &str) -> String {
        let deleted = self.delete(range);
        self.insert(range.start, text);
        // Only count as one revision for replace
        self.revision -= 1;
        deleted
    }

    /// Returns a slice of the buffer as a string.
    pub fn slice(&self, range: TextRange) -> String {
        let start = range.start.min(self.len_chars());
        let end = range.end.min(self.len_chars());
        if start >= end {
            return String::new();
        }
        self.rope.slice(start..end).to_string()
    }

    /// Returns the entire buffer content as a string.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Provides read access to the underlying Rope for syntax highlighting.
    /// This is a temporary escape hatch during refactoring.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = TextBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len_chars(), 0);
        assert_eq!(buf.len_lines(), 1); // Empty buffer has 1 line
    }

    #[test]
    fn test_from_str() {
        let buf = TextBuffer::from_str("Hello\nWorld");
        assert_eq!(buf.len_chars(), 11);
        assert_eq!(buf.len_lines(), 2);
        assert_eq!(buf.line(0), "Hello");
        assert_eq!(buf.line(1), "World");
    }

    #[test]
    fn test_insert() {
        let mut buf = TextBuffer::new();
        let range = buf.insert(0, "Hello");
        assert_eq!(range, TextRange::new(0, 5));
        assert_eq!(buf.to_string(), "Hello");
        assert_eq!(buf.revision(), 1);
    }

    #[test]
    fn test_insert_char() {
        let mut buf = TextBuffer::from_str("Hllo");
        buf.insert_char(1, 'e');
        assert_eq!(buf.to_string(), "Hello");
    }

    #[test]
    fn test_delete() {
        let mut buf = TextBuffer::from_str("Hello World");
        let deleted = buf.delete(TextRange::new(5, 11));
        assert_eq!(deleted, " World");
        assert_eq!(buf.to_string(), "Hello");
    }

    #[test]
    fn test_replace() {
        let mut buf = TextBuffer::from_str("Hello World");
        let deleted = buf.replace(TextRange::new(6, 11), "Rust");
        assert_eq!(deleted, "World");
        assert_eq!(buf.to_string(), "Hello Rust");
    }

    #[test]
    fn test_offset_to_position() {
        let buf = TextBuffer::from_str("Hello\nWorld\n!");
        
        assert_eq!(buf.offset_to_position(0), TextPosition::new(0, 0));
        assert_eq!(buf.offset_to_position(5), TextPosition::new(0, 5));
        assert_eq!(buf.offset_to_position(6), TextPosition::new(1, 0));
        assert_eq!(buf.offset_to_position(11), TextPosition::new(1, 5));
        assert_eq!(buf.offset_to_position(12), TextPosition::new(2, 0));
    }

    #[test]
    fn test_position_to_offset() {
        let buf = TextBuffer::from_str("Hello\nWorld");
        
        assert_eq!(buf.position_to_offset(TextPosition::new(0, 0)), 0);
        assert_eq!(buf.position_to_offset(TextPosition::new(0, 5)), 5);
        assert_eq!(buf.position_to_offset(TextPosition::new(1, 0)), 6);
        assert_eq!(buf.position_to_offset(TextPosition::new(1, 5)), 11);
    }

    #[test]
    fn test_position_clamps_to_line_end() {
        let buf = TextBuffer::from_str("Hi\nWorld");
        // Column 10 on line 0 should clamp to column 2
        assert_eq!(buf.position_to_offset(TextPosition::new(0, 10)), 2);
    }

    #[test]
    fn test_slice() {
        let buf = TextBuffer::from_str("Hello World");
        assert_eq!(buf.slice(TextRange::new(0, 5)), "Hello");
        assert_eq!(buf.slice(TextRange::new(6, 11)), "World");
    }

    #[test]
    fn test_line_len() {
        let buf = TextBuffer::from_str("Hello\nWorld\n");
        assert_eq!(buf.line_len(0), 5);
        assert_eq!(buf.line_len(1), 5);
        assert_eq!(buf.line_len(2), 0); // Empty line after trailing newline
    }

    #[test]
    fn test_revision_increments() {
        let mut buf = TextBuffer::new();
        assert_eq!(buf.revision(), 0);
        buf.insert(0, "a");
        assert_eq!(buf.revision(), 1);
        buf.insert_char(1, 'b');
        assert_eq!(buf.revision(), 2);
        buf.delete(TextRange::new(0, 1));
        assert_eq!(buf.revision(), 3);
    }
}
