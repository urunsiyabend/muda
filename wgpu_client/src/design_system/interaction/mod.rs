//! Interaction state management for UI components.
//!
//! This module provides a state machine for tracking component interaction states
//! (hover, pressed, focused, etc.) and managing state transitions.

mod state;
mod focus;

pub use state::{InteractionState, InteractionTracker};
pub use focus::{FocusManager, FocusDirection, Focusable};
