//! Layout system for UI components.
//!
//! Provides a flexbox-like layout system for arranging UI components.

mod flex;

pub use flex::{
    FlexLayout, FlexDirection, AlignItems, JustifyContent,
    LayoutConstraints, LayoutResult,
};
