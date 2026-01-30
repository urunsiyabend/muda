pub mod color;
pub mod spacing;
pub mod typography;

use crate::style::Color;

// Re-export PaletteColor for external use
pub use color::PaletteColor;

// Re-exports from spacing
pub use spacing::{sp, SpacingToken};

// Re-exports from typography
pub use typography::{TextSize, FontFamily};

/// Theme mode: dark or light
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Semantic color tokens for consistent UI theming
///
/// These tokens map to palette colors based on the current theme mode.
/// Prefer using these semantic tokens over direct palette access for
/// standard UI elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorToken {
    /// Primary background color (e.g., main window background)
    BgPrimary,
    /// Secondary background color (e.g., sidebar, panels)
    BgSecondary,
    /// Elevated background color (e.g., cards, popovers)
    BgElevated,
    /// Primary foreground/text color
    FgPrimary,
    /// Secondary foreground/text color (less prominent)
    FgSecondary,
    /// Muted foreground/text color (hints, placeholders)
    FgMuted,
    /// Accent color for primary actions
    Accent,
    /// Accent color on hover
    AccentHover,
    /// Accent color on active/pressed
    AccentActive,
    /// Border color
    Border,
    /// Error/destructive action color
    Error,
    /// Success/positive action color
    Success,
}

/// Theme configuration for the application
///
/// Provides semantic color tokens that automatically adjust based on
/// the current theme mode (dark or light).
pub struct Theme {
    mode: ThemeMode,
}

impl Theme {
    /// Create a dark theme
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
        }
    }

    /// Get the current theme mode
    pub fn mode(&self) -> ThemeMode {
        self.mode
    }

    /// Get a color for a semantic token
    ///
    /// This is the primary API for accessing colors. It returns the
    /// appropriate color based on the current theme mode.
    pub fn color(&self, token: ColorToken) -> Color {
        use color::*;

        match (self.mode, token) {
            // Dark mode mappings
            (ThemeMode::Dark, ColorToken::BgPrimary) => gray_900(),
            (ThemeMode::Dark, ColorToken::BgSecondary) => gray_800(),
            (ThemeMode::Dark, ColorToken::BgElevated) => gray_700(),
            (ThemeMode::Dark, ColorToken::FgPrimary) => gray_50(),
            (ThemeMode::Dark, ColorToken::FgSecondary) => gray_300(),
            (ThemeMode::Dark, ColorToken::FgMuted) => gray_500(),
            (ThemeMode::Dark, ColorToken::Accent) => blue_500(),
            (ThemeMode::Dark, ColorToken::AccentHover) => blue_400(),
            (ThemeMode::Dark, ColorToken::AccentActive) => blue_600(),
            (ThemeMode::Dark, ColorToken::Border) => gray_700(),
            (ThemeMode::Dark, ColorToken::Error) => red_500(),
            (ThemeMode::Dark, ColorToken::Success) => green_500(),

            // Light mode mappings
            (ThemeMode::Light, ColorToken::BgPrimary) => gray_50(),
            (ThemeMode::Light, ColorToken::BgSecondary) => gray_100(),
            (ThemeMode::Light, ColorToken::BgElevated) => Color::rgb(1.0, 1.0, 1.0), // white
            (ThemeMode::Light, ColorToken::FgPrimary) => gray_900(),
            (ThemeMode::Light, ColorToken::FgSecondary) => gray_600(),
            (ThemeMode::Light, ColorToken::FgMuted) => gray_400(),
            (ThemeMode::Light, ColorToken::Accent) => blue_600(),
            (ThemeMode::Light, ColorToken::AccentHover) => blue_500(),
            (ThemeMode::Light, ColorToken::AccentActive) => blue_700(),
            (ThemeMode::Light, ColorToken::Border) => gray_300(),
            (ThemeMode::Light, ColorToken::Error) => red_600(),
            (ThemeMode::Light, ColorToken::Success) => green_600(),
        }
    }

    /// Get a color directly from the palette
    ///
    /// This is an escape hatch for cases where you need direct access
    /// to a specific palette color that doesn't fit semantic tokens.
    /// Prefer using `color()` with semantic tokens when possible.
    pub fn palette(&self, palette: PaletteColor) -> Color {
        color::get(palette)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
