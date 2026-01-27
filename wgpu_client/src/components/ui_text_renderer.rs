//! Shared text renderer for UI components.
//!
//! This renderer accepts `TextBlock` arrays and renders them using glyphon.
//! It provides a unified way to render text across all new UI components.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};

use crate::design_system::primitives::{TextBlock, TextAlign};

/// Maximum number of text blocks that can be rendered in a single batch.
const MAX_TEXT_BLOCKS: usize = 256;

/// Shared text renderer for UI components using TextBlock arrays.
pub struct UITextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffers: Vec<Buffer>,
    prepared_count: usize,
    prepared_blocks: Vec<PreparedTextBlock>,
}

/// Internal prepared text block data for rendering.
struct PreparedTextBlock {
    left: f32,
    top: f32,
    scale: f32,
    bounds: TextBounds,
    color: glyphon::Color,
}

impl UITextRenderer {
    /// Create a new UI text renderer.
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

        // Pre-allocate buffers
        let mut buffers = Vec::with_capacity(MAX_TEXT_BLOCKS);
        for _ in 0..MAX_TEXT_BLOCKS {
            buffers.push(Buffer::new(&mut font_system, Metrics::new(14.0, 21.0)));
        }

        Self {
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            buffers,
            prepared_count: 0,
            prepared_blocks: Vec::with_capacity(MAX_TEXT_BLOCKS),
        }
    }

    /// Prepare text blocks for rendering.
    ///
    /// Must be called before `render()`. The `scale_factor` is the display scale,
    /// and `screen_size` is the physical screen dimensions (width, height).
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texts: &[TextBlock],
        scale_factor: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        self.prepared_count = 0;
        self.prepared_blocks.clear();

        if texts.is_empty() {
            return;
        }

        // Update viewport
        self.viewport.update(
            queue,
            Resolution {
                width: screen_width,
                height: screen_height,
            },
        );

        // Prepare each text block
        let count = texts.len().min(MAX_TEXT_BLOCKS);
        for (i, text_block) in texts.iter().take(count).enumerate() {
            self.prepare_text_block(i, text_block, scale_factor);
        }

        self.prepared_count = count;

        // Build text areas for glyphon
        let text_areas: Vec<TextArea> = (0..self.prepared_count)
            .map(|i| {
                let block = &self.prepared_blocks[i];
                TextArea {
                    buffer: &self.buffers[i],
                    left: block.left,
                    top: block.top,
                    scale: block.scale,
                    bounds: block.bounds,
                    default_color: block.color,
                    custom_glyphs: &[],
                }
            })
            .collect();

        // Prepare the text renderer
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
            .expect("Failed to prepare UI text");
    }

    fn prepare_text_block(&mut self, index: usize, text_block: &TextBlock, scale_factor: f32) {
        // Configure metrics
        let line_height = text_block.font_size * text_block.line_height;
        let metrics = Metrics::new(text_block.font_size, line_height);
        self.buffers[index].set_metrics(&mut self.font_system, metrics);

        // Set buffer size
        let physical_width = text_block.bounds.width * scale_factor;
        let physical_height = text_block.bounds.height * scale_factor;
        self.buffers[index].set_size(&mut self.font_system, Some(physical_width), Some(physical_height));

        // Build attributes
        let weight = if text_block.bold { Weight::BOLD } else { Weight::NORMAL };
        let attrs = Attrs::new()
            .family(Family::Monospace)
            .weight(weight)
            .color(text_block.color.to_glyphon());

        // Set text
        self.buffers[index].set_text(&mut self.font_system, &text_block.text, attrs, Shaping::Advanced);
        self.buffers[index].shape_until_scroll(&mut self.font_system, false);

        // Calculate text position based on alignment
        let text_width = Self::measure_text_width(&self.buffers[index]);
        let aligned_x = match text_block.align {
            TextAlign::Left => text_block.bounds.x,
            TextAlign::Center => text_block.bounds.x + (text_block.bounds.width - text_width / scale_factor) / 2.0,
            TextAlign::Right => text_block.bounds.x + text_block.bounds.width - text_width / scale_factor,
        };

        // Vertically center text within bounds
        let text_height = text_block.font_size;
        let vertical_offset = (text_block.bounds.height - text_height) / 2.0;
        let aligned_y = text_block.bounds.y + vertical_offset;

        // Store prepared block data
        self.prepared_blocks.push(PreparedTextBlock {
            left: aligned_x * scale_factor,
            top: aligned_y * scale_factor,
            scale: scale_factor,
            bounds: TextBounds {
                left: (text_block.bounds.x * scale_factor) as i32,
                top: (text_block.bounds.y * scale_factor) as i32,
                right: ((text_block.bounds.x + text_block.bounds.width) * scale_factor) as i32,
                bottom: ((text_block.bounds.y + text_block.bounds.height) * scale_factor) as i32,
            },
            color: text_block.color.to_glyphon(),
        });
    }

    fn measure_text_width(buffer: &Buffer) -> f32 {
        buffer
            .layout_runs()
            .map(|run| run.line_w)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Render the prepared text blocks.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared_count > 0 {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render UI text");
        }
    }

    /// Clear prepared state.
    pub fn clear(&mut self) {
        self.prepared_count = 0;
        self.prepared_blocks.clear();
    }
}
