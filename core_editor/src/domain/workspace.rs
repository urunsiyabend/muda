//! Workspace: The top-level runtime container for an editing session.
//!
//! Owns open documents, editor views, per-document undo histories,
//! and cross-cutting services like the event bus.
//!
//! # Architecture
//!
//! ```text
//! Workspace
//! ├── Documents (HashMap<DocumentId, Document>)
//! ├── Views (HashMap<ViewId, EditorView>)
//! ├── Histories (HashMap<DocumentId, CommandHistory>)
//! ├── ActiveViewId
//! ├── path_to_doc (HashMap<PathBuf, DocumentId>)  -- Buffer Registry
//! ├── tab_order (Vec<ViewId>)                      -- insertion-order tab strip
//! ├── mru_stack (Vec<ViewId>)                      -- activation history
//! └── EventBus
//! ```
//!
//! Multiple views can reference the same document (split-view support).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::commands::CommandHistory;
use crate::domain::protection::{DiscardAcknowledgment, ProtectedResult, ProtectionError, UnsavedDocument};
use crate::domain::{Document, DocumentId, TextPosition};
use crate::events::{DomainEvent, EventBus};
use crate::view::{EditorView, ViewId};

/// The top-level runtime container for an editing session.
///
/// Manages document lifecycle, view lifecycle, and coordinates
/// all editing operations through the event bus.
pub struct Workspace {
    /// All open documents, keyed by their unique ID.
    documents: HashMap<DocumentId, Document>,

    /// All open views, keyed by their unique ID.
    views: HashMap<ViewId, EditorView>,

    /// Per-document undo/redo history.
    histories: HashMap<DocumentId, CommandHistory>,

    /// The currently focused view (receives input).
    active_view_id: Option<ViewId>,

    /// Central event bus for domain events.
    event_bus: EventBus,

    /// Buffer Registry: maps canonical file paths to document IDs.
    ///
    /// Prevents duplicate Document creation when the same file is opened twice.
    path_to_doc: HashMap<PathBuf, DocumentId>,

    /// Visual tab strip order (insertion order).
    ///
    /// Every ViewId present in `views` exists exactly once here.
    tab_order: Vec<ViewId>,

    /// Activation history stack; front = most recently activated view.
    ///
    /// Used to select the new active view when the current one is closed.
    mru_stack: Vec<ViewId>,

    /// Monotonically increasing counter for naming untitled documents.
    ///
    /// Incremented each time `create_untitled_document` is called.
    /// Never resets within a session. Used to produce "Untitled", "Untitled (2)", etc.
    untitled_counter: u32,
}

impl Workspace {
    /// Creates a new empty workspace.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            views: HashMap::new(),
            histories: HashMap::new(),
            active_view_id: None,
            event_bus: EventBus::new(),
            path_to_doc: HashMap::new(),
            tab_order: Vec::new(),
            mru_stack: Vec::new(),
            untitled_counter: 0,
        }
    }

    /// Creates a workspace with a single new empty document and view.
    ///
    /// This is the typical startup state for a new editing session.
    pub fn with_new_document() -> Self {
        let mut workspace = Self::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);
        workspace
    }

    // =========================================================================
    // Document Lifecycle
    // =========================================================================

    /// Creates a new empty document and returns its ID.
    ///
    /// Also creates a command history for the document.
    pub fn create_document(&mut self) -> DocumentId {
        let document = Document::new();
        let doc_id = document.id();

        self.event_bus.publish(document.event_opened());
        self.documents.insert(doc_id, document);
        self.histories.insert(doc_id, CommandHistory::new());

        doc_id
    }

    /// Creates a new untitled document and returns its ID.
    ///
    /// Assigns a display name following the convention:
    /// - First: "Untitled"
    /// - Subsequent: "Untitled (2)", "Untitled (3)", etc.
    ///
    /// The counter is monotonically increasing within the session and never resets.
    pub fn create_untitled_document(&mut self) -> DocumentId {
        self.untitled_counter += 1;
        let name = if self.untitled_counter == 1 {
            "Untitled".to_string()
        } else {
            format!("Untitled ({})", self.untitled_counter)
        };

        let document = Document::new_with_name(name);
        let doc_id = document.id();

        self.event_bus.publish(document.event_opened());
        self.documents.insert(doc_id, document);
        self.histories.insert(doc_id, CommandHistory::new());

        doc_id
    }

    /// Opens a document from a file path.
    ///
    /// Returns `(doc_id, was_existing)`. If the canonical path is already open,
    /// returns the existing DocumentId with `was_existing = true` — no new
    /// Document is created. If the path is new, creates the document and
    /// registers it in the Buffer Registry.
    pub fn open_document(&mut self, path: &Path) -> std::io::Result<(DocumentId, bool)> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        if let Some(&existing_id) = self.path_to_doc.get(&canonical) {
            return Ok((existing_id, true));
        }

        let document = Document::open(path)?;
        let doc_id = document.id();

        self.event_bus.publish(document.event_opened());
        self.documents.insert(doc_id, document);
        self.histories.insert(doc_id, CommandHistory::new());
        self.path_to_doc.insert(canonical, doc_id);

        Ok((doc_id, false))
    }

    /// Closes a document and all views associated with it (force close).
    ///
    /// Returns `true` if the document was found and closed.
    ///
    /// **Note**: This method does not check for unsaved changes.
    /// Use `close_document_protected()` for safe closing with protection.
    pub(crate) fn close_document_force(&mut self, doc_id: DocumentId) -> bool {
        if let Some(document) = self.documents.remove(&doc_id) {
            // Emit close event
            self.event_bus.publish(document.event_closed());

            // Remove the history
            self.histories.remove(&doc_id);

            // Remove from Buffer Registry by value (canonical path may differ from stored path)
            self.path_to_doc.retain(|_, &mut id| id != doc_id);

            // Close all views referencing this document
            let views_to_close: Vec<ViewId> = self
                .views
                .iter()
                .filter(|(_, view)| view.document_id() == doc_id)
                .map(|(id, _)| *id)
                .collect();

            for view_id in views_to_close {
                self.close_view(view_id);
            }

            // Clear active view if it was pointing to a closed view
            if let Some(active_id) = self.active_view_id {
                if !self.views.contains_key(&active_id) {
                    self.active_view_id = self.mru_stack.first().copied();
                }
            }

            true
        } else {
            false
        }
    }

    /// Returns a reference to a document by ID.
    pub fn document(&self, doc_id: DocumentId) -> Option<&Document> {
        self.documents.get(&doc_id)
    }

    /// Returns a mutable reference to a document by ID.
    pub fn document_mut(&mut self, doc_id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(&doc_id)
    }

    /// Returns the number of open documents.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Returns an iterator over all documents.
    pub fn documents(&self) -> impl Iterator<Item = (&DocumentId, &Document)> {
        self.documents.iter()
    }

    /// Returns an iterator over all document IDs.
    pub fn document_ids(&self) -> impl Iterator<Item = &DocumentId> {
        self.documents.keys()
    }

    // =========================================================================
    // View Lifecycle
    // =========================================================================

    /// Creates a new view for a document.
    ///
    /// The new view becomes the active view.
    /// Inserted into `tab_order` after the current active tab (or at end if none).
    /// Pushed to the front of `mru_stack`.
    /// Returns the view ID.
    pub fn create_view(&mut self, doc_id: DocumentId) -> ViewId {
        let view = EditorView::new(doc_id);
        let view_id = view.id();

        self.event_bus.publish(view.event_created());
        self.views.insert(view_id, view);

        // Insert into tab_order after the current active tab position
        let insert_pos = self
            .active_view_id
            .and_then(|active_id| self.tab_order.iter().position(|&id| id == active_id))
            .map(|pos| pos + 1)
            .unwrap_or(self.tab_order.len());
        self.tab_order.insert(insert_pos, view_id);

        // Push to front of MRU stack
        self.mru_stack.insert(0, view_id);

        self.active_view_id = Some(view_id);

        view_id
    }

    /// Closes a view.
    ///
    /// Returns `true` if the view was found and closed.
    /// Does NOT close the underlying document.
    pub fn close_view(&mut self, view_id: ViewId) -> bool {
        if let Some(view) = self.views.remove(&view_id) {
            self.event_bus.publish(view.event_closed());

            // Remove from tab_order and mru_stack
            self.tab_order.retain(|&id| id != view_id);
            self.mru_stack.retain(|&id| id != view_id);

            // If this was the active view, select the MRU fallback
            if self.active_view_id == Some(view_id) {
                self.active_view_id = self.mru_stack.first().copied();
            }

            true
        } else {
            false
        }
    }

    /// Sets the active view.
    ///
    /// Promotes the view to the front of the MRU stack.
    /// Returns `true` if the view exists and was activated.
    pub fn set_active_view(&mut self, view_id: ViewId) -> bool {
        if self.views.contains_key(&view_id) {
            self.active_view_id = Some(view_id);
            // Promote to front of MRU stack
            self.mru_stack.retain(|&id| id != view_id);
            self.mru_stack.insert(0, view_id);
            true
        } else {
            false
        }
    }

    /// Returns a reference to a view by ID.
    pub fn view(&self, view_id: ViewId) -> Option<&EditorView> {
        self.views.get(&view_id)
    }

    /// Returns a mutable reference to a view by ID.
    pub fn view_mut(&mut self, view_id: ViewId) -> Option<&mut EditorView> {
        self.views.get_mut(&view_id)
    }

    /// Returns the number of open views.
    pub fn view_count(&self) -> usize {
        self.views.len()
    }

    /// Returns an iterator over all views.
    pub fn views(&self) -> impl Iterator<Item = (&ViewId, &EditorView)> {
        self.views.iter()
    }

    /// Returns the active view ID, if any.
    pub fn active_view_id(&self) -> Option<ViewId> {
        self.active_view_id
    }

    /// Returns the tab strip order as a slice of ViewIds.
    pub fn tab_order(&self) -> &[ViewId] {
        &self.tab_order
    }

    /// Returns the MRU activation stack as a slice of ViewIds.
    pub fn mru_stack(&self) -> &[ViewId] {
        &self.mru_stack
    }

    // =========================================================================
    // Active View/Document Accessors
    // =========================================================================

    /// Returns a reference to the active view.
    pub fn active_view(&self) -> Option<&EditorView> {
        self.active_view_id.and_then(|id| self.views.get(&id))
    }

    /// Returns a mutable reference to the active view.
    pub fn active_view_mut(&mut self) -> Option<&mut EditorView> {
        self.active_view_id.and_then(|id| self.views.get_mut(&id))
    }

    /// Returns a reference to the document of the active view.
    pub fn active_document(&self) -> Option<&Document> {
        self.active_view()
            .and_then(|view| self.documents.get(&view.document_id()))
    }

    /// Returns a mutable reference to the document of the active view.
    pub fn active_document_mut(&mut self) -> Option<&mut Document> {
        let doc_id = self.active_view().map(|v| v.document_id())?;
        self.documents.get_mut(&doc_id)
    }

    /// Returns the command history for the active document.
    pub fn active_history(&self) -> Option<&CommandHistory> {
        self.active_view()
            .and_then(|view| self.histories.get(&view.document_id()))
    }

    /// Returns a mutable command history for the active document.
    pub fn active_history_mut(&mut self) -> Option<&mut CommandHistory> {
        let doc_id = self.active_view().map(|v| v.document_id())?;
        self.histories.get_mut(&doc_id)
    }

    /// Returns the command history for a specific document.
    pub fn history(&self, doc_id: DocumentId) -> Option<&CommandHistory> {
        self.histories.get(&doc_id)
    }

    /// Returns a mutable command history for a specific document.
    pub fn history_mut(&mut self, doc_id: DocumentId) -> Option<&mut CommandHistory> {
        self.histories.get_mut(&doc_id)
    }

    // =========================================================================
    // Event Bus Access
    // =========================================================================

    /// Returns a reference to the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns a mutable reference to the event bus.
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// Publishes an event to the event bus.
    pub fn publish(&mut self, event: DomainEvent) {
        self.event_bus.publish(event);
    }

    // =========================================================================
    // Convenience Methods for Active View/Document Operations
    // =========================================================================

    /// Executes a closure with mutable access to the active context.
    ///
    /// This safely handles multiple mutable borrows by passing all needed
    /// references to a closure rather than returning them.
    ///
    /// Returns `None` if there's no active view or its document is missing.
    pub fn with_active_context<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut EditorView, &mut Document, &mut CommandHistory, &mut EventBus) -> R,
    {
        let view_id = self.active_view_id?;
        let doc_id = {
            let view = self.views.get(&view_id)?;
            view.document_id()
        };

        // Check that all components exist before getting mutable refs
        if !self.views.contains_key(&view_id)
            || !self.documents.contains_key(&doc_id)
            || !self.histories.contains_key(&doc_id)
        {
            return None;
        }

        // Get raw pointers to avoid borrow checker issues
        // SAFETY: We've verified all keys exist above, and the HashMaps have
        // different key types (ViewId vs DocumentId), so the entries are distinct.
        // This is safe because we're not modifying the HashMap structure itself.
        let view_ptr = self.views.get_mut(&view_id).unwrap() as *mut EditorView;
        let doc_ptr = self.documents.get_mut(&doc_id).unwrap() as *mut Document;
        let history_ptr = self.histories.get_mut(&doc_id).unwrap() as *mut CommandHistory;
        let event_bus_ptr = &mut self.event_bus as *mut EventBus;

        // SAFETY: These pointers point to different data structures within self.
        // The views HashMap, documents HashMap, histories HashMap, and event_bus
        // are all separate fields, so the mutable references don't alias.
        unsafe {
            Some(f(
                &mut *view_ptr,
                &mut *doc_ptr,
                &mut *history_ptr,
                &mut *event_bus_ptr,
            ))
        }
    }

    /// Saves the active document.
    ///
    /// Returns `Ok(true)` if saved, `Ok(false)` if no path, `Err` on IO error.
    pub fn save_active_document(&mut self) -> std::io::Result<bool> {
        if let Some(doc) = self.active_document_mut() {
            let result = doc.save()?;
            if result {
                let doc_id = doc.id();
                self.event_bus.publish(DomainEvent::DocumentSaved { document_id: doc_id });
                self.event_bus.publish(DomainEvent::DocumentMetadataChanged { document_id: doc_id });
            }
            Ok(result)
        } else {
            Ok(false)
        }
    }

    /// Returns whether the active document has unsaved changes.
    pub fn active_document_dirty(&self) -> bool {
        self.active_document().map(|d| d.is_dirty()).unwrap_or(false)
    }

    /// Returns the cursor position in the active view as a TextPosition.
    pub fn active_cursor_position(&self) -> Option<TextPosition> {
        let view = self.active_view()?;
        let doc = self.active_document()?;
        Some(doc.offset_to_position(view.caret_offset()))
    }

    /// Checks if we can close safely (no unsaved documents).
    pub fn has_unsaved_documents(&self) -> bool {
        self.documents.values().any(|d| d.is_dirty())
    }

    /// Returns all unsaved documents.
    pub fn unsaved_documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values().filter(|d| d.is_dirty())
    }

    // =========================================================================
    // Protected Operations (Unsaved File Protection)
    // =========================================================================

    /// Returns the display title for a document.
    fn document_title(&self, doc_id: DocumentId) -> String {
        self.documents
            .get(&doc_id)
            .and_then(|doc| doc.file_path())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[New File]")
            .to_string()
    }

    /// Attempts to close a document, returning an error if it has unsaved changes.
    ///
    /// This is the protected version that enforces acknowledgment of unsaved changes.
    pub fn close_document_protected(&mut self, doc_id: DocumentId) -> ProtectedResult<bool> {
        if let Some(doc) = self.documents.get(&doc_id) {
            if doc.is_dirty() {
                let unsaved = UnsavedDocument::new(doc_id, self.document_title(doc_id));
                return Err(ProtectionError::UnsavedChanges(unsaved));
            }
        }
        Ok(self.close_document_force(doc_id))
    }

    /// Closes a document after explicit acknowledgment of unsaved changes.
    ///
    /// The `_ack` parameter proves the caller confirmed the discard.
    pub fn close_document_acknowledged(
        &mut self,
        doc_id: DocumentId,
        _ack: DiscardAcknowledgment,
    ) -> bool {
        self.close_document_force(doc_id)
    }

    /// Checks if the application can exit safely.
    ///
    /// Returns `Ok(())` if all documents are saved, or an error listing
    /// all documents with unsaved changes.
    pub fn can_exit(&self) -> ProtectedResult<()> {
        let unsaved: Vec<UnsavedDocument> = self
            .documents
            .iter()
            .filter(|(_, doc)| doc.is_dirty())
            .map(|(id, _)| UnsavedDocument::new(*id, self.document_title(*id)))
            .collect();

        match unsaved.len() {
            0 => Ok(()),
            1 => Err(ProtectionError::UnsavedChanges(unsaved.into_iter().next().unwrap())),
            _ => Err(ProtectionError::MultipleUnsavedChanges(unsaved)),
        }
    }

    /// Checks if switching to a different active document is safe.
    ///
    /// Returns `Ok(())` if the active document is saved, or an error
    /// if it has unsaved changes.
    pub fn can_switch_active(&self) -> ProtectedResult<()> {
        if let Some(view) = self.active_view() {
            let doc_id = view.document_id();
            if let Some(doc) = self.documents.get(&doc_id) {
                if doc.is_dirty() {
                    let unsaved = UnsavedDocument::new(doc_id, self.document_title(doc_id));
                    return Err(ProtectionError::UnsavedChanges(unsaved));
                }
            }
        }
        Ok(())
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_workspace() {
        let workspace = Workspace::new();
        assert_eq!(workspace.document_count(), 0);
        assert_eq!(workspace.view_count(), 0);
        assert!(workspace.active_view_id().is_none());
    }

    #[test]
    fn test_with_new_document() {
        let workspace = Workspace::with_new_document();
        assert_eq!(workspace.document_count(), 1);
        assert_eq!(workspace.view_count(), 1);
        assert!(workspace.active_view_id().is_some());
        assert!(workspace.active_document().is_some());
    }

    #[test]
    fn test_create_document() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();

        assert_eq!(workspace.document_count(), 1);
        assert!(workspace.document(doc_id).is_some());
        assert!(workspace.history(doc_id).is_some());
    }

    #[test]
    fn test_create_view() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        let view_id = workspace.create_view(doc_id);

        assert_eq!(workspace.view_count(), 1);
        assert!(workspace.view(view_id).is_some());
        assert_eq!(workspace.active_view_id(), Some(view_id));
    }

    #[test]
    fn test_multiple_views_same_document() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        let view1_id = workspace.create_view(doc_id);
        let view2_id = workspace.create_view(doc_id);

        assert_eq!(workspace.view_count(), 2);
        assert_ne!(view1_id, view2_id);

        // Both views reference the same document
        let view1 = workspace.view(view1_id).unwrap();
        let view2 = workspace.view(view2_id).unwrap();
        assert_eq!(view1.document_id(), view2.document_id());
    }

    #[test]
    fn test_close_document_closes_views() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);
        workspace.create_view(doc_id);

        assert_eq!(workspace.view_count(), 2);

        workspace.close_document_force(doc_id);

        assert_eq!(workspace.document_count(), 0);
        assert_eq!(workspace.view_count(), 0);
    }

    #[test]
    fn test_close_view_keeps_document() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        let view_id = workspace.create_view(doc_id);

        workspace.close_view(view_id);

        assert_eq!(workspace.view_count(), 0);
        assert_eq!(workspace.document_count(), 1); // Document still exists
    }

    #[test]
    fn test_set_active_view() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        let view1_id = workspace.create_view(doc_id);
        let view2_id = workspace.create_view(doc_id);

        assert_eq!(workspace.active_view_id(), Some(view2_id)); // Last created is active

        workspace.set_active_view(view1_id);
        assert_eq!(workspace.active_view_id(), Some(view1_id));
    }

    #[test]
    fn test_active_view_fallback_on_close() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        let view1_id = workspace.create_view(doc_id);
        let view2_id = workspace.create_view(doc_id);

        workspace.set_active_view(view2_id);
        workspace.close_view(view2_id);

        // Should fall back to view1 via MRU stack
        assert_eq!(workspace.active_view_id(), Some(view1_id));
    }

    // =========================================================================
    // Protected Operations Tests
    // =========================================================================

    #[test]
    fn test_close_document_protected_clean() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);

        // Clean document should close without error
        let result = workspace.close_document_protected(doc_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(workspace.document_count(), 0);
    }

    #[test]
    fn test_close_document_protected_dirty() {
        use crate::domain::ProtectionError;

        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);

        // Make the document dirty
        if let Some(doc) = workspace.document_mut(doc_id) {
            doc.mark_dirty();
        }

        // Dirty document should return error
        let result = workspace.close_document_protected(doc_id);
        assert!(result.is_err());

        if let Err(ProtectionError::UnsavedChanges(unsaved)) = result {
            assert_eq!(unsaved.id, doc_id);
        } else {
            panic!("Expected UnsavedChanges error");
        }

        // Document should still be open
        assert_eq!(workspace.document_count(), 1);
    }

    #[test]
    fn test_close_document_acknowledged() {
        use crate::domain::DiscardAcknowledgment;

        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);

        // Make the document dirty
        if let Some(doc) = workspace.document_mut(doc_id) {
            doc.mark_dirty();
        }

        // With acknowledgment, close should succeed
        let ack = DiscardAcknowledgment::confirmed();
        let result = workspace.close_document_acknowledged(doc_id, ack);
        assert!(result);
        assert_eq!(workspace.document_count(), 0);
    }

    #[test]
    fn test_can_exit_all_clean() {
        let mut workspace = Workspace::new();
        workspace.create_document();
        workspace.create_document();

        // All clean documents - can exit
        let result = workspace.can_exit();
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_exit_one_dirty() {
        use crate::domain::ProtectionError;

        let mut workspace = Workspace::new();
        let doc1_id = workspace.create_document();
        let _doc2_id = workspace.create_document();

        // Make one document dirty
        if let Some(doc) = workspace.document_mut(doc1_id) {
            doc.mark_dirty();
        }

        let result = workspace.can_exit();
        assert!(result.is_err());

        if let Err(ProtectionError::UnsavedChanges(unsaved)) = result {
            assert_eq!(unsaved.id, doc1_id);
        } else {
            panic!("Expected UnsavedChanges error");
        }
    }

    #[test]
    fn test_can_exit_multiple_dirty() {
        use crate::domain::ProtectionError;

        let mut workspace = Workspace::new();
        let doc1_id = workspace.create_document();
        let doc2_id = workspace.create_document();

        // Make both documents dirty
        if let Some(doc) = workspace.document_mut(doc1_id) {
            doc.mark_dirty();
        }
        if let Some(doc) = workspace.document_mut(doc2_id) {
            doc.mark_dirty();
        }

        let result = workspace.can_exit();
        assert!(result.is_err());

        if let Err(ProtectionError::MultipleUnsavedChanges(unsaved)) = result {
            assert_eq!(unsaved.len(), 2);
        } else {
            panic!("Expected MultipleUnsavedChanges error");
        }
    }

    #[test]
    fn test_can_switch_active_clean() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);

        // Clean document - can switch
        let result = workspace.can_switch_active();
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_switch_active_dirty() {
        use crate::domain::ProtectionError;

        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();
        workspace.create_view(doc_id);

        // Make the document dirty
        if let Some(doc) = workspace.document_mut(doc_id) {
            doc.mark_dirty();
        }

        let result = workspace.can_switch_active();
        assert!(result.is_err());

        if let Err(ProtectionError::UnsavedChanges(unsaved)) = result {
            assert_eq!(unsaved.id, doc_id);
        } else {
            panic!("Expected UnsavedChanges error");
        }
    }

    // =========================================================================
    // Buffer Registry, Tab Order, and MRU Stack Tests
    // =========================================================================

    #[test]
    fn test_open_document_deduplication() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_open_document_dedup_11_01.txt");
        std::fs::write(&file_path, "hello dedup").unwrap();

        let mut workspace = Workspace::new();

        let (doc_id_1, was_existing_1) = workspace.open_document(&file_path).unwrap();
        assert!(!was_existing_1, "first open should not be existing");

        let (doc_id_2, was_existing_2) = workspace.open_document(&file_path).unwrap();
        assert!(was_existing_2, "second open of same path should be existing");
        assert_eq!(doc_id_1, doc_id_2, "both opens should return same DocumentId");

        // Only one document should exist in workspace
        assert_eq!(workspace.document_count(), 1);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_tab_order_maintained() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();

        let view1_id = workspace.create_view(doc_id);
        // view1 is active; view2 inserts after it
        let view2_id = workspace.create_view(doc_id);
        // view2 is active; view3 inserts after it
        let view3_id = workspace.create_view(doc_id);

        let order = workspace.tab_order();
        assert_eq!(order.len(), 3);
        // Insertion order: view1 at 0, view2 after view1, view3 after view2
        assert!(order.contains(&view1_id));
        assert!(order.contains(&view2_id));
        assert!(order.contains(&view3_id));

        // Each is at the expected position
        let pos1 = order.iter().position(|&id| id == view1_id).unwrap();
        let pos2 = order.iter().position(|&id| id == view2_id).unwrap();
        let pos3 = order.iter().position(|&id| id == view3_id).unwrap();
        assert!(pos1 < pos2, "view1 should come before view2");
        assert!(pos2 < pos3, "view2 should come before view3");
    }

    #[test]
    fn test_mru_stack_on_switch() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();

        let view1_id = workspace.create_view(doc_id);
        let view2_id = workspace.create_view(doc_id);
        let view3_id = workspace.create_view(doc_id);

        // Activate view1
        workspace.set_active_view(view1_id);
        assert_eq!(workspace.mru_stack()[0], view1_id);

        // Activate view3
        workspace.set_active_view(view3_id);
        assert_eq!(workspace.mru_stack()[0], view3_id);

        // Activate view2
        workspace.set_active_view(view2_id);
        assert_eq!(workspace.mru_stack()[0], view2_id);

        // MRU front matches last activated
        assert_eq!(workspace.active_view_id(), Some(view2_id));
    }

    #[test]
    fn test_close_activates_mru() {
        let mut workspace = Workspace::new();
        let doc_id = workspace.create_document();

        let view_a = workspace.create_view(doc_id);
        let view_b = workspace.create_view(doc_id);
        let view_c = workspace.create_view(doc_id);

        // Activation sequence: A -> B -> C
        workspace.set_active_view(view_a);
        workspace.set_active_view(view_b);
        workspace.set_active_view(view_c);

        assert_eq!(workspace.active_view_id(), Some(view_c));

        // Close C — should fall back to B (MRU)
        workspace.close_view(view_c);

        assert_eq!(workspace.active_view_id(), Some(view_b),
            "After closing active view C, B (MRU) should become active");
    }
}
