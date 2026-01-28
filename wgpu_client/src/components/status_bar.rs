//! Status bar component.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{Theme, ColorRole};
use core_editor::view_model::StatusPresentation;

/// Status bar at the bottom of the editor.
pub struct StatusBar {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffer: Buffer,
    rect_renderer: RectRenderer,
    height: f32,
    prepared: bool,
}

impl StatusBar {
    const DEFAULT_HEIGHT: f32 = 24.0;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let mut text_atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(device, &cache);
        let buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 18.0));
        let rect_renderer = RectRenderer::new(device, format);

        Self {
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            buffer,
            rect_renderer,
            height: Self::DEFAULT_HEIGHT,
            prepared: false,
        }
    }

    /// Returns the status bar height in logical pixels.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Prepares the status bar for rendering.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        status: &StatusPresentation,
        bounds: Bounds,
        theme: &Theme,
        scale_factor: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        let font_size = theme.font_size * 0.9;
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let physical_width = bounds.width * scale_factor;
        let physical_height = bounds.height * scale_factor;

        // Update viewport with full screen resolution (required by glyphon)
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: screen_width,
                height: screen_height,
            },
        );

        self.buffer.set_metrics(&mut self.font_system, metrics);
        self.buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_height),
        );

        // Build status text
        let text = if let Some(ref message) = status.message {
            format!(" {} ", message)
        } else {
            format!(
                " Ln {}, Col {} │ {} │ {} lines ",
                status.cursor_line, status.cursor_column, status.language, status.total_lines
            )
        };

        self.buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(theme.palette.get(ColorRole::StatusBarFg).to_glyphon()),
            Shaping::Advanced,
        );

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let text_areas = [glyphon::TextArea {
            buffer: &self.buffer,
            left: bounds.x * scale_factor,
            top: bounds.y * scale_factor + 4.0,
            scale: scale_factor,
            bounds: TextBounds {
                left: (bounds.x * scale_factor) as i32,
                top: (bounds.y * scale_factor) as i32,
                right: ((bounds.x + bounds.width) * scale_factor) as i32,
                bottom: ((bounds.y + bounds.height) * scale_factor) as i32,
            },
            default_color: theme.palette.get(ColorRole::StatusBarFg).to_glyphon(),
            custom_glyphs: &[],
        }];

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("Failed to prepare status bar text");

        self.prepared = true;
    }

    /// Renders the status bar background.
    pub fn render_background(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        bounds: Bounds,
        theme: &Theme,
        screen_width: f32,
        screen_height: f32,
        scale_factor: f32,
    ) {
        // Convert from logical to physical pixels
        let rect = Rect::new(
            bounds.x * scale_factor,
            bounds.y * scale_factor,
            bounds.width * scale_factor,
            bounds.height * scale_factor,
            theme.palette.get(ColorRole::StatusBarBg),
        );
        self.rect_renderer
            .render(encoder, view, queue, &[rect], screen_width, screen_height, None);
    }

    /// Renders the status bar text.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render status bar");
        }
    }
}
