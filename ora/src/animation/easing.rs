//! Easing functions and curves for animations.
//!
//! Provides the `Easing` enum (cubic math), `CubicBezier` (CSS-compatible
//! Newton-Raphson solver), and `Spring` (damped spring physics).

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

/// Bezier curve easing for custom CSS-compatible curves.
///
/// Control points are (x1, y1) and (x2, y2). The curve goes from (0,0) to (1,1).
/// Uses Newton-Raphson iteration to solve for the bezier parameter.
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    /// Create a new cubic bezier curve.
    ///
    /// x1 and x2 are clamped to [0, 1]; y1 and y2 may exceed that range
    /// for bounce/overshoot effects.
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x1: x1.clamp(0.0, 1.0),
            y1,
            x2: x2.clamp(0.0, 1.0),
            y2,
        }
    }

    /// Standard CSS `ease` — cubic-bezier(0.25, 0.1, 0.25, 1.0).
    pub fn ease() -> Self {
        Self::new(0.25, 0.1, 0.25, 1.0)
    }

    /// Standard CSS `ease-in` — cubic-bezier(0.42, 0.0, 1.0, 1.0).
    pub fn ease_in() -> Self {
        Self::new(0.42, 0.0, 1.0, 1.0)
    }

    /// Standard CSS `ease-out` — cubic-bezier(0.0, 0.0, 0.58, 1.0).
    pub fn ease_out() -> Self {
        Self::new(0.0, 0.0, 0.58, 1.0)
    }

    /// Standard CSS `ease-in-out` — cubic-bezier(0.42, 0.0, 0.58, 1.0).
    pub fn ease_in_out() -> Self {
        Self::new(0.42, 0.0, 0.58, 1.0)
    }

    /// Apply the bezier curve to a time value (0.0 to 1.0).
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        // Newton-Raphson: find the bezier parameter whose x equals t,
        // then evaluate y at that parameter.
        let x = self.solve_x(t);
        self.bezier_y(x)
    }

    /// Solve for the bezier parameter that gives x value `t`.
    fn solve_x(&self, t: f32) -> f32 {
        let mut x = t;
        for _ in 0..8 {
            let error = self.bezier_x(x) - t;
            if error.abs() < 0.001 {
                break;
            }
            let dx = self.bezier_x_derivative(x);
            if dx.abs() < 0.0001 {
                break;
            }
            x -= error / dx;
            x = x.clamp(0.0, 1.0);
        }
        x
    }

    /// Evaluate the x component of the bezier curve at parameter `t`.
    fn bezier_x(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        // B(t) = 3(1-t)²t·P1 + 3(1-t)t²·P2 + t³
        3.0 * mt2 * t * self.x1 + 3.0 * mt * t2 * self.x2 + t3
    }

    /// Evaluate the y component of the bezier curve at parameter `t`.
    fn bezier_y(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        3.0 * mt2 * t * self.y1 + 3.0 * mt * t2 * self.y2 + t3
    }

    /// Derivative of the x component with respect to the bezier parameter.
    fn bezier_x_derivative(&self, t: f32) -> f32 {
        let t2 = t * t;
        let mt = 1.0 - t;
        3.0 * mt * mt * self.x1 + 6.0 * mt * t * (self.x2 - self.x1) + 3.0 * t2 * (1.0 - self.x2)
    }
}

/// Spring physics for bouncy animations.
///
/// Computes a damped spring response. Values start at 0, may overshoot past 1,
/// and settle at 1. Use `apply(t)` where `t` is elapsed time in seconds.
pub struct Spring {
    /// Stiffness (higher = faster oscillation)
    pub stiffness: f32,
    /// Damping (higher = less bounce)
    pub damping: f32,
    /// Mass (higher = slower)
    pub mass: f32,
}

impl Spring {
    /// Create a new spring configuration.
    pub fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self { stiffness, damping, mass }
    }

    /// A gentle spring (slow, subtle bounce).
    pub fn gentle() -> Self {
        Self::new(100.0, 15.0, 1.0)
    }

    /// A wobbly spring (more bounce).
    pub fn wobbly() -> Self {
        Self::new(180.0, 12.0, 1.0)
    }

    /// A stiff spring (fast, minimal bounce).
    pub fn stiff() -> Self {
        Self::new(400.0, 30.0, 1.0)
    }

    /// Calculate the spring value at time `t` (seconds).
    ///
    /// Returns a value that starts at 0, overshoots past 1, and settles at 1.
    pub fn apply(&self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }

        let omega = (self.stiffness / self.mass).sqrt();
        let zeta = self.damping / (2.0 * (self.stiffness * self.mass).sqrt());

        if zeta < 1.0 {
            // Underdamped (oscillates)
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega * t).exp();
            1.0 - decay * ((zeta * omega * t).cos() + (zeta * omega / omega_d) * (omega_d * t).sin())
        } else if (zeta - 1.0).abs() < f32::EPSILON {
            // Critically damped
            1.0 - (1.0 + omega * t) * (-omega * t).exp()
        } else {
            // Overdamped
            let s1 = -omega * (zeta - (zeta * zeta - 1.0).sqrt());
            let s2 = -omega * (zeta + (zeta * zeta - 1.0).sqrt());
            let c2 = -s1 / (s2 - s1);
            let c1 = 1.0 - c2;
            1.0 - c1 * (s1 * t).exp() - c2 * (s2 * t).exp()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn test_easing_ease_out_boundaries() {
        assert!((Easing::EaseOut.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseOut.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_ease_in_boundaries() {
        assert!((Easing::EaseIn.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseIn.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_ease_in_out_boundaries() {
        assert!((Easing::EaseInOut.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::EaseInOut.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_spring_boundaries() {
        assert!((Easing::Spring.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::Spring.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_default_is_ease_out() {
        assert_eq!(Easing::default(), Easing::EaseOut);
    }

    #[test]
    fn test_cubic_bezier_ease() {
        let bezier = CubicBezier::ease();
        assert!((bezier.apply(0.0) - 0.0).abs() < 0.01);
        assert!((bezier.apply(1.0) - 1.0).abs() < 0.01);
        let mid = bezier.apply(0.5);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_cubic_bezier_ease_out_midpoint() {
        let bezier = CubicBezier::ease_out();
        // ease-out is faster at the start so midpoint > 0.5
        let mid = bezier.apply(0.5);
        assert!(mid > 0.5, "ease-out midpoint should be > 0.5, got {}", mid);
    }

    #[test]
    fn test_spring_settles_at_one() {
        let spring = Spring::gentle();
        // After enough time, spring should settle near 1.0
        let value = spring.apply(2.0);
        assert!((value - 1.0).abs() < 0.1, "spring should settle near 1.0, got {}", value);
    }

    #[test]
    fn test_spring_starts_at_zero() {
        let spring = Spring::stiff();
        assert_eq!(spring.apply(0.0), 0.0);
    }
}
