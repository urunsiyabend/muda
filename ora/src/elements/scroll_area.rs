//! Scrollable container element.
//!
//! Wraps child content in a clipped viewport with mouse wheel scrolling.
//! Content that exceeds the viewport height can be scrolled vertically.
//! Scroll offset is clamped to [0, content_height - viewport_height].

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::events::dispatch::DispatchPhase;
use crate::events::mouse::HitboxId;
use crate::style::*;

/// Shared scroll state that persists across frames.
pub type SharedScrollState = Rc<Cell<f32>>;

/// Create a new shared scroll state (initialized to 0).
pub fn scroll_state() -> SharedScrollState {
    Rc::new(Cell::new(0.0))
}

/// A scrollable container that clips overflow and handles mouse wheel events.
pub struct ScrollArea {
    children: Vec<AnyElement>,
    scroll: SharedScrollState,
    bg: Option<Color>,
    max_height: Option<f32>,
}

/// Constructor.
pub fn scroll_area(scroll: SharedScrollState) -> ScrollArea {
    ScrollArea {
        children: Vec::new(),
        scroll,
        bg: None,
        max_height: None,
    }
}

impl ScrollArea {
    pub fn child(mut self, child: impl Into<AnyElement>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set explicit max height for the viewport (required when layout
    /// engine doesn't constrain the container from parent).
    pub fn max_h(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
}

pub struct ScrollAreaState {
    layout_id: LayoutId,
    content_layout_id: LayoutId,
    hitbox_id: Option<HitboxId>,
}

impl Element for ScrollArea {
    type RequestLayoutState = ScrollAreaState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Container: fixed or bounded height viewport
        let mut container_style = Style::default();
        container_style.display = Display::Flex;
        container_style.flex_direction = FlexDirection::Column;
        container_style.flex_grow = 1.0;
        container_style.overflow = Overflow::Hidden;
        if let Some(h) = self.max_height {
            container_style.height = Length::Px(h);
        }
        let container_id = cx.request_layout(&container_style);

        // Inner content wrapper: unconstrained height, never shrunk by parent
        // NO negative margin — scroll offset applied via PushOffset in paint phase
        let mut content_style = Style::default();
        content_style.display = Display::Flex;
        content_style.flex_direction = FlexDirection::Column;
        content_style.flex_shrink = 0.0;
        let content_id = cx.request_layout(&content_style);
        cx.add_child(container_id, content_id);

        for child in &mut self.children {
            let child_id = child.request_layout(cx);
            cx.add_child(content_id, child_id);
        }

        (container_id, ScrollAreaState {
            layout_id: container_id,
            content_layout_id: content_id,
            hitbox_id: None,
        })
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let content_bounds = cx.bounds(state.content_layout_id);

        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // max_scroll: content that exceeds viewport can be scrolled
        // Half-viewport overscroll at bottom so last items aren't glued to edge
        let current_scroll = self.scroll.get();
        let content_h = content_bounds.size.height;
        let viewport_h = bounds.size.height;
        let bottom_padding = viewport_h * 0.1;
        let max_scroll = (content_h - viewport_h + bottom_padding).max(0.0);

        // Clamp and update
        let clamped = current_scroll.clamp(0.0, max_scroll);
        self.scroll.set(clamped);

        // Register scroll handler
        let scroll_state = self.scroll.clone();
        cx.on_mouse_scroll(hitbox_id, move |event, ctx| {
            if ctx.phase() != DispatchPhase::Bubble {
                return;
            }
            let delta = event.delta.y;
            let cur = scroll_state.get();
            let new_val = (cur + delta).clamp(0.0, max_scroll);
            if (new_val - cur).abs() > 0.01 {
                scroll_state.set(new_val);
            }
            ctx.stop_propagation();
        });

        // Push as parent so child hitboxes bubble scroll events to us.
        // Also push hitbox offset so child hitboxes align with visually-scrolled positions.
        cx.push_hitbox_parent(hitbox_id);
        let scroll_y = self.scroll.get();
        if scroll_y > 0.0 {
            cx.push_hitbox_offset(0.0, -scroll_y);
        }
        for child in &mut self.children {
            child.prepaint(cx);
        }
        if scroll_y > 0.0 {
            cx.pop_hitbox_offset();
        }
        cx.pop_hitbox_parent();
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        if let Some(bg) = self.bg {
            let mut style = Style::default();
            style.background = Background::Solid(bg);
            cx.paint_styled_rect(&style, &bounds);
        }

        // Clip to viewport, then shift content up by scroll offset
        cx.push_clip(bounds);
        let scroll_y = self.scroll.get();
        if scroll_y > 0.0 {
            cx.push_offset(0.0, -scroll_y);
        }

        for child in &mut self.children {
            child.paint(cx);
        }

        if scroll_y > 0.0 {
            cx.pop_offset();
        }
        cx.pop_clip();
    }
}
