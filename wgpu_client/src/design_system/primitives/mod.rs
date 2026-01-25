//! GPU-rendered primitives for UI components.
//!
//! These are the building blocks for all UI rendering - styled rectangles
//! with borders, shadows, and rounded corners, plus text blocks with alignment.

mod styled_rect;
mod styled_rect_renderer;
mod text_block;

pub use styled_rect::{StyledRect, StyledRectInstance};
pub use styled_rect_renderer::StyledRectRenderer;
pub use text_block::{TextBlock, TextAlign, TextOverflow};
