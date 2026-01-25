//! Interaction state machine for UI components.
//!
//! Components go through various interaction states based on user input.
//! This module provides a state machine to track and transition between states.

use crate::theme::Color;
use crate::design_system::tokens::{ColorRole, ColorPalette};

/// Interaction states for UI components.
///
/// States are mutually exclusive - a component is in exactly one state at a time.
/// The state machine handles transitions based on pointer and keyboard events.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionState {
    /// Default state - no interaction
    #[default]
    Idle,
    /// Pointer is over the component
    Hovered,
    /// Component is being pressed (mouse down)
    Pressed,
    /// Component has keyboard focus
    Focused,
    /// Component has focus and pointer is hovering
    FocusedHovered,
    /// Component is disabled and cannot be interacted with
    Disabled,
    /// Component is selected (e.g., toggle on, checkbox checked)
    Selected,
    /// Selected and pointer is hovering
    SelectedHovered,
    /// Selected and has keyboard focus
    SelectedFocused,
    /// Component has a validation error
    Error,
    /// Error state with keyboard focus
    ErrorFocused,
}

impl InteractionState {
    /// Check if the component is in a hovered state.
    pub fn is_hovered(self) -> bool {
        matches!(self,
            InteractionState::Hovered |
            InteractionState::FocusedHovered |
            InteractionState::SelectedHovered
        )
    }

    /// Check if the component is in a pressed state.
    pub fn is_pressed(self) -> bool {
        matches!(self, InteractionState::Pressed)
    }

    /// Check if the component has focus.
    pub fn is_focused(self) -> bool {
        matches!(self,
            InteractionState::Focused |
            InteractionState::FocusedHovered |
            InteractionState::SelectedFocused |
            InteractionState::ErrorFocused
        )
    }

    /// Check if the component is disabled.
    pub fn is_disabled(self) -> bool {
        matches!(self, InteractionState::Disabled)
    }

    /// Check if the component is selected.
    pub fn is_selected(self) -> bool {
        matches!(self,
            InteractionState::Selected |
            InteractionState::SelectedHovered |
            InteractionState::SelectedFocused
        )
    }

    /// Check if the component has an error.
    pub fn has_error(self) -> bool {
        matches!(self, InteractionState::Error | InteractionState::ErrorFocused)
    }

    /// Check if the component can receive interaction.
    pub fn is_interactive(self) -> bool {
        !self.is_disabled()
    }

    /// Get the appropriate background color role for this state.
    pub fn bg_color_role(self) -> ColorRole {
        match self {
            InteractionState::Idle => ColorRole::Interactive,
            InteractionState::Hovered => ColorRole::InteractiveHover,
            InteractionState::Pressed => ColorRole::InteractivePressed,
            InteractionState::Focused => ColorRole::Interactive,
            InteractionState::FocusedHovered => ColorRole::InteractiveHover,
            InteractionState::Disabled => ColorRole::InteractiveDisabled,
            InteractionState::Selected => ColorRole::AccentPrimary,
            InteractionState::SelectedHovered => ColorRole::AccentSecondary,
            InteractionState::SelectedFocused => ColorRole::AccentPrimary,
            InteractionState::Error => ColorRole::Error,
            InteractionState::ErrorFocused => ColorRole::Error,
        }
    }

    /// Get the appropriate border color role for this state.
    pub fn border_color_role(self) -> ColorRole {
        match self {
            InteractionState::Focused |
            InteractionState::FocusedHovered |
            InteractionState::SelectedFocused |
            InteractionState::ErrorFocused => ColorRole::BorderFocus,
            InteractionState::Error => ColorRole::Error,
            InteractionState::Disabled => ColorRole::BorderSubtle,
            _ => ColorRole::BorderDefault,
        }
    }

    /// Get the appropriate foreground color role for this state.
    pub fn fg_color_role(self) -> ColorRole {
        match self {
            InteractionState::Disabled => ColorRole::FgMuted,
            InteractionState::Selected |
            InteractionState::SelectedHovered |
            InteractionState::SelectedFocused => ColorRole::FgInverse,
            InteractionState::Error |
            InteractionState::ErrorFocused => ColorRole::Error,
            _ => ColorRole::FgPrimary,
        }
    }
}

/// Tracks and manages interaction state for a component.
///
/// Use this in your component to handle state transitions based on input events.
pub struct InteractionTracker {
    current_state: InteractionState,
    is_disabled: bool,
    is_selected: bool,
    has_error: bool,
    is_hovered: bool,
    is_pressed: bool,
    is_focused: bool,
}

impl InteractionTracker {
    /// Create a new interaction tracker.
    pub fn new() -> Self {
        Self {
            current_state: InteractionState::Idle,
            is_disabled: false,
            is_selected: false,
            has_error: false,
            is_hovered: false,
            is_pressed: false,
            is_focused: false,
        }
    }

    /// Get the current interaction state.
    pub fn state(&self) -> InteractionState {
        self.current_state
    }

    /// Set disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.is_disabled = disabled;
        self.update_state();
    }

    /// Set selected state (for toggleable components).
    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        self.update_state();
    }

    /// Set error state.
    pub fn set_error(&mut self, error: bool) {
        self.has_error = error;
        self.update_state();
    }

    /// Handle pointer entering the component bounds.
    pub fn on_pointer_enter(&mut self) {
        if !self.is_disabled {
            self.is_hovered = true;
            self.update_state();
        }
    }

    /// Handle pointer leaving the component bounds.
    pub fn on_pointer_leave(&mut self) {
        self.is_hovered = false;
        self.is_pressed = false;
        self.update_state();
    }

    /// Handle pointer press (mouse down).
    pub fn on_pointer_down(&mut self) {
        if !self.is_disabled {
            self.is_pressed = true;
            self.update_state();
        }
    }

    /// Handle pointer release (mouse up).
    pub fn on_pointer_up(&mut self) -> bool {
        let was_pressed = self.is_pressed;
        self.is_pressed = false;
        self.update_state();
        // Return true if this was a valid click (was pressed and pointer still over)
        was_pressed && self.is_hovered && !self.is_disabled
    }

    /// Handle focus gained.
    pub fn on_focus(&mut self) {
        if !self.is_disabled {
            self.is_focused = true;
            self.update_state();
        }
    }

    /// Handle focus lost.
    pub fn on_blur(&mut self) {
        self.is_focused = false;
        self.update_state();
    }

    /// Check if a click event should be fired.
    /// Call after on_pointer_up returns true.
    pub fn should_fire_click(&self) -> bool {
        !self.is_disabled && self.is_hovered
    }

    /// Update the computed state based on flags.
    fn update_state(&mut self) {
        self.current_state = if self.is_disabled {
            InteractionState::Disabled
        } else if self.has_error {
            if self.is_focused {
                InteractionState::ErrorFocused
            } else {
                InteractionState::Error
            }
        } else if self.is_pressed {
            InteractionState::Pressed
        } else if self.is_selected {
            if self.is_focused && self.is_hovered {
                InteractionState::SelectedFocused
            } else if self.is_focused {
                InteractionState::SelectedFocused
            } else if self.is_hovered {
                InteractionState::SelectedHovered
            } else {
                InteractionState::Selected
            }
        } else if self.is_focused {
            if self.is_hovered {
                InteractionState::FocusedHovered
            } else {
                InteractionState::Focused
            }
        } else if self.is_hovered {
            InteractionState::Hovered
        } else {
            InteractionState::Idle
        };
    }

    /// Check if the component is currently hovered.
    pub fn is_hovered(&self) -> bool {
        self.is_hovered
    }

    /// Check if the component is currently pressed.
    pub fn is_pressed(&self) -> bool {
        self.is_pressed
    }

    /// Check if the component is currently focused.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Check if the component is disabled.
    pub fn is_disabled(&self) -> bool {
        self.is_disabled
    }

    /// Check if the component is selected.
    pub fn is_selected(&self) -> bool {
        self.is_selected
    }
}

impl Default for InteractionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_state() {
        let tracker = InteractionTracker::new();
        assert_eq!(tracker.state(), InteractionState::Idle);
    }

    #[test]
    fn test_hover_transition() {
        let mut tracker = InteractionTracker::new();
        tracker.on_pointer_enter();
        assert_eq!(tracker.state(), InteractionState::Hovered);
        tracker.on_pointer_leave();
        assert_eq!(tracker.state(), InteractionState::Idle);
    }

    #[test]
    fn test_press_transition() {
        let mut tracker = InteractionTracker::new();
        tracker.on_pointer_enter();
        tracker.on_pointer_down();
        assert_eq!(tracker.state(), InteractionState::Pressed);
        let clicked = tracker.on_pointer_up();
        assert!(clicked);
        assert_eq!(tracker.state(), InteractionState::Hovered);
    }

    #[test]
    fn test_disabled_blocks_interaction() {
        let mut tracker = InteractionTracker::new();
        tracker.set_disabled(true);
        tracker.on_pointer_enter();
        assert_eq!(tracker.state(), InteractionState::Disabled);
    }

    #[test]
    fn test_selected_states() {
        let mut tracker = InteractionTracker::new();
        tracker.set_selected(true);
        assert_eq!(tracker.state(), InteractionState::Selected);
        tracker.on_pointer_enter();
        assert_eq!(tracker.state(), InteractionState::SelectedHovered);
    }
}
