use crate::context::AppContext;
use crate::effect::EntityId;
use crate::entity::Entity;
use std::any::{Any, TypeId};
use std::marker::PhantomData;

/// A reactive entity wrapper that supports observers and event subscriptions.
///
/// Model<T> wraps Entity<T> and provides a read/update API that integrates
/// with the effect queue system for reactive updates.
#[derive(Clone, Copy)]
pub struct Model<T> {
    pub(crate) entity: Entity<T>,
}

impl<T: 'static> Model<T> {
    /// Create a new Model wrapping an Entity.
    pub(crate) fn new(entity: Entity<T>) -> Self {
        Self { entity }
    }

    /// Read the model's data immutably.
    pub fn read<'a>(&self, cx: &'a AppContext) -> &'a T {
        cx.read_model(self)
    }

    /// Update the model's data mutably.
    ///
    /// The closure receives mutable access to the data and a ModelContext
    /// for queueing effects (notify/emit).
    pub fn update<R>(&self, cx: &mut AppContext, f: impl FnOnce(&mut T, &mut ModelContext<T>) -> R) -> R {
        cx.update_model(self, f)
    }

    /// Get the underlying entity ID.
    pub(crate) fn entity_id(&self) -> EntityId {
        self.entity.index
    }
}

impl<T> std::fmt::Debug for Model<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("type", &std::any::type_name::<T>())
            .field("entity", &self.entity)
            .finish()
    }
}

/// Context provided during Model updates for queueing effects.
///
/// Effects are queued locally and drained into AppContext.effect_queue
/// after the update closure completes.
pub struct ModelContext<'a, T> {
    entity_id: EntityId,
    pending_effects: Vec<PendingEffect>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> ModelContext<'a, T> {
    /// Create a new ModelContext for the given entity.
    pub(crate) fn new(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            pending_effects: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Queue a notify effect to trigger observers.
    pub fn notify(&mut self) {
        self.pending_effects.push(PendingEffect::Notify);
    }

    /// Queue an emit effect to send a typed event to subscribers.
    pub fn emit<E: 'static>(&mut self, event: E) {
        self.pending_effects.push(PendingEffect::Emit {
            event_type_id: TypeId::of::<E>(),
            event: Box::new(event),
        });
    }

    /// Drain all pending effects, pairing each with the entity ID.
    pub(crate) fn drain(&mut self) -> Vec<(EntityId, PendingEffect)> {
        self.pending_effects
            .drain(..)
            .map(|effect| (self.entity_id, effect))
            .collect()
    }
}

/// Effects queued during a model update.
#[derive(Debug)]
pub(crate) enum PendingEffect {
    /// Notify observers of this entity.
    Notify,
    /// Emit a typed event.
    Emit {
        event_type_id: TypeId,
        event: Box<dyn Any>,
    },
}
