//! Semantic color roles and color palette.
//!
//! Instead of using raw colors, components use semantic color roles that
//! describe the purpose of the color, making theming consistent and intuitive.

use crate::theme::Color;

/// Semantic color roles that describe the purpose of a color.
///
/// Using roles instead of raw colors ensures consistency and makes
/// theming straightforward - just remap the roles to new colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // =========================================================================
    // Backgrounds
    // =========================================================================
    /// Primary background (main editor area)
    BgPrimary,
    /// Secondary background (sidebars, panels)
    BgSecondary,
    /// Tertiary background (nested panels, cards)
    BgTertiary,
    /// Elevated background (dropdowns, tooltips, modals)
    BgElevated,
    /// Overlay background (modal backdrop)
    BgOverlay,

    // =========================================================================
    // Foregrounds
    // =========================================================================
    /// Primary text color
    FgPrimary,
    /// Secondary text color (less emphasis)
    FgSecondary,
    /// Muted text (placeholders, disabled)
    FgMuted,
    /// Inverse text (on accent backgrounds)
    FgInverse,

    // =========================================================================
    // Interactive Elements
    // =========================================================================
    /// Default interactive element (buttons, links)
    Interactive,
    /// Hovered interactive element
    InteractiveHover,
    /// Pressed/active interactive element
    InteractivePressed,
    /// Disabled interactive element
    InteractiveDisabled,

    // =========================================================================
    // Accents
    // =========================================================================
    /// Primary accent (brand color, primary buttons)
    AccentPrimary,
    /// Secondary accent (secondary actions)
    AccentSecondary,

    // =========================================================================
    // Semantic Colors
    // =========================================================================
    /// Success state
    Success,
    /// Warning state
    Warning,
    /// Error state
    Error,
    /// Informational state
    Info,

    // =========================================================================
    // Selection & Focus
    // =========================================================================
    /// Text selection background
    Selection,
    /// Focus ring/outline
    Focus,

    // =========================================================================
    // Borders
    // =========================================================================
    /// Default border
    BorderDefault,
    /// Subtle border (dividers)
    BorderSubtle,
    /// Focus border
    BorderFocus,
}

/// Complete semantic color palette that maps roles to actual colors.
#[derive(Clone, Debug)]
pub struct ColorPalette {
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub bg_elevated: Color,
    pub bg_overlay: Color,

    // Foregrounds
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub fg_inverse: Color,

    // Interactive
    pub interactive: Color,
    pub interactive_hover: Color,
    pub interactive_pressed: Color,
    pub interactive_disabled: Color,

    // Accents
    pub accent_primary: Color,
    pub accent_secondary: Color,

    // Semantic
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Selection & Focus
    pub selection: Color,
    pub focus: Color,

    // Borders
    pub border_default: Color,
    pub border_subtle: Color,
    pub border_focus: Color,
}

impl ColorPalette {
    /// Get the color for a specific role.
    pub fn get(&self, role: ColorRole) -> Color {
        match role {
            ColorRole::BgPrimary => self.bg_primary,
            ColorRole::BgSecondary => self.bg_secondary,
            ColorRole::BgTertiary => self.bg_tertiary,
            ColorRole::BgElevated => self.bg_elevated,
            ColorRole::BgOverlay => self.bg_overlay,

            ColorRole::FgPrimary => self.fg_primary,
            ColorRole::FgSecondary => self.fg_secondary,
            ColorRole::FgMuted => self.fg_muted,
            ColorRole::FgInverse => self.fg_inverse,

            ColorRole::Interactive => self.interactive,
            ColorRole::InteractiveHover => self.interactive_hover,
            ColorRole::InteractivePressed => self.interactive_pressed,
            ColorRole::InteractiveDisabled => self.interactive_disabled,

            ColorRole::AccentPrimary => self.accent_primary,
            ColorRole::AccentSecondary => self.accent_secondary,

            ColorRole::Success => self.success,
            ColorRole::Warning => self.warning,
            ColorRole::Error => self.error,
            ColorRole::Info => self.info,

            ColorRole::Selection => self.selection,
            ColorRole::Focus => self.focus,

            ColorRole::BorderDefault => self.border_default,
            ColorRole::BorderSubtle => self.border_subtle,
            ColorRole::BorderFocus => self.border_focus,
        }
    }

    /// Creates the default dark theme palette.
    pub fn dark() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::from_hex(0x1E1E1E),
            bg_secondary: Color::from_hex(0x252526),
            bg_tertiary: Color::from_hex(0x2D2D2D),
            bg_elevated: Color::from_hex(0x3C3C3C),
            bg_overlay: Color::new(0.0, 0.0, 0.0, 0.5),

            // Foregrounds
            fg_primary: Color::from_hex(0xD4D4D4),
            fg_secondary: Color::from_hex(0xA0A0A0),
            fg_muted: Color::from_hex(0x6E6E6E),
            fg_inverse: Color::from_hex(0xFFFFFF),

            // Interactive (button-like elements)
            interactive: Color::from_hex(0x3C3C3C),
            interactive_hover: Color::from_hex(0x4A4A4A),
            interactive_pressed: Color::from_hex(0x5A5A5A),
            interactive_disabled: Color::from_hex(0x2D2D2D),

            // Accents
            accent_primary: Color::from_hex(0x007ACC),
            accent_secondary: Color::from_hex(0x0098FF),

            // Semantic
            success: Color::from_hex(0x4EC9B0),
            warning: Color::from_hex(0xCCA700),
            error: Color::from_hex(0xF44747),
            info: Color::from_hex(0x3794FF),

            // Selection & Focus
            selection: Color::from_hex(0x264F78),
            focus: Color::from_hex(0x007ACC),

            // Borders
            border_default: Color::from_hex(0x3C3C3C),
            border_subtle: Color::from_hex(0x2D2D2D),
            border_focus: Color::from_hex(0x007ACC),
        }
    }

    /// Creates a light theme palette.
    pub fn light() -> Self {
        Self {
            // Backgrounds
            bg_primary: Color::from_hex(0xFFFFFF),
            bg_secondary: Color::from_hex(0xF3F3F3),
            bg_tertiary: Color::from_hex(0xE8E8E8),
            bg_elevated: Color::from_hex(0xFFFFFF),
            bg_overlay: Color::new(0.0, 0.0, 0.0, 0.3),

            // Foregrounds
            fg_primary: Color::from_hex(0x333333),
            fg_secondary: Color::from_hex(0x666666),
            fg_muted: Color::from_hex(0x999999),
            fg_inverse: Color::from_hex(0xFFFFFF),

            // Interactive
            interactive: Color::from_hex(0xE8E8E8),
            interactive_hover: Color::from_hex(0xDDDDDD),
            interactive_pressed: Color::from_hex(0xCCCCCC),
            interactive_disabled: Color::from_hex(0xF3F3F3),

            // Accents
            accent_primary: Color::from_hex(0x007ACC),
            accent_secondary: Color::from_hex(0x0066B8),

            // Semantic
            success: Color::from_hex(0x388A34),
            warning: Color::from_hex(0xBF8803),
            error: Color::from_hex(0xE51400),
            info: Color::from_hex(0x1A85FF),

            // Selection & Focus
            selection: Color::from_hex(0xADD6FF),
            focus: Color::from_hex(0x007ACC),

            // Borders
            border_default: Color::from_hex(0xE0E0E0),
            border_subtle: Color::from_hex(0xEEEEEE),
            border_focus: Color::from_hex(0x007ACC),
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}

// Extension methods for Color
impl Color {
    /// Blend this color with another using alpha compositing.
    pub fn blend_over(self, background: Color) -> Color {
        let alpha = self.a;
        let inv_alpha = 1.0 - alpha;

        Color {
            r: self.r * alpha + background.r * inv_alpha,
            g: self.g * alpha + background.g * inv_alpha,
            b: self.b * alpha + background.b * inv_alpha,
            a: alpha + background.a * inv_alpha,
        }
    }

    /// Lighten the color by a factor (0.0 = no change, 1.0 = white).
    pub fn lighten(self, factor: f32) -> Color {
        Color {
            r: self.r + (1.0 - self.r) * factor,
            g: self.g + (1.0 - self.g) * factor,
            b: self.b + (1.0 - self.b) * factor,
            a: self.a,
        }
    }

    /// Darken the color by a factor (0.0 = no change, 1.0 = black).
    pub fn darken(self, factor: f32) -> Color {
        Color {
            r: self.r * (1.0 - factor),
            g: self.g * (1.0 - factor),
            b: self.b * (1.0 - factor),
            a: self.a,
        }
    }

    /// Interpolate between two colors.
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}
