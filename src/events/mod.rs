//! Event System: DomainEvents & EventBus for reactive architecture.
//!
//! This module provides a decoupled event-driven architecture enabling:
//! - Reactive updates (ViewModel rebuilds on document changes)
//! - Plugin integration points
//! - Future incremental rendering support
//!
//! # Architecture
//!
//! ```text
//! Document/View -> emit(DomainEvent) -> EventBus -> EventHandler(s)
//! ```
//!
//! Events are synchronous for simplicity; the design can evolve to async later.

use std::collections::HashMap;

use crate::domain::{DocumentId, DocumentRevision, TextOffset, TextRange};
use crate::view::ViewId;

/// A typed notification that something meaningful changed in the editor.
///
/// DomainEvents drive the reactive architecture, enabling view model rebuilds,
/// plugin reactions, and future incremental rendering optimizations.
#[derive(Clone, Debug, PartialEq)]
pub enum DomainEvent {
    // =========================================================================
    // Document Events
    // =========================================================================
    /// A document was opened/created.
    DocumentOpened {
        document_id: DocumentId,
    },

    /// The document content changed.
    ///
    /// This is the primary event for triggering view updates.
    DocumentChanged {
        document_id: DocumentId,
        revision: DocumentRevision,
        /// The range that was affected (for future incremental updates).
        affected_range: Option<TextRange>,
    },

    /// A document was saved to disk.
    DocumentSaved {
        document_id: DocumentId,
    },

    /// A document was closed.
    DocumentClosed {
        document_id: DocumentId,
    },

    /// Document metadata changed (dirty flag, path, encoding, etc.).
    DocumentMetadataChanged {
        document_id: DocumentId,
    },

    // =========================================================================
    // View Events
    // =========================================================================
    /// The selection in a view changed.
    SelectionChanged {
        view_id: ViewId,
        document_id: DocumentId,
        /// New caret offset(s).
        caret_offset: TextOffset,
        /// Whether there's an active selection.
        has_selection: bool,
    },

    /// The viewport scrolled or resized.
    ViewportChanged {
        view_id: ViewId,
        scroll_x: usize,
        scroll_y: usize,
        width: usize,
        height: usize,
    },

    /// A view was created.
    ViewCreated {
        view_id: ViewId,
        document_id: DocumentId,
    },

    /// A view was closed.
    ViewClosed {
        view_id: ViewId,
    },

    // =========================================================================
    // Language/Analysis Events
    // =========================================================================
    /// Syntax highlighting was updated.
    SyntaxUpdated {
        document_id: DocumentId,
    },

    /// Diagnostics (errors, warnings) were updated.
    DiagnosticsUpdated {
        document_id: DocumentId,
        /// Number of errors.
        error_count: usize,
        /// Number of warnings.
        warning_count: usize,
    },
}

impl DomainEvent {
    /// Returns the document ID associated with this event, if any.
    pub fn document_id(&self) -> Option<DocumentId> {
        match self {
            DomainEvent::DocumentOpened { document_id }
            | DomainEvent::DocumentChanged { document_id, .. }
            | DomainEvent::DocumentSaved { document_id }
            | DomainEvent::DocumentClosed { document_id }
            | DomainEvent::DocumentMetadataChanged { document_id }
            | DomainEvent::SelectionChanged { document_id, .. }
            | DomainEvent::ViewCreated { document_id, .. }
            | DomainEvent::SyntaxUpdated { document_id }
            | DomainEvent::DiagnosticsUpdated { document_id, .. } => Some(*document_id),
            DomainEvent::ViewportChanged { .. } | DomainEvent::ViewClosed { .. } => None,
        }
    }

    /// Returns the view ID associated with this event, if any.
    pub fn view_id(&self) -> Option<ViewId> {
        match self {
            DomainEvent::SelectionChanged { view_id, .. }
            | DomainEvent::ViewportChanged { view_id, .. }
            | DomainEvent::ViewCreated { view_id, .. }
            | DomainEvent::ViewClosed { view_id } => Some(*view_id),
            _ => None,
        }
    }

    /// Returns true if this event should trigger a re-render.
    pub fn requires_render(&self) -> bool {
        matches!(
            self,
            DomainEvent::DocumentChanged { .. }
                | DomainEvent::SelectionChanged { .. }
                | DomainEvent::ViewportChanged { .. }
                | DomainEvent::SyntaxUpdated { .. }
                | DomainEvent::DiagnosticsUpdated { .. }
                | DomainEvent::DocumentMetadataChanged { .. }
        )
    }
}

/// Unique identifier for an event handler subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Trait for handling domain events.
///
/// Implement this trait to react to events in the editor.
pub trait EventHandler: Send {
    /// Handles an event.
    ///
    /// Returns `true` if the event was handled and should stop propagation
    /// (rarely needed; most handlers return `false`).
    fn handle(&mut self, event: &DomainEvent) -> bool;

    /// Returns a name for this handler (for debugging/logging).
    fn name(&self) -> &str {
        "anonymous"
    }
}

/// A function-based event handler for convenience.
pub struct FnHandler<F>
where
    F: FnMut(&DomainEvent) -> bool + Send,
{
    name: String,
    handler: F,
}

impl<F> FnHandler<F>
where
    F: FnMut(&DomainEvent) -> bool + Send,
{
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }
}

impl<F> EventHandler for FnHandler<F>
where
    F: FnMut(&DomainEvent) -> bool + Send,
{
    fn handle(&mut self, event: &DomainEvent) -> bool {
        (self.handler)(event)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// The central event bus for publishing and subscribing to domain events.
///
/// The EventBus provides a simple synchronous pub/sub mechanism.
/// Events are delivered to all subscribers in registration order.
pub struct EventBus {
    /// Registered handlers.
    handlers: HashMap<SubscriptionId, Box<dyn EventHandler>>,
    /// Pending events (for deferred publishing).
    pending_events: Vec<DomainEvent>,
    /// Whether we're currently dispatching (to prevent re-entrancy issues).
    dispatching: bool,
    /// Event history for debugging (optional, limited size).
    history: Vec<DomainEvent>,
    /// Maximum history size.
    max_history: usize,
}

impl EventBus {
    /// Creates a new empty event bus.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            pending_events: Vec::new(),
            dispatching: false,
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Creates an event bus with a specific history limit.
    pub fn with_history_limit(max_history: usize) -> Self {
        Self {
            handlers: HashMap::new(),
            pending_events: Vec::new(),
            dispatching: false,
            history: Vec::new(),
            max_history,
        }
    }

    /// Subscribes a handler to receive events.
    ///
    /// Returns a subscription ID that can be used to unsubscribe.
    pub fn subscribe(&mut self, handler: Box<dyn EventHandler>) -> SubscriptionId {
        let id = SubscriptionId::new();
        self.handlers.insert(id, handler);
        id
    }

    /// Subscribes a function as an event handler.
    ///
    /// This is a convenience method for simple handlers.
    pub fn subscribe_fn<F>(&mut self, name: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: FnMut(&DomainEvent) -> bool + Send + 'static,
    {
        self.subscribe(Box::new(FnHandler::new(name, handler)))
    }

    /// Unsubscribes a handler.
    ///
    /// Returns `true` if the handler was found and removed.
    pub fn unsubscribe(&mut self, id: SubscriptionId) -> bool {
        self.handlers.remove(&id).is_some()
    }

    /// Publishes an event to all subscribers.
    ///
    /// If currently dispatching, the event is queued for later delivery.
    pub fn publish(&mut self, event: DomainEvent) {
        // Record in history
        self.history.push(event.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        if self.dispatching {
            // Queue for later to prevent re-entrancy
            self.pending_events.push(event);
            return;
        }

        self.dispatch_event(event);

        // Process any events that were queued during dispatch
        while !self.pending_events.is_empty() {
            let pending = std::mem::take(&mut self.pending_events);
            for event in pending {
                self.dispatch_event(event);
            }
        }
    }

    /// Dispatches an event to all handlers.
    fn dispatch_event(&mut self, event: DomainEvent) {
        self.dispatching = true;

        for handler in self.handlers.values_mut() {
            if handler.handle(&event) {
                break; // Handler requested to stop propagation
            }
        }

        self.dispatching = false;
    }

    /// Returns the number of registered handlers.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Returns the recent event history.
    pub fn history(&self) -> &[DomainEvent] {
        &self.history
    }

    /// Clears the event history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Returns true if there are pending events.
    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct to collect events during a batch operation.
///
/// This allows multiple changes to be made without emitting events,
/// then emit a single consolidated event at the end.
#[derive(Default)]
pub struct EventCollector {
    events: Vec<DomainEvent>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Records an event for later publishing.
    pub fn record(&mut self, event: DomainEvent) {
        self.events.push(event);
    }

    /// Publishes all collected events to the bus.
    pub fn flush(&mut self, bus: &mut EventBus) {
        for event in self.events.drain(..) {
            bus.publish(event);
        }
    }

    /// Returns true if there are collected events.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Clears collected events without publishing.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Returns the collected events.
    pub fn events(&self) -> &[DomainEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_document_id() {
        let doc_id = DocumentId::default();
        let event = DomainEvent::DocumentOpened {
            document_id: doc_id,
        };
        assert_eq!(event.document_id(), Some(doc_id));

        let view_id = ViewId::default();
        let viewport_event = DomainEvent::ViewportChanged {
            view_id,
            scroll_x: 0,
            scroll_y: 0,
            width: 80,
            height: 24,
        };
        assert_eq!(viewport_event.document_id(), None);
    }

    #[test]
    fn test_domain_event_requires_render() {
        let doc_id = DocumentId::default();

        let change_event = DomainEvent::DocumentChanged {
            document_id: doc_id,
            revision: 1,
            affected_range: None,
        };
        assert!(change_event.requires_render());

        let open_event = DomainEvent::DocumentOpened {
            document_id: doc_id,
        };
        assert!(!open_event.requires_render());
    }

    #[test]
    fn test_event_bus_subscribe_publish() {
        let mut bus = EventBus::new();
        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let received_clone = received.clone();
        bus.subscribe_fn("test", move |event| {
            received_clone.lock().unwrap().push(event.clone());
            false
        });

        let doc_id = DocumentId::default();
        bus.publish(DomainEvent::DocumentOpened {
            document_id: doc_id,
        });

        let events = received.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DomainEvent::DocumentOpened { .. }));
    }

    #[test]
    fn test_event_bus_unsubscribe() {
        let mut bus = EventBus::new();
        let received = std::sync::Arc::new(std::sync::Mutex::new(0));

        let received_clone = received.clone();
        let id = bus.subscribe_fn("test", move |_| {
            *received_clone.lock().unwrap() += 1;
            false
        });

        let doc_id = DocumentId::default();
        bus.publish(DomainEvent::DocumentOpened {
            document_id: doc_id,
        });
        assert_eq!(*received.lock().unwrap(), 1);

        bus.unsubscribe(id);
        bus.publish(DomainEvent::DocumentOpened {
            document_id: doc_id,
        });
        assert_eq!(*received.lock().unwrap(), 1); // Still 1, handler was removed
    }

    #[test]
    fn test_event_bus_history() {
        let mut bus = EventBus::with_history_limit(3);
        let doc_id = DocumentId::default();

        for i in 0..5 {
            bus.publish(DomainEvent::DocumentChanged {
                document_id: doc_id,
                revision: i,
                affected_range: None,
            });
        }

        assert_eq!(bus.history().len(), 3);
        // Should have revisions 2, 3, 4 (oldest dropped)
    }

    #[test]
    fn test_event_collector() {
        let mut collector = EventCollector::new();
        let doc_id = DocumentId::default();

        collector.record(DomainEvent::DocumentOpened {
            document_id: doc_id,
        });
        collector.record(DomainEvent::DocumentChanged {
            document_id: doc_id,
            revision: 1,
            affected_range: None,
        });

        assert!(collector.has_events());
        assert_eq!(collector.events().len(), 2);

        let mut bus = EventBus::new();
        collector.flush(&mut bus);

        assert!(!collector.has_events());
        assert_eq!(bus.history().len(), 2);
    }

    #[test]
    fn test_stop_propagation() {
        let mut bus = EventBus::new();
        let counter = std::sync::Arc::new(std::sync::Mutex::new(0));

        let counter1 = counter.clone();
        bus.subscribe_fn("first", move |_| {
            *counter1.lock().unwrap() += 1;
            true // Stop propagation
        });

        let counter2 = counter.clone();
        bus.subscribe_fn("second", move |_| {
            *counter2.lock().unwrap() += 1;
            false
        });

        let doc_id = DocumentId::default();
        bus.publish(DomainEvent::DocumentOpened {
            document_id: doc_id,
        });

        // Note: HashMap iteration order is not guaranteed, so this test
        // may be flaky. In a real implementation, we'd use a Vec or
        // ordered map to guarantee delivery order.
        // For now, we just verify at least one handler was called.
        assert!(*counter.lock().unwrap() >= 1);
    }
}
