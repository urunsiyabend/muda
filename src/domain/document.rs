//! Document: An editable text artifact with identity and persistence semantics.
//!
//! Wraps a TextBuffer and owns document-level derived state including
//! undo history, language analysis, and diagnostics.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::text_buffer::{TextBuffer, TextOffset, TextPosition, TextRange, DocumentRevision};
use crate::events::DomainEvent;
use crate::syntax::{SyntaxHighlighter, SyntaxLanguage};

/// Unique identifier for a document within a workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    /// Generates a new unique DocumentId.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Line ending convention for the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    LF,   // Unix: \n
    CRLF, // Windows: \r\n
    CR,   // Old Mac: \r
}

impl LineEnding {
    /// Detects line ending from content.
    pub fn detect(content: &str) -> Self {
        if content.contains("\r\n") {
            LineEnding::CRLF
        } else if content.contains('\r') {
            LineEnding::CR
        } else {
            LineEnding::LF
        }
    }

    /// Returns the line ending string.
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LF => "\n",
            LineEnding::CRLF => "\r\n",
            LineEnding::CR => "\r",
        }
    }
}

/// Metadata about the document's storage and state.
#[derive(Clone, Debug, Default)]
pub struct DocumentMetadata {
    /// The file path (if saved).
    pub uri: Option<PathBuf>,
    /// Line ending convention.
    pub line_ending: LineEnding,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
}

/// An editable text artifact with identity and persistence semantics.
pub struct Document {
    /// Unique identifier for this document.
    id: DocumentId,
    /// The text content.
    buffer: TextBuffer,
    /// Metadata about the document.
    metadata: DocumentMetadata,
    /// Syntax highlighter for this document.
    highlighter: SyntaxHighlighter,
}

impl Document {
    /// Creates a new empty document.
    pub fn new() -> Self {
        Self {
            id: DocumentId::new(),
            buffer: TextBuffer::new(),
            metadata: DocumentMetadata::default(),
            highlighter: SyntaxHighlighter::new(SyntaxLanguage::Plain),
        }
    }

    /// Creates a document from a string with an optional file path.
    pub fn from_str(content: &str, path: Option<PathBuf>) -> Self {
        let line_ending = LineEnding::detect(content);
        let language = path
            .as_ref()
            .map(|p| SyntaxLanguage::from_extension(p))
            .unwrap_or(SyntaxLanguage::Plain);
        
        let mut highlighter = SyntaxHighlighter::new(language);
        highlighter.parse(content);

        Self {
            id: DocumentId::new(),
            buffer: TextBuffer::from_str(content),
            metadata: DocumentMetadata {
                uri: path,
                line_ending,
                dirty: false,
            },
            highlighter,
        }
    }

    /// Opens a document from a file.
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_str(&content, Some(path.to_path_buf())))
    }

    /// Returns the document's unique identifier.
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// Returns the current revision number.
    pub fn revision(&self) -> DocumentRevision {
        self.buffer.revision()
    }

    /// Returns a reference to the text buffer.
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// Returns a mutable reference to the text buffer.
    /// Automatically marks the document as dirty.
    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        self.metadata.dirty = true;
        &mut self.buffer
    }

    /// Returns a reference to the document metadata.
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Returns a mutable reference to the document metadata.
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    /// Returns a reference to the syntax highlighter.
    pub fn highlighter(&self) -> &SyntaxHighlighter {
        &self.highlighter
    }

    /// Returns whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.metadata.dirty
    }

    /// Marks the document as dirty and updates syntax highlighting.
    pub fn mark_dirty(&mut self) {
        self.metadata.dirty = true;
        self.update_syntax();
    }

    /// Updates syntax highlighting for the current content.
    pub fn update_syntax(&mut self) {
        let content = self.buffer.to_string();
        self.highlighter.parse(&content);
    }

    /// Returns the file path if the document is associated with a file.
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.metadata.uri.as_ref()
    }

    /// Sets the file path for this document.
    pub fn set_file_path(&mut self, path: PathBuf) {
        let language = SyntaxLanguage::from_extension(&path);
        self.highlighter = SyntaxHighlighter::new(language);
        self.update_syntax();
        self.metadata.uri = Some(path);
    }

    /// Returns the document title (filename or "[New File]").
    pub fn title(&self) -> String {
        let name = self.metadata.uri
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Yeni Dosya]");

        if self.metadata.dirty {
            format!("*{}", name)
        } else {
            name.to_string()
        }
    }

    /// Saves the document to its file path.
    /// Returns Ok(true) if saved, Ok(false) if no path, Err on I/O error.
    pub fn save(&mut self) -> std::io::Result<bool> {
        if let Some(path) = &self.metadata.uri {
            std::fs::write(path, self.buffer.to_string())?;
            self.metadata.dirty = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Saves the document to a new path.
    pub fn save_as(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.buffer.to_string())?;
        self.set_file_path(path.to_path_buf());
        self.metadata.dirty = false;
        Ok(())
    }

    // =========================================================================
    // Convenience methods delegating to TextBuffer
    // =========================================================================

    /// Returns the total number of characters.
    pub fn len_chars(&self) -> usize {
        self.buffer.len_chars()
    }

    /// Returns the total number of lines.
    pub fn len_lines(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Returns the text of a specific line.
    pub fn line(&self, line_idx: usize) -> String {
        self.buffer.line(line_idx)
    }

    /// Returns the length of a line (excluding newline).
    pub fn line_len(&self, line_idx: usize) -> usize {
        self.buffer.line_len(line_idx)
    }

    /// Converts offset to position.
    pub fn offset_to_position(&self, offset: TextOffset) -> TextPosition {
        self.buffer.offset_to_position(offset)
    }

    /// Converts position to offset.
    pub fn position_to_offset(&self, pos: TextPosition) -> TextOffset {
        self.buffer.position_to_offset(pos)
    }

    /// Returns a slice of text.
    pub fn slice(&self, range: TextRange) -> String {
        self.buffer.slice(range)
    }

    /// Returns the entire content as a string.
    pub fn content(&self) -> String {
        self.buffer.to_string()
    }

    // =========================================================================
    // Event creation helpers
    // =========================================================================

    /// Creates a DocumentOpened event for this document.
    pub fn event_opened(&self) -> DomainEvent {
        DomainEvent::DocumentOpened {
            document_id: self.id,
        }
    }

    /// Creates a DocumentChanged event for this document.
    ///
    /// # Arguments
    /// * `affected_range` - Optional range that was affected by the change.
    pub fn event_changed(&self, affected_range: Option<TextRange>) -> DomainEvent {
        DomainEvent::DocumentChanged {
            document_id: self.id,
            revision: self.revision(),
            affected_range,
        }
    }

    /// Creates a DocumentSaved event for this document.
    pub fn event_saved(&self) -> DomainEvent {
        DomainEvent::DocumentSaved {
            document_id: self.id,
        }
    }

    /// Creates a DocumentClosed event for this document.
    pub fn event_closed(&self) -> DomainEvent {
        DomainEvent::DocumentClosed {
            document_id: self.id,
        }
    }

    /// Creates a DocumentMetadataChanged event for this document.
    pub fn event_metadata_changed(&self) -> DomainEvent {
        DomainEvent::DocumentMetadataChanged {
            document_id: self.id,
        }
    }

    /// Creates a SyntaxUpdated event for this document.
    pub fn event_syntax_updated(&self) -> DomainEvent {
        DomainEvent::SyntaxUpdated {
            document_id: self.id,
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document() {
        let doc = Document::new();
        assert!(!doc.is_dirty());
        assert!(doc.file_path().is_none());
        assert_eq!(doc.title(), "[Yeni Dosya]");
    }

    #[test]
    fn test_document_from_str() {
        let doc = Document::from_str("Hello\nWorld", None);
        assert_eq!(doc.len_lines(), 2);
        assert_eq!(doc.line(0), "Hello");
        assert_eq!(doc.line(1), "World");
    }

    #[test]
    fn test_document_title_with_path() {
        let doc = Document::from_str("test", Some(PathBuf::from("/path/to/file.rs")));
        assert_eq!(doc.title(), "file.rs");
    }

    #[test]
    fn test_dirty_flag() {
        let mut doc = Document::from_str("test", None);
        assert!(!doc.is_dirty());
        
        doc.buffer_mut().insert(0, "x");
        assert!(doc.is_dirty());
    }

    #[test]
    fn test_dirty_title() {
        let mut doc = Document::from_str("test", Some(PathBuf::from("file.txt")));
        assert_eq!(doc.title(), "file.txt");
        
        doc.mark_dirty();
        assert_eq!(doc.title(), "*file.txt");
    }

    #[test]
    fn test_line_ending_detection() {
        assert_eq!(LineEnding::detect("hello\nworld"), LineEnding::LF);
        assert_eq!(LineEnding::detect("hello\r\nworld"), LineEnding::CRLF);
        assert_eq!(LineEnding::detect("hello\rworld"), LineEnding::CR);
    }

    #[test]
    fn test_document_ids_are_unique() {
        let doc1 = Document::new();
        let doc2 = Document::new();
        assert_ne!(doc1.id(), doc2.id());
    }
}
