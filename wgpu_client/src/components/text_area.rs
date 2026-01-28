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
use crate::theme::{self, Theme, ColorRole};
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

    /// Returns the actual character width by measuring a sample character.
    /// This queries the font metrics for accurate positioning.
    pub fn measure_char_width(&mut self, font_size: f32) -> f32 {
        // Create a temporary buffer to measure character width
        let metrics = Metrics::new(font_size, font_size * 1.5);
        let mut measure_buffer = Buffer::new(&mut self.font_system, metrics);
        measure_buffer.set_text(
            &mut self.font_system,
            "M", // Use 'M' as reference for monospace width
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
        measure_buffer.shape_until_scroll(&mut self.font_system, false);

        // Get the width from the layout
        if let Some(line) = measure_buffer.lines.first() {
            if let Some(layout) = line.layout_opt() {
                if let Some(glyph_run) = layout.first() {
                    if let Some(glyph) = glyph_run.glyphs.first() {
                        return glyph.w;
                    }
                }
            }
        }

        // Fallback to approximation if measurement fails
        font_size * 0.6
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
        screen_width: u32,
        screen_height: u32,
    ) {
        let metrics = Metrics::new(theme.font_size, theme.line_height_px());
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

        // Update buffer metrics
        self.buffer.set_metrics(&mut self.font_system, metrics);
        // Use None for width to disable word wrapping - code editors scroll horizontally
        self.buffer.set_size(
            &mut self.font_system,
            None,
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
            default_color: theme.palette.get(ColorRole::FgPrimary).to_glyphon(),
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
    /// Returns (current_line_rects, selection_rects) for proper z-ordering.
    pub fn build_selection_rects(
        &self,
        model: &RenderModel,
        bounds: Bounds,
        theme: &Theme,
        char_width: f32,
    ) -> (Vec<Rect>, Vec<Rect>) {
        let mut current_line_rects = Vec::new();
        let mut selection_rects = Vec::new();
        let line_height = theme.line_height_px();

        // Current line highlight (rendered first, below selection)
        if model.caret.visible {
            let current_line = model.caret.position.row;
            if current_line < model.visible_lines.len() {
                current_line_rects.push(Rect::new(
                    bounds.x,
                    bounds.y + (current_line as f32 * line_height),
                    bounds.width,
                    line_height,
                    theme.palette.get(ColorRole::CurrentLine),
                ));
            }
        }

        // Selection highlights (rendered on top of current line)
        for (line_idx, line) in model.visible_lines.iter().enumerate() {
            let mut x_offset = 0.0f32;

            for span in &line.spans {
                let style = theme::map_style(span.style, theme);

                if let Some(bg) = style.bg {
                    let span_width = span.text.chars().count() as f32 * char_width;
                    selection_rects.push(Rect::new(
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

        (current_line_rects, selection_rects)
    }

    /// Renders the text area.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        self.text_renderer
            .render(&self.text_atlas, &self.viewport, pass)
            .expect("Failed to render text");
    }

    /// Renders selection backgrounds with optional scissor clipping.
    pub fn render_selections(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        rects: &[Rect],
        screen_width: f32,
        screen_height: f32,
        scale_factor: f32,
        scissor: Option<(u32, u32, u32, u32)>,
    ) {
        // Convert from logical to physical pixels
        let physical_rects: Vec<Rect> = rects.iter().map(|r| {
            Rect::new(
                r.x * scale_factor,
                r.y * scale_factor,
                r.width * scale_factor,
                r.height * scale_factor,
                r.color,
            )
        }).collect();
        self.rect_renderer
            .render(encoder, view, queue, &physical_rects, screen_width, screen_height, scissor);
    }
}
