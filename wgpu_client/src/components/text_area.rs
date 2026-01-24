//! Main text editing area component.
//!
//! Responsible for rendering:
//! - Selection backgrounds
//! - Syntax-highlighted text lines
//! - Current line highlight

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{self, Theme};
use core_editor::view_model::RenderModel;

/// Text area component for the main editor content.
pub struct TextArea {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffer: Buffer,
    rect_renderer: RectRenderer,
}

impl TextArea {
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
        }
    }

    /// Prepares text buffers from the render model.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &RenderModel,
        bounds: Bounds,
        theme: &Theme,
        scale_factor: f32,
    ) {
        let metrics = Metrics::new(theme.font_size, theme.line_height_px());
        let physical_width = bounds.width * scale_factor;
        let physical_height = bounds.height * scale_factor;

        // Update viewport
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: physical_width as u32,
                height: physical_height as u32,
            },
        );

        // Update buffer metrics
        self.buffer.set_metrics(&mut self.font_system, metrics);
        self.buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_height),
        );

        // Build text content
        let mut text = String::new();
        for line in &model.visible_lines {
            for span in &line.spans {
                text.push_str(&span.text);
            }
            text.push('\n');
        }

        // Set text with default attributes
        self.buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        // Prepare text renderer
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
            default_color: theme.palette.fg.to_glyphon(),
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
            .expect("Failed to prepare text renderer");
    }

    /// Builds selection rectangles from the render model.
    pub fn build_selection_rects(
        &self,
        model: &RenderModel,
        bounds: Bounds,
        theme: &Theme,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();
        let line_height = theme.line_height_px();
        let char_width = theme.font_size * 0.6; // Approximate monospace width

        for (line_idx, line) in model.visible_lines.iter().enumerate() {
            let mut x_offset = 0.0f32;

            for span in &line.spans {
                let style = theme::map_style(span.style, theme);

                if let Some(bg) = style.bg {
                    let span_width = span.text.chars().count() as f32 * char_width;
                    rects.push(Rect::new(
                        bounds.x + x_offset,
                        bounds.y + (line_idx as f32 * line_height),
                        span_width,
                        line_height,
                        bg,
                    ));
                }

                x_offset += span.text.chars().count() as f32 * char_width;
            }
        }

        // Current line highlight
        if model.caret.visible {
            let current_line = model.caret.position.row;
            if current_line < model.visible_lines.len() {
                rects.push(Rect::new(
                    bounds.x,
                    bounds.y + (current_line as f32 * line_height),
                    bounds.width,
                    line_height,
                    theme.palette.current_line_bg,
                ));
            }
        }

        rects
    }

    /// Renders the text area.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        self.text_renderer
            .render(&self.text_atlas, &self.viewport, pass)
            .expect("Failed to render text");
    }

    /// Renders selection backgrounds.
    pub fn render_selections(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        rects: &[Rect],
        screen_width: f32,
        screen_height: f32,
    ) {
        self.rect_renderer
            .render(encoder, view, queue, rects, screen_width, screen_height);
    }
}
