//! Status bar view showing editor status information.
//!
//! Renders a horizontal bar at the bottom of the editor showing:
//! - Cursor position (line and column)
//! - Language mode
//! - File encoding
//! - Total lines
//! - Optional status messages
//!
//! Consumes `StatusPresentation` from core_editor for status data.

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::style::{px, pct, JustifyContent};
use crate::theme::ColorToken;
use crate::view::View;
use core_editor::view_model::StatusPresentation;

/// Minimum status bar height in logical pixels (content + padding determines actual).
pub const STATUS_BAR_MIN_HEIGHT: f32 = 28.0;

/// Font size for status bar text (readable size).
const STATUS_FONT_SIZE: f32 = 13.0;

/// Horizontal padding for status bar.
const STATUS_PADDING_H: f32 = 16.0;

/// Vertical padding for status bar.
const STATUS_PADDING_V: f32 = 8.0;

/// Separator character for status sections.
const SEPARATOR: &str = " | ";

/// Status bar view for displaying editor status information.
///
/// Consumes `StatusPresentation` from core_editor and renders
/// a horizontal bar with:
/// - Left section: cursor position "Ln {line}, Col {column}"
/// - Right section: language, encoding, total lines (separated by " | ")
/// - Optional message overlay (when status.message is Some)
///
/// # Example
///
/// ```ignore
/// let status_bar = StatusBarView::new(presentation);
/// // In a parent view's render():
/// Div::new().child(status_bar.render(cx))
/// ```
pub struct StatusBarView {
    /// The presentation data for the status bar.
    presentation: StatusPresentation,
}

impl StatusBarView {
    /// Creates a new status bar view with the given presentation data.
    pub fn new(presentation: StatusPresentation) -> Self {
        Self { presentation }
    }

    /// Updates the presentation data.
    pub fn set_presentation(&mut self, presentation: StatusPresentation) {
        self.presentation = presentation;
    }

    /// Renders the left section (cursor position).
    fn render_left_section(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();
        let text = format!(
            "Ln {}, Col {}",
            self.presentation.cursor_line,
            self.presentation.cursor_column
        );

        Div::new()
            .flex_row()
            .align_center()
            .child(
                TextElement::new(&text)
                    .size(STATUS_FONT_SIZE)
                    .color(theme.color(ColorToken::FgSecondary))
            )
    }

    /// Renders the right section (language, encoding, total lines).
    fn render_right_section(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();
        let text = format!(
            "{}{}{}{}{}",
            self.presentation.language,
            SEPARATOR,
            "UTF-8",  // encoding - StatusPresentation doesn't have encoding field, using default
            SEPARATOR,
            format!("{} lines", self.presentation.total_lines)
        );

        Div::new()
            .flex_row()
            .align_center()
            .child(
                TextElement::new(&text)
                    .size(STATUS_FONT_SIZE)
                    .color(theme.color(ColorToken::FgSecondary))
            )
    }

    /// Renders a status message (when present, replaces normal content).
    fn render_message(&self, message: &str, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        Div::new()
            .flex_row()
            .align_center()
            .child(
                TextElement::new(message)
                    .size(STATUS_FONT_SIZE)
                    .color(theme.color(ColorToken::FgPrimary))
            )
    }
}

impl View for StatusBarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Build content based on whether there's a message
        // (mutably borrows cx, so must complete before getting theme)
        let content = if let Some(ref message) = self.presentation.message {
            // Show message instead of normal status
            self.render_message(message, cx)
        } else {
            // Build left section first (cursor position)
            let left = self.render_left_section(cx);
            // Build right section (language, encoding, lines)
            let right = self.render_right_section(cx);

            // Container with space-between justification
            Div::new()
                .flex_row()
                .w(pct(100.0))
                .justify(JustifyContent::SpaceBetween)
                .align_center()
                .child(left)
                .child(right)
        };

        // Now get theme for container styling
        let theme = cx.theme();

        // Build the status bar container (min_h + padding, overflow hidden to prevent text overflow)
        Div::new()
            .flex_row()
            .w(pct(100.0))
            .min_h(px(STATUS_BAR_MIN_HEIGHT))
            .bg(theme.color(ColorToken::BgSecondary))
            .px(STATUS_PADDING_H)
            .py(STATUS_PADDING_V)
            .align_center()
            .overflow_hidden()
            .child(content)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_editor::view_model::StatusPresentation;

    #[test]
    fn test_status_bar_view_creation() {
        let presentation = StatusPresentation {
            title: "main.rs".to_string(),
            dirty: false,
            cursor_line: 42,
            cursor_column: 15,
            language: "Rust".to_string(),
            total_lines: 150,
            message: None,
        };

        let view = StatusBarView::new(presentation.clone());
        assert_eq!(view.presentation.cursor_line, 42);
        assert_eq!(view.presentation.cursor_column, 15);
        assert_eq!(view.presentation.language, "Rust");
        assert_eq!(view.presentation.total_lines, 150);
    }

    #[test]
    fn test_status_bar_view_with_message() {
        let presentation = StatusPresentation {
            title: "main.rs".to_string(),
            dirty: false,
            cursor_line: 1,
            cursor_column: 1,
            language: "Rust".to_string(),
            total_lines: 10,
            message: Some("File saved successfully".to_string()),
        };

        let view = StatusBarView::new(presentation.clone());
        assert!(view.presentation.message.is_some());
        assert_eq!(view.presentation.message.as_ref().unwrap(), "File saved successfully");
    }

    #[test]
    fn test_status_bar_constants() {
        assert_eq!(STATUS_BAR_MIN_HEIGHT, 28.0);
        assert_eq!(STATUS_FONT_SIZE, 13.0);
    }

    #[test]
    fn test_status_bar_set_presentation() {
        let mut view = StatusBarView::new(StatusPresentation::default());
        assert_eq!(view.presentation.cursor_line, 0);

        let new_presentation = StatusPresentation {
            title: "test.rs".to_string(),
            dirty: true,
            cursor_line: 100,
            cursor_column: 50,
            language: "Rust".to_string(),
            total_lines: 500,
            message: None,
        };

        view.set_presentation(new_presentation);
        assert_eq!(view.presentation.cursor_line, 100);
        assert_eq!(view.presentation.cursor_column, 50);
    }
}
