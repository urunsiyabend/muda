use std::sync::Arc;

/// Stable identifier for focusable elements.
/// Persists across re-renders as long as the FocusHandle is alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub(crate) u64);

/// Tracks how focus was gained.
/// Used to determine whether focus rings should be visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusSource {
    /// Focus gained via keyboard navigation (Tab, Shift+Tab, arrow keys).
    /// Focus rings should be visible.
    Keyboard,
    /// Focus gained via mouse click.
    /// Focus rings should be hidden.
    Mouse,
    /// Focus set programmatically via code.
    /// Focus rings should be hidden.
    Programmatic,
}

/// Reference-counted handle to a focusable element.
/// The focus identity remains stable across re-renders as long as any handle exists.
#[derive(Clone)]
pub struct FocusHandle {
    pub(crate) id: FocusId,
    /// Reference counting - focus is "alive" while any handle exists.
    _ref: Arc<()>,
}

impl FocusHandle {
    /// Get the stable FocusId for this handle.
    pub fn id(&self) -> FocusId {
        self.id
    }
}

/// Centralized focus state tracking in AppContext.
/// Manages the currently focused element, focus source, and tab navigation order.
pub struct FocusState {
    /// Currently focused element.
    focused_id: Option<FocusId>,
    /// How the current focus was gained.
    focus_source: FocusSource,
    /// Counter for generating unique FocusIds.
    next_focus_id: u64,
    /// Ordered list of focusable elements (built during prepaint).
    /// Used for Tab/Shift+Tab navigation.
    focus_order: Vec<FocusId>,
}

impl FocusState {
    /// Create a new FocusState with no focused element.
    pub fn new() -> Self {
        Self {
            focused_id: None,
            focus_source: FocusSource::Programmatic,
            next_focus_id: 0,
            focus_order: Vec::new(),
        }
    }

    /// Create a new FocusHandle with a unique FocusId.
    pub fn create_focus_handle(&mut self) -> FocusHandle {
        let id = FocusId(self.next_focus_id);
        self.next_focus_id += 1;
        FocusHandle {
            id,
            _ref: Arc::new(()),
        }
    }

    /// Set the focused element with the given source.
    pub fn set_focused(&mut self, id: FocusId, source: FocusSource) {
        self.focused_id = Some(id);
        self.focus_source = source;
    }

    /// Clear the current focus.
    pub fn clear_focus(&mut self) {
        self.focused_id = None;
    }

    /// Get the currently focused element ID.
    pub fn focused_id(&self) -> Option<FocusId> {
        self.focused_id
    }

    /// Returns true if focus was gained via keyboard and focus rings should be visible.
    pub fn is_keyboard_focused(&self) -> bool {
        matches!(self.focus_source, FocusSource::Keyboard)
    }

    /// Clear the focus order list.
    /// Called at the start of prepaint to rebuild the tab order.
    pub fn clear_focus_order(&mut self) {
        self.focus_order.clear();
    }

    /// Register a focusable element in tab order.
    /// Called during prepaint as elements are visited in tree order.
    pub fn register_focusable(&mut self, id: FocusId) {
        self.focus_order.push(id);
    }

    /// Move focus to the next element in tab order.
    /// Wraps around to the first element if at the end.
    pub fn focus_next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        let current_index = self.focused_id.and_then(|id| {
            self.focus_order.iter().position(|&focus_id| focus_id == id)
        });

        let next_index = match current_index {
            Some(idx) => (idx + 1) % self.focus_order.len(),
            None => 0,
        };

        if let Some(&next_id) = self.focus_order.get(next_index) {
            self.set_focused(next_id, FocusSource::Keyboard);
        }
    }

    /// Move focus to the previous element in tab order.
    /// Wraps around to the last element if at the beginning.
    pub fn focus_prev(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        let current_index = self.focused_id.and_then(|id| {
            self.focus_order.iter().position(|&focus_id| focus_id == id)
        });

        let prev_index = match current_index {
            Some(0) => self.focus_order.len() - 1,
            Some(idx) => idx - 1,
            None => self.focus_order.len() - 1,
        };

        if let Some(&prev_id) = self.focus_order.get(prev_index) {
            self.set_focused(prev_id, FocusSource::Keyboard);
        }
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}
