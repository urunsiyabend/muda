use crate::context::AppContext;
use crate::effect::EntityId;
use std::collections::HashMap;

pub type SubscriptionId = usize;

pub struct Subscription {
    id: SubscriptionId,
    cleanup: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    pub(crate) fn new(id: SubscriptionId, cleanup: Box<dyn FnOnce()>) -> Self {
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
            cleanup();
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
