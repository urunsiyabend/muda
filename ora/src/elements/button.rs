use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;

/// Button visual variants
#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

/// Button interaction states for styling
#[derive(Clone, Copy, Debug)]
pub enum ButtonState {
    Enabled,
    Hover,
    Active,
    Disabled,
}

/// Internal styling configuration for a button variant in a given state
struct ButtonStyle {
    bg: Color,
    text: Color,
    border_color: Option<Color>,
    border_width: f32,
}

impl ButtonVariant {
    /// Get the style configuration for this variant in the given state
    pub fn style(&self, state: ButtonState) -> ButtonStyle {
        match (self, state) {
            // Primary - blue tones
            (Self::Primary, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.25, 0.45, 0.85),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.20, 0.38, 0.75),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.15, 0.30, 0.65),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.25, 0.45, 0.85, 0.5),
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border_color: None,
                border_width: 0.0,
            },

            // Secondary - gray tones
            (Self::Secondary, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.90, 0.90, 0.91),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.85, 0.85, 0.86),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.78, 0.78, 0.80),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.90, 0.90, 0.91, 0.5),
                text: Color::rgba(0.1, 0.1, 0.1, 0.4),
                border_color: None,
                border_width: 0.0,
            },

            // Ghost - transparent with hover
            (Self::Ghost, ButtonState::Enabled) => ButtonStyle {
                bg: Color::transparent(),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgba(1.0, 1.0, 1.0, 0.1),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Active) => ButtonStyle {
                bg: Color::rgba(1.0, 1.0, 1.0, 0.2),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Disabled) => ButtonStyle {
                bg: Color::transparent(),
                text: Color::rgba(0.9, 0.9, 0.9, 0.4),
                border_color: None,
                border_width: 0.0,
            },

            // Destructive - red tones
            (Self::Destructive, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.85, 0.20, 0.20),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.75, 0.15, 0.15),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.65, 0.10, 0.10),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.85, 0.20, 0.20, 0.5),
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border_color: None,
                border_width: 0.0,
            },
        }
    }
}
