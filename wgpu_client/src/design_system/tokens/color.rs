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
    /// Hint/suggestion state
    Hint,

    // =========================================================================
    // Selection & Focus
    // =========================================================================
    /// Text selection background
    Selection,
    /// Text selection foreground
    SelectionFg,
    /// Focus ring/outline
    Focus,
    /// Current line highlight background
    CurrentLine,

    // =========================================================================
    // Borders
    // =========================================================================
    /// Default border
    BorderDefault,
    /// Subtle border (dividers)
    BorderSubtle,
    /// Focus border
    BorderFocus,

    // =========================================================================
    // Editor Chrome
    // =========================================================================
    /// Caret (cursor) color
    Caret,
    /// Gutter background
    GutterBg,
    /// Line number color
    LineNumber,
    /// Current line number color
    CurrentLineNumber,
    /// Status bar background
    StatusBarBg,
    /// Status bar foreground
    StatusBarFg,
    /// Sidebar background
    SidebarBg,
    /// Sidebar foreground
    SidebarFg,

    // =========================================================================
    // Syntax Highlighting
    // =========================================================================
    /// Keywords (if, else, fn, etc.)
    SyntaxKeyword,
    /// String literals
    SyntaxString,
    /// Numeric literals
    SyntaxNumber,
    /// Comments
    SyntaxComment,
    /// Type names
    SyntaxType,
    /// Function names
    SyntaxFunction,
    /// Variable names
    SyntaxVariable,
    /// Operators (+, -, *, etc.)
    SyntaxOperator,
    /// Punctuation (braces, parens, etc.)
    SyntaxPunctuation,
    /// Constants
    SyntaxConstant,
    /// Module/namespace names
    SyntaxModule,
    /// Attributes/decorators
    SyntaxAttribute,
    /// Macro names
    SyntaxMacro,
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
    pub hint: Color,

    // Selection & Focus
    pub selection: Color,
    pub selection_fg: Color,
    pub focus: Color,
    pub current_line: Color,

    // Borders
    pub border_default: Color,
    pub border_subtle: Color,
    pub border_focus: Color,

    // Editor Chrome
    pub caret: Color,
    pub gutter_bg: Color,
    pub line_number: Color,
    pub current_line_number: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub sidebar_bg: Color,
    pub sidebar_fg: Color,

    // Syntax Highlighting
    pub syntax_keyword: Color,
    pub syntax_string: Color,
    pub syntax_number: Color,
    pub syntax_comment: Color,
    pub syntax_type: Color,
    pub syntax_function: Color,
    pub syntax_variable: Color,
    pub syntax_operator: Color,
    pub syntax_punctuation: Color,
    pub syntax_constant: Color,
    pub syntax_module: Color,
    pub syntax_attribute: Color,
    pub syntax_macro: Color,
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
            ColorRole::Hint => self.hint,

            ColorRole::Selection => self.selection,
            ColorRole::SelectionFg => self.selection_fg,
            ColorRole::Focus => self.focus,
            ColorRole::CurrentLine => self.current_line,

            ColorRole::BorderDefault => self.border_default,
            ColorRole::BorderSubtle => self.border_subtle,
            ColorRole::BorderFocus => self.border_focus,

            // Editor Chrome
            ColorRole::Caret => self.caret,
            ColorRole::GutterBg => self.gutter_bg,
            ColorRole::LineNumber => self.line_number,
            ColorRole::CurrentLineNumber => self.current_line_number,
            ColorRole::StatusBarBg => self.status_bar_bg,
            ColorRole::StatusBarFg => self.status_bar_fg,
            ColorRole::SidebarBg => self.sidebar_bg,
            ColorRole::SidebarFg => self.sidebar_fg,

            // Syntax Highlighting
            ColorRole::SyntaxKeyword => self.syntax_keyword,
            ColorRole::SyntaxString => self.syntax_string,
            ColorRole::SyntaxNumber => self.syntax_number,
            ColorRole::SyntaxComment => self.syntax_comment,
            ColorRole::SyntaxType => self.syntax_type,
            ColorRole::SyntaxFunction => self.syntax_function,
            ColorRole::SyntaxVariable => self.syntax_variable,
            ColorRole::SyntaxOperator => self.syntax_operator,
            ColorRole::SyntaxPunctuation => self.syntax_punctuation,
            ColorRole::SyntaxConstant => self.syntax_constant,
            ColorRole::SyntaxModule => self.syntax_module,
            ColorRole::SyntaxAttribute => self.syntax_attribute,
            ColorRole::SyntaxMacro => self.syntax_macro,
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
            hint: Color::from_hex(0x6A9955),

            // Selection & Focus
            selection: Color::from_hex(0x264F78),
            selection_fg: Color::from_hex(0xFFFFFF),
            focus: Color::from_hex(0x007ACC),
            current_line: Color::from_hex(0x2A2D2E),

            // Borders
            border_default: Color::from_hex(0x3C3C3C),
            border_subtle: Color::from_hex(0x2D2D2D),
            border_focus: Color::from_hex(0x007ACC),

            // Editor Chrome
            caret: Color::from_hex(0xAEAFAD),
            gutter_bg: Color::from_hex(0x1E1E1E),
            line_number: Color::from_hex(0x858585),
            current_line_number: Color::from_hex(0xC6C6C6),
            status_bar_bg: Color::from_hex(0x007ACC),
            status_bar_fg: Color::from_hex(0xFFFFFF),
            sidebar_bg: Color::from_hex(0x252526),
            sidebar_fg: Color::from_hex(0xCCCCCC),

            // Syntax Highlighting (One Dark / VS Code inspired)
            syntax_keyword: Color::from_hex(0xC586C0),      // Purple
            syntax_string: Color::from_hex(0xCE9178),       // Orange
            syntax_number: Color::from_hex(0xB5CEA8),       // Light green
            syntax_comment: Color::from_hex(0x6A9955),      // Green
            syntax_type: Color::from_hex(0x4EC9B0),         // Cyan
            syntax_function: Color::from_hex(0xDCDCAA),     // Yellow
            syntax_variable: Color::from_hex(0x9CDCFE),     // Light blue
            syntax_operator: Color::from_hex(0xD4D4D4),     // White
            syntax_punctuation: Color::from_hex(0x808080),  // Gray
            syntax_constant: Color::from_hex(0x4FC1FF),     // Blue
            syntax_module: Color::from_hex(0x4EC9B0),       // Cyan
            syntax_attribute: Color::from_hex(0x9CDCFE),    // Light blue
            syntax_macro: Color::from_hex(0x569CD6),        // Blue
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
            hint: Color::from_hex(0x008000),

            // Selection & Focus
            selection: Color::from_hex(0xADD6FF),
            selection_fg: Color::from_hex(0x000000),
            focus: Color::from_hex(0x007ACC),
            current_line: Color::from_hex(0xFFFBDD),

            // Borders
            border_default: Color::from_hex(0xE0E0E0),
            border_subtle: Color::from_hex(0xEEEEEE),
            border_focus: Color::from_hex(0x007ACC),

            // Editor Chrome
            caret: Color::from_hex(0x000000),
            gutter_bg: Color::from_hex(0xF3F3F3),
            line_number: Color::from_hex(0x999999),
            current_line_number: Color::from_hex(0x333333),
            status_bar_bg: Color::from_hex(0x007ACC),
            status_bar_fg: Color::from_hex(0xFFFFFF),
            sidebar_bg: Color::from_hex(0xF3F3F3),
            sidebar_fg: Color::from_hex(0x333333),

            // Syntax Highlighting
            syntax_keyword: Color::from_hex(0x0000FF),      // Blue
            syntax_string: Color::from_hex(0xA31515),       // Red
            syntax_number: Color::from_hex(0x098658),       // Green
            syntax_comment: Color::from_hex(0x008000),      // Green
            syntax_type: Color::from_hex(0x267F99),         // Teal
            syntax_function: Color::from_hex(0x795E26),     // Brown
            syntax_variable: Color::from_hex(0x001080),     // Dark blue
            syntax_operator: Color::from_hex(0x000000),     // Black
            syntax_punctuation: Color::from_hex(0x000000),  // Black
            syntax_constant: Color::from_hex(0x0070C1),     // Blue
            syntax_module: Color::from_hex(0x267F99),       // Teal
            syntax_attribute: Color::from_hex(0x795E26),    // Brown
            syntax_macro: Color::from_hex(0x0000FF),        // Blue
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
