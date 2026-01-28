use crate::style::{Color, Size};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::{Device, MultisampleState, Queue, RenderPass};

/// Text entry for batch rendering
struct TextEntry {
    buffer: Buffer,
    left: f32,
    top: f32,
    bounds: TextBounds,
    default_color: GlyphonColor,
}

/// TextSystem wraps glyphon for framework-owned text rendering.
///
/// All font resources (FontSystem, TextAtlas, SwashCache) are owned by the framework.
/// Elements provide text content and style, and text is measured during layout
/// then batch-rendered during paint phase.
pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    // Collected text areas for batch rendering
    pending_buffers: Vec<TextEntry>,
}

impl TextSystem {
    /// Create a new TextSystem with the given wgpu resources.
    pub fn new(device: &Device, queue: &Queue, surface_format: wgpu::TextureFormat) -> Self {
        // Create FontSystem which loads system fonts automatically
        let font_system = FontSystem::new();

        // Create SwashCache for rasterization
        let swash_cache = SwashCache::new();

        // Create Cache for atlas management
        let cache = Cache::new(device);

        // Create TextAtlas for GPU texture atlas
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);

        // Create TextRenderer for rendering text
        let text_renderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        // Create viewport
        let viewport = Viewport::new(device, &cache);

        Self {
            font_system,
            swash_cache,
            cache,
            atlas,
            text_renderer,
            viewport,
            pending_buffers: Vec::new(),
        }
    }

    /// Measure text and return the Buffer (for reuse during paint) and measured size.
    ///
    /// CRITICAL: The returned Buffer MUST be used for rendering to ensure measurement
    /// and rendering use the same shaped text (RESEARCH.md Pitfall 4).
    pub fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        max_width: Option<f32>, // None = no wrap, Some(w) = wrap at w
    ) -> (Buffer, Size<f32>) {
        // Create metrics for font sizing
        let metrics = Metrics::new(font_size, line_height);

        // Create buffer with metrics
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Set buffer size for wrapping
        buffer.set_size(
            &mut self.font_system,
            max_width.map(|w| w as f32),
            None,
        );

        // Set text with default attributes
        buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(Family::Monospace), // Use monospace for code editor
            Shaping::Advanced,
        );

        // Shape the text
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Measure by iterating layout runs
        let mut max_width = 0.0f32;
        let mut total_lines = 0;

        for run in buffer.layout_runs() {
            max_width = max_width.max(run.line_w);
            total_lines += 1;
        }

        let height = if total_lines > 0 {
            total_lines as f32 * line_height
        } else {
            line_height // At least one line height for empty text
        };

        (buffer, Size::new(max_width, height))
    }

    /// Add a text area for batch rendering.
    ///
    /// The buffer should be the same Buffer returned from measure_text() to ensure
    /// measurement and rendering use the same shaped text.
    pub fn add_text_area(
        &mut self,
        buffer: Buffer,
        left: f32,
        top: f32,
        clip_bounds: crate::style::Rect, // From scissor/parent bounds
        color: Color,
    ) {
        // Convert Color (0.0..1.0 floats) to glyphon::Color (0..255 u8)
        let glyphon_color = GlyphonColor::rgba(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        );

        // Create text bounds from clip rect
        let bounds = TextBounds {
            left: clip_bounds.origin.x as i32,
            top: clip_bounds.origin.y as i32,
            right: (clip_bounds.origin.x + clip_bounds.size.width) as i32,
            bottom: (clip_bounds.origin.y + clip_bounds.size.height) as i32,
        };

        // Store as TextEntry
        self.pending_buffers.push(TextEntry {
            buffer,
            left,
            top,
            bounds,
            default_color: glyphon_color,
        });
    }

    /// Prepare all collected text for rendering.
    ///
    /// This must be called before render() and after all add_text_area() calls.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        _window_width: u32,
        _window_height: u32,
    ) -> Result<(), glyphon::PrepareError> {
        // Build Vec<TextArea> from pending_buffers
        let text_areas: Vec<TextArea> = self
            .pending_buffers
            .iter()
            .map(|entry| TextArea {
                buffer: &entry.buffer,
                left: entry.left,
                top: entry.top,
                scale: 1.0,
                bounds: entry.bounds,
                default_color: entry.default_color,
                custom_glyphs: &[], // No custom glyphs for Phase 2
            })
            .collect();

        // Prepare text renderer with all text areas
        self.text_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;

        Ok(())
    }

    /// Render all prepared text to the given render pass.
    ///
    /// This must be called after prepare().
    pub fn render<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
    ) -> Result<(), glyphon::RenderError> {
        self.text_renderer
            .render(&self.atlas, &self.viewport, render_pass)?;
        Ok(())
    }

    /// Clear all pending text areas.
    ///
    /// Call this at the start of each frame after rendering is complete.
    pub fn clear(&mut self) {
        self.pending_buffers.clear();
        self.atlas.trim(); // Free unused atlas space
    }

    /// Get mutable access to the FontSystem for advanced text operations.
    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
}
