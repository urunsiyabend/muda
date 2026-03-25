use crate::context::{AppContext, ViewContext};
use crate::element::AnyElement;
use crate::entity::{Entity, EntityStorage};
use crate::view::{AnyView, View};

/// Internal wrapper to store views as trait objects.
/// This allows heterogeneous view storage while maintaining the ability to render.
struct ViewBox {
    view: Box<dyn AnyView>,
}

/// Window that manages the root view and rendering.
pub struct OraWindow {
    /// The root view, stored as an entity for lifecycle management.
    root_view_entity: Option<Entity<ViewBox>>,
}

impl OraWindow {
    /// Create a new window with no root view.
    pub fn new() -> Self {
        Self {
            root_view_entity: None,
        }
    }

    /// Set the root view for this window.
    /// Stores the view in entity storage as a type-erased ViewBox.
    pub(crate) fn set_root_view<V: View>(&mut self, view: V, storage: &mut EntityStorage) {
        let view_box = ViewBox {
            view: Box::new(view),
        };
        let entity = storage.insert(view_box);
        self.root_view_entity = Some(entity);
    }

    /// Render the root view into an element tree.
    /// Returns None if no root view is set.
    pub(crate) fn render(&mut self, app_context: &mut AppContext) -> Option<AnyElement> {
        let entity = self.root_view_entity.as_ref()?;

        // We need to get a reference to the view and then call render
        // This requires some unsafe code or a refactoring to avoid the borrow checker issue
        // For now, we'll use a simpler approach: read the entity, create context, call render

        // SAFETY: We're using raw pointers to work around the borrow checker.
        // This is safe because:
        // 1. The ViewBox reference is only used to access the trait object
        // 2. The ViewContext doesn't modify the ViewBox itself
        // 3. The view's render() method only reads its own state
        let view_box_ptr = app_context.entity_storage.read(entity) as *const ViewBox;
        let mut view_context = ViewContext::new(app_context);

        unsafe {
            Some((*view_box_ptr).view.render(&mut view_context))
        }
    }

    /// Check if a root view is set.
    pub(crate) fn has_root_view(&self) -> bool {
        self.root_view_entity.is_some()
    }
}

impl Default for OraWindow {
    fn default() -> Self {
        Self::new()
    }
}
