//! Theme system for GPU rendering.
//!
//! Provides a consistent, configurable color palette and style mapping
//! from semantic `TextStyle` tokens to concrete GPU rendering attributes.

mod colors;
mod palette;

pub use colors::Color;
pub use palette::Theme;

#[allow(unused)]
pub use palette::Palette;

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
        TextStyle::Normal => GpuStyle::new(p.fg),

        // Selection
        TextStyle::Selection => GpuStyle::new(p.selection_fg).with_bg(p.selection_bg),

        // Syntax highlighting
        TextStyle::Keyword => GpuStyle::new(p.keyword).bold(),
        TextStyle::String => GpuStyle::new(p.string),
        TextStyle::Number => GpuStyle::new(p.number),
        TextStyle::Comment => GpuStyle::new(p.comment).italic(),
        TextStyle::Type => GpuStyle::new(p.type_name),
        TextStyle::Function => GpuStyle::new(p.function),
        TextStyle::Variable => GpuStyle::new(p.variable),
        TextStyle::Operator => GpuStyle::new(p.operator),
        TextStyle::Punctuation => GpuStyle::new(p.punctuation),
        TextStyle::Constant => GpuStyle::new(p.constant).bold(),
        TextStyle::Module => GpuStyle::new(p.module),
        TextStyle::Attribute => GpuStyle::new(p.attribute),
        TextStyle::Macro => GpuStyle::new(p.macro_name),

        // UI elements
        TextStyle::LineNumber => GpuStyle::new(p.line_number),
        TextStyle::CurrentLineNumber => GpuStyle::new(p.current_line_number).bold(),
        TextStyle::Error => GpuStyle::new(p.error).bold().underline(),
        TextStyle::Warning => GpuStyle::new(p.warning).underline(),
    }
}
