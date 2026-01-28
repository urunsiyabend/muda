use super::trait_def::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};

/// Internal trait for type-erasing Element implementations.
/// This allows storing heterogeneous elements in collections.
pub(crate) trait ElementObject {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId;
    fn prepaint(&mut self, cx: &mut PrepaintContext);
    fn paint(&mut self, cx: &mut PaintContext);
}

/// Wrapper that implements ElementObject for any Element + its RequestLayoutState.
/// The state is stored in an Option and populated during request_layout.
struct ElementObjectImpl<E: Element> {
    element: E,
    state: Option<E::RequestLayoutState>,
}

impl<E: Element> ElementObject for ElementObjectImpl<E> {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let (layout_id, state) = self.element.request_layout(cx);
        self.state = Some(state);
        layout_id
    }

    fn prepaint(&mut self, cx: &mut PrepaintContext) {
        if let Some(state) = &mut self.state {
            self.element.prepaint(state, cx);
        }
    }

    fn paint(&mut self, cx: &mut PaintContext) {
        if let Some(state) = &mut self.state {
            self.element.paint(state, cx);
        }
    }
}

/// Type-erased element that can hold any Element implementation.
/// Supports children for building element trees.
pub struct AnyElement {
    inner: Box<dyn ElementObject>,
    children: Vec<AnyElement>,
}

impl AnyElement {
    /// Create a new AnyElement wrapping the given element.
    pub fn new<E: Element>(element: E) -> Self {
        Self {
            inner: Box::new(ElementObjectImpl {
                element,
                state: None,
            }),
            children: Vec::new(),
        }
    }

    /// Add children to this element.
    pub fn with_children(mut self, children: Vec<AnyElement>) -> Self {
        self.children = children;
        self
    }

    /// Request layout for this element and all children.
    /// Called first in the rendering lifecycle.
    pub(crate) fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Request layout for self
        let layout_id = self.inner.request_layout(cx);

        // Request layout for all children
        for child in &mut self.children {
            child.request_layout(cx);
        }

        layout_id
    }

    /// Prepaint this element and all children.
    /// Called after layout is resolved.
    pub(crate) fn prepaint(&mut self, cx: &mut PrepaintContext) {
        // Prepaint self
        self.inner.prepaint(cx);

        // Prepaint all children
        for child in &mut self.children {
            child.prepaint(cx);
        }
    }

    /// Paint this element and all children.
    /// Called last, produces GPU rendering commands.
    pub(crate) fn paint(&mut self, cx: &mut PaintContext) {
        // Paint self
        self.inner.paint(cx);

        // Paint all children
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}

/// Ergonomic conversion from any Element to AnyElement.
impl<E: Element> From<E> for AnyElement {
    fn from(element: E) -> Self {
        AnyElement::new(element)
    }
}
