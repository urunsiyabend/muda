use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;
use crate::rendering::{GlyphCacheKey, RectangleRenderer, TextSystem};

/// GPU state holding wgpu resources for rendering.
pub struct GpuState {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub size: (u32, u32),
    pub(crate) rect_renderer: RectangleRenderer,
    pub(crate) text_system: TextSystem,
}

impl GpuState {
    /// Create a new GPU state for the given window.
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // Create wgpu instance with PRIMARY backends
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create surface from window
        let surface = instance.create_surface(window.clone()).unwrap();

        // Request adapter with compatible surface
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Request device and queue with default limits (wgpu 25: single-arg, returns tuple)
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ora-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        // Get surface capabilities
        let surface_caps = surface.get_capabilities(&adapter);

        // Pick sRGB surface format, fallback to first available
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        // Configure surface with vsync
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo, // vsync
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create rendering systems
        let rect_renderer = RectangleRenderer::new(&device, surface_format);
        let text_system = TextSystem::new(&device, &queue, surface_format);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size: (width, height),
            rect_renderer,
            text_system,
        }
    }

    /// Resize the surface configuration.
    /// Guards against zero-size by ignoring resize if width or height is 0.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Render a frame using the layered GPU pipeline with scissor clipping.
    /// Processes paint commands and renders rectangles and text.
    ///
    /// Paint commands are split at `LayerBoundary` markers (inserted by Stack).
    /// Within each layer, `SetScissor`/`ResetScissor` commands create "draw groups"
    /// that share a common scissor rect. Each draw group renders its rects then text
    /// with the appropriate scissor state.
    ///
    /// When no LayerBoundary markers are present (common case — no overlays),
    /// exactly one render pass is created, identical in cost to the old single-pass approach.
    pub fn render_frame(
        &mut self,
        commands: &[crate::element::PaintCommand],
    ) -> Result<(), wgpu::SurfaceError> {
        use crate::element::PaintCommand;
        use crate::rendering::RectInstance;

        let (surface_w, surface_h) = self.size;

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // A DrawGroup is a sequence of draw commands sharing the same scissor state.
        // Scissor changes (SetScissor/ResetScissor) create new draw groups.
        struct DrawGroup {
            scissor: Option<(u32, u32, u32, u32)>, // None = full viewport
            rect_instances: Vec<RectInstance>,
            text_data: Vec<(glyphon::Buffer, f32, f32, crate::style::Rect, crate::style::Color, Option<GlyphCacheKey>)>,
        }

        // Step 1: Group commands by LayerBoundary, then by scissor changes within each layer.
        let mut layers: Vec<Vec<DrawGroup>> = vec![vec![DrawGroup {
            scissor: None,
            rect_instances: Vec::new(),
            text_data: Vec::new(),
        }]];

        let mut offset_stack: Vec<(f32, f32)> = Vec::new();

        for command in commands {
            match command {
                PaintCommand::LayerBoundary => {
                    layers.push(vec![DrawGroup {
                        scissor: None,
                        rect_instances: Vec::new(),
                        text_data: Vec::new(),
                    }]);
                }
                PaintCommand::SetScissor { x, y, width, height } => {
                    // Start a new draw group with the given scissor rect.
                    let current_layer = layers.last_mut().unwrap();
                    current_layer.push(DrawGroup {
                        scissor: Some((*x, *y, *width, *height)),
                        rect_instances: Vec::new(),
                        text_data: Vec::new(),
                    });
                }
                PaintCommand::ResetScissor => {
                    // Start a new draw group with no scissor (full viewport).
                    let current_layer = layers.last_mut().unwrap();
                    current_layer.push(DrawGroup {
                        scissor: None,
                        rect_instances: Vec::new(),
                        text_data: Vec::new(),
                    });
                }
                PaintCommand::PushOffset { dx, dy } => {
                    let (cur_x, cur_y) = offset_stack.last().copied().unwrap_or((0.0, 0.0));
                    offset_stack.push((cur_x + dx, cur_y + dy));
                }
                PaintCommand::PopOffset => {
                    offset_stack.pop();
                }
                PaintCommand::StyledRect { bounds, style } => {
                    let (ox, oy) = offset_stack.last().copied().unwrap_or((0.0, 0.0));
                    let shifted = crate::style::Rect {
                        origin: crate::style::Point::new(bounds.origin.x + ox, bounds.origin.y + oy),
                        size: bounds.size,
                    };
                    let group = layers.last_mut().unwrap().last_mut().unwrap();
                    group.rect_instances.push(RectInstance::from_style(style, &shifted, self.size));
                }
                PaintCommand::Rect { x, y, width, height, color } => {
                    let (ox, oy) = offset_stack.last().copied().unwrap_or((0.0, 0.0));
                    let group = layers.last_mut().unwrap().last_mut().unwrap();
                    group.rect_instances.push(RectInstance::from_legacy(*x + ox, *y + oy, *width, *height, *color, self.size));
                }
                PaintCommand::Text { buffer, left, top, bounds, color, cache_key } => {
                    let (ox, oy) = offset_stack.last().copied().unwrap_or((0.0, 0.0));
                    let shifted_bounds = crate::style::Rect {
                        origin: crate::style::Point::new(bounds.origin.x + ox, bounds.origin.y + oy),
                        size: bounds.size,
                    };
                    let group = layers.last_mut().unwrap().last_mut().unwrap();
                    group.text_data.push((buffer.clone(), *left + ox, *top + oy, shifted_bounds, *color, *cache_key));
                }
            }
        }

        // Step 2: Flatten all rect instances into one buffer, track per-group ranges.
        // Single prepare() call avoids bind_group invalidation from multiple uploads.
        let mut all_rect_instances: Vec<RectInstance> = Vec::new();
        // Store (layer_idx, group_idx) -> rect range
        let mut group_rect_ranges: Vec<Vec<std::ops::Range<u32>>> = Vec::new();

        for layer_groups in &layers {
            let mut ranges = Vec::new();
            for group in layer_groups {
                let start = all_rect_instances.len() as u32;
                all_rect_instances.extend_from_slice(&group.rect_instances);
                let end = all_rect_instances.len() as u32;
                ranges.push(start..end);
            }
            group_rect_ranges.push(ranges);
        }

        // Step 3: Single prepare for all rect instances.
        self.rect_renderer.prepare(&self.device, &self.queue, &all_rect_instances);

        // Step 4: Count total draw groups for text renderer allocation.
        // Each draw group with text needs its own TextRenderer to allow independent
        // scissor-scoped rendering within a single render pass.
        let total_text_groups: usize = layers.iter()
            .flat_map(|layer| layer.iter())
            .filter(|g| !g.text_data.is_empty())
            .count();

        self.text_system.ensure_layer_renderers(&self.device, total_text_groups);

        // Assign text renderer indices to groups that have text.
        let mut text_renderer_idx = 0usize;
        let mut group_text_indices: Vec<Vec<Option<usize>>> = Vec::new();

        for layer_groups in &layers {
            let mut indices = Vec::new();
            for group in layer_groups {
                if group.text_data.is_empty() {
                    indices.push(None);
                } else {
                    // Add text to this renderer
                    for (buffer, left, top, bounds, color, cache_key) in &group.text_data {
                        self.text_system.add_text_to_layer(
                            text_renderer_idx,
                            buffer.clone(),
                            *left,
                            *top,
                            *bounds,
                            *color,
                            *cache_key,
                        );
                    }
                    indices.push(Some(text_renderer_idx));
                    text_renderer_idx += 1;
                }
            }
            group_text_indices.push(indices);
        }

        // Step 5: Prepare ALL text renderers before rendering any.
        // All text prepares must complete before any renders to stabilize the atlas
        // and prevent bind_group invalidation during the render phase.
        for idx in 0..text_renderer_idx {
            self.text_system
                .prepare_layer(&self.device, &self.queue, idx, surface_w, surface_h)
                .unwrap_or_else(|e| log::warn!("Text prepare error for group {}: {:?}", idx, e));
        }

        // Step 6: Create command encoder.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ora-render-encoder"),
            });

        // Step 7: Render each layer in its own render pass.
        // Within each layer, iterate draw groups: set scissor, draw rects, draw text.
        // First pass: Clear (required — Vulkan UB if Load on uninitialized texture).
        // Subsequent passes: Load (preserves previous layers for correct occlusion).
        for (layer_idx, layer_groups) in layers.iter().enumerate() {
            let load_op = if layer_idx == 0 {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.1,
                    b: 0.12,
                    a: 1.0,
                })
            } else {
                wgpu::LoadOp::Load
            };

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ora-render-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                for (group_idx, group) in layer_groups.iter().enumerate() {
                    // Apply scissor state for this draw group.
                    match group.scissor {
                        Some((x, y, w, h)) => {
                            render_pass.set_scissor_rect(x, y, w, h);
                        }
                        None => {
                            // Full viewport — reset scissor
                            render_pass.set_scissor_rect(0, 0, surface_w, surface_h);
                        }
                    }

                    // Draw rects for this group.
                    let range = group_rect_ranges[layer_idx][group_idx].clone();
                    if !range.is_empty() {
                        self.rect_renderer.render_range(&mut render_pass, range);
                    }

                    // Draw text for this group (if any).
                    if let Some(text_idx) = group_text_indices[layer_idx][group_idx] {
                        self.text_system
                            .render_layer(text_idx, &mut render_pass)
                            .unwrap_or_else(|e| log::warn!("Text render error for group {}: {:?}", text_idx, e));
                    }
                }
            } // render_pass dropped here — encoder borrow released
        }

        // Step 8: Single submit per frame (multiple submits hang on Vulkan).
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Step 9: Buffers are drained by the caller (event_loop) via
        // drain_buffers_for_cache() + return_buffers_to_cache() for LRU reuse.
        // The caller is responsible for atlas trimming (drain_buffers_for_cache calls atlas.trim()).

        Ok(())
    }
}
