use super::mouse::{HitboxId, MouseDownEvent, MouseMoveEvent, MouseScrollEvent, MouseUpEvent};
use std::cell::Cell;
use std::collections::HashMap;

/// Two-phase event dispatch: capture (root to target) then bubble (target to root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchPhase {
    /// Capture phase: events flow from root to target.
    Capture,
    /// Bubble phase: events flow from target back to root.
    Bubble,
}

/// Event context passed to handlers during dispatch.
/// Provides phase information and propagation control.
pub struct EventContext {
    phase: DispatchPhase,
    propagation_stopped: Cell<bool>,
    target: HitboxId,
}

impl EventContext {
    /// Create a new EventContext for the given phase and target.
    pub fn new(phase: DispatchPhase, target: HitboxId) -> Self {
        Self {
            phase,
            propagation_stopped: Cell::new(false),
            target,
        }
    }

    /// Get the current dispatch phase.
    pub fn phase(&self) -> DispatchPhase {
        self.phase
    }

    /// Stop event propagation.
    /// Prevents further handlers from being invoked in the current phase.
    pub fn stop_propagation(&self) {
        self.propagation_stopped.set(true);
    }

    /// Check if propagation has been stopped.
    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped.get()
    }

    /// Get the target hitbox ID (the element that was hit).
    pub fn target(&self) -> HitboxId {
        self.target
    }
}

/// Handler function types for mouse events.
pub type MouseDownHandler = Box<dyn FnMut(&MouseDownEvent, &EventContext)>;
pub type MouseUpHandler = Box<dyn FnMut(&MouseUpEvent, &EventContext)>;
pub type MouseMoveHandler = Box<dyn FnMut(&MouseMoveEvent, &EventContext)>;
pub type MouseScrollHandler = Box<dyn FnMut(&MouseScrollEvent, &EventContext)>;

/// Event handler registry.
/// Stores handlers for each hitbox and parent relationships for bubbling.
pub struct EventHandlers {
    /// Mouse down handlers indexed by hitbox ID.
    mouse_down_handlers: HashMap<HitboxId, Vec<MouseDownHandler>>,
    /// Mouse up handlers indexed by hitbox ID.
    mouse_up_handlers: HashMap<HitboxId, Vec<MouseUpHandler>>,
    /// Mouse move handlers indexed by hitbox ID.
    mouse_move_handlers: HashMap<HitboxId, Vec<MouseMoveHandler>>,
    /// Mouse scroll handlers indexed by hitbox ID.
    mouse_scroll_handlers: HashMap<HitboxId, Vec<MouseScrollHandler>>,
    /// Parent relationships for bubbling (child -> parent).
    pub(crate) parent_map: HashMap<HitboxId, HitboxId>,
}

impl EventHandlers {
    /// Create a new empty EventHandlers registry.
    pub fn new() -> Self {
        Self {
            mouse_down_handlers: HashMap::new(),
            mouse_up_handlers: HashMap::new(),
            mouse_move_handlers: HashMap::new(),
            mouse_scroll_handlers: HashMap::new(),
            parent_map: HashMap::new(),
        }
    }

    /// Register a mouse down handler for a hitbox.
    pub fn register_mouse_down(&mut self, id: HitboxId, handler: MouseDownHandler) {
        self.mouse_down_handlers.entry(id).or_default().push(handler);
    }

    /// Register a mouse up handler for a hitbox.
    pub fn register_mouse_up(&mut self, id: HitboxId, handler: MouseUpHandler) {
        self.mouse_up_handlers.entry(id).or_default().push(handler);
    }

    /// Register a mouse move handler for a hitbox.
    pub fn register_mouse_move(&mut self, id: HitboxId, handler: MouseMoveHandler) {
        self.mouse_move_handlers.entry(id).or_default().push(handler);
    }

    /// Register a mouse scroll handler for a hitbox.
    pub fn register_mouse_scroll(&mut self, id: HitboxId, handler: MouseScrollHandler) {
        self.mouse_scroll_handlers.entry(id).or_default().push(handler);
    }

    /// Register a parent-child relationship for bubbling.
    pub fn register_parent(&mut self, child: HitboxId, parent: HitboxId) {
        self.parent_map.insert(child, parent);
    }

    /// Check if any scroll handlers are registered (for debugging).
    pub fn has_scroll_handlers(&self) -> bool {
        !self.mouse_scroll_handlers.is_empty()
    }
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the dispatch path from root to target for capture/bubble phases.
/// Returns a Vec of HitboxIds from root (first) to target (last).
pub fn build_dispatch_path(parent_map: &HashMap<HitboxId, HitboxId>, target: HitboxId) -> Vec<HitboxId> {
    let mut path = vec![target];
    let mut current = target;

    // Walk up the parent chain
    while let Some(&parent) = parent_map.get(&current) {
        path.push(parent);
        current = parent;
    }

    // Reverse to get root-to-target order
    path.reverse();
    path
}

/// Dispatch a mouse down event through the two-phase system.
pub fn dispatch_mouse_down(handlers: &mut EventHandlers, event: &MouseDownEvent, target: HitboxId) {
    let path = build_dispatch_path(&handlers.parent_map, target);

    // Phase 1: Capture (root to target)
    let capture_ctx = EventContext::new(DispatchPhase::Capture, target);
    for &hitbox_id in &path {
        if capture_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_down_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &capture_ctx);
                if capture_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }

    // Phase 2: Bubble (target to root)
    let bubble_ctx = EventContext::new(DispatchPhase::Bubble, target);
    for &hitbox_id in path.iter().rev() {
        if bubble_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_down_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &bubble_ctx);
                if bubble_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }
}

/// Dispatch a mouse up event through the two-phase system.
pub fn dispatch_mouse_up(handlers: &mut EventHandlers, event: &MouseUpEvent, target: HitboxId) {
    let path = build_dispatch_path(&handlers.parent_map, target);

    // Phase 1: Capture (root to target)
    let capture_ctx = EventContext::new(DispatchPhase::Capture, target);
    for &hitbox_id in &path {
        if capture_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_up_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &capture_ctx);
                if capture_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }

    // Phase 2: Bubble (target to root)
    let bubble_ctx = EventContext::new(DispatchPhase::Bubble, target);
    for &hitbox_id in path.iter().rev() {
        if bubble_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_up_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &bubble_ctx);
                if bubble_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }
}

/// Dispatch a mouse move event through the two-phase system.
pub fn dispatch_mouse_move(handlers: &mut EventHandlers, event: &MouseMoveEvent, target: HitboxId) {
    let path = build_dispatch_path(&handlers.parent_map, target);

    // Phase 1: Capture (root to target)
    let capture_ctx = EventContext::new(DispatchPhase::Capture, target);
    for &hitbox_id in &path {
        if capture_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_move_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &capture_ctx);
                if capture_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }

    // Phase 2: Bubble (target to root)
    let bubble_ctx = EventContext::new(DispatchPhase::Bubble, target);
    for &hitbox_id in path.iter().rev() {
        if bubble_ctx.is_propagation_stopped() {
            return;
        }

        if let Some(handler_list) = handlers.mouse_move_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &bubble_ctx);
                if bubble_ctx.is_propagation_stopped() {
                    return;
                }
            }
        }
    }
}

/// Dispatch a mouse scroll event through the two-phase system.
/// Returns true if any handler consumed the event (stopped propagation).
pub fn dispatch_mouse_scroll(handlers: &mut EventHandlers, event: &MouseScrollEvent, target: HitboxId) -> bool {
    let path = build_dispatch_path(&handlers.parent_map, target);

    // Phase 1: Capture (root to target)
    let capture_ctx = EventContext::new(DispatchPhase::Capture, target);
    for &hitbox_id in &path {
        if capture_ctx.is_propagation_stopped() {
            return true;
        }
        if let Some(handler_list) = handlers.mouse_scroll_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &capture_ctx);
                if capture_ctx.is_propagation_stopped() {
                    return true;
                }
            }
        }
    }

    // Phase 2: Bubble (target to root)
    let bubble_ctx = EventContext::new(DispatchPhase::Bubble, target);
    for &hitbox_id in path.iter().rev() {
        if bubble_ctx.is_propagation_stopped() {
            return true;
        }
        if let Some(handler_list) = handlers.mouse_scroll_handlers.get_mut(&hitbox_id) {
            for handler in handler_list.iter_mut() {
                handler(event, &bubble_ctx);
                if bubble_ctx.is_propagation_stopped() {
                    return true;
                }
            }
        }
    }

    false
}
