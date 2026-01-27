//! Complete theme configuration combining color palette with typography.

use super::ColorPalette;

/// Complete theme configuration for the editor.
///
/// Combines the semantic color palette with typography settings.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Theme name for display.
    pub name: String,
    /// Semantic color palette.
    pub palette: ColorPalette,
    /// Font family for editor text.
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Line height multiplier.
    pub line_height: f32,
}

impl Theme {
    /// Monospace character width ratio (em-width / font-size).
    /// This is tuned for common monospace fonts. The default system monospace font
    /// typically has a ratio around 0.55. Adjust if using a different font.
    pub const CHAR_WIDTH_RATIO: f32 = 0.55;

    /// Creates the default dark theme (VS Code-inspired).
    pub fn dark() -> Self {
        Self {
            name: "Muda Dark".to_string(),
            palette: ColorPalette::dark(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.5,
        }
    }

    /// Creates a light theme.
    pub fn light() -> Self {
        Self {
            name: "Muda Light".to_string(),
            palette: ColorPalette::light(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.5,
        }
    }

    /// Returns the character width in pixels for monospace text.
    pub fn char_width(&self) -> f32 {
        self.font_size * Self::CHAR_WIDTH_RATIO
    }

    /// Calculates the line height in pixels.
    pub fn line_height_px(&self) -> f32 {
        self.font_size * self.line_height
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
