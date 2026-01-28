use std::any::{Any, TypeId};
use std::collections::{VecDeque, HashSet};

/// Entity ID is a generational arena index.
pub type EntityId = generational_arena::Index;

/// Effects that can be queued during updates.
#[derive(Debug)]
pub enum Effect {
    /// Mark an entity as dirty, triggering observer notifications.
    Notify { emitter: EntityId },
    /// Emit a typed event to subscribers.
    Emit {
        emitter: EntityId,
        event_type_id: TypeId,
        event: Box<dyn Any>,
    },
}

/// Priority-based effect queue with deduplication for notify effects.
///
/// Notify effects are processed before Emit effects to ensure observers
/// see consistent state before events propagate.
pub struct EffectQueue {
    /// High-priority queue for Notify effects (deduplicated).
    notify_queue: VecDeque<EntityId>,
    /// Set for O(1) deduplication of notify effects.
    notify_set: HashSet<EntityId>,
    /// Mid-priority queue for Emit effects (not deduplicated).
    emit_queue: VecDeque<(EntityId, TypeId, Box<dyn Any>)>,
    /// Current flush depth for cascade detection.
    depth: usize,
}

impl EffectQueue {
    /// Create a new empty effect queue.
    pub fn new() -> Self {
        Self {
            notify_queue: VecDeque::new(),
            notify_set: HashSet::new(),
            emit_queue: VecDeque::new(),
            depth: 0,
        }
    }

    /// Queue a notify effect (deduplicated per entity).
    pub fn push_notify(&mut self, emitter: EntityId) {
        if self.notify_set.insert(emitter) {
            self.notify_queue.push_back(emitter);
        }
    }

    /// Queue an emit effect (always pushed, no deduplication).
    pub fn push_emit(&mut self, emitter: EntityId, event_type_id: TypeId, event: Box<dyn Any>) {
        self.emit_queue.push_back((emitter, event_type_id, event));
    }

    /// Check if both queues are empty.
    pub fn is_empty(&self) -> bool {
        self.notify_queue.is_empty() && self.emit_queue.is_empty()
    }

    /// Drain all notify effects and clear the deduplication set.
    pub fn drain_notify(&mut self) -> Vec<EntityId> {
        self.notify_set.clear();
        self.notify_queue.drain(..).collect()
    }

    /// Drain all emit effects.
    pub fn drain_emit(&mut self) -> Vec<(EntityId, TypeId, Box<dyn Any>)> {
        self.emit_queue.drain(..).collect()
    }

    /// Get the current flush depth.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Increment the flush depth counter.
    pub fn inc_depth(&mut self) {
        self.depth += 1;
    }

    /// Decrement the flush depth counter.
    pub fn dec_depth(&mut self) {
        self.depth -= 1;
    }
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self::new()
    }
}
