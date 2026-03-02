use crate::style::{Background, Gradient, Rect, Style};

/// GPU instance data for a single rectangle.
/// CRITICAL: This layout must match the WGSL RectInstance struct byte-for-byte.
///
/// WGSL alignment rules differ from Rust's #[repr(C)]:
/// - vec2<f32> requires 8-byte alignment in WGSL, but [f32; 2] only needs 4 in Rust
/// - vec4<f32> requires 16-byte alignment in WGSL
/// - Struct stride (array element size) rounds up to max member alignment (16)
///
/// Explicit _pad fields ensure Rust layout matches WGSL layout exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    pub position: [f32; 2],           // offset 0   (WGSL vec2, align 8)
    pub size: [f32; 2],               // offset 8   (WGSL vec2, align 8)
    pub color: [f32; 4],              // offset 16  (WGSL vec4, align 16)
    pub border_color: [f32; 4],       // offset 32  (WGSL vec4, align 16)
    pub border_widths: [f32; 4],      // offset 48  (WGSL vec4, align 16)
    pub corners: [f32; 4],            // offset 64  (WGSL vec4, align 16)
    pub shadow_offset: [f32; 2],      // offset 80  (WGSL vec2, align 8)
    pub shadow_blur: f32,             // offset 88  (WGSL f32, align 4)
    pub shadow_spread: f32,           // offset 92  (WGSL f32, align 4)
    pub shadow_color: [f32; 4],       // offset 96  (WGSL vec4, align 16)
    pub gradient_end_color: [f32; 4], // offset 112 (WGSL vec4, align 16)
    pub gradient_angle: f32,          // offset 128 (WGSL f32, align 4)
    pub _pad1: f32,                   // offset 132 (explicit pad for vec2 alignment)
    pub window_size: [f32; 2],        // offset 136 (WGSL vec2, align 8)
    pub _pad2: [f32; 4],              // offset 144 (pad to WGSL struct stride of 160)
}

// Compile-time assertions for GPU struct compatibility
const _: () = assert!(
    std::mem::size_of::<RectInstance>() == 160,
    "RectInstance must be exactly 160 bytes to match WGSL struct stride"
);
const _: () = assert!(
    std::mem::size_of::<RectInstance>() % 16 == 0,
    "RectInstance must be 16-byte aligned for GPU"
);

impl RectInstance {
    /// Create a RectInstance from legacy PaintCommand::Rect data (backward compat)
    pub fn from_legacy(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        window_size: (u32, u32),
    ) -> Self {
        Self {
            position: [x, y],
            size: [width, height],
            color,
            border_color: [0.0, 0.0, 0.0, 0.0],
            border_widths: [0.0, 0.0, 0.0, 0.0],
            corners: [0.0, 0.0, 0.0, 0.0],
            shadow_offset: [0.0, 0.0],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_color: [0.0, 0.0, 0.0, 0.0],
            gradient_end_color: [0.0, 0.0, 0.0, 0.0],
            gradient_angle: 0.0,
            _pad1: 0.0,
            window_size: [window_size.0 as f32, window_size.1 as f32],
            _pad2: [0.0; 4],
        }
    }

    /// Create a RectInstance from a Style and bounds
    pub fn from_style(style: &Style, bounds: &Rect, window_size: (u32, u32)) -> Self {
        // Extract background color
        // For solid/none backgrounds, set gradient_end_color == color to prevent
        // false gradient detection in the shader (length check would be > 0 otherwise)
        let (color, gradient_end_color, gradient_angle) = match style.background {
            Background::None => ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], 0.0),
            Background::Solid(c) => {
                let arr = c.to_array();
                (arr, arr, 0.0) // Same color prevents gradient detection
            }
            Background::Linear(Gradient {
                start,
                end,
                angle_radians,
            }) => (start.to_array(), end.to_array(), angle_radians),
        };

        // Extract border properties
        let border_color = style.border.color.to_array();
        let border_widths = [
            style.border.widths.top,
            style.border.widths.right,
            style.border.widths.bottom,
            style.border.widths.left,
        ];

        // Extract corner radii
        let corners = [
            style.border_radius.top_left,
            style.border_radius.top_right,
            style.border_radius.bottom_right,
            style.border_radius.bottom_left,
        ];

        // Extract shadow properties
        let (shadow_offset, shadow_blur, shadow_spread, shadow_color) =
            if let Some(shadow) = style.box_shadow {
                (
                    [shadow.offset_x, shadow.offset_y],
                    shadow.blur,
                    shadow.spread,
                    shadow.color.to_array(),
                )
            } else {
                ([0.0, 0.0], 0.0, 0.0, [0.0, 0.0, 0.0, 0.0])
            };

        Self {
            position: [bounds.origin.x, bounds.origin.y],
            size: [bounds.size.width, bounds.size.height],
            color,
            border_color,
            border_widths,
            corners,
            shadow_offset,
            shadow_blur,
            shadow_spread,
            shadow_color,
            gradient_end_color,
            gradient_angle,
            _pad1: 0.0,
            window_size: [window_size.0 as f32, window_size.1 as f32],
            _pad2: [0.0; 4],
        }
    }
}

/// GPU rectangle batch renderer using instanced rendering
pub struct RectangleRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    instance_buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    instance_count: u32,
    max_instances: u32,
}

impl RectangleRenderer {
    /// Create a new RectangleRenderer
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // Load shader
        let shader_src = include_str!("shaders/rect.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect-shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        // Create bind group layout for storage buffer
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rect-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers - geometry generated in shader
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for UI rectangles
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create initial instance buffer with capacity for 1024 instances
        let max_instances = 1024;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect-instance-buffer"),
            size: (max_instances as usize * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            instance_buffer,
            bind_group: None,
            instance_count: 0,
            max_instances,
        }
    }

    /// Prepare rectangle instances for rendering
    /// Uploads instance data to GPU buffer, grows buffer if needed
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[RectInstance],
    ) {
        self.instance_count = instances.len() as u32;

        // Grow buffer if needed
        if instances.len() > self.max_instances as usize {
            // Double capacity
            self.max_instances = (instances.len() * 2) as u32;
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rect-instance-buffer"),
                size: (self.max_instances as usize * std::mem::size_of::<RectInstance>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            // Invalidate bind group since buffer changed
            self.bind_group = None;
        }

        // Write instance data to buffer
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        // Create bind group if needed
        if self.bind_group.is_none() {
            self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rect-bind-group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.instance_buffer.as_entire_binding(),
                }],
            }));
        }
    }

    /// Render all prepared rectangles
    /// Draws all instances in a single draw call
    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.instance_count == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        if let Some(ref bind_group) = self.bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }

        // Draw 6 vertices (two triangles forming a quad) for each instance
        render_pass.draw(0..6, 0..self.instance_count);
    }

    /// Render a subset of prepared rectangles by instance range.
    /// Used for per-layer rendering: draw only the instances belonging to a specific layer.
    /// `range` specifies the instance indices (e.g., 0..5 draws instances 0-4).
    pub fn render_range(&self, render_pass: &mut wgpu::RenderPass<'_>, range: std::ops::Range<u32>) {
        if range.is_empty() || self.bind_group.is_none() {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        if let Some(ref bind_group) = self.bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }
        render_pass.draw(0..6, range);
    }

    /// Get the number of prepared instances.
    pub fn instance_count(&self) -> u32 {
        self.instance_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Background, Color, Corners, Edges};

    #[test]
    fn test_rect_instance_alignment() {
        // Verify RectInstance is properly aligned for GPU
        assert_eq!(
            std::mem::size_of::<RectInstance>() % 16,
            0,
            "RectInstance must be 16-byte aligned"
        );

        // Must be exactly 160 bytes to match WGSL struct stride
        let size = std::mem::size_of::<RectInstance>();
        assert_eq!(
            size, 160,
            "RectInstance must be exactly 160 bytes, got {} bytes",
            size
        );
    }

    #[test]
    fn test_rect_instance_from_style() {
        let mut style = Style::default();
        style.background = Background::Solid(Color::rgb(1.0, 0.0, 0.0));
        style.border.widths = Edges::all(2.0);
        style.border.color = Color::rgb(0.0, 0.0, 1.0);
        style.border_radius = Corners::all(8.0);

        let bounds = Rect::new(10.0, 20.0, 100.0, 50.0);
        let instance = RectInstance::from_style(&style, &bounds, (800, 600));

        assert_eq!(instance.position, [10.0, 20.0]);
        assert_eq!(instance.size, [100.0, 50.0]);
        assert_eq!(instance.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(instance.border_color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(instance.border_widths, [2.0, 2.0, 2.0, 2.0]);
        assert_eq!(instance.corners, [8.0, 8.0, 8.0, 8.0]);
        assert_eq!(instance.window_size, [800.0, 600.0]);
    }
}
