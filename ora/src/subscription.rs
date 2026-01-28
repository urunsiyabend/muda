use crate::context::AppContext;
use crate::effect::EntityId;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;

pub type SubscriptionId = usize;

/// Actions to perform when a Subscription is dropped.
#[derive(Debug)]
pub enum CleanupAction {
    /// Remove an observer from the ObserverSet.
    RemoveObserver { observed: EntityId, id: SubscriptionId },
    /// Remove a subscriber from the SubscriberSet.
    RemoveSubscriber { emitter: EntityId, event_type: TypeId, id: SubscriptionId },
    /// Remove a global event subscriber from the GlobalEventBus.
    RemoveGlobalSubscriber { event_type: TypeId, id: SubscriptionId },
}

thread_local! {
    static PENDING_CLEANUPS: RefCell<Vec<CleanupAction>> = RefCell::new(Vec::new());
}

/// Drain all pending cleanup actions.
/// Called by AppContext at the start of flush_effects.
pub fn drain_pending_cleanups() -> Vec<CleanupAction> {
    PENDING_CLEANUPS.with(|cleanups| cleanups.borrow_mut().drain(..).collect())
}

pub struct Subscription {
    id: SubscriptionId,
    cleanup: Option<CleanupAction>,
}

impl Subscription {
    pub(crate) fn new(id: SubscriptionId, cleanup: CleanupAction) -> Self {
        Self {
            id,
            cleanup: Some(cleanup),
        }
    }

    pub fn detach(mut self) {
        self.cleanup = None;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            PENDING_CLEANUPS.with(|cleanups| {
                cleanups.borrow_mut().push(cleanup);
            });
        }
    }
}

pub type ObserverCallback = Box<dyn FnMut(&mut AppContext)>;

pub struct ObserverSet {
    observers: HashMap<EntityId, Vec<(SubscriptionId, ObserverCallback)>>,
    next_id: SubscriptionId,
}

impl ObserverSet {
    pub fn new() -> Self {
        Self {
            observers: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_observer(&mut self, observed: EntityId, callback: ObserverCallback) -> SubscriptionId {
        let id = self.next_id;
        self.next_id += 1;

        self.observers
            .entry(observed)
            .or_insert_with(Vec::new)
            .push((id, callback));

        id
    }

    pub fn remove_observer(&mut self, observed: EntityId, id: SubscriptionId) {
        if let Some(observers) = self.observers.get_mut(&observed) {
            observers.retain(|(sub_id, _)| *sub_id != id);
            if observers.is_empty() {
                self.observers.remove(&observed);
            }
        }
    }

    pub fn take_observers_for(&mut self, entity_id: EntityId) -> Vec<(SubscriptionId, ObserverCallback)> {
        self.observers.remove(&entity_id).unwrap_or_default()
    }

    pub fn restore_observers(&mut self, entity_id: EntityId, callbacks: Vec<(SubscriptionId, ObserverCallback)>) {
        if !callbacks.is_empty() {
            self.observers.insert(entity_id, callbacks);
        }
    }
}

impl Default for ObserverSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-erased subscriber callback that receives an event and mutable AppContext.
pub type SubscriberCallback = Box<dyn FnMut(&dyn Any, &mut AppContext)>;

/// Registry for typed event subscriptions.
///
/// Maps (entity_id, event_type) -> list of subscriber callbacks.
/// Subscribers receive type-erased events that they downcast to concrete types.
pub struct SubscriberSet {
    /// Subscribers organized by emitter entity and event type.
    subscribers: HashMap<EntityId, HashMap<TypeId, Vec<(SubscriptionId, SubscriberCallback)>>>,
    /// Next subscription ID to allocate.
    next_id: SubscriptionId,
}

impl SubscriberSet {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            next_id: 0,
        }
    }

    /// Subscribe to a specific event type from a specific entity.
    pub fn subscribe(&mut self, emitter: EntityId, event_type: TypeId, callback: SubscriberCallback) -> SubscriptionId {
        let id = self.next_id;
        self.next_id += 1;

        self.subscribers
            .entry(emitter)
            .or_insert_with(HashMap::new)
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push((id, callback));

        id
    }

    /// Remove a subscription by ID.
    pub fn remove_subscription(&mut self, emitter: EntityId, event_type: TypeId, id: SubscriptionId) {
        if let Some(event_map) = self.subscribers.get_mut(&emitter) {
            if let Some(callbacks) = event_map.get_mut(&event_type) {
                callbacks.retain(|(sub_id, _)| *sub_id != id);
                if callbacks.is_empty() {
                    event_map.remove(&event_type);
                }
            }
            if event_map.is_empty() {
                self.subscribers.remove(&emitter);
            }
        }
    }

    /// Remove all subscriptions for a specific entity (when entity is removed).
    pub fn remove_all_for_entity(&mut self, entity_id: EntityId) {
        self.subscribers.remove(&entity_id);
    }

    /// Take subscribers for a specific event type, removing them from the registry.
    /// Used during event dispatch to avoid aliased mutable borrows.
    pub fn take_subscribers_for(&mut self, emitter: EntityId, event_type: TypeId) -> Vec<(SubscriptionId, SubscriberCallback)> {
        if let Some(event_map) = self.subscribers.get_mut(&emitter) {
            event_map.remove(&event_type).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Restore subscribers after event dispatch.
    pub fn restore_subscribers(&mut self, emitter: EntityId, event_type: TypeId, callbacks: Vec<(SubscriptionId, SubscriberCallback)>) {
        if !callbacks.is_empty() {
            self.subscribers
                .entry(emitter)
                .or_insert_with(HashMap::new)
                .insert(event_type, callbacks);
        }
    }
}

impl Default for SubscriberSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Global event bus for app-wide events (theme changes, window resize, etc.).
///
/// Unlike SubscriberSet which is per-entity, GlobalEventBus has no emitter entity.
pub struct GlobalEventBus {
    /// Subscribers organized by event type.
    subscribers: HashMap<TypeId, Vec<(SubscriptionId, Box<dyn FnMut(&dyn Any, &mut AppContext)>)>>,
    /// Next subscription ID to allocate.
    next_id: SubscriptionId,
}

impl GlobalEventBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            next_id: 0,
        }
    }

    /// Subscribe to a global event type.
    pub fn subscribe(&mut self, event_type: TypeId, callback: Box<dyn FnMut(&dyn Any, &mut AppContext)>) -> SubscriptionId {
        let id = self.next_id;
        self.next_id += 1;

        self.subscribers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push((id, callback));

        id
    }

    /// Remove a global subscription by ID.
    pub fn remove_subscription(&mut self, event_type: TypeId, id: SubscriptionId) {
        if let Some(callbacks) = self.subscribers.get_mut(&event_type) {
            callbacks.retain(|(sub_id, _)| *sub_id != id);
            if callbacks.is_empty() {
                self.subscribers.remove(&event_type);
            }
        }
    }

    /// Take subscribers for a specific event type, removing them from the registry.
    /// Used during event dispatch to avoid aliased mutable borrows.
    pub fn take_subscribers_for(&mut self, event_type: TypeId) -> Vec<(SubscriptionId, Box<dyn FnMut(&dyn Any, &mut AppContext)>)> {
        self.subscribers.remove(&event_type).unwrap_or_default()
    }

    /// Restore subscribers after event dispatch.
    pub fn restore_subscribers(&mut self, event_type: TypeId, callbacks: Vec<(SubscriptionId, Box<dyn FnMut(&dyn Any, &mut AppContext)>)>) {
        if !callbacks.is_empty() {
            self.subscribers.insert(event_type, callbacks);
        }
    }
}

impl Default for GlobalEventBus {
    fn default() -> Self {
        Self::new()
    }
}
