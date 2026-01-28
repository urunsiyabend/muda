use std::marker::PhantomData;

/// A typed handle to an entity stored in EntityStorage.
/// This is a lightweight, copyable handle that references an entity by generational index.
#[derive(Clone, Copy)]
pub struct Entity<T> {
    pub(crate) index: generational_arena::Index,
    _phantom: PhantomData<T>,
}

impl<T> Entity<T> {
    pub(crate) fn new(index: generational_arena::Index) -> Self {
        Self {
            index,
            _phantom: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Entity<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entity")
            .field("type", &std::any::type_name::<T>())
            .field("index", &self.index)
            .finish()
    }
}
