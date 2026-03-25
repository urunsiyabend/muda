//! Animation primitives for the CSS-like transition system.
//!
//! Provides `Tween<T>` for value interpolation, the `Tweenable` trait,
//! `Easing` enum with cubic math, `CubicBezier` with Newton-Raphson solver,
//! and `Spring` with damped spring physics.

pub mod easing;
pub mod tween;
pub mod transition;

pub use easing::{Easing, CubicBezier, Spring};
pub use tween::{Tween, TweenState, Tweenable};
pub use transition::{TransitionId, TransitionSpec, TransitionConfig, TransitionState, TransitionRegistry};
