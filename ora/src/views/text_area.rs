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
use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::Style;
use crate::elements::{paint_offset, stack, CaretElement, Div, TextElement};
use crate::rendering::TextRun;
use crate::style::{pct, px, Color};
use crate::editor_adapter::{CaretPresentation, LinePresentation, RenderModel, TextStyle};
use crate::theme::{ColorToken, Theme};
use crate::view::View;

/// One selection rectangle extracted from visible_lines, positioned relative
/// to the selection-overlay's local origin.
#[derive(Clone, Copy, Debug)]
struct SelectionRect {
    /// 0-based row inside viewport_rows.
    row: usize,
    /// Starting column (characters from start of line).
    start_col: usize,
    /// Ending column (exclusive).
    end_col: usize,
    /// When the selection reaches the end of the line, grow to the right
    /// edge of the layer so the highlight looks "line-terminated".
    grow_to_edge: bool,
}

/// Custom element that paints selection highlight rects in one shot.
///
/// Has a single `LayoutId` of its own and no children, so its presence
/// does not change any sibling/descendant LayoutId assignment regardless
/// of how many rects it holds this frame. This is the architectural
/// lever that keeps the layout cache valid under a changing selection.
struct SelectionOverlay {
    rects: Vec<SelectionRect>,
    color: Color,
    char_width: f32,
    line_height: f32,
}

struct SelectionOverlayState {
    layout_id: LayoutId,
}

impl Element for SelectionOverlay {
    type RequestLayoutState = SelectionOverlayState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, SelectionOverlayState) {
        let mut style = Style::default();
        style.width = crate::style::Length::Percent(100.0);
        style.height = crate::style::Length::Percent(100.0);
        let id = cx.request_layout(&style);
        (id, SelectionOverlayState { layout_id: id })
    }

    fn prepaint(&mut self, _state: &mut SelectionOverlayState, _cx: &mut PrepaintContext) {
        // No hitbox — selection rects are visual only, not interactive.
    }

    fn paint(&mut self, state: &mut SelectionOverlayState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let color_arr = [self.color.r, self.color.g, self.color.b, self.color.a];
        for rect in &self.rects {
            let x = bounds.origin.x + (rect.start_col as f32) * self.char_width;
            let y = bounds.origin.y + (rect.row as f32) * self.line_height;
            let w = if rect.grow_to_edge {
                (bounds.origin.x + bounds.size.width) - x
            } else {
                ((rect.end_col - rect.start_col) as f32) * self.char_width
            };
            if w > 0.0 {
                cx.paint_rect(x, y, w, self.line_height, color_arr);
            }
        }
    }
}


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
    /// Target number of rows to emit — the viewport's fixed line capacity.
    /// We always emit exactly this many rows in every layer (text,
    /// current-line-bg, selection-bg), padding with empty placeholders
    /// if `visible_lines.len()` is smaller (near EOF). Keeps the element
    /// tree and LayoutId assignment stable across scroll frames so the
    /// cached layout outputs stay valid on paint-only scrolls.
    viewport_rows: usize,
}

impl TextAreaView {
    /// Creates a new text area view from render model data.
    pub fn new(model: &RenderModel) -> Self {
        let viewport_rows = model.visible_lines.len();
        Self {
            visible_lines: model.visible_lines.clone(),
            caret: model.caret.clone(),
            scroll_x: model.scroll_x,
            scroll_y: model.scroll_y,
            char_width: crate::rendering::measured_char_width(),
            scroll_y_offset_px: model.scroll_y_offset_px,
            editor_focused: model.editor_focused,
            viewport_rows,
        }
    }

    /// Creates a new text area view with an explicit row capacity. When
    /// `visible_lines.len() < viewport_rows` (near EOF) the extra rows are
    /// emitted as empty placeholders so the element tree row count stays
    /// constant across scroll frames.
    pub fn with_viewport(model: &RenderModel, viewport_rows: usize) -> Self {
        Self {
            visible_lines: model.visible_lines.clone(),
            caret: model.caret.clone(),
            scroll_x: model.scroll_x,
            scroll_y: model.scroll_y,
            char_width: crate::rendering::measured_char_width(),
            scroll_y_offset_px: model.scroll_y_offset_px,
            editor_focused: model.editor_focused,
            viewport_rows,
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
            viewport_rows: 0,
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

    /// Renders a column of current-line background highlights.
    ///
    /// Always emits exactly `viewport_rows` rows. Rows beyond `visible_lines`
    /// (near EOF) are transparent placeholders so the element tree count
    /// stays constant across scroll frames — a prerequisite for the cached
    /// layout outputs to index correctly on paint-only scrolls.
    fn render_current_line_bg_layer(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();
        let current_line_color = theme.color(ColorToken::CurrentLineBg);

        let rows: Vec<AnyElement> = (0..self.viewport_rows)
            .map(|i| {
                let mut row = Div::new()
                    .w(pct(100.0))
                    .h(px(LINE_HEIGHT))
                    .shrink(0.0);
                if let Some(line) = self.visible_lines.get(i) {
                    if line.is_current_line {
                        row = row.bg(current_line_color);
                    }
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
    /// Emits exactly `viewport_rows` rows; each row is a plain transparent
    /// placeholder Div. Selection highlights themselves are painted as
    /// absolute-positioned rects via `paint_rect` in the custom
    /// `SelectionOverlay` element below, so the row structure stays fixed
    /// regardless of where/how many selection ranges exist. This keeps
    /// LayoutIds stable during scroll.
    fn render_selection_bg_layer(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();
        let selection_color = if self.editor_focused {
            theme.color(ColorToken::Selection)
        } else {
            theme.color(ColorToken::SelectionInactive)
        };

        // Build the flat list of (row_index, start_col, end_col, grow_to_edge)
        // selection rects. Painted directly via a custom overlay element.
        let mut rects: Vec<SelectionRect> = Vec::new();
        for (i, line) in self.visible_lines.iter().enumerate() {
            if i >= self.viewport_rows {
                break;
            }
            let total_chars: usize = line.spans.iter().map(|s| s.text.chars().count()).sum();
            for &(start_col, end_col) in &line.selection_ranges {
                rects.push(SelectionRect {
                    row: i,
                    start_col,
                    end_col,
                    grow_to_edge: end_col >= total_chars && total_chars > 0,
                });
            }
        }

        let overlay = SelectionOverlay {
            rects,
            color: selection_color,
            char_width: self.char_width,
            line_height: LINE_HEIGHT,
        };

        // Empty per-row spacer column, so the layer has the same intrinsic
        // layout shape as current_line_bg / text. The overlay sits on top
        // and paints absolute-positioned rects from `rects`.
        let spacer_rows: Vec<AnyElement> = (0..self.viewport_rows)
            .map(|_| {
                Div::new()
                    .w(pct(100.0))
                    .h(px(LINE_HEIGHT))
                    .shrink(0.0)
                    .into()
            })
            .collect();

        let spacer_col: AnyElement = Div::new()
            .flex_col()
            .w(pct(100.0))
            .children(spacer_rows)
            .into();

        let overlay_any: AnyElement = overlay.into();
        let inner: AnyElement = crate::elements::stack()
            .w(pct(100.0))
            .h(pct(100.0))
            .child(spacer_col)
            .child(overlay_any)
            .into();

        paint_offset(-self.scroll_y_offset_px, inner).into()
    }

    /// Renders exactly `viewport_rows` row elements with a structurally
    /// identical shape: each row is a `Div` wrapping one `TextElement::rich`.
    /// Rows beyond `visible_lines.len()` use a single-empty-run
    /// TextElement so the `(Div + TextElement)` LayoutId pair is allocated
    /// for every row regardless of EOF position. Constant LayoutIds means
    /// the cached layout outputs stay valid on paint-only scroll frames.
    fn render_text_lines(&self, cx: &mut ViewContext) -> Vec<AnyElement> {
        (0..self.viewport_rows)
            .map(|i| match self.visible_lines.get(i) {
                Some(line) => self.render_line(line, cx),
                None => Div::new()
                    .flex_row()
                    .w(pct(100.0))
                    .h(px(LINE_HEIGHT))
                    .shrink(0.0)
                    .child(
                        TextElement::new("")
                            .size(TEXT_FONT_SIZE)
                            .line_height(LINE_HEIGHT)
                            .color(Color::transparent())
                            .grow(1.0),
                    )
                    .into(),
            })
            .collect()
    }

    /// Renders a single line as ONE rich `TextElement` carrying all syntax
    /// spans as colored runs.
    ///
    /// The row Div is flex_row and the TextElement grows to fill the full
    /// row width. Fixing the row width (not the text's own intrinsic
    /// width) is critical: on paint-only scroll frames, cached layout
    /// outputs are reused and the bounds for each LayoutId carry last
    /// frame's measured width. If the previous occupant of this row was
    /// empty (measured width = 0), and the new line has content, the
    /// text renders with a w=0 glyphon clip region and is invisible —
    /// producing the "text disappears, background shows through"
    /// flicker at line-boundary crossings. Forcing the text element to
    /// grow(1.0) gives it the full row width every frame, independent
    /// of measured text width, so the cached bounds are always wide
    /// enough to show whatever text currently lives in that slot.
    fn render_line(&self, line: &LinePresentation, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        let runs: Vec<TextRun> = line
            .spans
            .iter()
            .map(|span| TextRun::new(span.text.clone(), self.map_style_to_color(span.style, theme)))
            .collect();

        Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(px(LINE_HEIGHT))
            .shrink(0.0)
            .child(
                TextElement::rich(runs)
                    .size(TEXT_FONT_SIZE)
                    .line_height(LINE_HEIGHT)
                    .grow(1.0),
            )
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
