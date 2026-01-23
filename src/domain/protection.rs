//! Protection types for unsaved file operations.
//!
//! This module provides types for domain-level unsaved file protection,
//! ensuring that operations which would discard unsaved changes require
//! explicit acknowledgment.

use super::DocumentId;

/// Document with unsaved changes (for error reporting).
#[derive(Clone, Debug)]
pub struct UnsavedDocument {
    /// The document's unique identifier.
    pub id: DocumentId,
    /// The document's display title (filename or "[New File]").
    pub title: String,
}

impl UnsavedDocument {
    /// Creates a new UnsavedDocument.
    pub fn new(id: DocumentId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
        }
    }
}

/// Error when an operation would discard unsaved changes.
#[derive(Clone, Debug)]
pub enum ProtectionError {
    /// A single document has unsaved changes.
    UnsavedChanges(UnsavedDocument),
    /// Multiple documents have unsaved changes.
    MultipleUnsavedChanges(Vec<UnsavedDocument>),
}

impl ProtectionError {
    /// Returns the list of unsaved documents.
    pub fn unsaved_documents(&self) -> Vec<&UnsavedDocument> {
        match self {
            ProtectionError::UnsavedChanges(doc) => vec![doc],
            ProtectionError::MultipleUnsavedChanges(docs) => docs.iter().collect(),
        }
    }

    /// Returns the titles of all unsaved documents.
    pub fn document_titles(&self) -> Vec<&str> {
        self.unsaved_documents()
            .iter()
            .map(|doc| doc.title.as_str())
            .collect()
    }
}

/// Result type for protected operations.
pub type ProtectedResult<T> = Result<T, ProtectionError>;

/// Acknowledgment token - proves caller confirmed discard.
///
/// This type cannot be constructed outside this module without
/// calling `confirmed()`, ensuring that callers explicitly
/// acknowledge the potential loss of unsaved changes.
#[derive(Clone, Copy, Debug)]
pub struct DiscardAcknowledgment {
    _private: (),
}

impl DiscardAcknowledgment {
    /// Creates a new acknowledgment token.
    ///
    /// Call this only after the user has confirmed they want to
    /// proceed despite potential loss of unsaved changes.
    pub fn confirmed() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsaved_document() {
        let doc_id = DocumentId::new();
        let unsaved = UnsavedDocument::new(doc_id, "test.txt");
        assert_eq!(unsaved.id, doc_id);
        assert_eq!(unsaved.title, "test.txt");
    }

    #[test]
    fn test_protection_error_single() {
        let doc_id = DocumentId::new();
        let unsaved = UnsavedDocument::new(doc_id, "test.txt");
        let error = ProtectionError::UnsavedChanges(unsaved);

        let docs = error.unsaved_documents();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "test.txt");

        let titles = error.document_titles();
        assert_eq!(titles, vec!["test.txt"]);
    }

    #[test]
    fn test_protection_error_multiple() {
        let docs = vec![
            UnsavedDocument::new(DocumentId::new(), "file1.txt"),
            UnsavedDocument::new(DocumentId::new(), "file2.txt"),
        ];
        let error = ProtectionError::MultipleUnsavedChanges(docs);

        let titles = error.document_titles();
        assert_eq!(titles, vec!["file1.txt", "file2.txt"]);
    }

    #[test]
    fn test_discard_acknowledgment() {
        let _ack = DiscardAcknowledgment::confirmed();
        // The acknowledgment can be created
    }
}
