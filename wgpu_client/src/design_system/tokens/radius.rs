//! Border radius tokens.
//!
//! Consistent border radii create a cohesive visual language.
//! The scale ranges from sharp corners to fully rounded (pill) shapes.

/// Border radius scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Radius {
    /// 0px - Sharp corners
    None,
    /// 2px - Subtle rounding
    Xs,
    /// 4px - Default for small elements
    #[default]
    Sm,
    /// 6px - Default for buttons
    Md,
    /// 8px - Cards, larger elements
    Lg,
    /// 12px - Prominent rounding
    Xl,
    /// 9999px - Fully rounded (pill shape)
    Full,
}

impl Radius {
    /// Convert to pixel value.
    pub const fn px(self) -> f32 {
        match self {
            Radius::None => 0.0,
            Radius::Xs => 2.0,
            Radius::Sm => 4.0,
            Radius::Md => 6.0,
            Radius::Lg => 8.0,
            Radius::Xl => 12.0,
            Radius::Full => 9999.0,
        }
    }
}

impl From<Radius> for f32 {
    fn from(radius: Radius) -> f32 {
        radius.px()
    }
}

/// Per-corner radii for fine-grained control.
///
/// Follows CSS order: top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    /// Create uniform radius on all corners.
    pub const fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Create from a Radius token.
    pub const fn from_radius(radius: Radius) -> Self {
        Self::all(radius.px())
    }

    /// Create with different top and bottom radii.
    pub const fn vertical(top: f32, bottom: f32) -> Self {
        Self {
            top_left: top,
            top_right: top,
            bottom_right: bottom,
            bottom_left: bottom,
        }
    }

    /// Create with different left and right radii.
    pub const fn horizontal(left: f32, right: f32) -> Self {
        Self {
            top_left: left,
            top_right: right,
            bottom_right: right,
            bottom_left: left,
        }
    }

    /// Create with only top corners rounded (e.g., for tabs).
    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// Create with only bottom corners rounded.
    pub const fn bottom(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Create with only left corners rounded.
    pub const fn left(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: radius,
        }
    }

    /// Create with only right corners rounded.
    pub const fn right(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: radius,
            bottom_right: radius,
            bottom_left: 0.0,
        }
    }

    /// No rounding.
    pub const ZERO: Self = Self::all(0.0);

    /// Convert to array for shader uniforms [tl, tr, br, bl].
    pub const fn to_array(self) -> [f32; 4] {
        [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
    }

    /// Clamp radii to fit within the given dimensions.
    ///
    /// When a corner radius is larger than half the width or height,
    /// it needs to be clamped to prevent overlapping.
    pub fn clamped(self, width: f32, height: f32) -> Self {
        let max_radius = (width.min(height)) / 2.0;
        Self {
            top_left: self.top_left.min(max_radius),
            top_right: self.top_right.min(max_radius),
            bottom_right: self.bottom_right.min(max_radius),
            bottom_left: self.bottom_left.min(max_radius),
        }
    }
}
