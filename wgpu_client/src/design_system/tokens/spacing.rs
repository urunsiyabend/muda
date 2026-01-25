//! Spacing tokens based on a 4px grid system.
//!
//! Consistent spacing is crucial for visual harmony. All spacing values
//! are based on a 4px base unit, creating a predictable rhythm.

/// Spacing scale based on 4px base unit.
///
/// The scale provides named values for common spacing needs,
/// from tight component padding to generous section margins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Space {
    /// 0px - No spacing
    None,
    /// 2px - Extra extra small (half step)
    Xxs,
    /// 4px - Extra small (1 unit)
    Xs,
    /// 8px - Small (2 units)
    Sm,
    /// 12px - Medium (3 units)
    Md,
    /// 16px - Large (4 units)
    Lg,
    /// 24px - Extra large (6 units)
    Xl,
    /// 32px - Extra extra large (8 units)
    Xxl,
    /// 48px - Extra extra extra large (12 units)
    Xxxl,
}

impl Space {
    /// Convert to pixel value.
    pub const fn px(self) -> f32 {
        match self {
            Space::None => 0.0,
            Space::Xxs => 2.0,
            Space::Xs => 4.0,
            Space::Sm => 8.0,
            Space::Md => 12.0,
            Space::Lg => 16.0,
            Space::Xl => 24.0,
            Space::Xxl => 32.0,
            Space::Xxxl => 48.0,
        }
    }

    /// Create spacing from a raw pixel value (rounds to nearest token).
    pub fn from_px(px: f32) -> Self {
        match px as i32 {
            0 => Space::None,
            1..=2 => Space::Xxs,
            3..=5 => Space::Xs,
            6..=9 => Space::Sm,
            10..=13 => Space::Md,
            14..=19 => Space::Lg,
            20..=27 => Space::Xl,
            28..=39 => Space::Xxl,
            _ => Space::Xxxl,
        }
    }
}

impl Default for Space {
    fn default() -> Self {
        Space::None
    }
}

impl From<Space> for f32 {
    fn from(space: Space) -> f32 {
        space.px()
    }
}

/// Four-sided spacing (like CSS padding/margin).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spacing {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Spacing {
    /// Create uniform spacing on all sides.
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create spacing from a Space token.
    pub const fn from_space(space: Space) -> Self {
        Self::all(space.px())
    }

    /// Create symmetric spacing (vertical, horizontal).
    pub const fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create symmetric spacing from Space tokens.
    pub const fn symmetric_space(vertical: Space, horizontal: Space) -> Self {
        Self::symmetric(vertical.px(), horizontal.px())
    }

    /// Create spacing with individual values.
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    /// No spacing.
    pub const ZERO: Self = Self::all(0.0);

    /// Total horizontal spacing (left + right).
    pub const fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical spacing (top + bottom).
    pub const fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}
