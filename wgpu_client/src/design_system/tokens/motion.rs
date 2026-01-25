//! Motion and animation tokens.
//!
//! Consistent motion creates a polished, responsive feel. These tokens
//! define standard durations and easing functions for animations.

/// Animation duration scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Duration {
    /// 0ms - Instant (no animation)
    Instant,
    /// 100ms - Fast (micro-interactions, hover states)
    #[default]
    Fast,
    /// 200ms - Normal (standard transitions)
    Normal,
    /// 300ms - Slow (larger movements, emphasis)
    Slow,
    /// 500ms - Slower (complex animations)
    Slower,
}

impl Duration {
    /// Convert to milliseconds.
    pub const fn ms(self) -> u32 {
        match self {
            Duration::Instant => 0,
            Duration::Fast => 100,
            Duration::Normal => 200,
            Duration::Slow => 300,
            Duration::Slower => 500,
        }
    }

    /// Convert to seconds.
    pub const fn secs(self) -> f32 {
        self.ms() as f32 / 1000.0
    }
}

impl From<Duration> for std::time::Duration {
    fn from(d: Duration) -> std::time::Duration {
        std::time::Duration::from_millis(d.ms() as u64)
    }
}

/// Easing functions for smooth animations.
///
/// These follow standard CSS easing conventions for familiarity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Easing {
    /// Linear - constant speed
    Linear,
    /// Ease-in - starts slow, ends fast
    EaseIn,
    /// Ease-out - starts fast, ends slow (most common)
    #[default]
    EaseOut,
    /// Ease-in-out - slow start and end
    EaseInOut,
    /// Spring - overshoots then settles (for emphasis)
    Spring,
}

impl Easing {
    /// Apply the easing function to a normalized time value (0.0 to 1.0).
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => ease_in_cubic(t),
            Easing::EaseOut => ease_out_cubic(t),
            Easing::EaseInOut => ease_in_out_cubic(t),
            Easing::Spring => spring_ease(t),
        }
    }
}

// Cubic easing functions (smoother than quadratic)

fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t - 1.0;
    t * t * t + 1.0
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t = -2.0 * t + 2.0;
        1.0 - t * t * t / 2.0
    }
}

fn spring_ease(t: f32) -> f32 {
    // Damped spring simulation approximation
    // Overshoots slightly then settles
    const TENSION: f32 = 0.5;
    const FRICTION: f32 = 0.7;

    if t >= 1.0 {
        return 1.0;
    }

    let p = 1.0 - t;
    let spring = 1.0 - p * p * (1.0 + TENSION * (1.0 - FRICTION * (1.0 - p)));
    spring.clamp(0.0, 1.1) // Allow slight overshoot
}

/// Motion preferences for accessibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPreference {
    /// Full motion enabled
    #[default]
    Full,
    /// Reduced motion (for users who prefer less animation)
    Reduced,
    /// No motion (instant transitions only)
    None,
}

impl MotionPreference {
    /// Adjust a duration based on motion preference.
    pub fn adjust_duration(self, duration: Duration) -> Duration {
        match self {
            MotionPreference::Full => duration,
            MotionPreference::Reduced => {
                // Reduce durations but keep some motion
                match duration {
                    Duration::Instant => Duration::Instant,
                    Duration::Fast => Duration::Instant,
                    Duration::Normal => Duration::Fast,
                    Duration::Slow => Duration::Fast,
                    Duration::Slower => Duration::Normal,
                }
            }
            MotionPreference::None => Duration::Instant,
        }
    }
}
