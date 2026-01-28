//! Rectangle rendering for backgrounds, selections, and highlights.

use bytemuck::{Pod, Zeroable};
use crate::theme::Color;

/// A rectangle to be rendered.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Color) -> Self {
        Self { x, y, width, height, color }
    }
}

/// Vertex for rectangle rendering.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct RectVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl RectVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x4,  // color
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Renders filled rectangles using a simple shader pipeline.
pub struct RectRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    /// Maximum number of rectangles that can be rendered.
    max_rects: usize,
}

impl RectRenderer {
    /// Maximum rectangles per draw call.
    const MAX_RECTS: usize = 4096;

    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pre-allocate buffers for max rectangles
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect_vertex_buffer"),
            size: (Self::MAX_RECTS * 4 * std::mem::size_of::<RectVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect_index_buffer"),
            size: (Self::MAX_RECTS * 6 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            max_rects: Self::MAX_RECTS,
        }
    }

    /// Renders a batch of rectangles.
    ///
    /// Coordinates are in normalized device coordinates (-1 to 1).
    /// Optional scissor rect clips rendering to specified bounds (x, y, width, height in physical pixels).
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        rects: &[Rect],
        screen_width: f32,
        screen_height: f32,
        scissor: Option<(u32, u32, u32, u32)>,
    ) {
        if rects.is_empty() {
            return;
        }

        let rect_count = rects.len().min(self.max_rects);

        // Build vertex and index data
        let mut vertices = Vec::with_capacity(rect_count * 4);
        let mut indices = Vec::with_capacity(rect_count * 6);

        for (i, rect) in rects.iter().take(rect_count).enumerate() {
            // Convert pixel coordinates to NDC
            let x0 = (rect.x / screen_width) * 2.0 - 1.0;
            let y0 = 1.0 - (rect.y / screen_height) * 2.0;
            let x1 = ((rect.x + rect.width) / screen_width) * 2.0 - 1.0;
            let y1 = 1.0 - ((rect.y + rect.height) / screen_height) * 2.0;

            let color = rect.color.to_array();
            let base = (i * 4) as u32;

            // Quad vertices (top-left, top-right, bottom-right, bottom-left)
            vertices.push(RectVertex { position: [x0, y0], color });
            vertices.push(RectVertex { position: [x1, y0], color });
            vertices.push(RectVertex { position: [x1, y1], color });
            vertices.push(RectVertex { position: [x0, y1], color });

            // Two triangles
            indices.extend_from_slice(&[
                base, base + 1, base + 2,
                base, base + 2, base + 3,
            ]);
        }

        // Upload data
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        // Render pass
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rect_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Apply scissor rect if provided
        if let Some((sx, sy, sw, sh)) = scissor {
            pass.set_scissor_rect(sx, sy, sw, sh);
        }

        pass.draw_indexed(0..(rect_count * 6) as u32, 0, 0..1);
    }
}
