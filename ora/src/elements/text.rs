use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::events::mouse::HitboxId;
use crate::rendering::text::GlyphCacheKey;
use crate::rendering::TextRun;
use crate::style::*;

/// Text rendering element with glyphon-based measurement and rendering.
///
/// Two modes:
///   - Plain: single-color text set via `TextElement::new` + `.color`. Uses
///     `measure_text_cached` (one `Attrs` covers everything).
///   - Rich: multi-color text set via `TextElement::rich(runs)`. Uses
///     `measure_rich_text_cached`; each `TextRun` contributes its own color
///     via cosmic-text per-span `Attrs`. Produces a single shaped `Buffer`,
///     so one `TextElement` per visible line is enough regardless of how
///     many syntax spans the line has — the structural guarantee the layout
///     cache needs to stay valid across scroll frames.
pub struct TextElement {
    content: String,
    /// Rich-text runs. `Some` means use `measure_rich_text_cached`; the
    /// `color` field is then ignored (fallback-only for glyphs without
    /// explicit Attrs color, which won't happen here).
    runs: Option<Vec<TextRun>>,
    font_size: f32,
    line_height: f32,
    color: Color,
    style: Style,
    wrap: bool,
}

impl TextElement {
    pub fn new(content: impl Into<String>) -> Self {
        TextElement {
            content: content.into(),
            runs: None,
            font_size: 14.0,
            line_height: 16.8,
            color: Color::white(),
            style: Style::default(),
            wrap: false,
        }
    }

    /// Build a multi-color rich-text element from a list of colored runs.
    ///
    /// Measurement concatenates run text into one shaped `Buffer` with
    /// per-glyph color. Exactly one `TextElement` / `LayoutId` per line
    /// is produced, regardless of run count.
    pub fn rich(runs: Vec<TextRun>) -> Self {
        let content: String = runs.iter().map(|r| r.text.as_str()).collect();
        TextElement {
            content,
            runs: Some(runs),
            font_size: 14.0,
            line_height: 16.8,
            color: Color::white(),
            style: Style::default(),
            wrap: false,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.font_size = size;
        self.line_height = size * 1.2;
        self
    }

    pub fn line_height(mut self, lh: f32) -> Self {
        self.line_height = lh;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn w(mut self, w: f32) -> Self {
        self.style.width = Length::Px(w);
        self
    }

    pub fn h(mut self, h: f32) -> Self {
        self.style.height = Length::Px(h);
        self
    }

    pub fn m(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }

    pub fn p(mut self, p: f32) -> Self {
        self.style.padding = Edges::all(p);
        self
    }

    pub fn grow(mut self, v: f32) -> Self {
        self.style.flex_grow = v;
        self
    }

    /// Set the text color (for use during paint phase)
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }
}

/// State persisted through the rendering lifecycle
pub struct TextState {
    layout_id: LayoutId,
    measured_size: Size<f32>,
    pub(crate) buffer: Option<glyphon::Buffer>,
    cache_key: Option<GlyphCacheKey>,
    hitbox_id: Option<HitboxId>,
}

impl Element for TextElement {
    type RequestLayoutState = TextState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, TextState) {
        let id = cx.request_layout(&self.style);
        let max_width = match self.style.width {
            Length::Px(w) => Some(w),
            _ => None,
        };
        let (cache_key, buffer, measured) = if let Some(runs) = &self.runs {
            cx.measure_rich_text_cached(runs, self.font_size, self.line_height, max_width)
        } else {
            cx.measure_text_cached(&self.content, self.font_size, self.line_height, max_width)
        };
        cx.set_intrinsic_size(id, measured);
        (
            id,
            TextState {
                layout_id: id,
                measured_size: measured,
                buffer: Some(buffer),
                cache_key: Some(cache_key),
                hitbox_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut TextState, cx: &mut PrepaintContext) {
        // Register hitbox with computed bounds from layout
        // Text is not interactive - use opaque:false so it doesn't capture mouse events
        // This allows parent elements (buttons, etc.) to receive hover/click through text
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, false);
        state.hitbox_id = Some(hitbox_id);
    }

    fn paint(&mut self, state: &mut TextState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        if let Some(buffer) = state.buffer.take() {
            if let Some(key) = state.cache_key.take() {
                cx.paint_text_cached(buffer, &self.color, &bounds, key);
            } else {
                cx.paint_text(buffer, &self.color, &bounds);
            }
        }
    }
}
