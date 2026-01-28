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

    /// Render a frame using the full GPU pipeline.
    /// Processes paint commands and renders rectangles and text.
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

        // Phase 1: Convert PaintCommands to GPU data
        // For Phase 2, we batch all rects and text, ignoring scissor for now
        // (Scissor will be implemented when needed for overflow:hidden)
        let mut rect_instances = Vec::new();

        for command in commands {
            match command {
                PaintCommand::StyledRect { bounds, style } => {
                    rect_instances.push(RectInstance::from_style(style, bounds, self.size));
                }
                PaintCommand::Rect { x, y, width, height, color } => {
                    // Legacy backward compatibility
                    rect_instances.push(RectInstance::from_legacy(
                        *x, *y, *width, *height, *color, self.size
                    ));
                }
                PaintCommand::Text { buffer, left, top, bounds, color } => {
                    self.text_system.add_text_area(
                        buffer.clone(),
                        *left,
                        *top,
                        *bounds,
                        *color,
                    );
                }
                PaintCommand::SetScissor { x, y, width, height } => {
                    // TODO: Implement scissor rect for overflow:hidden
                    // For now, log it so we know it's being called
                    log::trace!("SetScissor({}, {}, {}, {})", x, y, width, height);
                }
                PaintCommand::ResetScissor => {
                    // TODO: Implement scissor reset
                    log::trace!("ResetScissor");
                }
            }
        }

        // Phase 2: Prepare GPU data
        self.rect_renderer.prepare(&self.device, &self.queue, &rect_instances);

        self.text_system.prepare(
            &self.device,
            &self.queue,
            self.size.0,
            self.size.1,
        ).unwrap_or_else(|e| {
            log::warn!("Text prepare error: {:?}", e);
        });

        // Phase 3: Render pass
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ora-render-encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ora-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw rectangles first (background layer)
            self.rect_renderer.render(&mut render_pass);

            // Draw text on top
            self.text_system.render(&mut render_pass).unwrap_or_else(|e| {
                log::warn!("Text render error: {:?}", e);
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Clear text system for next frame
        self.text_system.clear();

        Ok(())
    }
}
