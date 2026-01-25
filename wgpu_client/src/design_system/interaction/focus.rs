//! Focus management for keyboard navigation.
//!
//! This module provides focus tracking and tab-order navigation for accessible
//! keyboard-based UI interaction.

/// Unique identifier for focusable components.
pub type FocusId = u64;

/// Direction for focus navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDirection {
    /// Move to next focusable element (Tab)
    Next,
    /// Move to previous focusable element (Shift+Tab)
    Previous,
    /// Move up in a 2D layout
    Up,
    /// Move down in a 2D layout
    Down,
    /// Move left in a 2D layout
    Left,
    /// Move right in a 2D layout
    Right,
}

/// Trait for focusable components.
pub trait Focusable {
    /// Get the focus ID for this component.
    fn focus_id(&self) -> FocusId;

    /// Check if this component can receive focus.
    fn can_focus(&self) -> bool {
        true
    }

    /// Get the tab index for ordering (lower = earlier in tab order).
    fn tab_index(&self) -> i32 {
        0
    }

    /// Handle focus gained event.
    fn on_focus_gained(&mut self);

    /// Handle focus lost event.
    fn on_focus_lost(&mut self);
}

/// Manages focus state across multiple components.
pub struct FocusManager {
    /// Currently focused component ID
    current_focus: Option<FocusId>,
    /// Registered focusable components in tab order
    focus_order: Vec<FocusEntry>,
    /// Counter for generating unique IDs
    next_id: FocusId,
    /// Whether focus ring should be visible (hidden after mouse click)
    show_focus_ring: bool,
}

#[derive(Clone, Debug)]
struct FocusEntry {
    id: FocusId,
    tab_index: i32,
    can_focus: bool,
}

impl FocusManager {
    /// Create a new focus manager.
    pub fn new() -> Self {
        Self {
            current_focus: None,
            focus_order: Vec::new(),
            next_id: 1,
            show_focus_ring: true,
        }
    }

    /// Generate a new unique focus ID.
    pub fn next_id(&mut self) -> FocusId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Register a focusable component.
    pub fn register(&mut self, id: FocusId, tab_index: i32, can_focus: bool) {
        // Remove any existing entry with this ID
        self.focus_order.retain(|e| e.id != id);

        if can_focus {
            self.focus_order.push(FocusEntry { id, tab_index, can_focus });
            // Sort by tab_index, then by registration order (stable sort)
            self.focus_order.sort_by_key(|e| e.tab_index);
        }
    }

    /// Unregister a focusable component.
    pub fn unregister(&mut self, id: FocusId) {
        self.focus_order.retain(|e| e.id != id);
        if self.current_focus == Some(id) {
            self.current_focus = None;
        }
    }

    /// Set focus to a specific component.
    pub fn set_focus(&mut self, id: FocusId) -> Option<FocusId> {
        let old_focus = self.current_focus;
        if self.focus_order.iter().any(|e| e.id == id && e.can_focus) {
            self.current_focus = Some(id);
        }
        old_focus
    }

    /// Clear focus.
    pub fn clear_focus(&mut self) -> Option<FocusId> {
        self.current_focus.take()
    }

    /// Get the currently focused component.
    pub fn current_focus(&self) -> Option<FocusId> {
        self.current_focus
    }

    /// Check if a specific component has focus.
    pub fn has_focus(&self, id: FocusId) -> bool {
        self.current_focus == Some(id)
    }

    /// Navigate focus in a direction.
    /// Returns (old_focus, new_focus) if focus changed.
    pub fn navigate(&mut self, direction: FocusDirection) -> Option<(Option<FocusId>, FocusId)> {
        if self.focus_order.is_empty() {
            return None;
        }

        // Show focus ring when navigating with keyboard
        self.show_focus_ring = true;

        let focusable: Vec<_> = self.focus_order.iter()
            .filter(|e| e.can_focus)
            .collect();

        if focusable.is_empty() {
            return None;
        }

        let current_idx = self.current_focus
            .and_then(|id| focusable.iter().position(|e| e.id == id));

        let new_idx = match direction {
            FocusDirection::Next => {
                match current_idx {
                    Some(idx) => (idx + 1) % focusable.len(),
                    None => 0,
                }
            }
            FocusDirection::Previous => {
                match current_idx {
                    Some(idx) => {
                        if idx == 0 {
                            focusable.len() - 1
                        } else {
                            idx - 1
                        }
                    }
                    None => focusable.len() - 1,
                }
            }
            // For 2D navigation, we'd need position info. For now, treat as Next/Previous.
            FocusDirection::Down | FocusDirection::Right => {
                match current_idx {
                    Some(idx) => (idx + 1).min(focusable.len() - 1),
                    None => 0,
                }
            }
            FocusDirection::Up | FocusDirection::Left => {
                match current_idx {
                    Some(idx) => idx.saturating_sub(1),
                    None => 0,
                }
            }
        };

        let old_focus = self.current_focus;
        let new_focus = focusable[new_idx].id;

        if self.current_focus != Some(new_focus) {
            self.current_focus = Some(new_focus);
            Some((old_focus, new_focus))
        } else {
            None
        }
    }

    /// Focus the first focusable component.
    pub fn focus_first(&mut self) -> Option<FocusId> {
        let first = self.focus_order.iter()
            .find(|e| e.can_focus)
            .map(|e| e.id);

        if let Some(id) = first {
            self.current_focus = Some(id);
        }
        first
    }

    /// Focus the last focusable component.
    pub fn focus_last(&mut self) -> Option<FocusId> {
        let last = self.focus_order.iter()
            .rev()
            .find(|e| e.can_focus)
            .map(|e| e.id);

        if let Some(id) = last {
            self.current_focus = Some(id);
        }
        last
    }

    /// Called when user interacts with mouse.
    /// Hides the focus ring until keyboard navigation is used.
    pub fn on_mouse_interaction(&mut self) {
        self.show_focus_ring = false;
    }

    /// Check if the focus ring should be visible.
    pub fn should_show_focus_ring(&self) -> bool {
        self.show_focus_ring && self.current_focus.is_some()
    }

    /// Update the can_focus state for a component.
    pub fn set_can_focus(&mut self, id: FocusId, can_focus: bool) {
        if let Some(entry) = self.focus_order.iter_mut().find(|e| e.id == id) {
            entry.can_focus = can_focus;
            // If the currently focused element can no longer focus, clear it
            if !can_focus && self.current_focus == Some(id) {
                self.current_focus = None;
            }
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_navigation() {
        let mut fm = FocusManager::new();
        let id1 = fm.next_id();
        let id2 = fm.next_id();
        let id3 = fm.next_id();

        fm.register(id1, 0, true);
        fm.register(id2, 0, true);
        fm.register(id3, 0, true);

        // Navigate forward through all
        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id1));

        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id2));

        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id3));

        // Wrap around
        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id1));

        // Navigate backward
        fm.navigate(FocusDirection::Previous);
        assert_eq!(fm.current_focus(), Some(id3));
    }

    #[test]
    fn test_tab_index_ordering() {
        let mut fm = FocusManager::new();
        let id1 = fm.next_id(); // tab_index 2
        let id2 = fm.next_id(); // tab_index 1
        let id3 = fm.next_id(); // tab_index 0

        fm.register(id1, 2, true);
        fm.register(id2, 1, true);
        fm.register(id3, 0, true);

        fm.focus_first();
        assert_eq!(fm.current_focus(), Some(id3)); // Lowest tab_index

        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id2));

        fm.navigate(FocusDirection::Next);
        assert_eq!(fm.current_focus(), Some(id1));
    }
}
