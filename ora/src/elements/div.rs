use crate::animation::easing::Easing;
use crate::animation::transition::{TransitionConfig, TransitionId, TransitionSpec};
use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;

/// Tracks which transition property was most recently configured via a builder
/// method. Used by `.easing()` to apply the easing to the correct property.
#[derive(Clone, Copy)]
enum TransitionProperty {
    Bg,
    Opacity,
    Color,
    Position,
}

/// Styled rectangle container element.
/// Implements flexbox layout with background, border, border-radius, and box shadow.
pub struct Div {
    style: Style,
    children: Vec<AnyElement>,
    focus_handle: Option<FocusHandle>,
    // Interactive styling
    hover_bg: Option<Color>,
    active_bg: Option<Color>,
    focus_ring_color: Option<Color>,
    // Transition support
    transition_id: Option<TransitionId>,
    transition_spec: Option<TransitionSpec>,
    last_transition_property: Option<TransitionProperty>,
}

impl Div {
    pub fn new() -> Self {
        Div {
            style: Style::default(),
            children: Vec::new(),
            focus_handle: None,
            hover_bg: None,
            active_bg: None,
            focus_ring_color: None,
            transition_id: None,
            transition_spec: None,
            last_transition_property: None,
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

    // CSS-style aliases
    pub fn row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn column(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    // CSS-style alignment methods
    pub fn justify(mut self, j: JustifyContent) -> Self {
        self.style.justify_content = j;
        self
    }

    pub fn items(mut self, a: AlignItems) -> Self {
        self.style.align_items = a;
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
    pub fn w(mut self, w: impl Into<Length>) -> Self {
        self.style.width = w.into();
        self
    }

    pub fn h(mut self, h: impl Into<Length>) -> Self {
        self.style.height = h.into();
        self
    }

    pub fn min_w(mut self, w: impl Into<Length>) -> Self {
        self.style.min_width = w.into();
        self
    }

    pub fn min_h(mut self, h: impl Into<Length>) -> Self {
        self.style.min_height = h.into();
        self
    }

    pub fn max_w(mut self, w: impl Into<Length>) -> Self {
        self.style.max_width = w.into();
        self
    }

    pub fn max_h(mut self, h: impl Into<Length>) -> Self {
        self.style.max_height = h.into();
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

    pub fn pl(mut self, l: f32) -> Self {
        self.style.padding.left = l;
        self
    }

    pub fn pr(mut self, r: f32) -> Self {
        self.style.padding.right = r;
        self
    }

    pub fn pt(mut self, t: f32) -> Self {
        self.style.padding.top = t;
        self
    }

    pub fn pb(mut self, b: f32) -> Self {
        self.style.padding.bottom = b;
        self
    }

    pub fn m(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }

    pub fn mx(mut self, x: f32) -> Self {
        self.style.margin.left = x;
        self.style.margin.right = x;
        self
    }

    pub fn my(mut self, y: f32) -> Self {
        self.style.margin.top = y;
        self.style.margin.bottom = y;
        self
    }

    pub fn mt(mut self, t: f32) -> Self {
        self.style.margin.top = t;
        self
    }

    pub fn mb(mut self, b: f32) -> Self {
        self.style.margin.bottom = b;
        self
    }

    pub fn ml(mut self, l: f32) -> Self {
        self.style.margin.left = l;
        self
    }

    pub fn mr(mut self, r: f32) -> Self {
        self.style.margin.right = r;
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

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.style.border.widths.right = width;
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

    // Interactive styling
    pub fn hover_bg(mut self, color: Color) -> Self {
        self.hover_bg = Some(color);
        self
    }

    pub fn active_bg(mut self, color: Color) -> Self {
        self.active_bg = Some(color);
        self
    }

    pub fn focus_ring(mut self, color: Color) -> Self {
        self.focus_ring_color = Some(color);
        self
    }

    // Transition builders

    /// Assign a stable cross-frame identity for transition state lookup.
    /// Elements are recreated each frame; this ID persists in the TransitionRegistry.
    pub fn transition_id(mut self, id: TransitionId) -> Self {
        self.transition_id = Some(id);
        self
    }

    /// Animate the background color over `duration_ms` milliseconds (EaseOut default).
    /// Chain `.easing(Easing::Linear)` to override the easing function.
    pub fn transition_bg(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .bg = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut,
            });
        self.last_transition_property = Some(TransitionProperty::Bg);
        self
    }

    /// Animate the opacity over `duration_ms` milliseconds (EaseOut default).
    pub fn transition_opacity(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .opacity = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut,
            });
        self.last_transition_property = Some(TransitionProperty::Opacity);
        self
    }

    /// Animate the text/border color over `duration_ms` milliseconds (EaseOut default).
    pub fn transition_color(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .color = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut,
            });
        self.last_transition_property = Some(TransitionProperty::Color);
        self
    }

    /// Animate the paint-time position offset over `duration_ms` milliseconds (EaseOut default).
    pub fn transition_position(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .position = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut,
            });
        self.last_transition_property = Some(TransitionProperty::Position);
        self
    }

    /// Override the easing on the most recently configured transition property.
    ///
    /// Example: `.transition_bg(200).easing(Easing::EaseIn)`
    pub fn easing(mut self, easing: Easing) -> Self {
        if let (Some(spec), Some(prop)) = (&mut self.transition_spec, self.last_transition_property) {
            match prop {
                TransitionProperty::Bg => {
                    if let Some(c) = &mut spec.bg { c.easing = easing; }
                }
                TransitionProperty::Opacity => {
                    if let Some(c) = &mut spec.opacity { c.easing = easing; }
                }
                TransitionProperty::Color => {
                    if let Some(c) = &mut spec.color { c.easing = easing; }
                }
                TransitionProperty::Position => {
                    if let Some(c) = &mut spec.position { c.easing = easing; }
                }
            }
        }
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
    focus_id: Option<FocusId>,
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
            focus_id: None,
        })
    }

    fn prepaint(&mut self, state: &mut DivState, cx: &mut PrepaintContext) {
        // Register hitbox with computed bounds from layout
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Register as focusable if focus handle provided and store focus_id
        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
            state.focus_id = Some(focus_handle.id);
        }

        // Push this hitbox as parent so children establish parent chain for bubbling
        cx.push_hitbox_parent(hitbox_id);
        for child in &mut self.children {
            child.prepaint(cx);
        }
        cx.pop_hitbox_parent();
    }

    fn paint(&mut self, state: &mut DivState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Build style with interactive states applied
        let mut style = self.style.clone();

        // Determine base background color for transition interpolation
        let base_bg = match &self.style.background {
            Background::Solid(c) => *c,
            _ => Color::transparent(),
        };

        // Determine the target background based on interaction state
        let target_bg = if let Some(hitbox_id) = state.hitbox_id {
            if cx.is_active(hitbox_id) {
                self.active_bg.unwrap_or(base_bg)
            } else if cx.is_hovered(hitbox_id) {
                self.hover_bg.unwrap_or(base_bg)
            } else {
                base_bg
            }
        } else {
            base_bg
        };

        // Apply background: either via transition interpolation or instant switch
        let final_bg = if let (Some(tid), Some(spec)) = (self.transition_id, &self.transition_spec) {
            if let Some(bg_config) = &spec.bg {
                cx.advance_transition_bg(tid, target_bg, bg_config)
            } else {
                target_bg
            }
        } else {
            target_bg
        };

        // Only set background if we changed it from the base (preserves non-Solid backgrounds
        // when no transitions or interactive states are active)
        if final_bg != base_bg || self.transition_id.is_some() || state.hitbox_id.is_some() {
            style.background = Background::Solid(final_bg);
        }

        // Apply focus ring if keyboard-focused
        if let Some(focus_id) = state.focus_id {
            if cx.is_focused(focus_id) {
                if let Some(focus_color) = self.focus_ring_color {
                    style.border.widths = Edges::all(2.0);
                    style.border.color = focus_color;
                }
            }
        }

        // Emit PaintCommand::StyledRect with computed style
        cx.paint_styled_rect(&style, &bounds);

        // If overflow is hidden or scroll, clip children to this div's bounds
        let clipping = matches!(self.style.overflow, Overflow::Hidden | Overflow::Scroll);
        if clipping {
            cx.push_clip(bounds);
        }

        // Paint children
        for child in &mut self.children {
            child.paint(cx);
        }

        if clipping {
            cx.pop_clip();
        }
    }
}
