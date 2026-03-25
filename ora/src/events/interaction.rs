use super::mouse::HitboxId;
use super::types::MouseButton;
use std::collections::HashSet;

/// Mouse capture for drag operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseCapture {
    pub hitbox_id: HitboxId,
    pub button: MouseButton,
}

/// Interaction state tracking hover and active states for elements
pub struct InteractionState {
    /// The currently hovered hitbox (topmost under cursor)
    hovered: Option<HitboxId>,
    /// Set of all hovered hitboxes (element and ancestors in path)
    hover_path: HashSet<HitboxId>,
    /// The currently active hitbox (mouse button pressed on)
    active: Option<HitboxId>,
    /// Set of all active hitboxes (element and ancestors in path)
    active_path: HashSet<HitboxId>,
    /// Current mouse capture state
    mouse_capture: Option<MouseCapture>,
}

impl InteractionState {
    pub fn new() -> Self {
        Self {
            hovered: None,
            hover_path: HashSet::new(),
            active: None,
            active_path: HashSet::new(),
            mouse_capture: None,
        }
    }

    /// Update hover state based on hit test result
    /// In a real implementation, this would walk the hitbox tree to include ancestors
    pub fn update_hover(&mut self, hitbox_id: Option<HitboxId>) {
        self.hovered = hitbox_id;
        self.hover_path.clear();
        if let Some(id) = hitbox_id {
            // For now, just the element itself
            // Phase 5 builder API will add parent tracking for ancestor hover
            self.hover_path.insert(id);
        }
    }

    /// Set active state when mouse button is pressed
    pub fn set_active(&mut self, hitbox_id: HitboxId) {
        self.active = Some(hitbox_id);
        self.active_path.clear();
        // For now, just the element itself
        // Phase 5 builder API will add parent tracking for ancestor active
        self.active_path.insert(hitbox_id);
    }

    /// Clear active state when mouse button is released
    pub fn clear_active(&mut self) {
        self.active = None;
        self.active_path.clear();
    }

    /// Check if a hitbox is currently hovered (includes ancestors)
    pub fn is_hovered(&self, hitbox_id: HitboxId) -> bool {
        self.hover_path.contains(&hitbox_id)
    }

    /// Check if a hitbox is currently active (includes ancestors)
    pub fn is_active(&self, hitbox_id: HitboxId) -> bool {
        self.active_path.contains(&hitbox_id)
    }

    /// Get the currently hovered hitbox
    pub fn hovered_hitbox(&self) -> Option<HitboxId> {
        self.hovered
    }

    /// Get the currently active hitbox
    pub fn active_hitbox(&self) -> Option<HitboxId> {
        self.active
    }

    /// Capture mouse for drag operations
    /// While captured, mouse events are routed to the capturing element even outside bounds
    pub fn capture_mouse(&mut self, hitbox_id: HitboxId, button: MouseButton) {
        self.mouse_capture = Some(MouseCapture { hitbox_id, button });
    }

    /// Release mouse capture
    pub fn release_mouse_capture(&mut self) {
        self.mouse_capture = None;
    }

    /// Get current mouse capture state
    pub fn mouse_capture(&self) -> Option<MouseCapture> {
        self.mouse_capture
    }

    /// Check if mouse is currently captured
    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_capture.is_some()
    }

    /// Clear all interaction state (called when window loses focus)
    pub fn clear_all(&mut self) {
        self.hovered = None;
        self.hover_path.clear();
        self.active = None;
        self.active_path.clear();
        self.mouse_capture = None;
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_tracking() {
        let mut state = InteractionState::new();
        let id = HitboxId(1);

        assert_eq!(state.hovered_hitbox(), None);
        assert!(!state.is_hovered(id));

        state.update_hover(Some(id));
        assert_eq!(state.hovered_hitbox(), Some(id));
        assert!(state.is_hovered(id));

        state.update_hover(None);
        assert_eq!(state.hovered_hitbox(), None);
        assert!(!state.is_hovered(id));
    }

    #[test]
    fn test_active_tracking() {
        let mut state = InteractionState::new();
        let id = HitboxId(1);

        assert_eq!(state.active_hitbox(), None);
        assert!(!state.is_active(id));

        state.set_active(id);
        assert_eq!(state.active_hitbox(), Some(id));
        assert!(state.is_active(id));

        state.clear_active();
        assert_eq!(state.active_hitbox(), None);
        assert!(!state.is_active(id));
    }

    #[test]
    fn test_mouse_capture() {
        let mut state = InteractionState::new();
        let id = HitboxId(1);
        let button = MouseButton::Left;

        assert!(!state.is_mouse_captured());
        assert_eq!(state.mouse_capture(), None);

        state.capture_mouse(id, button);
        assert!(state.is_mouse_captured());
        assert_eq!(state.mouse_capture(), Some(MouseCapture { hitbox_id: id, button }));

        state.release_mouse_capture();
        assert!(!state.is_mouse_captured());
        assert_eq!(state.mouse_capture(), None);
    }

    #[test]
    fn test_clear_all() {
        let mut state = InteractionState::new();
        let id = HitboxId(1);

        state.update_hover(Some(id));
        state.set_active(id);
        state.capture_mouse(id, MouseButton::Left);

        assert!(state.is_hovered(id));
        assert!(state.is_active(id));
        assert!(state.is_mouse_captured());

        state.clear_all();

        assert!(!state.is_hovered(id));
        assert!(!state.is_active(id));
        assert!(!state.is_mouse_captured());
    }
}
