//! File explorer sidebar component.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{Theme, ColorRole};
use core_editor::view_model::SidebarPresentation;

/// File explorer sidebar component.
pub struct SidebarComponent {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffer: Option<Buffer>,
    rect_renderer: RectRenderer,
    prepared: bool,
}

impl SidebarComponent {
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
            buffer: Some(buffer),
            rect_renderer,
            prepared: false,
        }
    }

    /// Returns the sidebar width in logical pixels.
    pub fn width(&self, sidebar: &SidebarPresentation) -> f32 {
        if !sidebar.visible {
            return 0.0;
        }
        sidebar.width as f32
    }

    /// Prepares the sidebar for rendering.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sidebar: &SidebarPresentation,
        bounds: Bounds,
        theme: &Theme,
        scale_factor: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        if !sidebar.visible {
            self.prepared = false;
            return;
        }

        let font_size = theme.font_size * 0.9;
        let line_height = font_size * 1.4;
        let metrics = Metrics::new(font_size, line_height);
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

        // Build file list text
        let mut text = String::new();
        for entry in &sidebar.entries {
            let prefix = if entry.is_dir { "📁 " } else { "📄 " };
            text.push_str(&format!("{}{}\n", prefix, entry.name));
        }

        // Get or create the buffer
        let buffer = self.buffer.get_or_insert_with(|| {
            Buffer::new(&mut self.font_system, metrics)
        });

        buffer.set_metrics(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_height),
        );

        buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(theme.palette.get(ColorRole::SidebarFg).to_glyphon()),
            Shaping::Advanced,
        );

        buffer.shape_until_scroll(&mut self.font_system, false);

        // Prepare text renderer
        let text_areas = [glyphon::TextArea {
            buffer,
            left: (bounds.x + 8.0) * scale_factor, // Left padding
            top: bounds.y * scale_factor + 8.0,    // Top padding
            scale: scale_factor,
            bounds: TextBounds {
                left: (bounds.x * scale_factor) as i32,
                top: (bounds.y * scale_factor) as i32,
                right: ((bounds.x + bounds.width) * scale_factor) as i32,
                bottom: ((bounds.y + bounds.height) * scale_factor) as i32,
            },
            default_color: theme.palette.get(ColorRole::SidebarFg).to_glyphon(),
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
            .expect("Failed to prepare sidebar text");

        self.prepared = true;
    }

    /// Builds selection and highlight rectangles for the sidebar.
    pub fn build_rects(
        &self,
        sidebar: &SidebarPresentation,
        bounds: Bounds,
        theme: &Theme,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();

        if !sidebar.visible {
            return rects;
        }

        // Background
        rects.push(Rect::new(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            theme.palette.get(ColorRole::SidebarBg),
        ));

        // Selection highlight
        let line_height = theme.font_size * 0.9 * 1.4;
        for (i, entry) in sidebar.entries.iter().enumerate() {
            if entry.is_selected {
                rects.push(Rect::new(
                    bounds.x,
                    bounds.y + 8.0 + (i as f32 * line_height),
                    bounds.width,
                    line_height,
                    if sidebar.focused {
                        theme.palette.get(ColorRole::Selection)
                    } else {
                        theme.palette.get(ColorRole::InteractiveHover)
                    },
                ));
            }
        }

        // Border (right edge)
        rects.push(Rect::new(
            bounds.right() - 1.0,
            bounds.y,
            1.0,
            bounds.height,
            theme.palette.get(ColorRole::BorderDefault),
        ));

        rects
    }

    /// Renders the sidebar backgrounds and borders.
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
        self.rect_renderer.render(encoder, view, queue, &physical_rects, screen_width, screen_height);
    }

    /// Renders the sidebar text.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render sidebar");
        }
    }
}
