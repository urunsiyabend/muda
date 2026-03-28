//! Paint-phase offset element.
//!
//! Wraps a single child element and applies a Y translation during the paint
//! phase only. Layout is completely unaffected — the child participates in
//! flexbox at its natural position. Only the GPU draw commands are shifted.
//!
//! This is the mechanism used to apply fractional-pixel scroll offsets in
//! TextAreaView and GutterView without changing the element tree structure
//! between frames, which would invalidate the layout cache.

use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};

/// An element that shifts its child's paint output by `(0, dy)` pixels without
/// affecting layout at all.
///
/// The child is laid out normally. Only the paint phase emits `PushOffset` /
/// `PopOffset` commands so the GPU renderer draws the child shifted by `dy`.
pub struct PaintOffsetElement {
    /// Vertical offset to apply during paint (can be negative — shifts up).
    pub dy: f32,
    /// The child element to offset.
    pub child: AnyElement,
}

/// Constructor for ergonomic use.
pub fn paint_offset(dy: f32, child: impl Into<AnyElement>) -> PaintOffsetElement {
    PaintOffsetElement { dy, child: child.into() }
}

/// State produced during request_layout: the child's LayoutId.
pub struct PaintOffsetState {
    child_layout_id: LayoutId,
}

impl Element for PaintOffsetElement {
    type RequestLayoutState = PaintOffsetState;

    fn request_layout(
        &mut self,
        cx: &mut LayoutContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Delegate entirely to the child — PaintOffsetElement has no layout of its own.
        let child_layout_id = self.child.request_layout(cx);
        (child_layout_id, PaintOffsetState { child_layout_id })
    }

    fn prepaint(
        &mut self,
        _state: &mut Self::RequestLayoutState,
        cx: &mut PrepaintContext,
    ) {
        // No hitbox or special prepaint needed — delegate to child.
        self.child.prepaint(cx);
    }

    fn paint(
        &mut self,
        _state: &mut Self::RequestLayoutState,
        cx: &mut PaintContext,
    ) {
        // Apply the Y offset, paint the child, then restore.
        cx.push_offset(0.0, self.dy);
        self.child.paint(cx);
        cx.pop_offset();
    }
}
