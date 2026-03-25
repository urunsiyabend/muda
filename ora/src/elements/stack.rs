use crate::element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::style::*;

/// Stack container for z-axis layering of children.
/// All children are positioned at the same origin and paint in order (first = bottom, last = top).
///
/// # Implementation
///
/// Each child is automatically wrapped in an absolutely-positioned container with `position: absolute`,
/// `top: 0`, and `left: 0`. This leverages the existing Position::Absolute support in the flexbox
/// layout system (see `ora/src/layout/flexbox.rs:layout_absolute_child`), which positions absolute
/// children at their specified offsets relative to the parent.
///
/// Since all children have the same offset (0, 0), they overlay at the same position, creating the
/// z-layering effect. Paint order determines z-index: first child paints first (bottom layer),
/// last child paints last (top layer).
///
/// # Example
///
/// ```rust,ignore
/// stack()
///     .w(200.0)
///     .h(200.0)
///     .child(div().bg(Color::rgb(1.0, 0.0, 0.0))) // Red background (bottom)
///     .child(div().w(100.0).h(100.0).bg(Color::rgb(0.0, 0.0, 1.0))) // Blue square (top)
/// ```
pub struct Stack {
    style: Style,
    children: Vec<AnyElement>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            style: Style::default(),
            children: Vec::new(),
        }
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

    // Sizing - accepts impl Into<Length> for flexibility (px, pct, etc.)
    pub fn w(mut self, w: impl Into<Length>) -> Self {
        self.style.width = w.into();
        self
    }

    pub fn h(mut self, h: impl Into<Length>) -> Self {
        self.style.height = h.into();
        self
    }

    // Visual
    pub fn bg(mut self, color: Color) -> Self {
        self.style.background = Background::Solid(color);
        self
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

/// Constructor function for Stack
pub fn stack() -> Stack {
    Stack::new()
}

/// Internal wrapper element that positions a child absolutely at (0, 0)
struct AbsoluteWrapper {
    style: Style,
    child: Option<AnyElement>,
}

impl AbsoluteWrapper {
    fn new(child: AnyElement) -> Self {
        let mut style = Style::default();
        style.position = Position::Absolute;
        style.top = Length::Px(0.0);
        style.left = Length::Px(0.0);

        AbsoluteWrapper {
            style,
            child: Some(child),
        }
    }
}

struct AbsoluteWrapperState {
    layout_id: LayoutId,
}

impl Element for AbsoluteWrapper {
    type RequestLayoutState = AbsoluteWrapperState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, AbsoluteWrapperState) {
        let id = cx.request_layout(&self.style);

        if let Some(child) = &mut self.child {
            let child_id = child.request_layout(cx);
            cx.add_child(id, child_id);
        }

        (id, AbsoluteWrapperState { layout_id: id })
    }

    fn prepaint(&mut self, _state: &mut AbsoluteWrapperState, cx: &mut PrepaintContext) {
        if let Some(child) = &mut self.child {
            child.prepaint(cx);
        }
    }

    fn paint(&mut self, _state: &mut AbsoluteWrapperState, cx: &mut PaintContext) {
        if let Some(child) = &mut self.child {
            child.paint(cx);
        }
    }
}

/// State persisted through the rendering lifecycle
pub struct StackState {
    layout_id: LayoutId,
}

impl Element for Stack {
    type RequestLayoutState = StackState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, StackState) {
        let id = cx.request_layout(&self.style);

        // Wrap each child in an AbsoluteWrapper to position them at (0, 0)
        // This creates the overlapping/z-layering effect
        for child in std::mem::take(&mut self.children) {
            let wrapper = AbsoluteWrapper::new(child);
            let mut wrapped_element = AnyElement::new(wrapper);
            let wrapped_id = wrapped_element.request_layout(cx);
            cx.add_child(id, wrapped_id);

            // Store wrapped element back for prepaint/paint phases
            self.children.push(wrapped_element);
        }

        (id, StackState { layout_id: id })
    }

    fn prepaint(&mut self, _state: &mut StackState, cx: &mut PrepaintContext) {
        // Prepaint wrapped children
        for child in &mut self.children {
            child.prepaint(cx);
        }
    }

    fn paint(&mut self, _state: &mut StackState, cx: &mut PaintContext) {
        // Paint children in order (first = bottom, last = top)
        // This creates z-layering through paint order
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}
