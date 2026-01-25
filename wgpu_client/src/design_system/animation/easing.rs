//! Additional easing functions and utilities.
//!
//! This module re-exports the Easing enum from tokens and provides
//! additional easing utilities.

// Re-export the main Easing type
pub use crate::design_system::tokens::Easing;

/// Bezier curve easing for custom curves.
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    /// Create a new cubic bezier curve.
    ///
    /// Control points are (x1, y1) and (x2, y2).
    /// The curve goes from (0, 0) to (1, 1).
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x1: x1.clamp(0.0, 1.0),
            y1,
            x2: x2.clamp(0.0, 1.0),
            y2,
        }
    }

    /// Standard CSS ease.
    pub fn ease() -> Self {
        Self::new(0.25, 0.1, 0.25, 1.0)
    }

    /// Standard CSS ease-in.
    pub fn ease_in() -> Self {
        Self::new(0.42, 0.0, 1.0, 1.0)
    }

    /// Standard CSS ease-out.
    pub fn ease_out() -> Self {
        Self::new(0.0, 0.0, 0.58, 1.0)
    }

    /// Standard CSS ease-in-out.
    pub fn ease_in_out() -> Self {
        Self::new(0.42, 0.0, 0.58, 1.0)
    }

    /// Apply the bezier curve to a time value.
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        // Newton-Raphson to find t_bezier from t
        // First, find the x parameter that gives us the desired t
        let x = self.solve_x(t);

        // Then evaluate y at that parameter
        self.bezier_y(x)
    }

    /// Solve for the bezier parameter that gives x value `t`.
    fn solve_x(&self, t: f32) -> f32 {
        // Newton-Raphson iteration
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

    /// Evaluate the x component of the bezier curve.
    fn bezier_x(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;

        // B(t) = 3(1-t)²t·P1 + 3(1-t)t²·P2 + t³
        3.0 * mt2 * t * self.x1 + 3.0 * mt * t2 * self.x2 + t3
    }

    /// Evaluate the y component of the bezier curve.
    fn bezier_y(&self, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;

        3.0 * mt2 * t * self.y1 + 3.0 * mt * t2 * self.y2 + t3
    }

    /// Derivative of x component.
    fn bezier_x_derivative(&self, t: f32) -> f32 {
        let t2 = t * t;
        let mt = 1.0 - t;

        // d/dt B(t)
        3.0 * mt * mt * self.x1 + 6.0 * mt * t * (self.x2 - self.x1) + 3.0 * t2 * (1.0 - self.x2)
    }
}

/// Spring physics for bouncy animations.
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

    /// Calculate the spring value at time t.
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
        } else if zeta == 1.0 {
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
    fn test_cubic_bezier_ease() {
        let bezier = CubicBezier::ease();
        assert!((bezier.apply(0.0) - 0.0).abs() < 0.01);
        assert!((bezier.apply(1.0) - 1.0).abs() < 0.01);
        // Values should be in valid range
        let mid = bezier.apply(0.5);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_spring_settles_at_one() {
        let spring = Spring::gentle();
        // After "enough" time, spring should settle near 1.0
        let value = spring.apply(2.0);
        assert!((value - 1.0).abs() < 0.1);
    }
}
