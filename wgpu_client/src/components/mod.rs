//! GPU-rendered UI components.
//!
//! This module provides a composable component architecture for rendering
//! the editor UI. Each component is responsible for:
//!
//! 1. Preparing GPU resources (buffers, textures)
//! 2. Updating state from the RenderModel
//! 3. Drawing to the render pass
//!
//! # Component Hierarchy
//!
//! ```text
//! EditorCanvas
//! ├── Gutter          (line numbers)
//! ├── TextArea
//! │   ├── Selection   (background rectangles)
//! │   ├── TextLines   (glyphon text rendering)
//! │   └── Caret       (blinking cursor)
//! ├── StatusBar       (bottom info bar)
//! └── Sidebar         (file explorer)
//! ```

mod rect;
mod text_area;
mod caret;
mod gutter;
mod status_bar;
mod sidebar;
mod tab_bar;
mod dialog;
mod ui_text_renderer;

pub use rect::{Rect, RectRenderer};
pub use text_area::TextArea;
pub use caret::Caret;
pub use gutter::Gutter;
pub use status_bar::StatusBar;
pub use sidebar::SidebarComponent;
pub use tab_bar::TabBar;
pub use dialog::Dialog;
pub use ui_text_renderer::UITextRenderer;

use crate::theme::Theme;

/// Calculates a scissor rectangle that is guaranteed to be within screen bounds.
/// Converts from logical pixels to physical pixels and clamps to screen dimensions.
///
/// # Arguments
/// * `bounds` - Component bounds in logical pixels
/// * `scale_factor` - Display scale factor (e.g., 1.0, 1.5, 2.0)
/// * `screen_width` - Physical screen width in pixels
/// * `screen_height` - Physical screen height in pixels
///
/// # Returns
/// Tuple of (x, y, width, height) in physical pixels, clamped to screen bounds.
pub fn safe_scissor_rect(
    bounds: Bounds,
    scale_factor: f32,
    screen_width: u32,
    screen_height: u32,
) -> (u32, u32, u32, u32) {
    bounds.to_scissor_rect(scale_factor, screen_width, screen_height)
}

/// Rendering context passed to all components.
pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub target_format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub theme: &'a Theme,
}

impl<'a> RenderContext<'a> {
    /// Physical width in pixels.
    pub fn physical_width(&self) -> f32 {
        self.width as f32
    }

    /// Physical height in pixels.
    pub fn physical_height(&self) -> f32 {
        self.height as f32
    }

    /// Logical width (accounting for scale factor).
    pub fn logical_width(&self) -> f32 {
        self.width as f32 / self.scale_factor
    }

    /// Logical height (accounting for scale factor).
    pub fn logical_height(&self) -> f32 {
        self.height as f32 / self.scale_factor
    }
}

/// Viewport bounds for component layout.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Calculates a scissor rectangle that is guaranteed to be within screen bounds.
    /// Converts from logical pixels to physical pixels and clamps to screen dimensions.
    ///
    /// # Arguments
    /// * `scale_factor` - Display scale factor (e.g., 1.0, 1.5, 2.0)
    /// * `screen_width` - Physical screen width in pixels
    /// * `screen_height` - Physical screen height in pixels
    ///
    /// # Returns
    /// Tuple of (x, y, width, height) in physical pixels, clamped to screen bounds.
    pub fn to_scissor_rect(
        &self,
        scale_factor: f32,
        screen_width: u32,
        screen_height: u32,
    ) -> (u32, u32, u32, u32) {
        // Convert to physical pixels
        let x = (self.x * scale_factor).max(0.0) as u32;
        let y = (self.y * scale_factor).max(0.0) as u32;

        // Calculate right/bottom edges, clamped to screen bounds
        let right = ((self.x + self.width) * scale_factor)
            .min(screen_width as f32)
            .max(0.0) as u32;
        let bottom = ((self.y + self.height) * scale_factor)
            .min(screen_height as f32)
            .max(0.0) as u32;

        // Calculate width/height (saturating to avoid underflow)
        let width = right.saturating_sub(x);
        let height = bottom.saturating_sub(y);

        (x, y, width, height)
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Shrink bounds by padding.
    pub fn inset(&self, padding: f32) -> Self {
        Self {
            x: self.x + padding,
            y: self.y + padding,
            width: (self.width - padding * 2.0).max(0.0),
            height: (self.height - padding * 2.0).max(0.0),
        }
    }

    /// Split horizontally, returning (left, right).
    pub fn split_horizontal(&self, left_width: f32) -> (Self, Self) {
        let left = Self {
            x: self.x,
            y: self.y,
            width: left_width.min(self.width),
            height: self.height,
        };
        let right = Self {
            x: self.x + left.width,
            y: self.y,
            width: (self.width - left.width).max(0.0),
            height: self.height,
        };
        (left, right)
    }

    /// Split vertically, returning (top, bottom).
    pub fn split_vertical(&self, top_height: f32) -> (Self, Self) {
        let top = Self {
            x: self.x,
            y: self.y,
            width: self.width,
            height: top_height.min(self.height),
        };
        let bottom = Self {
            x: self.x,
            y: self.y + top.height,
            width: self.width,
            height: (self.height - top.height).max(0.0),
        };
        (top, bottom)
    }
}
