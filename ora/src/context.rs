use crate::entity::{Entity, EntityStorage};
use crate::view::View;
use crate::window::OraWindow;
use std::sync::Arc;
use winit::window::Window;

/// Application context providing access to entity storage.
/// Owns the entity storage and provides methods for entity management.
pub struct AppContext {
    pub(crate) entity_storage: EntityStorage,
}

impl AppContext {
    pub(crate) fn new(entity_storage: EntityStorage) -> Self {
        Self { entity_storage }
    }

    pub(crate) fn into_storage(self) -> EntityStorage {
        self.entity_storage
    }

    /// Insert a new entity and return a handle to it.
    pub fn insert<T: 'static>(&mut self, value: T) -> Entity<T> {
        self.entity_storage.insert(value)
    }

    /// Read an entity by handle.
    pub fn read<T: 'static>(&self, entity: &Entity<T>) -> &T {
        self.entity_storage.read(entity)
    }

    /// Update an entity by handle.
    pub fn update<T: 'static, R>(&mut self, entity: &Entity<T>, f: impl FnOnce(&mut T) -> R) -> R {
        self.entity_storage.update(entity, f)
    }

    /// Remove an entity by handle.
    pub fn remove<T: 'static>(&mut self, entity: &Entity<T>) -> Option<T> {
        self.entity_storage.remove(entity)
    }
}

/// View context for rendering views.
/// Provides access to entity storage for reading/updating state.
/// This is a simplified non-generic version for Phase 1.
pub struct ViewContext<'a> {
    app_context: &'a mut AppContext,
}

impl<'a> ViewContext<'a> {
    pub(crate) fn new(app_context: &'a mut AppContext) -> Self {
        Self { app_context }
    }

    /// Insert a new entity and return a handle to it.
    pub fn insert<T: 'static>(&mut self, value: T) -> Entity<T> {
        self.app_context.insert(value)
    }

    /// Read an entity by handle.
    pub fn read<T: 'static>(&self, entity: &Entity<T>) -> &T {
        self.app_context.read(entity)
    }

    /// Update an entity by handle.
    pub fn update<T: 'static, R>(&mut self, entity: &Entity<T>, f: impl FnOnce(&mut T) -> R) -> R {
        self.app_context.update(entity, f)
    }

    /// Notify that this view needs to be re-rendered.
    /// Stub for Phase 3 reactivity - does nothing in Phase 1.
    pub fn notify(&mut self) {
        // Phase 3 will implement reactivity tracking here
    }
}

/// Window context for window-level operations.
/// Provides entity access plus window-specific methods.
pub struct WindowContext<'a> {
    app_context: &'a mut AppContext,
    ora_window: &'a mut OraWindow,
    winit_window: &'a Arc<Window>,
}

impl<'a> WindowContext<'a> {
    pub(crate) fn new(
        app_context: &'a mut AppContext,
        ora_window: &'a mut OraWindow,
        winit_window: &'a Arc<Window>,
    ) -> Self {
        Self {
            app_context,
            ora_window,
            winit_window,
        }
    }

    /// Insert a new entity and return a handle to it.
    pub fn insert<T: 'static>(&mut self, value: T) -> Entity<T> {
        self.app_context.insert(value)
    }

    /// Read an entity by handle.
    pub fn read<T: 'static>(&self, entity: &Entity<T>) -> &T {
        self.app_context.read(entity)
    }

    /// Update an entity by handle.
    pub fn update<T: 'static, R>(&mut self, entity: &Entity<T>, f: impl FnOnce(&mut T) -> R) -> R {
        self.app_context.update(entity, f)
    }

    /// Set the root view for this window.
    /// The view is stored in entity storage and wrapped for type erasure.
    pub fn set_root_view<V: View>(&mut self, view: V) {
        self.ora_window
            .set_root_view(view, &mut self.app_context.entity_storage);
    }

    /// Request the window to be redrawn.
    pub fn request_redraw(&self) {
        self.winit_window.request_redraw();
    }
}
