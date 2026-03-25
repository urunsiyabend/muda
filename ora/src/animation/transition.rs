//! Transition infrastructure for CSS-like property animation.
//!
//! Provides:
//! - `TransitionId` — stable cross-frame identity assigned by callers
//! - `TransitionSpec` — declares which properties can transition and how
//! - `TransitionConfig` — duration + easing for a single property
//! - `TransitionState` — live tween data for one element
//! - `TransitionRegistry` — per-element state store owned by AppContext

use std::collections::HashMap;
use std::time::Duration;
use super::easing::Easing;
use super::tween::{Tween, TweenState};
use crate::style::Color;

// ─────────────────────────────────────────────
// TransitionId
// ─────────────────────────────────────────────

/// Stable identity for transition state across frames.
///
/// Elements are ephemeral (recreated each frame), but `TransitionId` persists
/// inside the `TransitionRegistry`. Callers must assign IDs explicitly — no
/// auto-generation from hitbox IDs or element positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TransitionId(pub u64);

// ─────────────────────────────────────────────
// TransitionConfig
// ─────────────────────────────────────────────

/// Configuration for a single property transition.
#[derive(Clone, Copy, Debug)]
pub struct TransitionConfig {
    /// Duration in milliseconds.
    pub duration_ms: u32,
    /// Easing function applied to the interpolation.
    pub easing: Easing,
}

impl TransitionConfig {
    /// Convenience constructor.
    pub fn new(duration_ms: u32, easing: Easing) -> Self {
        Self { duration_ms, easing }
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms as u64)
    }
}

// ─────────────────────────────────────────────
// TransitionSpec
// ─────────────────────────────────────────────

/// Declares which properties on a `Div` can transition and how.
///
/// Stored on the element struct and checked during paint. `None` means the
/// property changes instantly; `Some(config)` means it is animated.
#[derive(Clone, Debug, Default)]
pub struct TransitionSpec {
    /// Background color transition.
    pub bg: Option<TransitionConfig>,
    /// Opacity transition (0.0..=1.0).
    pub opacity: Option<TransitionConfig>,
    /// Text / border color transition.
    pub color: Option<TransitionConfig>,
    /// Paint-time transform offset (position).
    pub position: Option<TransitionConfig>,
}

// ─────────────────────────────────────────────
// TransitionState
// ─────────────────────────────────────────────

/// Live transition state for one element, stored in `TransitionRegistry`.
///
/// Each property maintains its own `Tween<T>` and a cache of the last known
/// target value. On every frame the element calls `advance_*()` which:
///
/// 1. Detects whether the target has changed.
/// 2. Retargets (or creates) the tween if so.
/// 3. Advances the tween by wall-clock elapsed time.
/// 4. Returns the interpolated value to use for painting.
pub struct TransitionState {
    /// Active tween for background color, if any.
    pub bg_tween: Option<Tween<Color>>,
    /// Active tween for opacity, if any.
    pub opacity_tween: Option<Tween<f32>>,
    /// Active tween for text/border color, if any.
    pub color_tween: Option<Tween<Color>>,
    /// Active tween for horizontal position offset, if any.
    pub position_x_tween: Option<Tween<f32>>,
    /// Active tween for vertical position offset, if any.
    pub position_y_tween: Option<Tween<f32>>,

    // Cached targets — used to detect when the caller changes the desired value.
    last_bg_target: Option<Color>,
    last_opacity_target: Option<f32>,
    last_color_target: Option<Color>,
    last_position_x_target: Option<f32>,
    last_position_y_target: Option<f32>,
}

impl TransitionState {
    /// Create a fresh state with no active tweens.
    pub fn new() -> Self {
        Self {
            bg_tween: None,
            opacity_tween: None,
            color_tween: None,
            position_x_tween: None,
            position_y_tween: None,
            last_bg_target: None,
            last_opacity_target: None,
            last_color_target: None,
            last_position_x_target: None,
            last_position_y_target: None,
        }
    }

    /// Advance the background-color tween toward `target`.
    ///
    /// Returns the current interpolated color (which equals `target` once the
    /// tween completes, or on the first call if no tween exists yet).
    pub fn advance_bg(&mut self, target: Color, config: &TransitionConfig) -> Color {
        let target_changed = self.last_bg_target.map_or(true, |prev| prev != target);

        if target_changed {
            // Capture the current value so we don't jump.
            let from = self.bg_tween.as_ref()
                .map(|t| t.value_owned())
                .unwrap_or(target);
            let mut tween = Tween::new(from, target, config.duration(), config.easing);
            tween.start();
            tween.update(); // advance once immediately so 0ms tweens complete
            self.bg_tween = Some(tween);
            self.last_bg_target = Some(target);
        } else if let Some(tween) = &mut self.bg_tween {
            tween.update();
        }

        self.bg_tween.as_ref().map(|t| t.value_owned()).unwrap_or(target)
    }

    /// Advance the opacity tween toward `target` (0.0..=1.0).
    pub fn advance_opacity(&mut self, target: f32, config: &TransitionConfig) -> f32 {
        let target_changed = self.last_opacity_target
            .map_or(true, |prev| (prev - target).abs() > f32::EPSILON);

        if target_changed {
            let from = self.opacity_tween.as_ref()
                .map(|t| t.value_owned())
                .unwrap_or(target);
            let mut tween = Tween::new(from, target, config.duration(), config.easing);
            tween.start();
            tween.update();
            self.opacity_tween = Some(tween);
            self.last_opacity_target = Some(target);
        } else if let Some(tween) = &mut self.opacity_tween {
            tween.update();
        }

        self.opacity_tween.as_ref().map(|t| t.value_owned()).unwrap_or(target)
    }

    /// Advance the text/border color tween toward `target`.
    pub fn advance_color(&mut self, target: Color, config: &TransitionConfig) -> Color {
        let target_changed = self.last_color_target.map_or(true, |prev| prev != target);

        if target_changed {
            let from = self.color_tween.as_ref()
                .map(|t| t.value_owned())
                .unwrap_or(target);
            let mut tween = Tween::new(from, target, config.duration(), config.easing);
            tween.start();
            tween.update();
            self.color_tween = Some(tween);
            self.last_color_target = Some(target);
        } else if let Some(tween) = &mut self.color_tween {
            tween.update();
        }

        self.color_tween.as_ref().map(|t| t.value_owned()).unwrap_or(target)
    }

    /// Advance the horizontal position-offset tween toward `target`.
    pub fn advance_position_x(&mut self, target: f32, config: &TransitionConfig) -> f32 {
        let target_changed = self.last_position_x_target
            .map_or(true, |prev| (prev - target).abs() > f32::EPSILON);

        if target_changed {
            let from = self.position_x_tween.as_ref()
                .map(|t| t.value_owned())
                .unwrap_or(target);
            let mut tween = Tween::new(from, target, config.duration(), config.easing);
            tween.start();
            tween.update();
            self.position_x_tween = Some(tween);
            self.last_position_x_target = Some(target);
        } else if let Some(tween) = &mut self.position_x_tween {
            tween.update();
        }

        self.position_x_tween.as_ref().map(|t| t.value_owned()).unwrap_or(target)
    }

    /// Advance the vertical position-offset tween toward `target`.
    pub fn advance_position_y(&mut self, target: f32, config: &TransitionConfig) -> f32 {
        let target_changed = self.last_position_y_target
            .map_or(true, |prev| (prev - target).abs() > f32::EPSILON);

        if target_changed {
            let from = self.position_y_tween.as_ref()
                .map(|t| t.value_owned())
                .unwrap_or(target);
            let mut tween = Tween::new(from, target, config.duration(), config.easing);
            tween.start();
            tween.update();
            self.position_y_tween = Some(tween);
            self.last_position_y_target = Some(target);
        } else if let Some(tween) = &mut self.position_y_tween {
            tween.update();
        }

        self.position_y_tween.as_ref().map(|t| t.value_owned()).unwrap_or(target)
    }

    /// Returns `true` if any tween is currently in the `Playing` state.
    pub fn is_active(&self) -> bool {
        self.bg_tween.as_ref().map_or(false, |t| t.state() == TweenState::Playing)
            || self.opacity_tween.as_ref().map_or(false, |t| t.state() == TweenState::Playing)
            || self.color_tween.as_ref().map_or(false, |t| t.state() == TweenState::Playing)
            || self.position_x_tween.as_ref().map_or(false, |t| t.state() == TweenState::Playing)
            || self.position_y_tween.as_ref().map_or(false, |t| t.state() == TweenState::Playing)
    }
}

impl Default for TransitionState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────
// TransitionRegistry
// ─────────────────────────────────────────────

/// Per-element transition state store, owned by `AppContext`.
///
/// Elements are recreated every frame; this registry is the persistent store
/// that survives across frames.  Each entry is keyed by a caller-assigned
/// `TransitionId`.
pub struct TransitionRegistry {
    states: HashMap<TransitionId, TransitionState>,
}

impl TransitionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Get the mutable transition state for `id`, creating it if absent.
    pub fn get_or_create(&mut self, id: TransitionId) -> &mut TransitionState {
        self.states.entry(id).or_insert_with(TransitionState::new)
    }

    /// Returns `true` if at least one tween is in the `Playing` state.
    pub fn has_active_transitions(&self) -> bool {
        self.states.values().any(|s| s.is_active())
    }

    /// Remove all entries whose tweens have all completed (or never started).
    /// Call this once per frame to prevent unbounded growth.
    pub fn cleanup_completed(&mut self) {
        self.states.retain(|_, state| state.is_active());
    }
}

impl Default for TransitionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn config_fast() -> TransitionConfig {
        TransitionConfig::new(200, Easing::Linear)
    }

    #[test]
    fn test_transition_state_new_is_inactive() {
        let state = TransitionState::new();
        assert!(!state.is_active(), "fresh state must not be active");
    }

    #[test]
    fn test_advance_bg_first_call_returns_target() {
        // On the very first call the tween is created with start == target,
        // so the returned value is the target itself.
        let mut state = TransitionState::new();
        let config = config_fast();
        let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        let result = state.advance_bg(red, &config);
        // start == end == red, so current should be red
        assert!((result.r - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_advance_bg_retarget_activates_tween() {
        let mut state = TransitionState::new();
        let config = config_fast();
        let start_color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        let end_color   = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

        // Establish start
        state.advance_bg(start_color, &config);
        // Now change target — should activate a new playing tween
        state.advance_bg(end_color, &config);

        assert!(state.is_active(), "tween must be playing after retarget");
    }

    #[test]
    fn test_registry_get_or_create_persists() {
        let mut registry = TransitionRegistry::new();
        let id = TransitionId(42);

        // First access creates it
        let _ = registry.get_or_create(id);
        // Second access must return the same entry (not a new blank one)
        let config = config_fast();
        let color = Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };
        registry.get_or_create(id).advance_bg(color, &config);

        // Verify the entry exists and has the tween
        let state = registry.get_or_create(id);
        assert!(state.bg_tween.is_some());
    }

    #[test]
    fn test_has_active_transitions_false_when_empty() {
        let registry = TransitionRegistry::new();
        assert!(!registry.has_active_transitions());
    }

    #[test]
    fn test_has_active_transitions_true_after_retarget() {
        let mut registry = TransitionRegistry::new();
        let id = TransitionId(1);
        let config = config_fast();

        let c1 = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        let c2 = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

        // Establish baseline then retarget
        registry.get_or_create(id).advance_bg(c1, &config);
        registry.get_or_create(id).advance_bg(c2, &config);

        assert!(registry.has_active_transitions());
    }

    #[test]
    fn test_cleanup_removes_inactive() {
        let mut registry = TransitionRegistry::new();
        let id = TransitionId(99);
        // Use zero-duration so the tween completes on the first update() call.
        let config_instant = TransitionConfig::new(0, Easing::Linear);
        let color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

        // Advance once: tween created with 0ms duration, completes immediately.
        registry.get_or_create(id).advance_bg(color, &config_instant);

        // Not active — tween should be Completed (0ms elapsed >= 0ms duration).
        assert!(!registry.has_active_transitions(), "0ms tween must complete immediately");

        // cleanup_completed should remove the entry (no active tweens).
        registry.cleanup_completed();
        // After cleanup, still not active.
        assert!(!registry.has_active_transitions());
    }
}
