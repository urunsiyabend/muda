use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;
use crate::rendering::{RectangleRenderer, TextSystem};

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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
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

        // Request device and queue with default limits
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ora-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
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

    /// Render a frame using the layered GPU pipeline.
    /// Processes paint commands and renders rectangles and text.
    ///
    /// Paint commands are split at `LayerBoundary` markers (inserted by Stack).
    /// Each layer group renders its rects then text before the next layer,
    /// ensuring correct z-ordering when overlays occlude lower content.
    ///
    /// When no LayerBoundary markers are present (common case — no overlays),
    /// exactly one render pass is created, identical in cost to the old single-pass approach.
    pub fn render_frame(
        &mut self,
        commands: &[crate::element::PaintCommand],
    ) -> Result<(), wgpu::SurfaceError> {
        use crate::element::PaintCommand;
        use crate::rendering::RectInstance;

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Step 1: Group commands by LayerBoundary.
        // Each group contains rects and text entries for one layer.
        // The first group starts implicitly (no leading LayerBoundary needed).
        let mut layer_rect_instances: Vec<Vec<RectInstance>> = vec![Vec::new()];
        let mut layer_text_data: Vec<Vec<(glyphon::Buffer, f32, f32, crate::style::Rect, crate::style::Color)>> = vec![Vec::new()];

        for command in commands {
            match command {
                PaintCommand::LayerBoundary => {
                    layer_rect_instances.push(Vec::new());
                    layer_text_data.push(Vec::new());
                }
                PaintCommand::StyledRect { bounds, style } => {
                    layer_rect_instances.last_mut().unwrap()
                        .push(RectInstance::from_style(style, bounds, self.size));
                }
                PaintCommand::Rect { x, y, width, height, color } => {
                    layer_rect_instances.last_mut().unwrap()
                        .push(RectInstance::from_legacy(*x, *y, *width, *height, *color, self.size));
                }
                PaintCommand::Text { buffer, left, top, bounds, color } => {
                    layer_text_data.last_mut().unwrap()
                        .push((buffer.clone(), *left, *top, *bounds, *color));
                }
                PaintCommand::SetScissor { .. } => {} // TODO: future scissor support
                PaintCommand::ResetScissor => {}
            }
        }

        let layer_count = layer_rect_instances.len();

        // Step 2: Flatten all rect instances into one buffer, track per-layer ranges.
        // Single prepare() call avoids bind_group invalidation from multiple uploads.
        let mut all_rect_instances: Vec<RectInstance> = Vec::new();
        let mut layer_ranges: Vec<std::ops::Range<u32>> = Vec::new();

        for layer_rects in &layer_rect_instances {
            let start = all_rect_instances.len() as u32;
            all_rect_instances.extend_from_slice(layer_rects);
            let end = all_rect_instances.len() as u32;
            layer_ranges.push(start..end);
        }

        // Step 3: Single prepare for all rect instances.
        self.rect_renderer.prepare(&self.device, &self.queue, &all_rect_instances);

        // Step 4: Add text entries to per-layer text system and ensure pool size.
        self.text_system.ensure_layer_renderers(&self.device, layer_count);

        for (layer_idx, text_entries) in layer_text_data.iter().enumerate() {
            for (buffer, left, top, bounds, color) in text_entries {
                self.text_system.add_text_to_layer(
                    layer_idx,
                    buffer.clone(),
                    *left,
                    *top,
                    *bounds,
                    *color,
                );
            }
        }

        // Step 5: Prepare ALL layers before rendering any.
        // All text prepares must complete before any renders to stabilize the atlas
        // and prevent bind_group invalidation during the render phase.
        for layer_idx in 0..layer_count {
            self.text_system
                .prepare_layer(&self.device, &self.queue, layer_idx, self.size.0, self.size.1)
                .unwrap_or_else(|e| log::warn!("Text prepare error for layer {}: {:?}", layer_idx, e));
        }

        // Step 6: Create command encoder.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ora-render-encoder"),
            });

        // Step 7: Render each layer in its own render pass.
        // First pass: Clear (required — Vulkan UB if Load on uninitialized texture).
        // Subsequent passes: Load (preserves previous layers for correct occlusion).
        for layer_idx in 0..layer_count {
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

                // Draw rects for this layer (range slice of the single prepared buffer).
                let range = layer_ranges[layer_idx].clone();
                if !range.is_empty() {
                    self.rect_renderer.render_range(&mut render_pass, range);
                }

                // Draw text for this layer.
                self.text_system
                    .render_layer(layer_idx, &mut render_pass)
                    .unwrap_or_else(|e| log::warn!("Text render error for layer {}: {:?}", layer_idx, e));
            } // render_pass dropped here — encoder borrow released
        }

        // Step 8: Single submit per frame (multiple submits hang on Vulkan).
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Step 9: Clear text system for next frame (trims atlas once, not per layer).
        self.text_system.clear();

        Ok(())
    }
}
