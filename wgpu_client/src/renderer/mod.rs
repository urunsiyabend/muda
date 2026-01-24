//! GPU renderer orchestrating all components.

use crate::components::{Bounds, Caret, Gutter, SidebarComponent, StatusBar, TextArea};
use crate::theme::Theme;
use core_editor::view_model::RenderModel;

/// Main GPU renderer that coordinates all UI components.
pub struct GpuRenderer {
    // Components
    text_area: TextArea,
    gutter: Gutter,
    caret: Caret,
    status_bar: StatusBar,
    sidebar: SidebarComponent,

    // State
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    theme: Theme,
}

impl GpuRenderer {
    /// Creates a new GPU renderer for the given window.
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("muda_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Initialize theme
        let theme = Theme::dark();

        // Create components
        let text_area = TextArea::new(&device, &queue, surface_format);
        let gutter = Gutter::new(&device, &queue, surface_format);
        let caret = Caret::new(&device, surface_format);
        let status_bar = StatusBar::new(&device, &queue, surface_format);
        let sidebar = SidebarComponent::new(&device, &queue, surface_format);

        Ok(Self {
            text_area,
            gutter,
            caret,
            status_bar,
            sidebar,
            surface,
            device,
            queue,
            config,
            theme,
        })
    }

    /// Returns a reference to the theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Sets the theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Handles window resize.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Notifies the renderer of user activity (for caret blinking).
    pub fn on_activity(&mut self) {
        self.caret.on_activity();
    }

    /// Updates animation state. Returns true if a redraw is needed.
    pub fn update(&mut self) -> bool {
        self.caret.update()
    }

    /// Returns the time until the next animation frame is needed.
    pub fn time_until_next_frame(&self) -> std::time::Duration {
        self.caret.time_until_next_blink()
    }

    /// Renders a frame from the given RenderModel.
    pub fn render(&mut self, model: &RenderModel, scale_factor: f32) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen_width = self.config.width as f32;
        let screen_height = self.config.height as f32;

        // Calculate layout
        let layout = self.calculate_layout(model, scale_factor);

        // Prepare all components
        self.sidebar.prepare(
            &self.device,
            &self.queue,
            &model.sidebar,
            layout.sidebar,
            &self.theme,
            scale_factor,
        );

        self.gutter.prepare(
            &self.device,
            &self.queue,
            model,
            layout.gutter,
            &self.theme,
            scale_factor,
        );

        self.text_area.prepare(
            &self.device,
            &self.queue,
            model,
            layout.text_area,
            &self.theme,
            scale_factor,
        );

        self.status_bar.prepare(
            &self.device,
            &self.queue,
            &model.status,
            layout.status_bar,
            &self.theme,
            scale_factor,
        );

        // Build all rectangles
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Clear the screen
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.theme.palette.bg.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Render sidebar backgrounds
        if model.sidebar.visible {
            let sidebar_rects = self.sidebar.build_rects(&model.sidebar, layout.sidebar, &self.theme);
            self.sidebar.render_background(&mut encoder, &view, &self.queue, &sidebar_rects, screen_width, screen_height);
        }

        // Render gutter background
        if model.gutter.visible {
            self.gutter.render_background(&mut encoder, &view, &self.queue, layout.gutter, &self.theme, screen_width, screen_height);
        }

        // Render selection backgrounds
        let selection_rects = self.text_area.build_selection_rects(model, layout.text_area, &self.theme);
        self.text_area.render_selections(&mut encoder, &view, &self.queue, &selection_rects, screen_width, screen_height);

        // Render status bar background
        self.status_bar.render_background(&mut encoder, &view, &self.queue, layout.status_bar, &self.theme, screen_width, screen_height);

        // Render caret
        self.caret.render(
            &mut encoder,
            &view,
            &self.queue,
            &model.caret,
            layout.text_area,
            &self.theme,
            screen_width,
            screen_height,
        );

        // Render text (requires a render pass)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            if model.sidebar.visible {
                self.sidebar.render(&mut pass);
            }
            if model.gutter.visible {
                self.gutter.render(&mut pass);
            }
            self.text_area.render(&mut pass);
            self.status_bar.render(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Calculates the layout bounds for all components.
    fn calculate_layout(&self, model: &RenderModel, scale_factor: f32) -> Layout {
        let screen_width = self.config.width as f32 / scale_factor;
        let screen_height = self.config.height as f32 / scale_factor;
        let status_height = self.status_bar.height();

        // Total bounds
        let total = Bounds::new(0.0, 0.0, screen_width, screen_height);

        // Split off status bar at bottom
        let (main_area, status_bar) = total.split_vertical(screen_height - status_height);

        // Split sidebar if visible
        let (sidebar, editor_area) = if model.sidebar.visible {
            main_area.split_horizontal(self.sidebar.width(&model.sidebar))
        } else {
            (Bounds::default(), main_area)
        };

        // Split gutter if visible
        let (gutter, text_area) = if model.gutter.visible {
            let gutter_width = self.gutter.width(&model.gutter, &self.theme);
            editor_area.split_horizontal(gutter_width)
        } else {
            (Bounds::default(), editor_area)
        };

        Layout {
            sidebar,
            gutter,
            text_area,
            status_bar,
        }
    }
}

/// Layout bounds for all components.
struct Layout {
    sidebar: Bounds,
    gutter: Bounds,
    text_area: Bounds,
    status_bar: Bounds,
}
