//! Theme system for GPU rendering.
//!
//! Provides a consistent, configurable color palette and style mapping
//! from semantic `TextStyle` tokens to concrete GPU rendering attributes.
//!
//! This module re-exports from the design_system for a unified theming API.

mod colors;

pub use colors::Color;

// Re-export Theme and ColorPalette from design_system
pub use crate::design_system::{Theme, ColorPalette, ColorRole};

use core_editor::view_model::TextStyle;

/// Visual attributes for rendering styled text.
#[derive(Clone, Copy, Debug)]
pub struct GpuStyle {
    /// Foreground (text) color.
    pub fg: Color,
    /// Background color (None = transparent).
    pub bg: Option<Color>,
    /// Whether to render bold.
    pub bold: bool,
    /// Whether to render italic.
    pub italic: bool,
    /// Whether to render underline.
    pub underline: bool,
}

impl GpuStyle {
    pub const fn new(fg: Color) -> Self {
        Self {
            fg,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }

    pub const fn with_bg(mut self, bg: Color) -> Self {
        self.bg = Some(bg);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// Maps semantic `TextStyle` to GPU rendering style using the current theme.
pub fn map_style(style: TextStyle, theme: &Theme) -> GpuStyle {
    let p = &theme.palette;

    match style {
        // Normal text
        TextStyle::Normal => GpuStyle::new(p.get(ColorRole::FgPrimary)),

        // Selection
        TextStyle::Selection => GpuStyle::new(p.get(ColorRole::SelectionFg))
            .with_bg(p.get(ColorRole::Selection)),

        // Syntax highlighting
        TextStyle::Keyword => GpuStyle::new(p.get(ColorRole::SyntaxKeyword)).bold(),
        TextStyle::String => GpuStyle::new(p.get(ColorRole::SyntaxString)),
        TextStyle::Number => GpuStyle::new(p.get(ColorRole::SyntaxNumber)),
        TextStyle::Comment => GpuStyle::new(p.get(ColorRole::SyntaxComment)).italic(),
        TextStyle::Type => GpuStyle::new(p.get(ColorRole::SyntaxType)),
        TextStyle::Function => GpuStyle::new(p.get(ColorRole::SyntaxFunction)),
        TextStyle::Variable => GpuStyle::new(p.get(ColorRole::SyntaxVariable)),
        TextStyle::Operator => GpuStyle::new(p.get(ColorRole::SyntaxOperator)),
        TextStyle::Punctuation => GpuStyle::new(p.get(ColorRole::SyntaxPunctuation)),
        TextStyle::Constant => GpuStyle::new(p.get(ColorRole::SyntaxConstant)).bold(),
        TextStyle::Module => GpuStyle::new(p.get(ColorRole::SyntaxModule)),
        TextStyle::Attribute => GpuStyle::new(p.get(ColorRole::SyntaxAttribute)),
        TextStyle::Macro => GpuStyle::new(p.get(ColorRole::SyntaxMacro)),

        // UI elements
        TextStyle::LineNumber => GpuStyle::new(p.get(ColorRole::LineNumber)),
        TextStyle::CurrentLineNumber => GpuStyle::new(p.get(ColorRole::CurrentLineNumber)).bold(),
        TextStyle::Error => GpuStyle::new(p.get(ColorRole::Error)).bold().underline(),
        TextStyle::Warning => GpuStyle::new(p.get(ColorRole::Warning)).underline(),
    }
}
