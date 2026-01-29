use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::style::*;

/// Text rendering element with glyphon-based measurement and rendering.
pub struct TextElement {
    content: String,
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
}

/// State persisted through the rendering lifecycle
pub struct TextState {
    layout_id: LayoutId,
    measured_size: Size<f32>,
    buffer: Option<glyphon::Buffer>,
}

impl Element for TextElement {
    type RequestLayoutState = TextState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, TextState) {
        let id = cx.request_layout(&self.style);
        let max_width = match self.style.width {
            Length::Px(w) => Some(w),
            _ => None,
        };
        let (buffer, measured) = cx.measure_text(&self.content, self.font_size, self.line_height, max_width);
        cx.set_intrinsic_size(id, measured);
        (
            id,
            TextState {
                layout_id: id,
                measured_size: measured,
                buffer: Some(buffer),
            },
        )
    }

    fn prepaint(&mut self, _state: &mut TextState, _cx: &mut PrepaintContext) {}

    fn paint(&mut self, state: &mut TextState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        if let Some(buffer) = state.buffer.take() {
            cx.paint_text(buffer, &self.color, &bounds);
        }
    }
}
