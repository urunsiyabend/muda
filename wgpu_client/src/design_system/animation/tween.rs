//! Tween animation for interpolating values over time.

use std::time::{Duration, Instant};
use crate::design_system::tokens::Easing;

/// Current state of a tween animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TweenState {
    /// Animation hasn't started
    Idle,
    /// Animation is playing
    Playing,
    /// Animation has completed
    Completed,
}

/// A tween that interpolates a value from start to end over a duration.
#[derive(Clone, Debug)]
pub struct Tween<T: Tweenable> {
    /// Starting value
    start: T,
    /// Target value
    end: T,
    /// Current interpolated value
    current: T,
    /// Animation duration
    duration: Duration,
    /// Easing function
    easing: Easing,
    /// When the animation started
    start_time: Option<Instant>,
    /// Current state
    state: TweenState,
}

impl<T: Tweenable + Clone> Tween<T> {
    /// Create a new tween.
    pub fn new(start: T, end: T, duration: Duration, easing: Easing) -> Self {
        Self {
            start: start.clone(),
            end,
            current: start,
            duration,
            easing,
            start_time: None,
            state: TweenState::Idle,
        }
    }

    /// Create a tween with default easing (EaseOut).
    pub fn ease_out(start: T, end: T, duration: Duration) -> Self {
        Self::new(start, end, duration, Easing::EaseOut)
    }

    /// Create an instant tween (no animation).
    pub fn instant(value: T) -> Self {
        Self {
            start: value.clone(),
            end: value.clone(),
            current: value,
            duration: Duration::ZERO,
            easing: Easing::Linear,
            start_time: None,
            state: TweenState::Completed,
        }
    }

    /// Start or restart the animation.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.state = TweenState::Playing;
        self.current = self.start.clone();
    }

    /// Start with a new target value, animating from current position.
    pub fn retarget(&mut self, new_end: T) {
        self.start = self.current.clone();
        self.end = new_end;
        self.start_time = Some(Instant::now());
        self.state = TweenState::Playing;
    }

    /// Start with new start and end values.
    pub fn restart(&mut self, start: T, end: T) {
        self.start = start;
        self.end = end;
        self.start_time = Some(Instant::now());
        self.state = TweenState::Playing;
    }

    /// Update the animation. Returns true if the value changed.
    pub fn update(&mut self) -> bool {
        match self.state {
            TweenState::Idle => false,
            TweenState::Completed => false,
            TweenState::Playing => {
                let Some(start_time) = self.start_time else {
                    return false;
                };

                let elapsed = start_time.elapsed();

                if elapsed >= self.duration {
                    self.current = self.end.clone();
                    self.state = TweenState::Completed;
                    true
                } else {
                    let t = elapsed.as_secs_f32() / self.duration.as_secs_f32();
                    let eased_t = self.easing.apply(t);
                    let old = self.current.clone();
                    self.current = self.start.lerp(&self.end, eased_t);
                    self.current != old
                }
            }
        }
    }

    /// Get the current value.
    pub fn value(&self) -> &T {
        &self.current
    }

    /// Get the current value (owned).
    pub fn value_owned(&self) -> T {
        self.current.clone()
    }

    /// Get the target value.
    pub fn target(&self) -> &T {
        &self.end
    }

    /// Get the current state.
    pub fn state(&self) -> TweenState {
        self.state
    }

    /// Check if the animation is currently playing.
    pub fn is_playing(&self) -> bool {
        self.state == TweenState::Playing
    }

    /// Check if the animation is complete.
    pub fn is_complete(&self) -> bool {
        self.state == TweenState::Completed
    }

    /// Skip to the end immediately.
    pub fn finish(&mut self) {
        self.current = self.end.clone();
        self.state = TweenState::Completed;
    }

    /// Reset to the start.
    pub fn reset(&mut self) {
        self.current = self.start.clone();
        self.start_time = None;
        self.state = TweenState::Idle;
    }

    /// Get the progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        match self.state {
            TweenState::Idle => 0.0,
            TweenState::Completed => 1.0,
            TweenState::Playing => {
                let Some(start_time) = self.start_time else {
                    return 0.0;
                };
                let elapsed = start_time.elapsed().as_secs_f32();
                let duration = self.duration.as_secs_f32();
                if duration == 0.0 {
                    1.0
                } else {
                    (elapsed / duration).min(1.0)
                }
            }
        }
    }
}

/// Trait for values that can be tweened (interpolated).
pub trait Tweenable: Clone + PartialEq {
    /// Linearly interpolate between self and other.
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

// Implement Tweenable for common types

impl Tweenable for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Tweenable for f64 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t as f64
    }
}

impl Tweenable for i32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other - *self) as f32 * t).round() as i32
    }
}

impl Tweenable for u8 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t).round() as u8
    }
}

impl Tweenable for (f32, f32) {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (
            self.0.lerp(&other.0, t),
            self.1.lerp(&other.1, t),
        )
    }
}

impl Tweenable for [f32; 2] {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        [
            self[0].lerp(&other[0], t),
            self[1].lerp(&other[1], t),
        ]
    }
}

impl Tweenable for [f32; 4] {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        [
            self[0].lerp(&other[0], t),
            self[1].lerp(&other[1], t),
            self[2].lerp(&other[2], t),
            self[3].lerp(&other[3], t),
        ]
    }
}

// Implement for Color
use crate::theme::Color;

impl Tweenable for Color {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Color {
            r: self.r.lerp(&other.r, t),
            g: self.g.lerp(&other.g, t),
            b: self.b.lerp(&other.b, t),
            a: self.a.lerp(&other.a, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_tween() {
        let tween: Tween<f32> = Tween::instant(100.0);
        assert_eq!(*tween.value(), 100.0);
        assert!(tween.is_complete());
    }

    #[test]
    fn test_f32_lerp() {
        let a = 0.0_f32;
        assert_eq!(a.lerp(&100.0, 0.0), 0.0);
        assert_eq!(a.lerp(&100.0, 0.5), 50.0);
        assert_eq!(a.lerp(&100.0, 1.0), 100.0);
    }

    #[test]
    fn test_color_lerp() {
        let a = Color::BLACK;
        let b = Color::WHITE;
        let mid = a.lerp(b, 0.5);
        assert!((mid.r - 0.5).abs() < 0.001);
        assert!((mid.g - 0.5).abs() < 0.001);
        assert!((mid.b - 0.5).abs() < 0.001);
    }
}
