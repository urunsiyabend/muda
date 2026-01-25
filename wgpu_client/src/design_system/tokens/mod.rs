//! Design tokens for consistent UI styling.
//!
//! Tokens are the foundational values that define the visual language of the UI.
//! They ensure consistency across all components and make theming straightforward.

mod color;
mod spacing;
mod sizing;
mod radius;
mod elevation;
mod motion;

pub use color::{ColorRole, ColorPalette};
pub use spacing::{Space, Spacing};
pub use sizing::ComponentSize;
pub use radius::{Radius, CornerRadii};
pub use elevation::{Elevation, Shadow};
pub use motion::{Duration, Easing};
