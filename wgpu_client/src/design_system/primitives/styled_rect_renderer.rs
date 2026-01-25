//! GPU renderer for styled rectangles.
//!
//! This renderer uses the styled_rect.wgsl shader to render rounded rectangles
//! with borders, shadows, and per-corner radii using signed distance functions.

use wgpu::util::DeviceExt;

use crate::design_system::primitives::{StyledRect, StyledRectInstance};

/// Maximum number of styled rectangles that can be rendered in a single batch.
const MAX_INSTANCES: usize = 2048;

/// GPU renderer for styled rectangles.
pub struct StyledRectRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    instances: Vec<StyledRectInstance>,
}

impl StyledRectRenderer {
    /// Create a new styled rectangle renderer.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Styled Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../components/shaders/styled_rect.wgsl").into()),
        });

        // Create pipeline layout (no bind groups needed - all data in vertex buffer)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Styled Rect Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Styled Rect Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[StyledRectInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for 2D
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create instance buffer
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Styled Rect Instance Buffer"),
            size: (MAX_INSTANCES * std::mem::size_of::<StyledRectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            instance_buffer,
            instances: Vec::with_capacity(MAX_INSTANCES),
        }
    }

    /// Clear all queued rectangles.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Queue a styled rectangle for rendering.
    pub fn push(&mut self, rect: &StyledRect, screen_width: f32, screen_height: f32) {
        if self.instances.len() < MAX_INSTANCES {
            self.instances.push(rect.to_instance(screen_width, screen_height));
        }
    }

    /// Queue multiple styled rectangles for rendering.
    pub fn push_all(&mut self, rects: &[StyledRect], screen_width: f32, screen_height: f32) {
        for rect in rects {
            self.push(rect, screen_width, screen_height);
        }
    }

    /// Upload queued instances to the GPU and prepare for rendering.
    pub fn prepare(&self, queue: &wgpu::Queue) {
        if !self.instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }
    }

    /// Render all queued rectangles.
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

        // Each instance draws 6 vertices (2 triangles for a quad)
        let vertex_count = 6;
        let instance_count = self.instances.len() as u32;
        render_pass.draw(0..vertex_count, 0..instance_count);
    }

    /// Get the number of queued rectangles.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
