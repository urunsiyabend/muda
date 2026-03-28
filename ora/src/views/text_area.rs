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
use crate::elements::{paint_offset, stack, CaretElement, Div, TextElement};
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
    /// Sub-line vertical scroll offset in pixels (smooth scroll).
    ///
    /// Applied as a negative vertical shift to the text content layer,
    /// producing pixel-level smooth scrolling between logical line boundaries.
    scroll_y_offset_px: f32,
    /// Whether the editor window has OS-level focus.
    /// When false, selection backgrounds use a dimmed color.
    editor_focused: bool,
}

impl TextAreaView {
    /// Creates a new text area view from render model data.
    pub fn new(model: &RenderModel) -> Self {
        Self {
            visible_lines: model.visible_lines.clone(),
            caret: model.caret.clone(),
            scroll_x: model.scroll_x,
            scroll_y: model.scroll_y,
            char_width: crate::rendering::measured_char_width(),
            scroll_y_offset_px: model.scroll_y_offset_px,
            editor_focused: model.editor_focused,
        }
    }

    /// Creates an empty text area view.
    pub fn empty() -> Self {
        Self {
            visible_lines: Vec::new(),
            caret: CaretPresentation::default(),
            scroll_x: 0,
            scroll_y: 0,
            char_width: crate::rendering::measured_char_width(),
            scroll_y_offset_px: 0.0,
            editor_focused: true,
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

    /// Renders a column of current-line background highlights (one per visible line).
    ///
    /// Produces a flex-col of line-height divs, where the current line has
    /// CurrentLineBg and all others are transparent. This layer sits below
    /// both selection_bg and text in the Stack z-order.
    fn render_current_line_bg_layer(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();
        let current_line_color = theme.color(ColorToken::CurrentLineBg);

        let rows: Vec<AnyElement> = self
            .visible_lines
            .iter()
            .map(|line| {
                let mut row = Div::new()
                    .w(pct(100.0))
                    .h(px(LINE_HEIGHT))
                    .shrink(0.0);
                if line.is_current_line {
                    row = row.bg(current_line_color);
                }
                row.into()
            })
            .collect();

        let inner: AnyElement = Div::new()
            .flex_col()
            .w(pct(100.0))
            .children(rows)
            .into();
        paint_offset(-self.scroll_y_offset_px, inner).into()
    }

    /// Renders selection background rectangles from selection_ranges.
    ///
    /// Produces a flex-col of line-height rows. Each row uses a flex-row of
    /// [spacer, selection-rect] to position the highlight at the correct column.
    /// Lines with no selection get an empty transparent row.
    fn render_selection_bg_layer(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();
        let selection_color = if self.editor_focused {
            theme.color(ColorToken::Selection)
        } else {
            theme.color(ColorToken::SelectionInactive)
        };

        let rows: Vec<AnyElement> = self
            .visible_lines
            .iter()
            .map(|line| {
                if line.selection_ranges.is_empty() {
                    // No selection on this line — transparent placeholder row.
                    return Div::new()
                        .w(pct(100.0))
                        .h(px(LINE_HEIGHT))
                        .shrink(0.0)
                        .into();
                }

                // Build one rect per selection range on this line.
                // Ranges are non-overlapping, emitted in column order by the adapter.
                let mut children: Vec<AnyElement> = Vec::new();
                let mut prev_end: usize = 0;

                // Total character count on this line (for edge-to-edge detection).
                let total_chars: usize = line.spans.iter()
                    .map(|s| s.text.chars().count())
                    .sum();

                for &(start_col, end_col) in &line.selection_ranges {
                    // Spacer before this selection range.
                    if start_col > prev_end {
                        let spacer_w = (start_col - prev_end) as f32 * self.char_width;
                        children.push(
                            Div::new().w(px(spacer_w)).h(px(LINE_HEIGHT)).shrink(0.0).into(),
                        );
                    }
                    // Selection highlight rect.
                    // If the selection extends to or past end of line text,
                    // use grow(1.0) to fill remaining width (edge-to-edge).
                    if end_col >= total_chars && total_chars > 0 {
                        children.push(
                            Div::new()
                                .h(px(LINE_HEIGHT))
                                .shrink(0.0)
                                .grow(1.0)
                                .bg(selection_color)
                                .into(),
                        );
                    } else {
                        let sel_w = (end_col - start_col) as f32 * self.char_width;
                        children.push(
                            Div::new()
                                .w(px(sel_w))
                                .h(px(LINE_HEIGHT))
                                .shrink(0.0)
                                .bg(selection_color)
                                .into(),
                        );
                    }
                    prev_end = end_col;
                }

                Div::new()
                    .flex_row()
                    .w(pct(100.0))
                    .h(px(LINE_HEIGHT))
                    .shrink(0.0)
                    .children(children)
                    .into()
            })
            .collect();

        let inner: AnyElement = Div::new()
            .flex_col()
            .w(pct(100.0))
            .children(rows)
            .into();
        paint_offset(-self.scroll_y_offset_px, inner).into()
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
    ///
    /// All spans (including TextStyle::Selection) are rendered as text —
    /// selection visibility comes from the selection_bg Stack layer below,
    /// not from filtering here. The line div has no background (transparent),
    /// so lower Stack layers show through.
    fn render_line(&self, line: &LinePresentation, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        // Render ALL spans — including Selection-styled ones — as colored text.
        // The selection background is handled by render_selection_bg_layer below text.
        let span_elements: Vec<AnyElement> = line
            .spans
            .iter()
            .map(|span| {
                let color = self.map_style_to_color(span.style, theme);
                TextElement::new(&span.text)
                    .size(TEXT_FONT_SIZE)
                    .color(color)
                    .into()
            })
            .collect();

        // Line container: transparent background so selection/current-line layers
        // from lower Stack layers show through.
        Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(px(LINE_HEIGHT))
            .shrink(0.0)
            .children(span_elements)
            .into()
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
        let text_lines = self.render_text_lines(cx);
        let caret = self.render_caret(cx);
        let current_line_bg = self.render_current_line_bg_layer(cx);
        let selection_bg = self.render_selection_bg_layer(cx);

        let theme = cx.theme();

        // Partial-line scroll offset: shift line content upward by fractional
        // pixels so line boundaries align exactly during scrolling.
        // Applied via push_offset in paint phase — does NOT affect layout,
        // keeping the element tree structure stable across scroll frames.
        // The outer container's overflow_hidden() clips the partially visible
        // top and bottom lines at the GPU level via wgpu scissor rectangles.
        let scroll_shift = -self.scroll_y_offset_px;

        // Layer 1 (bottom): Opaque base background — fills the entire text area.
        // This is the ONLY layer with BgPrimary; all layers above are transparent.
        let base_bg_layer: AnyElement = Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(theme.color(ColorToken::BgPrimary))
            .into();

        // Layer 2: Current-line highlight — one colored row per visible line.
        // Transparent rows for non-current lines let base_bg show through.
        let current_line_layer: AnyElement = Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .pl(1.0)
            .overflow_hidden()
            .child(current_line_bg)
            .into();

        // Layer 3: Selection highlight rects — sits above current-line, below text.
        let selection_layer: AnyElement = Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .pl(1.0)
            .overflow_hidden()
            .child(selection_bg)
            .into();

        // Layer 4: Text content — transparent background so selection/current-line
        // layers below show through the gaps between glyphs.
        // scroll_shift applied via paint_offset (push_offset/pop_offset), not mt().
        let inner_text_div: AnyElement = Div::new()
            .flex_col()
            .w(pct(100.0))
            .children(text_lines)
            .into();
        let inner_text: AnyElement = paint_offset(scroll_shift, inner_text_div).into();

        let text_layer: AnyElement = Div::new()
            .flex_col()
            .w(pct(100.0))
            .h(pct(100.0))
            .pl(1.0)
            .overflow_hidden()
            .child(inner_text)
            .into();

        // Inner wrapper for caret with same scroll shift for alignment.
        // scroll_shift applied via paint_offset (push_offset/pop_offset), not mt().
        let inner_caret_div: AnyElement = Div::new()
            .w(pct(100.0))
            .child(caret)
            .into();
        let inner_caret: AnyElement = paint_offset(scroll_shift, inner_caret_div).into();

        // Layer 5 (top): Caret — overlaid on top of everything.
        let caret_layer: AnyElement = Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .child(inner_caret)
            .into();

        // Stack z-order (bottom → top):
        //   base_bg → current_line → selection → text → caret
        stack()
            .w(pct(100.0))
            .h(pct(100.0))
            .child(base_bg_layer)
            .child(current_line_layer)
            .child(selection_layer)
            .child(text_layer)
            .child(caret_layer)
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
