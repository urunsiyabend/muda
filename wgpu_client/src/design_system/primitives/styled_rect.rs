//! Styled rectangle primitive with borders, shadows, and rounded corners.
//!
//! This is the foundational rendering primitive for UI components. It uses
//! SDF (Signed Distance Function) based rendering for smooth anti-aliased
//! edges and per-corner radius control.

use crate::theme::Color;
use crate::components::Bounds;
use crate::design_system::tokens::{CornerRadii, Shadow, Elevation};

/// A styled rectangle that can be rendered with borders, shadows, and rounded corners.
#[derive(Clone, Debug)]
pub struct StyledRect {
    /// Rectangle bounds (position and size)
    pub bounds: Bounds,
    /// Fill color (None for transparent)
    pub fill: Option<Color>,
    /// Border width in pixels
    pub border_width: f32,
    /// Border color (None for no border)
    pub border_color: Option<Color>,
    /// Per-corner border radii
    pub corner_radii: CornerRadii,
    /// Drop shadow (None for no shadow)
    pub shadow: Option<Shadow>,
}

impl StyledRect {
    /// Create a new styled rectangle with just bounds.
    pub fn new(bounds: Bounds) -> Self {
        Self {
            bounds,
            fill: None,
            border_width: 0.0,
            border_color: None,
            corner_radii: CornerRadii::ZERO,
            shadow: None,
        }
    }

    /// Set the fill color.
    pub fn with_fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    /// Set the fill color from an Option.
    pub fn with_fill_opt(mut self, color: Option<Color>) -> Self {
        self.fill = color;
        self
    }

    /// Set the border.
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = Some(color);
        self
    }

    /// Set uniform corner radius.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.corner_radii = CornerRadii::all(radius);
        self
    }

    /// Set per-corner radii.
    pub fn with_corner_radii(mut self, radii: CornerRadii) -> Self {
        self.corner_radii = radii;
        self
    }

    /// Set the shadow.
    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Set elevation (which provides a preset shadow).
    pub fn with_elevation(mut self, elevation: Elevation) -> Self {
        self.shadow = elevation.shadow();
        self
    }

    /// Convert to an instance for GPU rendering.
    pub fn to_instance(&self, screen_width: f32, screen_height: f32) -> StyledRectInstance {
        // Clamp radii to fit within bounds
        let clamped_radii = self.corner_radii.clamped(self.bounds.width, self.bounds.height);

        StyledRectInstance {
            // Position and size
            pos: [self.bounds.x, self.bounds.y],
            size: [self.bounds.width, self.bounds.height],

            // Fill color
            fill_color: self.fill.map(|c| c.to_array()).unwrap_or([0.0, 0.0, 0.0, 0.0]),

            // Border
            border_color: self.border_color.map(|c| c.to_array()).unwrap_or([0.0, 0.0, 0.0, 0.0]),
            border_width: self.border_width,

            // Corner radii [tl, tr, br, bl]
            corner_radii: clamped_radii.to_array(),

            // Shadow
            shadow_color: self.shadow.map(|s| s.color.to_array()).unwrap_or([0.0, 0.0, 0.0, 0.0]),
            shadow_offset: self.shadow.map(|s| [s.offset_x, s.offset_y]).unwrap_or([0.0, 0.0]),
            shadow_blur: self.shadow.map(|s| s.blur).unwrap_or(0.0),
            shadow_spread: self.shadow.map(|s| s.spread).unwrap_or(0.0),

            // Screen dimensions for coordinate conversion
            screen_size: [screen_width, screen_height],
        }
    }

    /// Check if a point is inside the rounded rectangle.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        // First check bounding box
        if x < self.bounds.x || x > self.bounds.x + self.bounds.width ||
           y < self.bounds.y || y > self.bounds.y + self.bounds.height {
            return false;
        }

        // For rectangles without rounded corners, bounding box is sufficient
        if self.corner_radii == CornerRadii::ZERO {
            return true;
        }

        // Check rounded corners using SDF
        let local_x = x - self.bounds.x;
        let local_y = y - self.bounds.y;

        self.sdf_contains(local_x, local_y)
    }

    /// SDF-based point containment check for rounded rectangles.
    fn sdf_contains(&self, x: f32, y: f32) -> bool {
        let w = self.bounds.width;
        let h = self.bounds.height;
        let radii = self.corner_radii.clamped(w, h);

        // Determine which corner we're near
        let radius = if x < w / 2.0 {
            if y < h / 2.0 {
                radii.top_left
            } else {
                radii.bottom_left
            }
        } else {
            if y < h / 2.0 {
                radii.top_right
            } else {
                radii.bottom_right
            }
        };

        // If no radius, point is inside (already passed bounding box check)
        if radius <= 0.0 {
            return true;
        }

        // Check corner regions
        let corner_x = if x < radius { radius - x } else if x > w - radius { x - (w - radius) } else { 0.0 };
        let corner_y = if y < radius { radius - y } else if y > h - radius { y - (h - radius) } else { 0.0 };

        // If not in a corner region, point is inside
        if corner_x <= 0.0 || corner_y <= 0.0 {
            return true;
        }

        // Check if point is within the corner circle
        corner_x * corner_x + corner_y * corner_y <= radius * radius
    }
}

impl Default for StyledRect {
    fn default() -> Self {
        Self::new(Bounds::default())
    }
}

/// GPU instance data for a styled rectangle.
///
/// This struct is designed to be efficiently packed for GPU upload.
/// All coordinates are in logical pixels (not NDC).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StyledRectInstance {
    /// Position (x, y) in logical pixels
    pub pos: [f32; 2],
    /// Size (width, height) in logical pixels
    pub size: [f32; 2],
    /// Fill color (RGBA)
    pub fill_color: [f32; 4],
    /// Border color (RGBA)
    pub border_color: [f32; 4],
    /// Border width in pixels
    pub border_width: f32,
    /// Corner radii [top_left, top_right, bottom_right, bottom_left]
    pub corner_radii: [f32; 4],
    /// Shadow color (RGBA)
    pub shadow_color: [f32; 4],
    /// Shadow offset (x, y)
    pub shadow_offset: [f32; 2],
    /// Shadow blur radius
    pub shadow_blur: f32,
    /// Shadow spread radius
    pub shadow_spread: f32,
    /// Screen size for coordinate conversion
    pub screen_size: [f32; 2],
}

impl StyledRectInstance {
    /// Get the vertex buffer layout for this instance.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<StyledRectInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // pos
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // size
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // fill_color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // border_width
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                // corner_radii
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 13]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // shadow_color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 17]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // shadow_offset
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 21]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // shadow_blur
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 23]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32,
                },
                // shadow_spread
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32,
                },
                // screen_size
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 25]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
