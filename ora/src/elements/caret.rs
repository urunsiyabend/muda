//! Blinking caret element for text editing.
//!
//! Displays a thin beam cursor that blinks with configurable rate and activity timeout.
//! The caret stays solid during typing activity to avoid distracting blinks while editing.
//!
//! # Blink Behavior
//!
//! - Default blink rate: 500ms (safe: 2 blinks/sec < 3/sec WCAG limit)
//! - Activity timeout: 500ms (caret stays solid for 500ms after keyboard activity)
//! - Width: 2px thin beam style (per CONTEXT decision)
//!
//! # Usage
//!
//! ```ignore
//! let mut caret = CaretElement::new(x, y, line_height);
//!
//! // On keyboard activity (typing):
//! caret.on_activity();
//!
//! // In frame loop (call periodically):
//! if caret.update() {
//!     // Request window redraw
//! }
//! ```

use std::time::{Duration, Instant};

use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::style::{Color, Style, Length, Background};
use crate::style::units::{Rect, Size};
use crate::theme::ColorToken;

/// Blink rate: 500ms per blink state (2 blinks/sec, WCAG-safe)
pub const BLINK_RATE: Duration = Duration::from_millis(500);

/// Activity timeout: caret stays solid for 500ms after user activity
pub const ACTIVITY_TIMEOUT: Duration = Duration::from_millis(500);

/// Caret width: 2px thin beam style
pub const CARET_WIDTH: f32 = 2.0;

/// Blinking caret element for text editing.
///
/// The caret displays as a thin vertical beam at the specified position.
/// It blinks at a configurable rate but stays solid during typing activity
/// to avoid distracting visual changes while the user is editing.
///
/// # Blink Logic
///
/// The caret maintains two timing states:
/// - `last_activity`: When the user last typed (keyboard activity)
/// - `last_blink`: When the blink state last toggled
///
/// During the activity timeout period (500ms after last activity),
/// the caret remains solid (visible). After the timeout, it begins
/// blinking at the configured rate.
///
/// # Example
///
/// ```ignore
/// let mut caret = CaretElement::new(100.0, 50.0, 21.0);
///
/// // When user types, call on_activity() to keep caret solid:
/// caret.on_activity();
///
/// // In frame loop, call update() and redraw if needed:
/// if caret.update() {
///     window.request_redraw();
/// }
/// ```
pub struct CaretElement {
    /// Whether the caret is currently visible (blink state).
    blink_visible: bool,
    /// Timestamp of last blink toggle.
    last_blink: Instant,
    /// Timestamp of last user activity (keyboard input).
    last_activity: Instant,
    /// Blink rate (duration per state).
    blink_rate: Duration,
    /// X position in pixels.
    x: f32,
    /// Y position in pixels.
    y: f32,
    /// Height in pixels (typically line height).
    height: f32,
    /// Custom color override (uses AccentPrimary if None).
    color: Option<Color>,
}

impl CaretElement {
    /// Creates a new caret element at the specified position.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in pixels
    /// * `y` - Y coordinate in pixels
    /// * `height` - Height in pixels (typically line height)
    pub fn new(x: f32, y: f32, height: f32) -> Self {
        let now = Instant::now();
        Self {
            blink_visible: true,
            last_blink: now,
            last_activity: now,
            blink_rate: BLINK_RATE,
            x,
            y,
            height,
            color: None,
        }
    }

    /// Sets the caret position.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    /// Gets the current X position.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Gets the current Y position.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// Gets the caret height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Sets the caret height.
    pub fn set_height(&mut self, height: f32) {
        self.height = height;
    }

    /// Sets a custom color for the caret.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the blink rate.
    pub fn set_blink_rate(&mut self, rate: Duration) {
        self.blink_rate = rate;
    }

    /// Returns whether the caret is currently visible.
    pub fn is_visible(&self) -> bool {
        self.blink_visible
    }

    /// Call when user activity occurs (keyboard input).
    ///
    /// This resets the activity timer and makes the caret solid (visible).
    /// The caret will stay solid for ACTIVITY_TIMEOUT (500ms) after the
    /// last activity before resuming blinking.
    pub fn on_activity(&mut self) {
        self.last_activity = Instant::now();
        self.blink_visible = true;
    }

    /// Updates the blink state.
    ///
    /// Call this periodically (e.g., in frame loop or timer).
    /// Returns `true` if the blink state changed (requires redraw).
    ///
    /// # Returns
    ///
    /// `true` if the visibility state changed and a redraw is needed,
    /// `false` if no visual change occurred.
    pub fn update(&mut self) -> bool {
        let now = Instant::now();

        // Don't blink if user was recently active
        if now.duration_since(self.last_activity) < ACTIVITY_TIMEOUT {
            return false;
        }

        // Toggle blink state if enough time has passed
        if now.duration_since(self.last_blink) >= self.blink_rate {
            self.blink_visible = !self.blink_visible;
            self.last_blink = now;
            return true; // Redraw needed
        }

        false
    }

    /// Returns duration until the next blink state change.
    ///
    /// Useful for scheduling timers to avoid polling every frame.
    pub fn time_until_next_blink(&self) -> Duration {
        let now = Instant::now();

        // If in activity timeout, return time until timeout ends
        let since_activity = now.duration_since(self.last_activity);
        if since_activity < ACTIVITY_TIMEOUT {
            return ACTIVITY_TIMEOUT - since_activity;
        }

        // Otherwise, return time until next blink toggle
        let since_blink = now.duration_since(self.last_blink);
        if since_blink < self.blink_rate {
            self.blink_rate - since_blink
        } else {
            Duration::ZERO
        }
    }
}

/// State persisted through the rendering lifecycle.
pub struct CaretState {
    layout_id: LayoutId,
}

impl Element for CaretElement {
    type RequestLayoutState = CaretState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, CaretState) {
        // Create a style with fixed size
        let mut style = Style::default();
        style.width = Length::Px(CARET_WIDTH);
        style.height = Length::Px(self.height);

        let id = cx.request_layout(&style);
        cx.set_intrinsic_size(id, Size::new(CARET_WIDTH, self.height));

        (id, CaretState { layout_id: id })
    }

    fn prepaint(&mut self, _state: &mut CaretState, _cx: &mut PrepaintContext) {
        // Caret doesn't need hitbox (not interactive)
    }

    fn paint(&mut self, state: &mut CaretState, cx: &mut PaintContext) {
        if !self.blink_visible {
            return;
        }

        let theme = cx.theme();
        let color = self.color.unwrap_or_else(|| theme.color(ColorToken::Accent));

        // Offset caret position relative to where the element was laid out.
        // self.x/self.y are offsets within the text area (column * char_width,
        // row * line_height). The layout system positions the caret element
        // within the text area container, so we add computed_bounds.origin to
        // convert to absolute window coordinates.
        let computed_bounds = cx.bounds(state.layout_id);
        let bounds = Rect::new(
            computed_bounds.origin.x + self.x,
            computed_bounds.origin.y + self.y,
            CARET_WIDTH,
            self.height,
        );

        // Paint the caret as a simple filled rectangle
        let style = Style {
            background: Background::Solid(color),
            ..Style::default()
        };
        cx.paint_styled_rect(&style, &bounds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caret_creation() {
        let caret = CaretElement::new(100.0, 50.0, 21.0);
        assert_eq!(caret.x(), 100.0);
        assert_eq!(caret.y(), 50.0);
        assert_eq!(caret.height(), 21.0);
        assert!(caret.is_visible());
    }

    #[test]
    fn test_caret_position() {
        let mut caret = CaretElement::new(0.0, 0.0, 21.0);
        caret.set_position(100.0, 200.0);
        assert_eq!(caret.x(), 100.0);
        assert_eq!(caret.y(), 200.0);
    }

    #[test]
    fn test_caret_on_activity() {
        let mut caret = CaretElement::new(0.0, 0.0, 21.0);
        caret.blink_visible = false;
        caret.on_activity();
        assert!(caret.is_visible());
    }

    #[test]
    fn test_caret_constants() {
        assert_eq!(BLINK_RATE, Duration::from_millis(500));
        assert_eq!(ACTIVITY_TIMEOUT, Duration::from_millis(500));
        assert_eq!(CARET_WIDTH, 2.0);
    }

    #[test]
    fn test_caret_no_blink_during_activity() {
        let mut caret = CaretElement::new(0.0, 0.0, 21.0);
        caret.on_activity();

        // Update immediately after activity should not change visibility
        let changed = caret.update();
        assert!(!changed);
        assert!(caret.is_visible());
    }

    #[test]
    fn test_caret_time_until_next_blink() {
        let caret = CaretElement::new(0.0, 0.0, 21.0);
        let time = caret.time_until_next_blink();
        // Should be approximately ACTIVITY_TIMEOUT since we just created it
        assert!(time <= ACTIVITY_TIMEOUT);
    }
}
