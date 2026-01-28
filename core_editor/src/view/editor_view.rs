//! EditorView: A viewport onto a Document.
//!
//! Combines caret, selection, viewport, and view-specific options.
//! Multiple EditorViews can reference the same Document.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::{DocumentId, TextOffset, TextPosition};
use crate::events::DomainEvent;
use crate::view::{CaretSet, SelectionSet, Viewport};

/// Unique identifier for an editor view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(u64);

impl ViewId {
    /// Generates a new unique ViewId.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Creates a ViewId from a raw u64 value.
    /// Used for reconstructing ViewId from serialized data (e.g., tab clicks).
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying u64 value.
    /// Used for passing view ID through UI layers that don't depend on core_editor types.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for ViewId {
    fn default() -> Self {
        Self::new()
    }
}

/// View-specific display options.
#[derive(Clone, Debug)]
pub struct ViewOptions {
    /// Whether to show line numbers in the gutter.
    pub show_line_numbers: bool,
    /// Tab width in spaces.
    pub tab_width: usize,
    /// Scroll offset (lines/columns to keep visible around cursor).
    pub scroll_off: usize,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            tab_width: 4,
            scroll_off: 2,
        }
    }
}

/// A viewport onto a Document.
///
/// Contains all view-specific state: cursor position, selection,
/// scroll position, and display options.
#[derive(Clone, Debug)]
pub struct EditorView {
    /// Unique identifier for this view.
    id: ViewId,
    /// The document this view is displaying.
    document_id: DocumentId,
    /// The visible region.
    pub viewport: Viewport,
    /// Cursor positions.
    pub carets: CaretSet,
    /// Text selections.
    pub selections: SelectionSet,
    /// Display options.
    pub options: ViewOptions,
}

impl EditorView {
    /// Creates a new EditorView for the given document.
    pub fn new(document_id: DocumentId) -> Self {
        Self {
            id: ViewId::new(),
            document_id,
            viewport: Viewport::default(),
            carets: CaretSet::at_start(),
            selections: SelectionSet::new(0),
            options: ViewOptions::default(),
        }
    }

    /// Returns the view's unique identifier.
    pub fn id(&self) -> ViewId {
        self.id
    }

    /// Returns the document ID this view is displaying.
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    /// Returns the current caret offset.
    pub fn caret_offset(&self) -> TextOffset {
        self.carets.offset()
    }

    /// Moves the caret to a new offset, optionally extending selection.
    pub fn move_caret_to(&mut self, offset: TextOffset, extend_selection: bool) {
        if extend_selection {
            self.selections.extend_to(offset);
        } else {
            self.selections.collapse_to(offset);
        }
        self.carets.move_to(offset);
    }

    /// Sets up the view to start a selection from the current caret position.
    pub fn begin_selection(&mut self) {
        let offset = self.carets.offset();
        self.selections.begin_selection(offset);
    }

    /// Clears any active selection.
    pub fn clear_selection(&mut self) {
        self.selections.collapse();
    }

    /// Returns true if there is an active selection.
    pub fn has_selection(&self) -> bool {
        !self.selections.is_empty()
    }

    /// Returns the selection range if there is one.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selections.range().map(|r| (r.start, r.end))
    }

    /// Selects all text (anchor at 0, head at given end).
    pub fn select_all(&mut self, document_length: usize) {
        self.selections.select_all(document_length);
        self.carets.move_to(document_length);
    }

    /// Ensures the caret is visible, adjusting viewport if needed.
    pub fn ensure_caret_visible(&mut self, caret_line: usize, caret_column: usize) {
        self.viewport
            .ensure_visible(caret_line, caret_column, self.options.scroll_off);
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.viewport.resize(width, height);
    }

    /// Returns whether line numbers should be shown.
    pub fn show_line_numbers(&self) -> bool {
        self.options.show_line_numbers
    }

    /// Toggles line number display.
    pub fn toggle_line_numbers(&mut self) {
        self.options.show_line_numbers = !self.options.show_line_numbers;
    }

    /// Returns cursor position as (column, line) for backwards compatibility.
    pub fn cursor_position(&self, position: TextPosition) -> (usize, usize) {
        (position.column, position.line)
    }

    // =========================================================================
    // Event creation helpers
    // =========================================================================

    /// Creates a SelectionChanged event for this view.
    pub fn event_selection_changed(&self) -> DomainEvent {
        DomainEvent::SelectionChanged {
            view_id: self.id,
            document_id: self.document_id,
            caret_offset: self.caret_offset(),
            has_selection: self.has_selection(),
        }
    }

    /// Creates a ViewportChanged event for this view.
    pub fn event_viewport_changed(&self) -> DomainEvent {
        DomainEvent::ViewportChanged {
            view_id: self.id,
            scroll_x: self.viewport.scroll_x,
            scroll_y: self.viewport.scroll_y,
            width: self.viewport.width,
            height: self.viewport.height,
        }
    }

    /// Creates a ViewCreated event for this view.
    pub fn event_created(&self) -> DomainEvent {
        DomainEvent::ViewCreated {
            view_id: self.id,
            document_id: self.document_id,
        }
    }

    /// Creates a ViewClosed event for this view.
    pub fn event_closed(&self) -> DomainEvent {
        DomainEvent::ViewClosed { view_id: self.id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_view_new() {
        let doc_id = DocumentId::new();
        let view = EditorView::new(doc_id);
        assert_eq!(view.document_id(), doc_id);
        assert_eq!(view.caret_offset(), 0);
        assert!(!view.has_selection());
    }

    #[test]
    fn test_move_caret() {
        let view_id = DocumentId::new();
        let mut view = EditorView::new(view_id);
        view.move_caret_to(100, false);
        assert_eq!(view.caret_offset(), 100);
        assert!(!view.has_selection());
    }

    #[test]
    fn test_move_caret_with_selection() {
        let doc_id = DocumentId::new();
        let mut view = EditorView::new(doc_id);
        view.begin_selection();
        view.move_caret_to(50, true);
        assert!(view.has_selection());
        assert_eq!(view.selection_range(), Some((0, 50)));
    }

    #[test]
    fn test_select_all() {
        let doc_id = DocumentId::new();
        let mut view = EditorView::new(doc_id);
        view.select_all(1000);
        assert!(view.has_selection());
        assert_eq!(view.selection_range(), Some((0, 1000)));
        assert_eq!(view.caret_offset(), 1000);
    }

    #[test]
    fn test_view_ids_are_unique() {
        let doc_id = DocumentId::new();
        let view1 = EditorView::new(doc_id);
        let view2 = EditorView::new(doc_id);
        assert_ne!(view1.id(), view2.id());
    }
}
