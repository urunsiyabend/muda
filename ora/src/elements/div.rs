use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::events::focus::FocusHandle;
use crate::events::mouse::HitboxId;
use crate::style::*;

/// Styled rectangle container element.
/// Implements flexbox layout with background, border, border-radius, and box shadow.
pub struct Div {
    style: Style,
    children: Vec<AnyElement>,
    focus_handle: Option<FocusHandle>,
}

impl Div {
    pub fn new() -> Self {
        Div {
            style: Style::default(),
            children: Vec::new(),
            focus_handle: None,
        }
    }

    // Layout builders
    pub fn flex(mut self) -> Self {
        self.style.display = Display::Flex;
        self
    }

    pub fn flex_row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = gap;
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.style.justify_content = JustifyContent::Center;
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.style.justify_content = JustifyContent::End;
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceBetween;
        self
    }

    pub fn align_center(mut self) -> Self {
        self.style.align_items = AlignItems::Center;
        self
    }

    pub fn align_end(mut self) -> Self {
        self.style.align_items = AlignItems::End;
        self
    }

    pub fn align_stretch(mut self) -> Self {
        self.style.align_items = AlignItems::Stretch;
        self
    }

    pub fn grow(mut self, v: f32) -> Self {
        self.style.flex_grow = v;
        self
    }

    pub fn shrink(mut self, v: f32) -> Self {
        self.style.flex_shrink = v;
        self
    }

    // Sizing builders
    pub fn w(mut self, w: f32) -> Self {
        self.style.width = Length::Px(w);
        self
    }

    pub fn h(mut self, h: f32) -> Self {
        self.style.height = Length::Px(h);
        self
    }

    pub fn w_pct(mut self, p: f32) -> Self {
        self.style.width = Length::Percent(p);
        self
    }

    pub fn h_pct(mut self, p: f32) -> Self {
        self.style.height = Length::Percent(p);
        self
    }

    pub fn min_w(mut self, w: f32) -> Self {
        self.style.min_width = Length::Px(w);
        self
    }

    pub fn min_h(mut self, h: f32) -> Self {
        self.style.min_height = Length::Px(h);
        self
    }

    pub fn max_w(mut self, w: f32) -> Self {
        self.style.max_width = Length::Px(w);
        self
    }

    pub fn max_h(mut self, h: f32) -> Self {
        self.style.max_height = Length::Px(h);
        self
    }

    // Spacing builders
    pub fn p(mut self, p: f32) -> Self {
        self.style.padding = Edges::all(p);
        self
    }

    pub fn px(mut self, x: f32) -> Self {
        self.style.padding.left = x;
        self.style.padding.right = x;
        self
    }

    pub fn py(mut self, y: f32) -> Self {
        self.style.padding.top = y;
        self.style.padding.bottom = y;
        self
    }

    pub fn m(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }

    // Visual builders
    pub fn bg(mut self, color: Color) -> Self {
        self.style.background = Background::Solid(color);
        self
    }

    pub fn bg_gradient(mut self, start: Color, end: Color, angle: f32) -> Self {
        self.style.background = Background::Linear(Gradient {
            start,
            end,
            angle_radians: angle,
        });
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.style.border.widths = Edges::all(width);
        self.style.border.color = color;
        self
    }

    pub fn border_radius(mut self, r: f32) -> Self {
        self.style.border_radius = Corners::all(r);
        self
    }

    pub fn shadow(mut self, ox: f32, oy: f32, blur: f32, spread: f32, color: Color) -> Self {
        self.style.box_shadow = Some(BoxShadow {
            offset_x: ox,
            offset_y: oy,
            blur,
            spread,
            color,
        });
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.style.overflow = Overflow::Hidden;
        self
    }

    pub fn overflow_scroll(mut self) -> Self {
        self.style.overflow = Overflow::Scroll;
        self
    }

    // Children
    pub fn child(mut self, child: impl Into<AnyElement>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }

    // Focus
    pub fn focusable(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }
}

impl Default for Div {
    fn default() -> Self {
        Self::new()
    }
}

/// State persisted through the rendering lifecycle
pub struct DivState {
    layout_id: LayoutId,
    hitbox_id: Option<HitboxId>,
}

impl Element for Div {
    type RequestLayoutState = DivState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, DivState) {
        let id = cx.request_layout(&self.style);
        // Register children
        for child in &mut self.children {
            let child_id = child.request_layout(cx);
            cx.add_child(id, child_id);
        }
        (id, DivState {
            layout_id: id,
            hitbox_id: None,
        })
    }

    fn prepaint(&mut self, state: &mut DivState, cx: &mut PrepaintContext) {
        // Register hitbox with computed bounds from layout
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Register as focusable if focus handle provided
        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
        }

        // Prepaint children
        for child in &mut self.children {
            child.prepaint(cx);
        }
    }

    fn paint(&mut self, state: &mut DivState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        // Emit PaintCommand::StyledRect with style properties and computed bounds
        cx.paint_styled_rect(&self.style, &bounds);
        // Paint children
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}
