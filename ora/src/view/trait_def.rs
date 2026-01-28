use crate::element::AnyElement;
use crate::entity::EntityStorage;

/// Context for rendering views.
/// Provides access to entity storage for reading/updating state.
/// This is a simplified non-generic version for Phase 1.
/// Phase 3 will add generic ViewContext<V> for typed view access.
pub struct ViewContext<'a> {
    pub(crate) entity_storage: &'a mut EntityStorage,
}

impl<'a> ViewContext<'a> {
    pub(crate) fn new(entity_storage: &'a mut EntityStorage) -> Self {
        Self { entity_storage }
    }
}

/// View trait for stateful UI components.
///
/// Views own state and produce element trees via render().
/// Elements are the rendering primitives; views are the stateful components.
///
/// Views are stored in entity storage and accessed via Entity<V> handles.
/// When a view's render() is called, it produces an AnyElement tree that
/// defines the visual representation.
pub trait View: 'static + Sized {
    /// Render this view into an element tree.
    /// Called when the view needs to be displayed or updated.
    fn render(&self, cx: &mut ViewContext) -> AnyElement;
}

/// Internal trait for type-erasing View implementations.
/// Allows storing heterogeneous views (e.g., as root view in Window).
pub(crate) trait AnyView {
    /// Render this view using a non-generic context.
    fn render(&self, cx: &mut ViewContext) -> AnyElement;
}

/// Blanket implementation: any View is also an AnyView.
impl<V: View> AnyView for V {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        View::render(self, cx)
    }
}
