//! Modal dialog component for confirmations and alerts.

use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{Color, Theme, ColorRole};
use core_editor::view_model::DialogPresentation;

/// Modal dialog component.
pub struct Dialog {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    // Multiple buffers for different text elements
    title_buffer: Buffer,
    content_buffer: Buffer,
    button1_buffer: Buffer,
    button2_buffer: Buffer,
    button3_buffer: Buffer,
    rect_renderer: RectRenderer,
    prepared: bool,
}

impl Dialog {
    const DIALOG_WIDTH: f32 = 420.0;
    const DIALOG_PADDING: f32 = 24.0;
    const BUTTON_HEIGHT: f32 = 36.0;
    const BUTTON_MIN_WIDTH: f32 = 110.0;
    const BUTTON_SPACING: f32 = 12.0;
    const BUTTON_MARGIN_TOP: f32 = 20.0;

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
        let title_buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
        let content_buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
        let button1_buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 16.0));
        let button2_buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 16.0));
        let button3_buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 16.0));
        let rect_renderer = RectRenderer::new(device, format);

        Self {
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            title_buffer,
            content_buffer,
            button1_buffer,
            button2_buffer,
            button3_buffer,
            rect_renderer,
            prepared: false,
        }
    }

    /// Returns whether a dialog should be shown.
    pub fn is_visible(dialog: &DialogPresentation) -> bool {
        !matches!(dialog, DialogPresentation::None)
    }

    /// Prepares the dialog for rendering.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dialog: &DialogPresentation,
        screen_width: f32,
        screen_height: f32,
        theme: &Theme,
        scale_factor: f32,
    ) {
        let DialogPresentation::UnsavedChangesConfirmation {
            action_description,
            unsaved_documents,
        } = dialog else {
            self.prepared = false;
            return;
        };

        // Update viewport
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: (screen_width * scale_factor) as u32,
                height: (screen_height * scale_factor) as u32,
            },
        );

        // Calculate dialog bounds
        let dialog_bounds = self.calculate_dialog_bounds(screen_width, screen_height, unsaved_documents.len());

        // --- Title buffer ---
        let title_metrics = Metrics::new(14.0, 20.0);
        self.title_buffer.set_metrics(&mut self.font_system, title_metrics);
        self.title_buffer.set_size(
            &mut self.font_system,
            Some((dialog_bounds.width - Self::DIALOG_PADDING) * scale_factor),
            Some(36.0 * scale_factor),
        );
        self.title_buffer.set_text(
            &mut self.font_system,
            "Unsaved Changes",
            Attrs::new()
                .family(Family::SansSerif)
                .weight(glyphon::Weight::BOLD)
                .color(theme.palette.get(ColorRole::FgPrimary).to_glyphon()),
            Shaping::Advanced,
        );
        self.title_buffer.shape_until_scroll(&mut self.font_system, false);

        // --- Content buffer ---
        let content_metrics = Metrics::new(14.0, 21.0);
        self.content_buffer.set_metrics(&mut self.font_system, content_metrics);
        self.content_buffer.set_size(
            &mut self.font_system,
            Some((dialog_bounds.width - Self::DIALOG_PADDING * 2.0) * scale_factor),
            Some((dialog_bounds.height - Self::DIALOG_PADDING * 2.0 - Self::BUTTON_HEIGHT - Self::BUTTON_MARGIN_TOP - 40.0) * scale_factor),
        );

        let mut text = String::new();
        text.push_str(&format!("Do you want to save changes before {}?\n\n", action_description.to_lowercase()));

        if !unsaved_documents.is_empty() {
            text.push_str("Unsaved files:\n");
            for doc in unsaved_documents {
                text.push_str(&format!("  - {}\n", doc));
            }
        }

        self.content_buffer.set_text(
            &mut self.font_system,
            &text,
            Attrs::new()
                .family(Family::SansSerif)
                .color(theme.palette.get(ColorRole::FgPrimary).to_glyphon()),
            Shaping::Advanced,
        );
        self.content_buffer.shape_until_scroll(&mut self.font_system, false);

        // --- Button buffers ---
        let button_metrics = Metrics::new(12.0, 16.0);
        let button_text_color = glyphon::Color::rgb(255, 255, 255);

        // Button 1: Save
        self.button1_buffer.set_metrics(&mut self.font_system, button_metrics);
        self.button1_buffer.set_size(
            &mut self.font_system,
            Some(Self::BUTTON_MIN_WIDTH * scale_factor),
            Some(Self::BUTTON_HEIGHT * scale_factor),
        );
        self.button1_buffer.set_text(
            &mut self.font_system,
            "Save (Y)",
            Attrs::new()
                .family(Family::SansSerif)
                .color(button_text_color),
            Shaping::Advanced,
        );
        self.button1_buffer.shape_until_scroll(&mut self.font_system, false);

        // Button 2: Don't Save
        self.button2_buffer.set_metrics(&mut self.font_system, button_metrics);
        self.button2_buffer.set_size(
            &mut self.font_system,
            Some(Self::BUTTON_MIN_WIDTH * scale_factor),
            Some(Self::BUTTON_HEIGHT * scale_factor),
        );
        self.button2_buffer.set_text(
            &mut self.font_system,
            "Don't Save (N)",
            Attrs::new()
                .family(Family::SansSerif)
                .color(button_text_color),
            Shaping::Advanced,
        );
        self.button2_buffer.shape_until_scroll(&mut self.font_system, false);

        // Button 3: Cancel
        self.button3_buffer.set_metrics(&mut self.font_system, button_metrics);
        self.button3_buffer.set_size(
            &mut self.font_system,
            Some(Self::BUTTON_MIN_WIDTH * scale_factor),
            Some(Self::BUTTON_HEIGHT * scale_factor),
        );
        self.button3_buffer.set_text(
            &mut self.font_system,
            "Cancel (Esc)",
            Attrs::new()
                .family(Family::SansSerif)
                .color(button_text_color),
            Shaping::Advanced,
        );
        self.button3_buffer.shape_until_scroll(&mut self.font_system, false);

        // --- Calculate positions ---
        let title_x = (dialog_bounds.x + 12.0) * scale_factor;
        let title_y = (dialog_bounds.y + 8.0) * scale_factor;

        let content_x = (dialog_bounds.x + Self::DIALOG_PADDING) * scale_factor;
        let content_y = (dialog_bounds.y + 48.0) * scale_factor;

        let buttons_y = dialog_bounds.y + dialog_bounds.height - Self::DIALOG_PADDING - Self::BUTTON_HEIGHT;
        let total_button_width = Self::BUTTON_MIN_WIDTH * 3.0 + Self::BUTTON_SPACING * 2.0;
        let buttons_x = dialog_bounds.x + (dialog_bounds.width - total_button_width) / 2.0;

        // Center text within buttons
        let button_text_offset_x = 8.0; // Approximate centering
        let button_text_offset_y = 10.0;

        let button1_x = (buttons_x + button_text_offset_x) * scale_factor;
        let button1_y = (buttons_y + button_text_offset_y) * scale_factor;

        let button2_x = (buttons_x + Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING + button_text_offset_x) * scale_factor;
        let button2_y = (buttons_y + button_text_offset_y) * scale_factor;

        let button3_x = (buttons_x + (Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING) * 2.0 + button_text_offset_x) * scale_factor;
        let button3_y = (buttons_y + button_text_offset_y) * scale_factor;

        // Create text areas for all elements
        let text_areas = [
            // Title
            glyphon::TextArea {
                buffer: &self.title_buffer,
                left: title_x,
                top: title_y,
                scale: scale_factor,
                bounds: TextBounds {
                    left: title_x as i32,
                    top: title_y as i32,
                    right: ((dialog_bounds.x + dialog_bounds.width) * scale_factor) as i32,
                    bottom: ((dialog_bounds.y + 36.0) * scale_factor) as i32,
                },
                default_color: theme.palette.get(ColorRole::FgPrimary).to_glyphon(),
                custom_glyphs: &[],
            },
            // Content
            glyphon::TextArea {
                buffer: &self.content_buffer,
                left: content_x,
                top: content_y,
                scale: scale_factor,
                bounds: TextBounds {
                    left: content_x as i32,
                    top: content_y as i32,
                    right: ((dialog_bounds.x + dialog_bounds.width - Self::DIALOG_PADDING) * scale_factor) as i32,
                    bottom: ((buttons_y - 10.0) * scale_factor) as i32,
                },
                default_color: theme.palette.get(ColorRole::FgPrimary).to_glyphon(),
                custom_glyphs: &[],
            },
            // Button 1: Save
            glyphon::TextArea {
                buffer: &self.button1_buffer,
                left: button1_x,
                top: button1_y,
                scale: scale_factor,
                bounds: TextBounds {
                    left: (buttons_x * scale_factor) as i32,
                    top: (buttons_y * scale_factor) as i32,
                    right: ((buttons_x + Self::BUTTON_MIN_WIDTH) * scale_factor) as i32,
                    bottom: ((buttons_y + Self::BUTTON_HEIGHT) * scale_factor) as i32,
                },
                default_color: button_text_color,
                custom_glyphs: &[],
            },
            // Button 2: Don't Save
            glyphon::TextArea {
                buffer: &self.button2_buffer,
                left: button2_x,
                top: button2_y,
                scale: scale_factor,
                bounds: TextBounds {
                    left: ((buttons_x + Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING) * scale_factor) as i32,
                    top: (buttons_y * scale_factor) as i32,
                    right: ((buttons_x + Self::BUTTON_MIN_WIDTH * 2.0 + Self::BUTTON_SPACING) * scale_factor) as i32,
                    bottom: ((buttons_y + Self::BUTTON_HEIGHT) * scale_factor) as i32,
                },
                default_color: button_text_color,
                custom_glyphs: &[],
            },
            // Button 3: Cancel
            glyphon::TextArea {
                buffer: &self.button3_buffer,
                left: button3_x,
                top: button3_y,
                scale: scale_factor,
                bounds: TextBounds {
                    left: ((buttons_x + (Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING) * 2.0) * scale_factor) as i32,
                    top: (buttons_y * scale_factor) as i32,
                    right: ((buttons_x + Self::BUTTON_MIN_WIDTH * 3.0 + Self::BUTTON_SPACING * 2.0) * scale_factor) as i32,
                    bottom: ((buttons_y + Self::BUTTON_HEIGHT) * scale_factor) as i32,
                },
                default_color: button_text_color,
                custom_glyphs: &[],
            },
        ];

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
            .expect("Failed to prepare dialog text");

        self.prepared = true;
    }

    /// Calculates the dialog bounds centered on screen.
    fn calculate_dialog_bounds(&self, screen_width: f32, screen_height: f32, doc_count: usize) -> Bounds {
        let content_height = 60.0 + (doc_count as f32 * 22.0); // Title + message + file list
        let dialog_height = Self::DIALOG_PADDING * 2.0 + content_height + Self::BUTTON_HEIGHT + Self::BUTTON_MARGIN_TOP + 40.0;

        let x = (screen_width - Self::DIALOG_WIDTH) / 2.0;
        let y = (screen_height - dialog_height) / 2.0;

        Bounds::new(x, y, Self::DIALOG_WIDTH, dialog_height)
    }

    /// Builds rectangles for the dialog overlay, box, and buttons.
    pub fn build_rects(
        &self,
        dialog: &DialogPresentation,
        screen_width: f32,
        screen_height: f32,
        theme: &Theme,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();

        let DialogPresentation::UnsavedChangesConfirmation { unsaved_documents, .. } = dialog else {
            return rects;
        };

        // Semi-transparent overlay covering entire screen
        let overlay_color = Color::from_u8(0, 0, 0, 180);
        rects.push(Rect::new(0.0, 0.0, screen_width, screen_height, overlay_color));

        // Dialog box
        let dialog_bounds = self.calculate_dialog_bounds(screen_width, screen_height, unsaved_documents.len());

        // Shadow
        let shadow_color = Color::from_u8(0, 0, 0, 100);
        rects.push(Rect::new(
            dialog_bounds.x + 6.0,
            dialog_bounds.y + 6.0,
            dialog_bounds.width,
            dialog_bounds.height,
            shadow_color,
        ));

        // Dialog background
        let dialog_bg = Color::from_u8(45, 45, 48, 255);
        rects.push(Rect::new(
            dialog_bounds.x,
            dialog_bounds.y,
            dialog_bounds.width,
            dialog_bounds.height,
            dialog_bg,
        ));

        // Title bar background
        let title_bar_bg = Color::from_u8(60, 60, 65, 255);
        rects.push(Rect::new(
            dialog_bounds.x,
            dialog_bounds.y,
            dialog_bounds.width,
            36.0,
            title_bar_bg,
        ));

        // Warning accent on title bar
        let warning_color = Color::from_u8(255, 180, 0, 255);
        rects.push(Rect::new(
            dialog_bounds.x,
            dialog_bounds.y,
            4.0,
            36.0,
            warning_color,
        ));

        // Buttons area
        let buttons_y = dialog_bounds.y + dialog_bounds.height - Self::DIALOG_PADDING - Self::BUTTON_HEIGHT;
        let total_button_width = Self::BUTTON_MIN_WIDTH * 3.0 + Self::BUTTON_SPACING * 2.0;
        let buttons_x = dialog_bounds.x + (dialog_bounds.width - total_button_width) / 2.0;

        // Button colors
        let save_button_bg = Color::from_u8(0, 122, 204, 255); // Blue for primary action
        let dont_save_bg = Color::from_u8(80, 80, 85, 255);    // Gray for secondary
        let cancel_bg = Color::from_u8(80, 80, 85, 255);       // Gray for cancel

        // Save button (Y/Enter)
        rects.push(Rect::new(
            buttons_x,
            buttons_y,
            Self::BUTTON_MIN_WIDTH,
            Self::BUTTON_HEIGHT,
            save_button_bg,
        ));

        // Don't Save button (N)
        rects.push(Rect::new(
            buttons_x + Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING,
            buttons_y,
            Self::BUTTON_MIN_WIDTH,
            Self::BUTTON_HEIGHT,
            dont_save_bg,
        ));

        // Cancel button (Esc)
        rects.push(Rect::new(
            buttons_x + (Self::BUTTON_MIN_WIDTH + Self::BUTTON_SPACING) * 2.0,
            buttons_y,
            Self::BUTTON_MIN_WIDTH,
            Self::BUTTON_HEIGHT,
            cancel_bg,
        ));

        rects
    }

    /// Renders the dialog background and overlay.
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
            .render(encoder, view, queue, &physical_rects, screen_width, screen_height);
    }

    /// Renders the dialog text.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.prepared {
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, pass)
                .expect("Failed to render dialog");
        }
    }
}
