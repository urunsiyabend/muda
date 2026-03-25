//! Caret: A single insertion point (cursor) in the document.
//!
//! Supports multi-cursor editing through CaretSet.

use crate::domain::TextOffset;

/// A single insertion point in the document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    /// The character offset position of the caret.
    pub offset: TextOffset,
    /// Preferred column for vertical movement (sticky column).
    /// This helps maintain column position when moving through lines of varying length.
    pub preferred_column: Option<usize>,
}

impl Caret {
    /// Creates a new caret at the given offset.
    pub fn new(offset: TextOffset) -> Self {
        Self {
            offset,
            preferred_column: None,
        }
    }

    /// Creates a caret at offset 0.
    pub fn at_start() -> Self {
        Self::new(0)
    }

    /// Sets the preferred column for vertical navigation.
    pub fn set_preferred_column(&mut self, column: usize) {
        self.preferred_column = Some(column);
    }

    /// Clears the preferred column.
    pub fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }

    /// Moves the caret to a new offset.
    pub fn move_to(&mut self, offset: TextOffset) {
        self.offset = offset;
    }
}

/// An ordered set of carets for multi-cursor editing.
///
/// Currently supports a single caret. Multi-cursor support
/// can be added later by extending this struct.
#[derive(Clone, Debug, Default)]
pub struct CaretSet {
    /// The primary (and currently only) caret.
    primary: Caret,
}

impl CaretSet {
    /// Creates a new CaretSet with a single caret at the given offset.
    pub fn new(offset: TextOffset) -> Self {
        Self {
            primary: Caret::new(offset),
        }
    }

    /// Creates a CaretSet with the caret at the document start.
    pub fn at_start() -> Self {
        Self {
            primary: Caret::at_start(),
        }
    }

    /// Returns a reference to the primary caret.
    pub fn primary(&self) -> &Caret {
        &self.primary
    }

    /// Returns a mutable reference to the primary caret.
    pub fn primary_mut(&mut self) -> &mut Caret {
        &mut self.primary
    }

    /// Returns the offset of the primary caret.
    pub fn offset(&self) -> TextOffset {
        self.primary.offset
    }

    /// Moves the primary caret to a new offset.
    pub fn move_to(&mut self, offset: TextOffset) {
        self.primary.move_to(offset);
    }

    /// Returns the number of carets.
    pub fn len(&self) -> usize {
        1 // Currently single caret only
    }

    /// Returns true if there are no carets (always false for now).
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caret_new() {
        let caret = Caret::new(10);
        assert_eq!(caret.offset, 10);
        assert_eq!(caret.preferred_column, None);
    }

    #[test]
    fn test_caret_preferred_column() {
        let mut caret = Caret::new(0);
        caret.set_preferred_column(5);
        assert_eq!(caret.preferred_column, Some(5));
        caret.clear_preferred_column();
        assert_eq!(caret.preferred_column, None);
    }

    #[test]
    fn test_caret_set() {
        let mut set = CaretSet::new(100);
        assert_eq!(set.offset(), 100);
        set.move_to(200);
        assert_eq!(set.offset(), 200);
        assert_eq!(set.len(), 1);
    }
}
