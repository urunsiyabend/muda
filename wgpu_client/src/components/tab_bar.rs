//! Tab bar component showing open documents.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{Theme, ColorRole};
use core_editor::view_model::TabBarPresentation;

/// Tab bar component showing open document tabs.
pub struct TabBar {
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

impl TabBar {
    const DEFAULT_HEIGHT: f32 = 28.0;
    const TAB_PADDING_H: f32 = 12.0;
    const TAB_PADDING_V: f32 = 6.0;

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
        let buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 16.0));
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

    /// Returns the tab bar height in logical pixels.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Prepares the tab bar for rendering.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tab_bar: &TabBarPresentation,
        bounds: Bounds,
        theme: &Theme,
        scale_factor: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        if !tab_bar.visible || tab_bar.tabs.is_empty() {
            self.prepared = false;
            return;
        }

        let font_size = 12.0;
        let metrics = Metrics::new(font_size, font_size * 1.3);
        let physical_width = bounds.width * scale_factor;
        let physical_height = bounds.height * scale_factor;

        // Update viewport with full screen resolution
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

        // Build tab text with proper spacing
        let text: String = tab_bar.tabs
            .iter()
            .map(|tab| {
                let dirty_marker = if tab.is_dirty { "● " } else { "" };
                format!("  {}{}  ", dirty_marker, tab.title)
            })
            .collect::<Vec<_>>()
            .join("");

        self.buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(theme.palette.get(ColorRole::FgPrimary).to_glyphon()),
            Shaping::Advanced,
        );

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let text_areas = [glyphon::TextArea {
            buffer: &self.buffer,
            left: bounds.x * scale_factor,
            top: (bounds.y + Self::TAB_PADDING_V) * scale_factor,
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
            .expect("Failed to prepare tab bar text");

        self.prepared = true;
    }

    /// Builds background rectangles for the tab bar.
    pub fn build_rects(
        &self,
        tab_bar: &TabBarPresentation,
        bounds: Bounds,
        theme: &Theme,
        char_width: f32,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();

        if !tab_bar.visible || tab_bar.tabs.is_empty() {
            return rects;
        }

        // Tab bar background (slightly darker than editor)
        rects.push(Rect::new(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            theme.palette.get(ColorRole::GutterBg),
        ));

        // Bottom border line
        rects.push(Rect::new(
            bounds.x,
            bounds.y + bounds.height - 1.0,
            bounds.width,
            1.0,
            theme.palette.get(ColorRole::LineNumber),
        ));

        // Calculate tab positions and highlight active
        let tab_char_width = char_width * 0.85; // Slightly narrower for tab text
        let mut x_offset = bounds.x;

        for tab in &tab_bar.tabs {
            let dirty_chars = if tab.is_dirty { 2 } else { 0 }; // "● " = 2 chars
            let tab_text_len = tab.title.len() + dirty_chars + 4; // 4 spaces padding
            let tab_width = tab_text_len as f32 * tab_char_width;

            if tab.is_active {
                // Active tab background
                rects.push(Rect::new(
                    x_offset,
                    bounds.y,
                    tab_width,
                    bounds.height - 1.0, // Don't cover the bottom border
                    theme.palette.get(ColorRole::BgPrimary), // Same as editor background
                ));

                // Active tab indicator (accent line at bottom)
                rects.push(Rect::new(
                    x_offset,
                    bounds.y + bounds.height - 2.0,
                    tab_width,
                    2.0,
                    theme.palette.get(ColorRole::Caret), // Use caret color as accent
                ));
            }

            x_offset += tab_width;
        }

        rects
    }

    /// Renders the tab bar background.
    pub fn render_background(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        rects: &[Rect],
        screen_width: f32,
        screen_height: f32,
        scale_factor: f32,
    ) {
        let physical_rects: Vec<Rect> = rects
            .iter()
            .map(|r| {
                Rect::new(
                    r.x * scale_factor,
                    r.y * scale_factor,
                    r.width * scale_factor,
                    r.height * scale_factor,
                    r.color,
                )
            })
            .collect();
        self.rect_renderer
            .render(encoder, view, queue, &physical_rects, screen_width, screen_height, None);
    }

    /// Renders the tab bar text.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render tab bar");
        }
    }
}
