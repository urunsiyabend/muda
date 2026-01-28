use crate::entity::Entity;
use generational_arena::Arena;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Centralized entity storage using generational arenas.
/// Each type T gets its own Arena<T>, stored in a type-erased HashMap.
pub struct EntityStorage {
    arenas: HashMap<TypeId, Box<dyn Any>>,
}

impl EntityStorage {
    /// Create a new empty entity storage.
    pub fn new() -> Self {
        Self {
            arenas: HashMap::new(),
        }
    }

    /// Get or create the arena for type T.
    fn get_or_create_arena<T: 'static>(&mut self) -> &mut Arena<T> {
        self.arenas
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Arena::<T>::new()))
            .downcast_mut::<Arena<T>>()
            .expect("Type mismatch in arena storage")
    }

    /// Get the arena for type T, if it exists.
    fn get_arena<T: 'static>(&self) -> Option<&Arena<T>> {
        self.arenas
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<Arena<T>>())
    }

    /// Get the arena for type T mutably, if it exists.
    fn get_arena_mut<T: 'static>(&mut self) -> Option<&mut Arena<T>> {
        self.arenas
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut::<Arena<T>>())
    }

    /// Insert a new entity and return a handle to it.
    pub fn insert<T: 'static>(&mut self, value: T) -> Entity<T> {
        let arena = self.get_or_create_arena::<T>();
        let index = arena.insert(value);
        Entity::new(index)
    }

    /// Read an entity by handle.
    /// Panics if the entity is invalid (dangling or removed).
    pub fn read<T: 'static>(&self, entity: &Entity<T>) -> &T {
        self.get_arena::<T>()
            .and_then(|arena| arena.get(entity.index))
            .unwrap_or_else(|| {
                panic!(
                    "Entity<{}> handle is invalid (dangling or removed)",
                    std::any::type_name::<T>()
                )
            })
    }

    /// Read an entity by handle, returning None if invalid.
    pub fn read_opt<T: 'static>(&self, entity: &Entity<T>) -> Option<&T> {
        self.get_arena::<T>()
            .and_then(|arena| arena.get(entity.index))
    }

    /// Update an entity by handle, applying the given closure.
    /// Returns the result of the closure.
    /// Panics if the entity is invalid (dangling or removed).
    pub fn update<T: 'static, R>(&mut self, entity: &Entity<T>, f: impl FnOnce(&mut T) -> R) -> R {
        self.get_arena_mut::<T>()
            .and_then(|arena| arena.get_mut(entity.index))
            .map(f)
            .unwrap_or_else(|| {
                panic!(
                    "Entity<{}> handle is invalid (dangling or removed)",
                    std::any::type_name::<T>()
                )
            })
    }

    /// Remove an entity by handle, returning the value if it exists.
    pub fn remove<T: 'static>(&mut self, entity: &Entity<T>) -> Option<T> {
        self.get_arena_mut::<T>()
            .and_then(|arena| arena.remove(entity.index))
    }
}

impl Default for EntityStorage {
    fn default() -> Self {
        Self::new()
    }
}
