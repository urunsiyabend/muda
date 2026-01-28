/// CSS-like length values supporting pixels, percentages, and auto sizing
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    Px(f32),
    Percent(f32),
    #[default]
    Auto,
}

/// Edges representing values for top, right, bottom, left (padding, margin, border widths)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn xy(horizontal: T, vertical: T) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

impl Edges<f32> {
    pub fn zero() -> Self {
        Self::all(0.0)
    }
}

/// Corners representing values for each corner (border-radius)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub fn all(value: T) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

impl Corners<f32> {
    pub fn zero() -> Self {
        Self::all(0.0)
    }
}

/// Size with width and height
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// Point with x and y coordinates
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

/// Rectangle with origin and size
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point<f32>,
    pub size: Size<f32>,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn zero() -> Self {
        Self {
            origin: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
        }
    }

    /// Compute intersection of two rectangles (for scissor clipping)
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x1 = self.origin.x.max(other.origin.x);
        let y1 = self.origin.y.max(other.origin.y);
        let x2 = (self.origin.x + self.size.width).min(other.origin.x + other.size.width);
        let y2 = (self.origin.y + self.size.height).min(other.origin.y + other.size.height);

        let width = (x2 - x1).max(0.0);
        let height = (y2 - y1).max(0.0);

        Rect::new(x1, y1, width, height)
    }
}
