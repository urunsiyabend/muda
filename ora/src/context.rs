use crate::entity::{Entity, EntityStorage};

/// Application context providing access to entity storage.
/// This is the minimal context for the on_open callback.
/// Full context hierarchy comes in Plan 02.
pub struct AppContext {
    entity_storage: EntityStorage,
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
