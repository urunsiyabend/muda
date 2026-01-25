//! Widget rendering integration.
//!
//! This module provides utilities for rendering widgets to the GPU,
//! bridging the widget system with the styled rectangle renderer.

use crate::components::Bounds;
use crate::design_system::{StyledRect, StyledRectRenderer, TextBlock};
use crate::widgets::{Widget, WidgetOutput};

/// Collects render output from widgets and prepares it for GPU rendering.
pub struct WidgetRenderCollector {
    /// Collected styled rectangles
    pub rects: Vec<StyledRect>,
    /// Collected text blocks
    pub texts: Vec<TextBlock>,
    /// Screen dimensions for coordinate conversion
    screen_width: f32,
    screen_height: f32,
}

impl WidgetRenderCollector {
    /// Create a new render collector.
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            rects: Vec::new(),
            texts: Vec::new(),
            screen_width,
            screen_height,
        }
    }

    /// Clear all collected output.
    pub fn clear(&mut self) {
        self.rects.clear();
        self.texts.clear();
    }

    /// Update screen dimensions.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Collect render output from a widget.
    pub fn collect(&mut self, output: WidgetOutput) {
        self.rects.extend(output.rects);
        self.texts.extend(output.texts);
    }

    /// Collect render output from multiple widgets.
    pub fn collect_all<W: Widget>(&mut self, widgets: &mut [W]) {
        for widget in widgets {
            self.collect(widget.build());
        }
    }

    /// Push all collected rectangles to a StyledRectRenderer.
    pub fn prepare_rects(&self, renderer: &mut StyledRectRenderer) {
        renderer.push_all(&self.rects, self.screen_width, self.screen_height);
    }

    /// Get collected text blocks for rendering with glyphon.
    pub fn texts(&self) -> &[TextBlock] {
        &self.texts
    }

    /// Get the number of rectangles collected.
    pub fn rect_count(&self) -> usize {
        self.rects.len()
    }

    /// Get the number of text blocks collected.
    pub fn text_count(&self) -> usize {
        self.texts.len()
    }
}

/// Example usage demonstrating widget rendering.
///
/// This shows how to set up and render widgets with the new design system.
///
/// ```rust,ignore
/// use wgpu_client::widgets::{Button, ButtonProps, Toggle, Input};
/// use wgpu_client::design_system::{StyledRectRenderer, LayoutConstraints};
/// use wgpu_client::widgets::widget_renderer::WidgetRenderCollector;
///
/// // In your renderer initialization:
/// let styled_rect_renderer = StyledRectRenderer::new(&device, surface_format);
/// let mut collector = WidgetRenderCollector::new(800.0, 600.0);
///
/// // Create widgets
/// let mut button = Button::primary("Save");
/// let mut toggle = Toggle::new(false);
/// let mut input = Input::with_placeholder("Enter text...");
///
/// // Layout widgets
/// let constraints = LayoutConstraints::loose(200.0, 50.0);
/// let (w, h) = button.layout(constraints);
/// button.set_bounds(Bounds { x: 10.0, y: 10.0, width: w, height: h });
///
/// // Update widgets (for animations)
/// let dt = 1.0 / 60.0; // delta time in seconds
/// button.update(dt);
/// toggle.update(dt);
///
/// // Build and collect render output
/// collector.clear();
/// collector.collect(button.build());
/// collector.collect(toggle.build());
/// collector.collect(input.build());
///
/// // Prepare for GPU rendering
/// styled_rect_renderer.clear();
/// collector.prepare_rects(&mut styled_rect_renderer);
/// styled_rect_renderer.prepare(&queue);
///
/// // In your render pass:
/// styled_rect_renderer.render(&mut render_pass);
///
/// // For text, use glyphon with the collected TextBlocks
/// for text_block in collector.texts() {
///     // Configure glyphon buffer with text_block properties
/// }
/// ```
#[allow(dead_code)]
fn _example_usage() {}
