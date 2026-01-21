//! Viewport: The visible region of a document.
//!
//! Represents scroll position and dimensions of the editor view.

/// The visible region of a document.
#[derive(Clone, Debug, Default)]
pub struct Viewport {
    /// Horizontal scroll offset (in characters).
    pub scroll_x: usize,
    /// Vertical scroll offset (in lines).
    pub scroll_y: usize,
    /// Width of the viewport (in characters).
    pub width: usize,
    /// Height of the viewport (in lines).
    pub height: usize,
}

impl Viewport {
    /// Creates a new viewport with the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            width,
            height,
        }
    }

    /// Returns the range of visible lines.
    pub fn visible_line_range(&self) -> std::ops::Range<usize> {
        self.scroll_y..self.scroll_y + self.height
    }

    /// Returns true if the given line is visible.
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.scroll_y && line < self.scroll_y + self.height
    }

    /// Adjusts scroll to ensure the given position is visible.
    /// Returns true if scroll changed.
    pub fn ensure_visible(&mut self, line: usize, column: usize, scroll_off: usize) -> bool {
        let mut changed = false;

        // Vertical scrolling
        if line < self.scroll_y + scroll_off {
            self.scroll_y = line.saturating_sub(scroll_off);
            changed = true;
        } else if line >= self.scroll_y + self.height.saturating_sub(scroll_off) {
            self.scroll_y = (line + scroll_off + 1).saturating_sub(self.height);
            changed = true;
        }

        // Horizontal scrolling
        if column < self.scroll_x + scroll_off {
            self.scroll_x = column.saturating_sub(scroll_off);
            changed = true;
        } else if column >= self.scroll_x + self.width.saturating_sub(scroll_off) {
            self.scroll_x = (column + scroll_off + 1).saturating_sub(self.width);
            changed = true;
        }

        changed
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_viewport() {
        let vp = Viewport::new(80, 24);
        assert_eq!(vp.width, 80);
        assert_eq!(vp.height, 24);
        assert_eq!(vp.scroll_x, 0);
        assert_eq!(vp.scroll_y, 0);
    }

    #[test]
    fn test_visible_line_range() {
        let mut vp = Viewport::new(80, 10);
        vp.scroll_y = 5;
        assert_eq!(vp.visible_line_range(), 5..15);
    }

    #[test]
    fn test_is_line_visible() {
        let mut vp = Viewport::new(80, 10);
        vp.scroll_y = 5;
        assert!(!vp.is_line_visible(4));
        assert!(vp.is_line_visible(5));
        assert!(vp.is_line_visible(14));
        assert!(!vp.is_line_visible(15));
    }

    #[test]
    fn test_ensure_visible_scrolls_down() {
        let mut vp = Viewport::new(80, 10);
        let changed = vp.ensure_visible(15, 0, 2);
        assert!(changed);
        assert!(vp.scroll_y > 0);
    }

    #[test]
    fn test_ensure_visible_no_change_when_visible() {
        let mut vp = Viewport::new(80, 10);
        let changed = vp.ensure_visible(5, 5, 2);
        assert!(!changed);
        assert_eq!(vp.scroll_y, 0);
    }
}
