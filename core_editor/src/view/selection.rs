//! Selection: A range of text associated with a caret.
//!
//! Supports multi-selection through SelectionSet.

use crate::domain::{TextOffset, TextRange};

/// A selection of text with an anchor and head (caret position).
///
/// The anchor is where the selection started, the head is where
/// the caret currently is. They may be equal (empty selection).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The anchor point (where selection started).
    pub anchor: TextOffset,
    /// The head point (current caret position).
    pub head: TextOffset,
}

impl Selection {
    /// Creates a new selection with given anchor and head.
    pub fn new(anchor: TextOffset, head: TextOffset) -> Self {
        Self { anchor, head }
    }

    /// Creates an empty selection (caret only) at the given offset.
    pub fn caret(offset: TextOffset) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    /// Returns true if the selection is empty (anchor == head).
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns the selection as a normalized TextRange (start < end).
    pub fn range(&self) -> TextRange {
        TextRange::normalized(self.anchor, self.head)
    }

    /// Returns the start of the selection (minimum of anchor and head).
    pub fn start(&self) -> TextOffset {
        self.anchor.min(self.head)
    }

    /// Returns the end of the selection (maximum of anchor and head).
    pub fn end(&self) -> TextOffset {
        self.anchor.max(self.head)
    }

    /// Returns the length of the selection.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Extends the selection by moving the head.
    pub fn extend_to(&mut self, offset: TextOffset) {
        self.head = offset;
    }

    /// Collapses the selection to a caret at the head position.
    pub fn collapse(&mut self) {
        self.anchor = self.head;
    }

    /// Collapses the selection to a caret at the given offset.
    pub fn collapse_to(&mut self, offset: TextOffset) {
        self.anchor = offset;
        self.head = offset;
    }
}

/// A set of selections for multi-selection editing.
///
/// Currently supports a single selection. Multi-selection support
/// can be added later by extending this struct.
#[derive(Clone, Debug, Default)]
pub struct SelectionSet {
    /// The primary (and currently only) selection.
    primary: Selection,
}

impl SelectionSet {
    /// Creates a new SelectionSet with an empty selection at the given offset.
    pub fn new(offset: TextOffset) -> Self {
        Self {
            primary: Selection::caret(offset),
        }
    }

    /// Returns a reference to the primary selection.
    pub fn primary(&self) -> &Selection {
        &self.primary
    }

    /// Returns a mutable reference to the primary selection.
    pub fn primary_mut(&mut self) -> &mut Selection {
        &mut self.primary
    }

    /// Returns true if the primary selection is empty.
    pub fn is_empty(&self) -> bool {
        self.primary.is_empty()
    }

    /// Returns the range of the primary selection.
    pub fn range(&self) -> Option<TextRange> {
        if self.primary.is_empty() {
            None
        } else {
            Some(self.primary.range())
        }
    }

    /// Starts a new selection from the current head position.
    pub fn start_selection(&mut self) {
        // Anchor stays where it is, ready to extend
    }

    /// Extends the primary selection to the given offset.
    pub fn extend_to(&mut self, offset: TextOffset) {
        self.primary.extend_to(offset);
    }

    /// Collapses the primary selection.
    pub fn collapse(&mut self) {
        self.primary.collapse();
    }

    /// Collapses and moves to a new offset.
    pub fn collapse_to(&mut self, offset: TextOffset) {
        self.primary.collapse_to(offset);
    }

    /// Sets up for selection: sets anchor at current head.
    pub fn begin_selection(&mut self, offset: TextOffset) {
        self.primary.anchor = offset;
        self.primary.head = offset;
    }

    /// Selects all (sets anchor to 0, head to given end).
    pub fn select_all(&mut self, end: TextOffset) {
        self.primary.anchor = 0;
        self.primary.head = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_caret() {
        let sel = Selection::caret(10);
        assert!(sel.is_empty());
        assert_eq!(sel.anchor, 10);
        assert_eq!(sel.head, 10);
    }

    #[test]
    fn test_selection_range() {
        let sel = Selection::new(5, 15);
        assert!(!sel.is_empty());
        assert_eq!(sel.start(), 5);
        assert_eq!(sel.end(), 15);
        assert_eq!(sel.len(), 10);
    }

    #[test]
    fn test_selection_backwards() {
        let sel = Selection::new(15, 5);
        assert_eq!(sel.start(), 5);
        assert_eq!(sel.end(), 15);
        assert_eq!(sel.range(), TextRange::new(5, 15));
    }

    #[test]
    fn test_selection_extend() {
        let mut sel = Selection::caret(10);
        sel.extend_to(20);
        assert!(!sel.is_empty());
        assert_eq!(sel.anchor, 10);
        assert_eq!(sel.head, 20);
    }

    #[test]
    fn test_selection_set() {
        let mut set = SelectionSet::new(0);
        assert!(set.is_empty());
        set.select_all(100);
        assert!(!set.is_empty());
        assert_eq!(set.range(), Some(TextRange::new(0, 100)));
    }
}
