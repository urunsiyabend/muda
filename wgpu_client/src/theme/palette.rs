//! Color palettes and theme definitions.

use super::Color;

/// Complete color palette for editor rendering.
#[derive(Clone, Debug)]
pub struct Palette {
    // =========================================================================
    // Editor chrome
    // =========================================================================
    /// Main background color.
    pub bg: Color,
    /// Default foreground (text) color.
    pub fg: Color,
    /// Gutter (line numbers area) background.
    pub gutter_bg: Color,
    /// Status bar background.
    pub status_bar_bg: Color,
    /// Status bar foreground.
    pub status_bar_fg: Color,
    /// Sidebar background.
    pub sidebar_bg: Color,
    /// Sidebar foreground.
    pub sidebar_fg: Color,
    /// Border color.
    pub border: Color,

    // =========================================================================
    // Cursor & Selection
    // =========================================================================
    /// Caret (cursor) color.
    pub caret: Color,
    /// Selection background.
    pub selection_bg: Color,
    /// Selection foreground.
    pub selection_fg: Color,
    /// Current line highlight.
    pub current_line_bg: Color,

    // =========================================================================
    // Line numbers
    // =========================================================================
    /// Normal line number color.
    pub line_number: Color,
    /// Current line number color.
    pub current_line_number: Color,

    // =========================================================================
    // Syntax highlighting
    // =========================================================================
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub type_name: Color,
    pub function: Color,
    pub variable: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub constant: Color,
    pub module: Color,
    pub attribute: Color,
    pub macro_name: Color,

    // =========================================================================
    // Diagnostics
    // =========================================================================
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub hint: Color,

    // =========================================================================
    // UI accents
    // =========================================================================
    /// Primary accent color (buttons, focus).
    pub accent: Color,
    /// Hover state.
    pub hover: Color,
    /// Active/pressed state.
    pub active: Color,
}

/// Complete theme configuration.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
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
            palette: Palette::dark(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.5,
        }
    }

    /// Returns the character width in pixels for monospace text.
    pub fn char_width(&self) -> f32 {
        self.font_size * Self::CHAR_WIDTH_RATIO
    }

    /// Creates a light theme.
    pub fn light() -> Self {
        Self {
            name: "Muda Light".to_string(),
            palette: Palette::light(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.5,
        }
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

impl Palette {
    /// Dark theme palette (VS Code / One Dark inspired).
    pub fn dark() -> Self {
        Self {
            // Editor chrome
            bg: Color::from_hex(0x1E1E1E),
            fg: Color::from_hex(0xD4D4D4),
            gutter_bg: Color::from_hex(0x1E1E1E),
            status_bar_bg: Color::from_hex(0x007ACC),
            status_bar_fg: Color::from_hex(0xFFFFFF),
            sidebar_bg: Color::from_hex(0x252526),
            sidebar_fg: Color::from_hex(0xCCCCCC),
            border: Color::from_hex(0x3C3C3C),

            // Cursor & Selection
            caret: Color::from_hex(0xAEAFAD),
            selection_bg: Color::from_hex(0x264F78),
            selection_fg: Color::from_hex(0xFFFFFF),
            current_line_bg: Color::from_hex(0x2A2D2E),

            // Line numbers
            line_number: Color::from_hex(0x858585),
            current_line_number: Color::from_hex(0xC6C6C6),

            // Syntax highlighting (One Dark inspired)
            keyword: Color::from_hex(0xC586C0),      // Purple
            string: Color::from_hex(0xCE9178),       // Orange
            number: Color::from_hex(0xB5CEA8),       // Light green
            comment: Color::from_hex(0x6A9955),      // Green
            type_name: Color::from_hex(0x4EC9B0),    // Cyan
            function: Color::from_hex(0xDCDCAA),     // Yellow
            variable: Color::from_hex(0x9CDCFE),     // Light blue
            operator: Color::from_hex(0xD4D4D4),     // White
            punctuation: Color::from_hex(0x808080),  // Gray
            constant: Color::from_hex(0x4FC1FF),     // Blue
            module: Color::from_hex(0x4EC9B0),       // Cyan
            attribute: Color::from_hex(0x9CDCFE),    // Light blue
            macro_name: Color::from_hex(0x569CD6),   // Blue

            // Diagnostics
            error: Color::from_hex(0xF44747),
            warning: Color::from_hex(0xCCA700),
            info: Color::from_hex(0x3794FF),
            hint: Color::from_hex(0x6A9955),

            // UI accents
            accent: Color::from_hex(0x007ACC),
            hover: Color::from_hex(0x2A2D2E),
            active: Color::from_hex(0x094771),
        }
    }

    /// Light theme palette.
    pub fn light() -> Self {
        Self {
            // Editor chrome
            bg: Color::from_hex(0xFFFFFF),
            fg: Color::from_hex(0x333333),
            gutter_bg: Color::from_hex(0xF3F3F3),
            status_bar_bg: Color::from_hex(0x007ACC),
            status_bar_fg: Color::from_hex(0xFFFFFF),
            sidebar_bg: Color::from_hex(0xF3F3F3),
            sidebar_fg: Color::from_hex(0x333333),
            border: Color::from_hex(0xE0E0E0),

            // Cursor & Selection
            caret: Color::from_hex(0x000000),
            selection_bg: Color::from_hex(0xADD6FF),
            selection_fg: Color::from_hex(0x000000),
            current_line_bg: Color::from_hex(0xFFFBDD),

            // Line numbers
            line_number: Color::from_hex(0x999999),
            current_line_number: Color::from_hex(0x333333),

            // Syntax highlighting
            keyword: Color::from_hex(0x0000FF),      // Blue
            string: Color::from_hex(0xA31515),       // Red
            number: Color::from_hex(0x098658),       // Green
            comment: Color::from_hex(0x008000),      // Green
            type_name: Color::from_hex(0x267F99),    // Teal
            function: Color::from_hex(0x795E26),     // Brown
            variable: Color::from_hex(0x001080),     // Dark blue
            operator: Color::from_hex(0x000000),     // Black
            punctuation: Color::from_hex(0x000000),  // Black
            constant: Color::from_hex(0x0070C1),     // Blue
            module: Color::from_hex(0x267F99),       // Teal
            attribute: Color::from_hex(0x795E26),    // Brown
            macro_name: Color::from_hex(0x0000FF),   // Blue

            // Diagnostics
            error: Color::from_hex(0xE51400),
            warning: Color::from_hex(0xBF8803),
            info: Color::from_hex(0x1A85FF),
            hint: Color::from_hex(0x008000),

            // UI accents
            accent: Color::from_hex(0x007ACC),
            hover: Color::from_hex(0xE8E8E8),
            active: Color::from_hex(0xCCEAFF),
        }
    }
}
