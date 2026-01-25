//! Text block primitive with alignment and truncation.
//!
//! Provides a high-level abstraction for text rendering with support for
//! alignment, truncation, and basic text styling.

use crate::theme::Color;
use crate::components::Bounds;

/// Text horizontal alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Align text to the left (start)
    #[default]
    Left,
    /// Center text horizontally
    Center,
    /// Align text to the right (end)
    Right,
}

/// Text overflow behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    /// Clip text at bounds (no indicator)
    #[default]
    Clip,
    /// Add ellipsis (...) when text is truncated
    Ellipsis,
    /// Allow text to overflow bounds
    Visible,
}

/// A text block with alignment and styling options.
#[derive(Clone, Debug)]
pub struct TextBlock {
    /// The text content
    pub text: String,
    /// Bounds for the text block
    pub bounds: Bounds,
    /// Text color
    pub color: Color,
    /// Font size in pixels
    pub font_size: f32,
    /// Horizontal alignment
    pub align: TextAlign,
    /// Overflow behavior
    pub overflow: TextOverflow,
    /// Line height multiplier
    pub line_height: f32,
    /// Whether text is bold
    pub bold: bool,
    /// Whether text is italic
    pub italic: bool,
}

impl TextBlock {
    /// Create a new text block.
    pub fn new(text: impl Into<String>, bounds: Bounds) -> Self {
        Self {
            text: text.into(),
            bounds,
            color: Color::WHITE,
            font_size: 14.0,
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            line_height: 1.5,
            bold: false,
            italic: false,
        }
    }

    /// Set the text color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the font size.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the horizontal alignment.
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set the overflow behavior.
    pub fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Set the line height multiplier.
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    /// Set bold style.
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Set italic style.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Calculate the X position for text based on alignment and measured width.
    pub fn aligned_x(&self, measured_width: f32) -> f32 {
        match self.align {
            TextAlign::Left => self.bounds.x,
            TextAlign::Center => self.bounds.x + (self.bounds.width - measured_width) / 2.0,
            TextAlign::Right => self.bounds.x + self.bounds.width - measured_width,
        }
    }

    /// Calculate the line height in pixels.
    pub fn line_height_px(&self) -> f32 {
        self.font_size * self.line_height
    }

    /// Truncate text to fit within bounds (for Ellipsis mode).
    ///
    /// `char_width` is the average character width in the current font.
    /// Returns the truncated text with ellipsis if needed.
    pub fn truncated_text(&self, char_width: f32) -> String {
        if self.overflow != TextOverflow::Ellipsis {
            return self.text.clone();
        }

        let max_chars = (self.bounds.width / char_width) as usize;
        if self.text.len() <= max_chars {
            return self.text.clone();
        }

        if max_chars <= 3 {
            return "...".to_string();
        }

        let truncate_at = max_chars - 3;
        let mut result: String = self.text.chars().take(truncate_at).collect();
        result.push_str("...");
        result
    }
}

impl Default for TextBlock {
    fn default() -> Self {
        Self {
            text: String::new(),
            bounds: Bounds::default(),
            color: Color::WHITE,
            font_size: 14.0,
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            line_height: 1.5,
            bold: false,
            italic: false,
        }
    }
}
