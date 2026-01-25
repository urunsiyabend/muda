//! Status line component at the bottom of the IDE.
//!
//! Displays editor state, cursor position, file info, and quick actions.
//! Uses consistent design tokens for professional appearance.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextAlign, CornerRadii,
    Space, Radius,
    tokens::ColorPalette,
};

/// Status line segment types for different information.
#[derive(Clone, Debug)]
pub enum StatusSegment {
    /// Branch indicator (git)
    Branch(String),
    /// Error/warning count
    Diagnostics { errors: usize, warnings: usize },
    /// Cursor position
    Position { line: usize, column: usize },
    /// File encoding
    Encoding(String),
    /// Line ending (LF/CRLF)
    LineEnding(String),
    /// Language/mode
    Language(String),
    /// Custom text
    Text(String),
    /// Notification/message
    Message { text: String, is_error: bool },
}

/// The status line component.
pub struct StatusLine {
    left_segments: Vec<StatusSegment>,
    right_segments: Vec<StatusSegment>,
    bounds: Bounds,
    palette: ColorPalette,
    message: Option<(String, bool)>, // (text, is_error)
    message_time: Option<std::time::Instant>,
}

impl StatusLine {
    const HEIGHT: f32 = 22.0;
    const PADDING_H: f32 = 10.0;
    const SEGMENT_GAP: f32 = 16.0;
    const CHAR_WIDTH: f32 = 7.0;

    pub fn new() -> Self {
        Self {
            left_segments: vec![
                StatusSegment::Branch("main".into()),
            ],
            right_segments: vec![
                StatusSegment::Position { line: 1, column: 1 },
                StatusSegment::Encoding("UTF-8".into()),
                StatusSegment::LineEnding("LF".into()),
                StatusSegment::Language("Plain Text".into()),
            ],
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            message: None,
            message_time: None,
        }
    }

    pub fn height(&self) -> f32 {
        Self::HEIGHT
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }

    /// Update cursor position.
    pub fn set_position(&mut self, line: usize, column: usize) {
        for segment in &mut self.right_segments {
            if let StatusSegment::Position { line: l, column: c } = segment {
                *l = line;
                *c = column;
                break;
            }
        }
    }

    /// Update language/mode.
    pub fn set_language(&mut self, language: &str) {
        for segment in &mut self.right_segments {
            if let StatusSegment::Language(l) = segment {
                *l = language.to_string();
                break;
            }
        }
    }

    /// Update branch name.
    pub fn set_branch(&mut self, branch: &str) {
        for segment in &mut self.left_segments {
            if let StatusSegment::Branch(b) = segment {
                *b = branch.to_string();
                break;
            }
        }
    }

    /// Set diagnostics count.
    pub fn set_diagnostics(&mut self, errors: usize, warnings: usize) {
        // Find or add diagnostics segment
        let mut found = false;
        for segment in &mut self.left_segments {
            if let StatusSegment::Diagnostics { errors: e, warnings: w } = segment {
                *e = errors;
                *w = warnings;
                found = true;
                break;
            }
        }
        if !found && (errors > 0 || warnings > 0) {
            self.left_segments.push(StatusSegment::Diagnostics { errors, warnings });
        }
    }

    /// Show a temporary message.
    pub fn show_message(&mut self, text: &str, is_error: bool) {
        self.message = Some((text.to_string(), is_error));
        self.message_time = Some(std::time::Instant::now());
    }

    /// Clear message after timeout.
    pub fn update(&mut self) -> bool {
        if let Some(time) = self.message_time {
            if time.elapsed() > std::time::Duration::from_secs(5) {
                self.message = None;
                self.message_time = None;
                return true;
            }
        }
        false
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        let mut rects = Vec::new();

        // Background
        rects.push(
            StyledRect::new(self.bounds)
                .with_fill(self.palette.accent_primary)
        );

        // If there's an error message, show error background
        if let Some((_, true)) = &self.message {
            rects.push(
                StyledRect::new(self.bounds)
                    .with_fill(self.palette.error)
            );
        }

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        let mut texts = Vec::new();

        // If there's a message, show only that
        if let Some((text, is_error)) = &self.message {
            texts.push(
                TextBlock::new(text, Bounds {
                    x: self.bounds.x + Self::PADDING_H,
                    y: self.bounds.y,
                    width: self.bounds.width - Self::PADDING_H * 2.0,
                    height: self.bounds.height,
                })
                .with_color(self.palette.fg_inverse)
                .with_font_size(12.0)
            );
            return texts;
        }

        // Left segments
        let mut x = self.bounds.x + Self::PADDING_H;
        for segment in &self.left_segments {
            let text = self.segment_text(segment);
            let width = text.len() as f32 * Self::CHAR_WIDTH;

            // Segment-specific styling
            let (fg_color, icon) = match segment {
                StatusSegment::Branch(_) => (self.palette.fg_inverse, Some("⎇ ")),
                StatusSegment::Diagnostics { errors, warnings } => {
                    let icon = if *errors > 0 { "✕ " } else if *warnings > 0 { "⚠ " } else { "" };
                    let color = if *errors > 0 { self.palette.error } else { self.palette.warning };
                    (color, Some(icon))
                }
                _ => (self.palette.fg_inverse, None),
            };

            let full_text = if let Some(icon) = icon {
                format!("{}{}", icon, text)
            } else {
                text
            };

            let full_width = full_text.len() as f32 * Self::CHAR_WIDTH;

            texts.push(
                TextBlock::new(&full_text, Bounds {
                    x,
                    y: self.bounds.y,
                    width: full_width,
                    height: self.bounds.height,
                })
                .with_color(fg_color)
                .with_font_size(12.0)
            );

            x += full_width + Self::SEGMENT_GAP;
        }

        // Right segments (right-aligned)
        let mut right_x = self.bounds.x + self.bounds.width - Self::PADDING_H;
        for segment in self.right_segments.iter().rev() {
            let text = self.segment_text(segment);
            let width = text.len() as f32 * Self::CHAR_WIDTH;

            right_x -= width;

            texts.push(
                TextBlock::new(&text, Bounds {
                    x: right_x,
                    y: self.bounds.y,
                    width,
                    height: self.bounds.height,
                })
                .with_color(self.palette.fg_inverse)
                .with_font_size(12.0)
            );

            right_x -= Self::SEGMENT_GAP;
        }

        texts
    }

    fn segment_text(&self, segment: &StatusSegment) -> String {
        match segment {
            StatusSegment::Branch(name) => name.clone(),
            StatusSegment::Diagnostics { errors, warnings } => {
                if *errors > 0 && *warnings > 0 {
                    format!("{} {}", errors, warnings)
                } else if *errors > 0 {
                    errors.to_string()
                } else {
                    warnings.to_string()
                }
            }
            StatusSegment::Position { line, column } => {
                format!("Ln {}, Col {}", line, column)
            }
            StatusSegment::Encoding(enc) => enc.clone(),
            StatusSegment::LineEnding(le) => le.clone(),
            StatusSegment::Language(lang) => lang.clone(),
            StatusSegment::Text(text) => text.clone(),
            StatusSegment::Message { text, .. } => text.clone(),
        }
    }

    /// Handle click - returns segment at position for potential actions.
    pub fn on_click(&self, x: f32, _y: f32) -> Option<&StatusSegment> {
        // Simple hit test - in real implementation would track segment bounds
        if x > self.bounds.x + self.bounds.width - 200.0 {
            // Right side - could click on language, encoding, etc.
            return self.right_segments.first();
        }
        None
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self::new()
    }
}
