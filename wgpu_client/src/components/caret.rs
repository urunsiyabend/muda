//! Caret (cursor) component with blinking support.

use std::time::{Duration, Instant};

use super::{Bounds, Rect, RectRenderer};
use crate::theme::{Theme, ColorRole};
use core_editor::view_model::CaretPresentation;

/// Caret blinking state and rendering.
pub struct Caret {
    /// Whether the caret is currently visible (for blinking).
    blink_visible: bool,
    /// Last time the blink state was toggled.
    last_blink: Instant,
    /// Last time there was user activity.
    last_activity: Instant,
    /// Blink interval.
    blink_rate: Duration,
    /// Caret width in pixels.
    width: f32,
    /// Rect renderer for drawing.
    rect_renderer: RectRenderer,
}

impl Caret {
    /// Default blink rate (500ms).
    const DEFAULT_BLINK_RATE: Duration = Duration::from_millis(500);
    /// Activity timeout before blinking starts.
    const ACTIVITY_TIMEOUT: Duration = Duration::from_millis(500);
    /// Default caret width.
    const DEFAULT_WIDTH: f32 = 2.0;

    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            blink_visible: true,
            last_blink: Instant::now(),
            last_activity: Instant::now(),
            blink_rate: Self::DEFAULT_BLINK_RATE,
            width: Self::DEFAULT_WIDTH,
            rect_renderer: RectRenderer::new(device, format),
        }
    }

    /// Call this when the user performs any action (typing, navigation, etc.).
    pub fn on_activity(&mut self) {
        self.last_activity = Instant::now();
        self.blink_visible = true;
        self.last_blink = Instant::now();
    }

    /// Updates the blink state. Returns true if a redraw is needed.
    pub fn update(&mut self) -> bool {
        let now = Instant::now();

        // Don't blink if there was recent activity
        if now.duration_since(self.last_activity) < Self::ACTIVITY_TIMEOUT {
            if !self.blink_visible {
                self.blink_visible = true;
                return true;
            }
            return false;
        }

        // Toggle blink state
        if now.duration_since(self.last_blink) >= self.blink_rate {
            self.blink_visible = !self.blink_visible;
            self.last_blink = now;
            return true;
        }

        false
    }

    /// Returns whether the caret should be drawn.
    pub fn is_visible(&self) -> bool {
        self.blink_visible
    }

    /// Calculates the caret rectangle from the presentation and bounds.
    pub fn calculate_rect(
        &self,
        caret: &CaretPresentation,
        bounds: Bounds,
        theme: &Theme,
        char_width: f32,
    ) -> Option<Rect> {
        if !caret.visible || !self.blink_visible {
            return None;
        }

        let line_height = theme.line_height_px();

        let x = bounds.x + (caret.position.column as f32 * char_width);
        let y = bounds.y + (caret.position.row as f32 * line_height);

        Some(Rect::new(x, y, self.width, line_height, theme.palette.get(ColorRole::Caret)))
    }

    /// Renders the caret with optional scissor clipping.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        caret: &CaretPresentation,
        bounds: Bounds,
        theme: &Theme,
        char_width: f32,
        screen_width: f32,
        screen_height: f32,
        scale_factor: f32,
        scissor: Option<(u32, u32, u32, u32)>,
    ) {
        if let Some(rect) = self.calculate_rect(caret, bounds, theme, char_width) {
            // Convert from logical to physical pixels
            let physical_rect = Rect::new(
                rect.x * scale_factor,
                rect.y * scale_factor,
                rect.width * scale_factor,
                rect.height * scale_factor,
                rect.color,
            );
            self.rect_renderer.render(encoder, view, queue, &[physical_rect], screen_width, screen_height, scissor);
        }
    }

    /// Returns the time until the next blink toggle.
    pub fn time_until_next_blink(&self) -> Duration {
        let now = Instant::now();

        // If recent activity, return the activity timeout
        let since_activity = now.duration_since(self.last_activity);
        if since_activity < Self::ACTIVITY_TIMEOUT {
            return Self::ACTIVITY_TIMEOUT - since_activity;
        }

        // Otherwise return time until next blink
        let since_blink = now.duration_since(self.last_blink);
        if since_blink >= self.blink_rate {
            Duration::ZERO
        } else {
            self.blink_rate - since_blink
        }
    }
}
