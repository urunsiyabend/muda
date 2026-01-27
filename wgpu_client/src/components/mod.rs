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
