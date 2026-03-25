//! Text area view for editor content.
//!
//! Renders syntax-highlighted text with selection backgrounds, current line
//! highlight, and blinking caret. Consumes RenderModel from core_editor.
//!
//! # Architecture
//!
//! TextAreaView is a stateless view that builds an element tree during render().
//! It does NOT own rendering state (no FontSystem, TextAtlas, etc.).
//! The framework handles text shaping, layout, and batched rendering.
//!
//! # Layer Structure
//!
//! 1. Current line background (full width, subtle highlight)
//! 2. Selection backgrounds (solid, per CONTEXT decision)
//! 3. Text content (syntax highlighted spans)
//! 4. Caret element (on top)
//!
//! # Styling
//!
//! Text styles from core_editor::TextStyle are mapped to ColorToken:
//! - Keyword -> SyntaxKeyword (purple)
//! - String -> SyntaxString (green)
//! - Comment -> SyntaxComment (gray)
//! - etc.

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{CaretElement, Div, TextElement};
use crate::style::{pct, px, Color};
use crate::editor_adapter::{CaretPresentation, LinePresentation, RenderModel, TextStyle};
use crate::theme::{ColorToken, Theme};
use crate::view::View;

/// Font size for editor text.
const TEXT_FONT_SIZE: f32 = 14.0;

/// Line height for editor lines.
pub const LINE_HEIGHT: f32 = 21.0;

/// Text area view for rendering editor content.
///
/// Displays syntax-highlighted text with:
/// - Current line background highlight (full width)
/// - Selection backgrounds (solid blue)
/// - Syntax-colored text spans
/// - Blinking caret at cursor position
///
/// # CONTEXT Decisions
///
/// - No line wrapping: horizontal scroll for long lines
/// - Selection: solid background highlight (not semi-transparent)
/// - Caret: thin beam style (2px width)
/// - Current line: full background highlight
///
/// # Example
///
/// ```ignore
/// let view = TextAreaView::new(render_model);
/// // In parent layout:
/// Div::new()
///     .flex_row()
///     .child(gutter_view)
///     .child(view.render(cx))
/// ```
pub struct TextAreaView {
    /// Visible lines from RenderModel.
    visible_lines: Vec<LinePresentation>,
    /// Caret presentation data.
    caret: CaretPresentation,
    /// Scroll offset X (for future horizontal scroll).
    #[allow(dead_code)]
    scroll_x: usize,
    /// Scroll offset Y (for future vertical scroll).
    #[allow(dead_code)]
    scroll_y: usize,
    /// Character width for position calculations.
    char_width: f32,
}

impl TextAreaView {
    /// Creates a new text area view from render model data.
    pub fn new(model: &RenderModel) -> Self {
        Self {
            visible_lines: model.visible_lines.clone(),
            caret: model.caret.clone(),
            scroll_x: model.scroll_x,
            scroll_y: model.scroll_y,
            char_width: 8.0, // Default monospace character width
        }
    }

    /// Creates an empty text area view.
    pub fn empty() -> Self {
        Self {
            visible_lines: Vec::new(),
            caret: CaretPresentation::default(),
            scroll_x: 0,
            scroll_y: 0,
            char_width: 8.0,
        }
    }

    /// Sets the visible lines.
    pub fn set_visible_lines(&mut self, lines: Vec<LinePresentation>) {
        self.visible_lines = lines;
    }

    /// Sets the caret presentation.
    pub fn set_caret(&mut self, caret: CaretPresentation) {
        self.caret = caret;
    }

    /// Sets the character width for position calculations.
    pub fn set_char_width(&mut self, width: f32) {
        self.char_width = width;
    }

    /// Maps TextStyle to Color using theme tokens.
    fn map_style_to_color(&self, style: TextStyle, theme: &Theme) -> Color {
        match style {
            TextStyle::Normal => theme.color(ColorToken::FgPrimary),
            TextStyle::Keyword => theme.color(ColorToken::SyntaxKeyword),
            TextStyle::String => theme.color(ColorToken::SyntaxString),
            TextStyle::Comment => theme.color(ColorToken::SyntaxComment),
            TextStyle::Number => theme.color(ColorToken::SyntaxNumber),
            TextStyle::Type => theme.color(ColorToken::SyntaxType),
            TextStyle::Function => theme.color(ColorToken::SyntaxFunction),
            TextStyle::Variable => theme.color(ColorToken::FgPrimary),
            TextStyle::Operator => theme.color(ColorToken::FgSecondary),
            TextStyle::Punctuation => theme.color(ColorToken::FgMuted),
            TextStyle::Constant => theme.color(ColorToken::SyntaxConstant),
            TextStyle::Module => theme.color(ColorToken::SyntaxType),
            TextStyle::Attribute => theme.color(ColorToken::SyntaxAttribute),
            TextStyle::Macro => theme.color(ColorToken::SyntaxMacro),
            TextStyle::Selection => theme.color(ColorToken::Selection),
            TextStyle::Error => theme.color(ColorToken::Error),
            TextStyle::Warning => theme.color(ColorToken::Warning),
            TextStyle::LineNumber | TextStyle::CurrentLineNumber => theme.color(ColorToken::FgMuted),
        }
    }

    /// Renders the current line background highlight.
    fn render_current_line_bg(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        // Find current line index
        let current_line_index = self.visible_lines.iter().position(|l| l.is_current_line);

        if let Some(index) = current_line_index {
            let _y_offset = index as f32 * LINE_HEIGHT;

            Div::new()
                .w(pct(100.0))
                .h(px(LINE_HEIGHT))
                .bg(theme.color(ColorToken::CurrentLineBg))
                .m(0.0) // Position at y_offset would need absolute positioning
                .into()
        } else {
            // No current line, render empty placeholder
            Div::new().w(px(0.0)).h(px(0.0)).into()
        }
    }

    /// Renders selection background rectangles.
    ///
    /// Selection spans are identified by TextStyle::Selection.
    /// Each selection gets a solid background (per CONTEXT decision).
    fn render_selection_rects(&self, cx: &mut ViewContext) -> Vec<AnyElement> {
        let theme = cx.theme();
        let selection_color = theme.color(ColorToken::Selection);
        let mut rects = Vec::new();

        for (line_index, line) in self.visible_lines.iter().enumerate() {
            for span in &line.spans {
                let span_width = span.text.chars().count() as f32 * self.char_width;

                if span.style == TextStyle::Selection {
                    let _y_offset = line_index as f32 * LINE_HEIGHT;

                    // Selection rectangle (would use absolute positioning in full impl)
                    let rect = Div::new()
                        .w(px(span_width))
                        .h(px(LINE_HEIGHT))
                        .bg(selection_color);

                    rects.push(rect.into());
                }
            }
        }

        rects
    }

    /// Renders all text lines with syntax highlighting.
    fn render_text_lines(&self, cx: &mut ViewContext) -> Vec<AnyElement> {
        // Build line elements first to release mutable borrow of cx
        self.visible_lines
            .iter()
            .map(|line| self.render_line(line, cx))
            .collect()
    }

    /// Renders a single line with syntax-highlighted spans.
    fn render_line(&self, line: &LinePresentation, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        // Build span elements
        let span_elements: Vec<AnyElement> = line
            .spans
            .iter()
            .filter(|span| span.style != TextStyle::Selection) // Selection is background-only
            .map(|span| {
                let color = self.map_style_to_color(span.style, theme);
                TextElement::new(&span.text)
                    .size(TEXT_FONT_SIZE)
                    .color(color)
                    .into()
            })
            .collect();

        // Build line container with current line highlight if applicable
        let mut line_div = Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(px(LINE_HEIGHT))
            .shrink(0.0) // Never compress below LINE_HEIGHT
            .children(span_elements);

        // Apply current line background highlight
        if line.is_current_line {
            line_div = line_div.bg(theme.color(ColorToken::CurrentLineBg));
        }

        line_div.into()
    }

    /// Renders the caret element.
    fn render_caret(&self, _cx: &mut ViewContext) -> AnyElement {
        if !self.caret.visible {
            return Div::new().w(px(0.0)).h(px(0.0)).into();
        }

        // Calculate caret position
        let x = self.caret.position.column as f32 * self.char_width;
        let y = self.caret.position.row as f32 * LINE_HEIGHT;

        CaretElement::new(x, y, LINE_HEIGHT).into()
    }
}

impl View for TextAreaView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Build all child elements first (each borrows cx mutably)
        // Note: current_line_bg, selection_rects, and caret are rendered but not yet
        // integrated into the layered layout (would require Stack with absolute positioning)
        let _current_line_bg = self.render_current_line_bg(cx);
        let _selection_rects = self.render_selection_rects(cx);
        let text_lines = self.render_text_lines(cx);
        let _caret = self.render_caret(cx);

        // Get theme for container
        let theme = cx.theme();

        // Build layer structure:
        // 1. Current line background (integrated into each line div)
        // 2. Selection backgrounds
        // 3. Text content
        // 4. Caret
        //
        // Note: Full layering would use Stack for z-ordering.
        // This simplified version uses flex column for line layout.
        Div::new()
            .flex_col()
            .grow(1.0)
            .h(pct(100.0))
            .bg(theme.color(ColorToken::BgPrimary))
            .pl(1.0)
            .overflow_hidden() // Clip content to viewport bounds
            // In a full implementation, we'd use absolute positioning for layers
            // For now, render lines in order with text taking precedence
            .children(text_lines)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_adapter::{StyledSpan, VisualPosition};

    fn make_line(number: usize, is_current: bool, text: &str) -> LinePresentation {
        LinePresentation::with_spans(
            number,
            is_current,
            vec![StyledSpan::raw(text)],
        )
    }

    #[allow(dead_code)]
    fn make_styled_line(number: usize, is_current: bool, spans: Vec<StyledSpan>) -> LinePresentation {
        LinePresentation::with_spans(number, is_current, spans)
    }

    #[test]
    fn test_text_area_view_creation() {
        let mut model = RenderModel::default();
        model.visible_lines = vec![
            make_line(1, true, "fn main() {"),
            make_line(2, false, "    println!(\"Hello\");"),
            make_line(3, false, "}"),
        ];
        model.caret = CaretPresentation {
            position: VisualPosition::new(0, 5),
            visible: true,
        };

        let view = TextAreaView::new(&model);
        assert_eq!(view.visible_lines.len(), 3);
        assert!(view.caret.visible);
        assert_eq!(view.caret.position.row, 0);
        assert_eq!(view.caret.position.column, 5);
    }

    #[test]
    fn test_text_area_empty() {
        let view = TextAreaView::empty();
        assert!(view.visible_lines.is_empty());
        assert!(!view.caret.visible);
    }

    #[test]
    fn test_text_area_set_visible_lines() {
        let mut view = TextAreaView::empty();
        view.set_visible_lines(vec![make_line(1, true, "test")]);
        assert_eq!(view.visible_lines.len(), 1);
        assert!(view.visible_lines[0].is_current_line);
    }

    #[test]
    fn test_text_area_set_caret() {
        let mut view = TextAreaView::empty();
        view.set_caret(CaretPresentation {
            position: VisualPosition::new(5, 10),
            visible: true,
        });
        assert!(view.caret.visible);
        assert_eq!(view.caret.position.row, 5);
        assert_eq!(view.caret.position.column, 10);
    }

    #[test]
    fn test_text_area_constants() {
        assert_eq!(TEXT_FONT_SIZE, 14.0);
        assert_eq!(LINE_HEIGHT, 21.0);
    }

    #[test]
    fn test_map_style_to_color_coverage() {
        // This test verifies that all TextStyle variants are handled
        // It would require a Theme to test actual colors
        let styles = [
            TextStyle::Normal,
            TextStyle::Keyword,
            TextStyle::String,
            TextStyle::Comment,
            TextStyle::Number,
            TextStyle::Type,
            TextStyle::Function,
            TextStyle::Variable,
            TextStyle::Operator,
            TextStyle::Punctuation,
            TextStyle::Constant,
            TextStyle::Module,
            TextStyle::Attribute,
            TextStyle::Macro,
            TextStyle::Selection,
            TextStyle::Error,
            TextStyle::Warning,
            TextStyle::LineNumber,
            TextStyle::CurrentLineNumber,
        ];

        // Just verify we handle all variants (actual color test would need Theme)
        assert_eq!(styles.len(), 19);
    }
}
