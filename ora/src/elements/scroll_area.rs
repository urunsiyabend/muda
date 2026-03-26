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
}

/// Constructor.
pub fn scroll_area(scroll: SharedScrollState) -> ScrollArea {
    ScrollArea {
        children: Vec::new(),
        scroll,
        bg: None,
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
}

pub struct ScrollAreaState {
    layout_id: LayoutId,
    content_layout_id: LayoutId,
    hitbox_id: Option<HitboxId>,
}

impl Element for ScrollArea {
    type RequestLayoutState = ScrollAreaState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Container: fills available space
        let mut container_style = Style::default();
        container_style.display = Display::Flex;
        container_style.flex_direction = FlexDirection::Column;
        container_style.flex_grow = 1.0;
        container_style.overflow = Overflow::Hidden;
        let container_id = cx.request_layout(&container_style);

        // Inner content wrapper: unconstrained height, shifted by scroll offset
        let mut content_style = Style::default();
        content_style.display = Display::Flex;
        content_style.flex_direction = FlexDirection::Column;
        // Negative top margin shifts content up — same pattern as TextAreaView
        content_style.margin.top = -self.scroll.get();
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

        // max_scroll: how far content can scroll before bottom is reached
        // content_bounds.size.height includes the negative margin, so actual
        // content height = content_bounds.size.height + current scroll offset
        let current_scroll = self.scroll.get();
        let actual_content_h = content_bounds.size.height + current_scroll;
        let viewport_h = bounds.size.height;
        let max_scroll = (actual_content_h - viewport_h).max(0.0);

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
            // Always consume scroll events on a scroll area
            ctx.stop_propagation();
        });

        for child in &mut self.children {
            child.prepaint(cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        if let Some(bg) = self.bg {
            let mut style = Style::default();
            style.background = Background::Solid(bg);
            cx.paint_styled_rect(&style, &bounds);
        }

        // Clip to viewport
        cx.push_clip(bounds);

        for child in &mut self.children {
            child.paint(cx);
        }

        cx.pop_clip();
    }
}
