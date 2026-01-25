//! Animation primitives for smooth UI transitions.
//!
//! This module provides tweening and interpolation utilities for animating
//! UI properties like colors, positions, and sizes.

mod tween;
mod easing;

pub use tween::{Tween, TweenState};
pub use easing::*;
