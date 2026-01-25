//! Widget trait and associated types.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::components::Bounds;
use crate::design_system::{
    InteractionState, StyledRect, TextBlock,
    LayoutConstraints,
};

// Global widget ID counter
static NEXT_WIDGET_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a widget instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Generate a new unique widget ID.
    pub fn new() -> Self {
        Self(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

/// Mouse/pointer button types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
}

/// Pointer (mouse/touch) events.
#[derive(Clone, Debug)]
pub enum PointerEvent {
    /// Pointer entered the widget bounds
    Enter,
    /// Pointer left the widget bounds
    Leave,
    /// Pointer moved within the widget
    Move { x: f32, y: f32 },
    /// Pointer button pressed
    Down { button: PointerButton, x: f32, y: f32 },
    /// Pointer button released
    Up { button: PointerButton, x: f32, y: f32 },
    /// Click event (down + up in same widget)
    Click { button: PointerButton, x: f32, y: f32 },
    /// Scroll event
    Scroll { delta_x: f32, delta_y: f32 },
}

/// Keyboard events.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    /// The key that was pressed/released
    pub key: KeyCode,
    /// Whether this is a key down or up event
    pub pressed: bool,
    /// Modifier keys held during the event
    pub modifiers: Modifiers,
    /// Text input (for typing)
    pub text: Option<String>,
}

/// Key codes for keyboard events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    // Navigation
    Tab,
    Enter,
    Escape,
    Space,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,

    // Letters (for shortcuts)
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Other
    Other,
}

/// Modifier key state.
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    pub fn with_shift() -> Self {
        Self { shift: true, ..Default::default() }
    }
}

/// Events that widgets can emit.
#[derive(Clone, Debug)]
pub enum WidgetEvent {
    /// Widget was clicked
    Clicked,
    /// Widget value changed (for inputs, toggles, etc.)
    ValueChanged(WidgetValue),
    /// Widget gained focus
    FocusGained,
    /// Widget lost focus
    FocusLost,
    /// Custom event with string identifier
    Custom(String),
}

/// Generic widget value for value-changed events.
#[derive(Clone, Debug)]
pub enum WidgetValue {
    Bool(bool),
    String(String),
    Number(f64),
    None,
}

/// Output from building a widget for rendering.
#[derive(Clone, Debug, Default)]
pub struct WidgetOutput {
    /// Styled rectangles to render
    pub rects: Vec<StyledRect>,
    /// Text blocks to render
    pub texts: Vec<TextBlock>,
    /// Events emitted during this build
    pub events: Vec<WidgetEvent>,
}

impl WidgetOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rect(mut self, rect: StyledRect) -> Self {
        self.rects.push(rect);
        self
    }

    pub fn with_text(mut self, text: TextBlock) -> Self {
        self.texts.push(text);
        self
    }

    pub fn with_event(mut self, event: WidgetEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn add_rect(&mut self, rect: StyledRect) {
        self.rects.push(rect);
    }

    pub fn add_text(&mut self, text: TextBlock) {
        self.texts.push(text);
    }

    pub fn emit(&mut self, event: WidgetEvent) {
        self.events.push(event);
    }

    pub fn merge(&mut self, other: WidgetOutput) {
        self.rects.extend(other.rects);
        self.texts.extend(other.texts);
        self.events.extend(other.events);
    }
}

/// The core widget trait that all widgets implement.
pub trait Widget {
    /// Get the unique ID of this widget.
    fn id(&self) -> WidgetId;

    /// Get the current interaction state.
    fn interaction_state(&self) -> InteractionState;

    /// Update widget state (animations, timers, etc.)
    /// Called every frame. Returns true if the widget needs to be redrawn.
    fn update(&mut self, dt: f32) -> bool;

    /// Calculate the widget's size given constraints.
    /// Returns the actual size the widget will use.
    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32);

    /// Set the widget's bounds after layout.
    fn set_bounds(&mut self, bounds: Bounds);

    /// Get the widget's current bounds.
    fn bounds(&self) -> Bounds;

    /// Build the render output for this widget.
    fn build(&mut self) -> WidgetOutput;

    /// Handle a pointer event.
    /// Returns true if the event was handled.
    fn on_pointer(&mut self, event: PointerEvent) -> bool;

    /// Handle a keyboard event.
    /// Returns true if the event was handled.
    fn on_key(&mut self, event: KeyEvent) -> bool;

    /// Handle focus change.
    fn on_focus(&mut self, gained: bool);

    /// Check if this widget can receive focus.
    fn can_focus(&self) -> bool {
        true
    }

    /// Check if a point is within this widget's bounds.
    fn hit_test(&self, x: f32, y: f32) -> bool {
        let b = self.bounds();
        x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height
    }
}
