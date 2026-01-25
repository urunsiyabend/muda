//! Elevation and shadow tokens.
//!
//! Elevation creates visual hierarchy through shadows. Higher elevation
//! indicates elements that float above others (dropdowns, modals, etc.).

use crate::theme::Color;

/// Elevation levels for layered UI elements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// No elevation (flat on surface)
    #[default]
    None,
    /// Subtle elevation (1dp) - hover states, cards
    Low,
    /// Medium elevation (4dp) - dropdowns, popovers
    Medium,
    /// High elevation (8dp) - modals, dialogs
    High,
    /// Highest elevation (16dp) - notifications, toasts
    Highest,
}

impl Elevation {
    /// Get the shadow configuration for this elevation level.
    pub fn shadow(self) -> Option<Shadow> {
        match self {
            Elevation::None => None,
            Elevation::Low => Some(Shadow {
                offset_x: 0.0,
                offset_y: 1.0,
                blur: 3.0,
                spread: 0.0,
                color: Color::new(0.0, 0.0, 0.0, 0.12),
            }),
            Elevation::Medium => Some(Shadow {
                offset_x: 0.0,
                offset_y: 4.0,
                blur: 6.0,
                spread: -1.0,
                color: Color::new(0.0, 0.0, 0.0, 0.15),
            }),
            Elevation::High => Some(Shadow {
                offset_x: 0.0,
                offset_y: 8.0,
                blur: 16.0,
                spread: -2.0,
                color: Color::new(0.0, 0.0, 0.0, 0.2),
            }),
            Elevation::Highest => Some(Shadow {
                offset_x: 0.0,
                offset_y: 16.0,
                blur: 32.0,
                spread: -4.0,
                color: Color::new(0.0, 0.0, 0.0, 0.25),
            }),
        }
    }

    /// Get the z-index hint for this elevation.
    pub const fn z_index(self) -> u32 {
        match self {
            Elevation::None => 0,
            Elevation::Low => 10,
            Elevation::Medium => 20,
            Elevation::High => 30,
            Elevation::Highest => 40,
        }
    }
}

/// Shadow configuration for drop shadows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Horizontal offset in pixels
    pub offset_x: f32,
    /// Vertical offset in pixels
    pub offset_y: f32,
    /// Blur radius in pixels (larger = softer shadow)
    pub blur: f32,
    /// Spread radius in pixels (larger = bigger shadow, can be negative)
    pub spread: f32,
    /// Shadow color (typically black with low alpha)
    pub color: Color,
}

impl Shadow {
    /// Create a custom shadow.
    pub const fn new(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: Color) -> Self {
        Self { offset_x, offset_y, blur, spread, color }
    }

    /// Create a simple drop shadow with default color.
    pub fn drop(offset_y: f32, blur: f32) -> Self {
        Self {
            offset_x: 0.0,
            offset_y,
            blur,
            spread: 0.0,
            color: Color::new(0.0, 0.0, 0.0, 0.15),
        }
    }

    /// Create an inset shadow (inner shadow).
    pub fn inset(blur: f32, color: Color) -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur,
            spread: -blur / 2.0, // Negative spread for inset effect
            color,
        }
    }

    /// Convert to shader-friendly array [offset_x, offset_y, blur, spread].
    pub const fn to_array(&self) -> [f32; 4] {
        [self.offset_x, self.offset_y, self.blur, self.spread]
    }
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 2.0,
            blur: 4.0,
            spread: 0.0,
            color: Color::new(0.0, 0.0, 0.0, 0.15),
        }
    }
}
