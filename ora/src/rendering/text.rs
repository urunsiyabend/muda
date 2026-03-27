use crate::style::{Color, Size};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use lru::LruCache;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use wgpu::{Device, MultisampleState, Queue, RenderPass};

/// Cache key for a shaped glyph buffer.
/// Encodes text content (as a hash), font size, and line height.
/// Two lines with identical text/size/height share the same shaped Buffer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GlyphCacheKey {
    pub text_hash: u64,
    pub font_size_bits: u32,
    pub line_height_bits: u32,
}

impl GlyphCacheKey {
    pub fn new(text: &str, font_size: f32, line_height: f32) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let text_hash = hasher.finish();
        Self {
            text_hash,
            font_size_bits: font_size.to_bits(),
            line_height_bits: line_height.to_bits(),
        }
    }
}

/// Text entry for batch rendering
struct TextEntry {
    buffer: Buffer,
    left: f32,
    top: f32,
    bounds: TextBounds,
    default_color: GlyphonColor,
    cache_key: Option<GlyphCacheKey>,
}

/// TextSystem wraps glyphon for framework-owned text rendering.
///
/// All font resources (FontSystem, TextAtlas, SwashCache) are owned by the framework.
/// Elements provide text content and style, and text is measured during layout
/// then batch-rendered during paint phase.
///
/// Multi-layer rendering: maintains a pool of TextRenderer instances (one per layer),
/// all sharing the same TextAtlas. Each layer's text is prepared independently without
/// overwriting other layers' glyph data. The single-layer API (add_text_area/prepare/render)
/// is preserved for backward compatibility.
pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,    // Keep for backward compat (single-layer API)
    viewport: Viewport,
    pending_buffers: Vec<TextEntry>, // Keep for backward compat
    // Multi-layer support
    layer_renderers: Vec<TextRenderer>,
    pending_layers: Vec<Vec<TextEntry>>,
    // Glyph buffer LRU cache
    glyph_cache: LruCache<GlyphCacheKey, Buffer>,
    cache_hits: u64,
    cache_misses: u64,
    cache_enabled: bool,
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

        // Capacity: 2048 shaped Buffers. Each Buffer is ~1-2 KB, so total ~2-4 MB.
        let glyph_cache = LruCache::new(NonZeroUsize::new(2048).unwrap());

        Self {
            font_system,
            swash_cache,
            cache,
            atlas,
            text_renderer,
            viewport,
            pending_buffers: Vec::new(),
            layer_renderers: Vec::new(),
            pending_layers: Vec::new(),
            glyph_cache,
            cache_hits: 0,
            cache_misses: 0,
            cache_enabled: true,
        }
    }

    /// Measure the advance width of a single monospace character at the given font size.
    ///
    /// Shapes a 10-character reference string and divides the total width by 10
    /// for better accuracy (avoids rounding from a single glyph).
    pub fn measure_monospace_char_width(&mut self, font_size: f32, line_height: f32) -> f32 {
        let reference = "0000000000"; // 10 identical chars
        let (_buf, size) = self.measure_text(reference, font_size, line_height, None);
        size.width / 10.0
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

    /// Cache-aware text measurement.
    ///
    /// Computes a GlyphCacheKey from (text, font_size, line_height). If the key exists
    /// in the LRU cache, pops and returns the pre-shaped Buffer (avoiding set_text +
    /// shape_until_scroll). On miss, falls back to measure_text() and shapes fresh.
    ///
    /// The Buffer is POPPED (removed) from the cache because it is moved into
    /// PaintCommand::Text. It is returned to the cache via return_buffers_to_cache()
    /// after the frame's render_frame() completes.
    ///
    /// Returns (GlyphCacheKey, Buffer, Size<f32>).
    pub fn measure_text_cached(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        max_width: Option<f32>,
    ) -> (GlyphCacheKey, Buffer, Size<f32>) {
        let key = GlyphCacheKey::new(text, font_size, line_height);

        if self.cache_enabled {
            if let Some(buffer) = self.glyph_cache.peek(&key) {
                // Cache hit: clone the buffer (stays in cache for next caller)
                // and measure size from its existing layout runs.
                let cloned = buffer.clone();
                let mut measured_width = 0.0f32;
                let mut total_lines = 0;

                for run in cloned.layout_runs() {
                    measured_width = measured_width.max(run.line_w);
                    total_lines += 1;
                }

                let height = if total_lines > 0 {
                    total_lines as f32 * line_height
                } else {
                    line_height
                };

                self.cache_hits += 1;
                return (key, cloned, Size::new(measured_width, height));
            }
        }

        // Cache miss: shape fresh and insert into cache
        let (buffer, size) = self.measure_text(text, font_size, line_height, max_width);
        // Store a clone in cache; return the original
        self.glyph_cache.put(key, buffer.clone());
        self.cache_misses += 1;
        (key, buffer, size)
    }

    /// Return shaped Buffers to the LRU cache after a frame's render completes.
    ///
    /// Buffers are extracted from TextSystem before clear() and fed back here.
    /// Only called when cache_enabled is true.
    pub fn return_buffers_to_cache(&mut self, buffers: Vec<(GlyphCacheKey, Buffer)>) {
        if !self.cache_enabled {
            return;
        }
        // With peek+clone in measure_text_cached, entries persist in cache.
        // This return path refreshes LRU recency for actively-used entries.
        for (key, buffer) in buffers {
            self.glyph_cache.put(key, buffer);
        }
    }

    /// Return the glyph cache hit rate as a value in [0.0, 1.0].
    /// Returns 0.0 if no measure_text_cached() calls have been made yet.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Enable or disable the glyph buffer cache.
    /// When disabled, measure_text_cached() always shapes fresh (hits are 0%).
    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_enabled = enabled;
    }

    /// Reset hit/miss counters. Call at start of each measurement window if desired.
    pub fn reset_cache_stats(&mut self) {
        self.cache_hits = 0;
        self.cache_misses = 0;
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
        cache_key: Option<GlyphCacheKey>,
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
            cache_key,
        });
    }

    /// Prepare all collected text for rendering.
    ///
    /// This must be called before render() and after all add_text_area() calls.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        window_width: u32,
        window_height: u32,
    ) -> Result<(), glyphon::PrepareError> {
        // Update viewport with current window dimensions — required for glyphon to render
        self.viewport.update(
            queue,
            Resolution {
                width: window_width,
                height: window_height,
            },
        );

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

    /// Clear pending text areas without trimming the atlas.
    /// Used between layer groups within a single frame.
    pub fn clear_pending(&mut self) {
        self.pending_buffers.clear();
    }

    /// Clear all pending text areas and trim unused atlas space.
    ///
    /// Call this at the start of each frame after rendering is complete.
    pub fn clear(&mut self) {
        self.pending_buffers.clear();
        // Clear per-layer pending entries
        for layer in &mut self.pending_layers {
            layer.clear();
        }
        self.atlas.trim(); // Free unused atlas space
    }

    /// Drain all pending buffers (single-layer and all layers) and collect
    /// those tagged with a cache_key for return to the LRU cache.
    ///
    /// This replaces the clear() call at end-of-frame. After draining,
    /// pending_buffers and pending_layers are empty. atlas.trim() is called.
    ///
    /// The returned Vec is passed to return_buffers_to_cache() by the caller.
    pub fn drain_buffers_for_cache(&mut self) -> Vec<(GlyphCacheKey, Buffer)> {
        let mut returnable: Vec<(GlyphCacheKey, Buffer)> = Vec::new();
        // Drain single-layer pending buffers
        for entry in self.pending_buffers.drain(..) {
            if let Some(key) = entry.cache_key {
                returnable.push((key, entry.buffer));
            }
        }

        // Drain all layer-specific pending buffers
        for layer in &mut self.pending_layers {
            for entry in layer.drain(..) {
                if let Some(key) = entry.cache_key {
                    returnable.push((key, entry.buffer));
                }
            }
        }

        self.atlas.trim();
        returnable
    }

    /// Get mutable access to the FontSystem for advanced text operations.
    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    // -------------------------------------------------------------------------
    // Multi-layer API
    // -------------------------------------------------------------------------

    /// Ensure at least `count` TextRenderer instances exist for layer rendering.
    /// New renderers are created on demand sharing the same TextAtlas.
    /// Renderers are pooled and reused across frames — never shrunk.
    pub fn ensure_layer_renderers(&mut self, device: &wgpu::Device, count: usize) {
        while self.layer_renderers.len() < count {
            let renderer = TextRenderer::new(
                &mut self.atlas,
                device,
                MultisampleState::default(),
                None,
            );
            self.layer_renderers.push(renderer);
        }
        // Ensure pending_layers matches
        while self.pending_layers.len() < count {
            self.pending_layers.push(Vec::new());
        }
    }

    /// Add a text entry to a specific layer for batch rendering.
    /// The layer_idx must be less than the count passed to ensure_layer_renderers().
    pub fn add_text_to_layer(
        &mut self,
        layer_idx: usize,
        buffer: Buffer,
        left: f32,
        top: f32,
        clip_bounds: crate::style::Rect,
        color: Color,
        cache_key: Option<GlyphCacheKey>,
    ) {
        let glyphon_color = GlyphonColor::rgba(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        );
        let bounds = TextBounds {
            left: clip_bounds.origin.x as i32,
            top: clip_bounds.origin.y as i32,
            right: (clip_bounds.origin.x + clip_bounds.size.width) as i32,
            bottom: (clip_bounds.origin.y + clip_bounds.size.height) as i32,
        };
        if layer_idx < self.pending_layers.len() {
            self.pending_layers[layer_idx].push(TextEntry {
                buffer,
                left,
                top,
                bounds,
                default_color: glyphon_color,
                cache_key,
            });
        }
    }

    /// Prepare text for a specific layer.
    /// Must be called after all add_text_to_layer() calls for this layer,
    /// and before render_layer(). All layer prepares should complete before
    /// any render_layer() calls to stabilize the atlas.
    pub fn prepare_layer(
        &mut self,
        device: &Device,
        queue: &Queue,
        layer_idx: usize,
        window_width: u32,
        window_height: u32,
    ) -> Result<(), glyphon::PrepareError> {
        // Update viewport (idempotent — same resolution each call within a frame)
        self.viewport.update(
            queue,
            Resolution {
                width: window_width,
                height: window_height,
            },
        );

        if layer_idx >= self.pending_layers.len() || layer_idx >= self.layer_renderers.len() {
            return Ok(());
        }

        let text_areas: Vec<TextArea> = self.pending_layers[layer_idx]
            .iter()
            .map(|entry| TextArea {
                buffer: &entry.buffer,
                left: entry.left,
                top: entry.top,
                scale: 1.0,
                bounds: entry.bounds,
                default_color: entry.default_color,
                custom_glyphs: &[],
            })
            .collect();

        self.layer_renderers[layer_idx].prepare(
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

    /// Render prepared text for a specific layer.
    /// Must be called after prepare_layer() for this layer.
    pub fn render_layer(
        &self,
        layer_idx: usize,
        render_pass: &mut RenderPass<'_>,
    ) -> Result<(), glyphon::RenderError> {
        if layer_idx >= self.layer_renderers.len() {
            return Ok(());
        }
        self.layer_renderers[layer_idx]
            .render(&self.atlas, &self.viewport, render_pass)?;
        Ok(())
    }

    /// Get the number of available layer renderers.
    pub fn layer_count(&self) -> usize {
        self.layer_renderers.len()
    }
}
