//! Line number gutter component.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::Theme;
use core_editor::view_model::{GutterModel, RenderModel};

/// Line number gutter component.
pub struct Gutter {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffer: Buffer,
    rect_renderer: RectRenderer,
    prepared: bool,
}

impl Gutter {
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
        let buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 21.0));
        let rect_renderer = RectRenderer::new(device, format);

        Self {
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            buffer,
            rect_renderer,
            prepared: false,
        }
    }

    /// Calculates the gutter width in pixels.
    pub fn width(&self, gutter: &GutterModel, theme: &Theme) -> f32 {
        if !gutter.visible {
            return 0.0;
        }

        let char_width = theme.font_size * 0.6;
        let digit_count = gutter.line_number_width();
        let padding = 2;

        (digit_count + padding) as f32 * char_width
    }

    /// Prepares the gutter for rendering.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &RenderModel,
        bounds: Bounds,
        theme: &Theme,
        scale_factor: f32,
    ) {
        if !model.gutter.visible {
            self.prepared = false;
            return;
        }

        let metrics = Metrics::new(theme.font_size, theme.line_height_px());
        let physical_width = bounds.width * scale_factor;
        let physical_height = bounds.height * scale_factor;

        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: physical_width as u32,
                height: physical_height as u32,
            },
        );

        self.buffer.set_metrics(&mut self.font_system, metrics);
        self.buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_height),
        );

        // Build line numbers text
        let mut text = String::new();
        let width = model.gutter.line_number_width();

        for line in &model.visible_lines {
            text.push_str(&format!("{:>width$} │\n", line.line_number, width = width));
        }

        self.buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new()
                .family(Family::Monospace)
                .color(theme.palette.line_number.to_glyphon()),
            Shaping::Advanced,
        );

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let text_areas = [glyphon::TextArea {
            buffer: &self.buffer,
            left: bounds.x * scale_factor,
            top: bounds.y * scale_factor,
            scale: scale_factor,
            bounds: TextBounds {
                left: (bounds.x * scale_factor) as i32,
                top: (bounds.y * scale_factor) as i32,
                right: ((bounds.x + bounds.width) * scale_factor) as i32,
                bottom: ((bounds.y + bounds.height) * scale_factor) as i32,
            },
            default_color: theme.palette.line_number.to_glyphon(),
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
            .expect("Failed to prepare gutter text");

        self.prepared = true;
    }

    /// Renders the gutter background.
    pub fn render_background(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        bounds: Bounds,
        theme: &Theme,
        screen_width: f32,
        screen_height: f32,
    ) {
        let rect = Rect::new(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            theme.palette.gutter_bg,
        );
        self.rect_renderer
            .render(encoder, view, queue, &[rect], screen_width, screen_height);
    }

    /// Renders the gutter text.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render gutter");
        }
    }
}
