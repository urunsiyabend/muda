//! Line number gutter view.
//!
//! Renders a vertical gutter showing line numbers aligned to editor content.
//! Consumes `GutterModel` and `RenderModel` from core_editor for line data.
//!
//! # Architecture
//!
//! The gutter width is calculated dynamically based on the total line count:
//! - LEFT_PADDING + (digit_count * char_width) + separator_padding
//! - This ensures proper alignment as documents grow beyond 10/100/1000 lines
//!
//! # Styling
//!
//! - Current line number: ColorToken::FgPrimary (brighter)
//! - Other line numbers: ColorToken::FgMuted (dimmer)
//! - Background: ColorToken::BgSecondary
//! - Separator: ColorToken::BorderDefault

use crate::context::ViewContext;
use crate::editor_adapter::{GutterModel, LinePresentation};
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::style::px;
use crate::theme::ColorToken;
use crate::view::View;

/// Left padding in pixels before line numbers.
pub const LEFT_PADDING: f32 = 8.0;

/// Right padding in pixels after line numbers (before border).
pub const RIGHT_PADDING: f32 = 12.0;

/// Font size for line numbers (matches editor text).
const LINE_NUMBER_FONT_SIZE: f32 = 14.0;

/// Line height for line number rows (matches editor lines).
const LINE_HEIGHT: f32 = 21.0;

/// Gutter view for displaying line numbers.
///
/// Renders a vertical column showing line numbers with:
/// - Right-aligned numbers within calculated width
/// - Separator character (|) between numbers and editor
/// - Current line highlighting (brighter color)
/// - Dynamic width based on total line count
///
/// # Width Calculation
///
/// Width = LEFT_PADDING + (digit_count * char_width) + SEPARATOR_PADDING
///
/// For a 150-line document with ~8px char width:
/// - digit_count = 3 (for "150")
/// - width = 8 + (3 * 8) + 16 = 48px
///
/// # CONTEXT Decision
///
/// Current line highlight spans gutter AND editor area.
/// The current line background will be handled by parent layout, not GutterView itself.
/// GutterView only changes the text color for the current line number.
///
/// # Example
///
/// ```ignore
/// let gutter = GutterView::new(gutter_model, visible_lines);
/// // Get width for layout calculations:
/// let width = gutter.calculate_width(8.0); // assuming 8px char width
/// // In a parent view's render():
/// Div::new().child(gutter.render(cx))
/// ```
pub struct GutterView {
    /// The gutter model from core_editor.
    gutter: GutterModel,
    /// The visible lines to display.
    visible_lines: Vec<LinePresentation>,
    /// Monospace character width for calculations (default 8.0).
    char_width: f32,
}

impl GutterView {
    /// Creates a new gutter view with the given model and visible lines.
    pub fn new(gutter: GutterModel, visible_lines: Vec<LinePresentation>) -> Self {
        Self {
            gutter,
            visible_lines,
            char_width: crate::rendering::measured_char_width(),
        }
    }

    /// Creates a gutter view with a specific character width.
    pub fn with_char_width(
        gutter: GutterModel,
        visible_lines: Vec<LinePresentation>,
        char_width: f32,
    ) -> Self {
        Self {
            gutter,
            visible_lines,
            char_width,
        }
    }

    /// Updates the gutter model.
    pub fn set_gutter(&mut self, gutter: GutterModel) {
        self.gutter = gutter;
    }

    /// Updates the visible lines.
    pub fn set_visible_lines(&mut self, visible_lines: Vec<LinePresentation>) {
        self.visible_lines = visible_lines;
    }

    /// Sets the monospace character width for width calculations.
    pub fn set_char_width(&mut self, char_width: f32) {
        self.char_width = char_width;
    }

    /// Returns the calculated gutter width in pixels.
    ///
    /// Width = LEFT_PADDING + (digit_count * char_width) + SEPARATOR_PADDING
    ///
    /// Returns 0.0 if gutter is not visible.
    pub fn calculate_width(&self) -> f32 {
        calculate_width(&self.gutter, self.char_width)
    }

    /// Renders a single line number row.
    fn render_line_row(&self, line: &LinePresentation, digit_width: usize, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Color based on current line status
        let number_color = if line.is_current_line {
            theme.color(ColorToken::FgPrimary) // Brighter for current line
        } else {
            theme.color(ColorToken::FgMuted) // Dimmer for other lines
        };

        // Format line number right-aligned with appropriate width
        let line_text = format!("{:>width$}", line.line_number, width = digit_width);

        // Line number text
        let number_element = TextElement::new(line_text)
            .size(LINE_NUMBER_FONT_SIZE)
            .color(number_color);

        // Row container
        Div::new()
            .flex_row()
            .h(px(LINE_HEIGHT))
            .shrink(0.0) // Never compress below LINE_HEIGHT
            .justify_end()
            .align_center()
            .child(number_element)
    }
}

/// Calculates the gutter width in pixels.
///
/// # Arguments
///
/// * `gutter` - The gutter model containing visibility and line count info
/// * `char_width` - The width of a monospace character in pixels
///
/// # Returns
///
/// The calculated width, or 0.0 if the gutter is not visible.
pub fn calculate_width(gutter: &GutterModel, char_width: f32) -> f32 {
    if !gutter.visible {
        return 0.0;
    }

    let digit_count = gutter.line_number_width();
    LEFT_PADDING + (digit_count as f32 * char_width) + RIGHT_PADDING
}

impl View for GutterView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // If gutter is not visible, render empty placeholder
        if !self.gutter.visible {
            return Div::new()
                .w(px(0.0))
                .h(px(0.0))
                .into();
        }

        // Calculate gutter width
        let width = self.calculate_width();
        let digit_width = self.gutter.line_number_width();

        // Build line rows (render_line_row borrows cx mutably, so build first)
        let mut line_rows: Vec<AnyElement> = Vec::with_capacity(self.visible_lines.len());
        for line in &self.visible_lines {
            line_rows.push(self.render_line_row(line, digit_width, cx).into());
        }

        // Now get theme for container styling
        let theme = cx.theme();

        // Build gutter container (same bg as editor, right border as separator)
        Div::new()
            .flex_col()
            .w(px(width))
            .shrink(0.0)  // Don't shrink below calculated width
            .overflow_hidden() // Clip line rows when window is shorter than content
            .bg(theme.color(ColorToken::BgPrimary))
            .pl(LEFT_PADDING)
            .pr(RIGHT_PADDING)
            .border_right(1.0, theme.color(ColorToken::Border))
            .children(line_rows)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_adapter::{GutterModel, LinePresentation, StyledSpan};

    fn make_line(number: usize, is_current: bool) -> LinePresentation {
        LinePresentation::with_spans(number, is_current, vec![StyledSpan::raw("")])
    }

    #[test]
    fn test_gutter_view_creation() {
        let gutter = GutterModel::new(true, 100);
        let lines = vec![
            make_line(1, true),
            make_line(2, false),
            make_line(3, false),
        ];

        let view = GutterView::new(gutter.clone(), lines);
        assert_eq!(view.gutter.line_number_width(), 3); // "100" = 3 digits
        assert_eq!(view.visible_lines.len(), 3);
    }

    #[test]
    fn test_calculate_width_visible() {
        // 100 lines = 3 digits
        let gutter = GutterModel::new(true, 100);
        let char_width = 8.0;

        let width = calculate_width(&gutter, char_width);
        // LEFT_PADDING(8) + digits(3)*char_width(8) + RIGHT_PADDING(12) = 44
        assert_eq!(width, 44.0);
    }

    #[test]
    fn test_calculate_width_not_visible() {
        let gutter = GutterModel::new(false, 100);
        let char_width = 8.0;

        let width = calculate_width(&gutter, char_width);
        assert_eq!(width, 0.0);
    }

    #[test]
    fn test_calculate_width_different_line_counts() {
        let char_width = 8.0;

        // Single digit (9 lines or less)
        let gutter = GutterModel::new(true, 9);
        let width = calculate_width(&gutter, char_width);
        // LEFT_PADDING(8) + digits(1)*8 + RIGHT_PADDING(12) = 28
        assert_eq!(width, 28.0);

        // Two digits (10-99 lines)
        let gutter = GutterModel::new(true, 50);
        let width = calculate_width(&gutter, char_width);
        // LEFT_PADDING(8) + digits(2)*8 + RIGHT_PADDING(12) = 36
        assert_eq!(width, 36.0);

        // Four digits (1000+ lines)
        let gutter = GutterModel::new(true, 1500);
        let width = calculate_width(&gutter, char_width);
        // LEFT_PADDING(8) + digits(4)*8 + RIGHT_PADDING(12) = 52
        assert_eq!(width, 52.0);
    }

    #[test]
    fn test_gutter_view_width_method() {
        let gutter = GutterModel::new(true, 100);
        let lines = vec![];

        // Use explicit char_width for test determinism (measured_char_width()
        // returns a fallback in tests since no GPU is initialised).
        let mut view = GutterView::new(gutter, lines);
        view.set_char_width(8.0);
        assert_eq!(view.calculate_width(), 44.0);

        let mut view2 = GutterView::new(GutterModel::new(true, 100), vec![]);
        view2.set_char_width(10.0);
        // LEFT_PADDING(8) + digits(3)*char_width(10) + RIGHT_PADDING(12) = 50
        assert_eq!(view2.calculate_width(), 50.0);
    }

    #[test]
    fn test_gutter_constants() {
        assert_eq!(LEFT_PADDING, 8.0);
        assert_eq!(RIGHT_PADDING, 12.0);
        assert_eq!(LINE_NUMBER_FONT_SIZE, 14.0);
        assert_eq!(LINE_HEIGHT, 21.0);
    }
}
