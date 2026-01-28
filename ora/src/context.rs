use crate::effect::{EffectQueue, EntityId};
use crate::entity::{Entity, EntityStorage, Model};
use crate::entity::model::{ModelContext, PendingEffect};
use crate::subscription::{CleanupAction, GlobalEventBus, ObserverSet, SubscriberSet, Subscription};
use crate::view::View;
use crate::window::OraWindow;
use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;
use winit::window::Window;

/// Application context providing access to entity storage.
/// Owns the entity storage and provides methods for entity management.
pub struct AppContext {
    pub(crate) entity_storage: EntityStorage,
    pub(crate) effect_queue: EffectQueue,
    pub(crate) observer_set: ObserverSet,
    pub(crate) subscriber_set: SubscriberSet,
    pub(crate) global_event_bus: GlobalEventBus,
    pub(crate) dirty_entities: HashSet<EntityId>,
    pub(crate) update_depth: usize,
}

impl AppContext {
    pub(crate) fn new(entity_storage: EntityStorage) -> Self {
        Self {
            entity_storage,
            effect_queue: EffectQueue::new(),
            observer_set: ObserverSet::new(),
            subscriber_set: SubscriberSet::new(),
            global_event_bus: GlobalEventBus::new(),
            dirty_entities: HashSet::new(),
            update_depth: 0,
        }
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
        let entity_id = entity.index;

        // Clean up all subscriptions for this entity
        self.observer_set.take_observers_for(entity_id);
        self.subscriber_set.remove_all_for_entity(entity_id);

        self.entity_storage.remove(entity)
    }

    /// Create a new reactive model and return a handle to it.
    pub fn new_model<T: 'static>(&mut self, value: T) -> Model<T> {
        let entity = self.entity_storage.insert(value);
        Model::new(entity)
    }

    /// Read a model's data immutably.
    pub fn read_model<T: 'static>(&self, model: &Model<T>) -> &T {
        self.entity_storage.read(&model.entity)
    }

    /// Update a model's data mutably with a ModelContext for queueing effects.
    pub fn update_model<T: 'static, R>(
        &mut self,
        model: &Model<T>,
        f: impl FnOnce(&mut T, &mut ModelContext<T>) -> R,
    ) -> R {
        let entity_id = model.entity_id();
        self.update_depth += 1;

        // Create ModelContext to collect effects during the update
        let mut model_cx = ModelContext::new(entity_id);

        // Execute the update closure
        let result = self.entity_storage.update(&model.entity, |data| {
            f(data, &mut model_cx)
        });

        // Drain effects from ModelContext into the effect queue
        for (eid, effect) in model_cx.drain() {
            match effect {
                PendingEffect::Notify => {
                    self.effect_queue.push_notify(eid);
                }
                PendingEffect::Emit { event_type_id, event } => {
                    self.effect_queue.push_emit(eid, event_type_id, event);
                }
            }
        }

        self.update_depth -= 1;

        // Flush effects when we return to the top level
        if self.update_depth == 0 {
            self.flush_effects();
        }

        result
    }

    /// Observe a model and register a callback to be invoked when it calls cx.notify().
    /// Returns a Subscription handle. Call .detach() to make it persist for the view's lifetime.
    pub fn observe<T: 'static>(
        &mut self,
        model: &Model<T>,
        callback: impl FnMut(&mut AppContext) + 'static,
    ) -> Subscription {
        let entity_id = model.entity_id();
        let id = self.observer_set.add_observer(entity_id, Box::new(callback));

        Subscription::new(id, CleanupAction::RemoveObserver {
            observed: entity_id,
            id,
        })
    }

    /// Subscribe to typed events emitted by a specific model.
    /// The callback receives the typed event and mutable AppContext.
    pub fn subscribe<T: 'static, E: 'static>(
        &mut self,
        model: &Model<T>,
        mut callback: impl FnMut(&E, &mut AppContext) + 'static,
    ) -> Subscription {
        let emitter = model.entity_id();
        let event_type = TypeId::of::<E>();

        // Wrap the typed callback to work with type-erased &dyn Any
        let wrapped = Box::new(move |event: &dyn std::any::Any, cx: &mut AppContext| {
            if let Some(typed_event) = event.downcast_ref::<E>() {
                callback(typed_event, cx);
            }
        });

        let id = self.subscriber_set.subscribe(emitter, event_type, wrapped);

        Subscription::new(id, CleanupAction::RemoveSubscriber {
            emitter,
            event_type,
            id,
        })
    }

    /// Subscribe to global app-wide events (theme changes, window resize, etc.).
    /// The callback receives the typed event and mutable AppContext.
    pub fn subscribe_global<E: 'static>(
        &mut self,
        mut callback: impl FnMut(&E, &mut AppContext) + 'static,
    ) -> Subscription {
        let event_type = TypeId::of::<E>();

        // Wrap the typed callback to work with type-erased &dyn Any
        let wrapped = Box::new(move |event: &dyn std::any::Any, cx: &mut AppContext| {
            if let Some(typed_event) = event.downcast_ref::<E>() {
                callback(typed_event, cx);
            }
        });

        let id = self.global_event_bus.subscribe(event_type, wrapped);

        Subscription::new(id, CleanupAction::RemoveGlobalSubscriber {
            event_type,
            id,
        })
    }

    /// Emit a global app-wide event.
    /// The event will be dispatched to all global subscribers during flush_effects.
    pub fn emit_global<E: 'static>(&mut self, event: E) {
        let event_type_id = TypeId::of::<E>();
        self.effect_queue.push_global_emit(event_type_id, Box::new(event));

        // If not in an update, flush immediately
        if self.update_depth == 0 {
            self.flush_effects();
        }
    }

    /// Check if any entities are marked dirty and need re-render.
    pub fn has_dirty_entities(&self) -> bool {
        !self.dirty_entities.is_empty()
    }

    /// Clear the dirty entity set.
    pub fn clear_dirty(&mut self) {
        self.dirty_entities.clear();
    }

    /// Flush all queued effects.
    /// Processes notify and emit effects, invoking observer callbacks and marking entities dirty.
    /// Supports cascading effects with a depth limit to prevent infinite loops.
    fn flush_effects(&mut self) {
        const DEPTH_LIMIT: usize = 10;

        let mut depth = 0;
        loop {
            depth += 1;
            if depth > DEPTH_LIMIT {
                panic!(
                    "Effect cascade depth limit exceeded ({}). Possible infinite loop in observers.",
                    DEPTH_LIMIT
                );
            }
            if depth > 5 {
                log::warn!("Effect cascade depth {} (limit: {})", depth, DEPTH_LIMIT);
            }

            // Priority 0: Process pending subscription cleanups
            if depth == 1 {
                let cleanups = crate::subscription::drain_pending_cleanups();
                for cleanup in cleanups {
                    match cleanup {
                        CleanupAction::RemoveObserver { observed, id } => {
                            self.observer_set.remove_observer(observed, id);
                        }
                        CleanupAction::RemoveSubscriber { emitter, event_type, id } => {
                            self.subscriber_set.remove_subscription(emitter, event_type, id);
                        }
                        CleanupAction::RemoveGlobalSubscriber { event_type, id } => {
                            self.global_event_bus.remove_subscription(event_type, id);
                        }
                    }
                }
            }

            // Priority 1: Process all notify effects
            let notified = self.effect_queue.drain_notify();
            if notified.is_empty() && self.effect_queue.is_empty() {
                break; // All queues empty
            }

            for entity_id in notified {
                // Mark entity as dirty
                self.dirty_entities.insert(entity_id);

                // Invoke observers - take observers out, invoke, put back
                // This avoids aliased mutable borrows
                let mut observers = self.observer_set.take_observers_for(entity_id);

                // Invoke each observer callback with mutable AppContext
                // Each observer may queue more effects
                for (_id, callback) in observers.iter_mut() {
                    callback(self);
                }

                // Restore observers back to the set
                self.observer_set.restore_observers(entity_id, observers);
            }

            // Priority 2: Process all entity-specific emit effects
            let emitted = self.effect_queue.drain_emit();
            for (emitter, event_type_id, event) in emitted {
                // Take subscribers, invoke with type-erased event, restore
                let mut callbacks = self.subscriber_set.take_subscribers_for(emitter, event_type_id);
                for (_id, callback) in callbacks.iter_mut() {
                    callback(&*event, self);
                }
                self.subscriber_set.restore_subscribers(emitter, event_type_id, callbacks);
            }

            // Priority 3: Process global emit effects
            let global_emitted = self.effect_queue.drain_global_emit();
            for (event_type_id, event) in global_emitted {
                // Take global subscribers, invoke, restore
                let mut callbacks = self.global_event_bus.take_subscribers_for(event_type_id);
                for (_id, callback) in callbacks.iter_mut() {
                    callback(&*event, self);
                }
                self.global_event_bus.restore_subscribers(event_type_id, callbacks);
            }

            // If new effects were queued during processing, loop again
            if self.effect_queue.is_empty() {
                break;
            }
        }
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
