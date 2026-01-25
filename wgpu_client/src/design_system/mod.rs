//! Design System for MUDA IDE UI Components.
//!
//! This module provides a unified design token system and primitives for building
//! production-ready, IDE-grade UI components with GPU rendering.
//!
//! # Architecture
//!
//! ```text
//! design_system/
//! ├── tokens/          # Design tokens (colors, spacing, sizing, etc.)
//! ├── primitives/      # GPU-rendered primitives (styled rects, text blocks)
//! ├── interaction/     # Interaction state management
//! ├── layout/          # Flexbox-like layout system
//! └── animation/       # Animation and motion primitives
//! ```

pub mod tokens;
pub mod primitives;
pub mod interaction;
pub mod layout;
pub mod animation;

// Re-export commonly used types
pub use tokens::{
    ColorRole, ColorPalette,
    Space, Spacing,
    ComponentSize,
    Radius, CornerRadii,
    Elevation, Shadow,
    Duration, Easing,
};
pub use primitives::{StyledRect, StyledRectRenderer, TextBlock, TextAlign, TextOverflow};
pub use interaction::{InteractionState, InteractionTracker};
pub use layout::{FlexLayout, FlexDirection, AlignItems, JustifyContent, LayoutConstraints};
pub use animation::{Tween, TweenState};
