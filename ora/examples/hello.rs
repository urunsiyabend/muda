use ora::{AnyElement, Element, Style, View, ViewContext};

/// A simple element that renders a colored rectangle.
struct ColorRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    style: Style,
}

impl ColorRect {
    fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
            style: Style::default(),
        }
    }
}

/// RequestLayoutState for ColorRect - empty for Phase 1 stub.
struct ColorRectState;

impl Element for ColorRect {
    type RequestLayoutState = ColorRectState;

    fn request_layout(
        &mut self,
        cx: &mut ora::element::LayoutContext,
    ) -> (ora::element::LayoutId, Self::RequestLayoutState) {
        // Request layout with style
        (cx.request_layout(&self.style), ColorRectState)
    }

    fn prepaint(
        &mut self,
        _state: &mut Self::RequestLayoutState,
        _cx: &mut ora::element::PrepaintContext,
    ) {
        // Phase 1 stub: no hitbox registration yet
    }

    fn paint(
        &mut self,
        _state: &mut Self::RequestLayoutState,
        cx: &mut ora::element::PaintContext,
    ) {
        // Draw the colored rectangle
        cx.paint_rect(self.x, self.y, self.width, self.height, self.color);
    }
}

/// A simple view that demonstrates parent-child element composition.
struct HelloView;

impl HelloView {
    fn new() -> Self {
        Self
    }
}

impl View for HelloView {
    fn render(&self, _cx: &mut ViewContext) -> AnyElement {
        // Create a parent element (purple background)
        let parent = ColorRect::new(0.0, 0.0, 800.0, 600.0, [0.5, 0.2, 0.8, 1.0]);

        // Create child elements (smaller colored rectangles)
        let child1 = ColorRect::new(100.0, 100.0, 200.0, 150.0, [0.9, 0.3, 0.2, 1.0]);
        let child2 = ColorRect::new(400.0, 300.0, 250.0, 200.0, [0.2, 0.8, 0.4, 1.0]);

        // Build element tree: parent with children
        // Note: Phase 1 stub rendering only shows the parent's color as the clear color
        // In Phase 2+, proper rendering will composite all rects
        AnyElement::new(parent).with_children(vec![
            AnyElement::new(child1),
            AnyElement::new(child2),
        ])
    }
}

fn main() {
    ora::App::new()
        .title("ora - hello world")
        .size(800, 600)
        .on_open(|cx| {
            cx.set_root_view(HelloView::new());
        })
        .run();
}
