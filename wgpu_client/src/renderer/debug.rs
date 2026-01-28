//! Debug overlay rendering for development.
//!
//! This module is only compiled in debug builds (#[cfg(debug_assertions)]).
//! It provides visualization of component bounds, scissor regions, and hit areas.

use crate::components::Bounds;
use crate::theme::Color;

/// Debug overlay colors for different component types.
pub mod colors {
    use crate::theme::Color;

    pub const SIDEBAR: Color = Color::from_u8(255, 100, 100, 128);    // Red
    pub const TAB_BAR: Color = Color::from_u8(100, 255, 100, 128);    // Green
    pub const TEXT_AREA: Color = Color::from_u8(100, 100, 255, 128);  // Blue
    pub const GUTTER: Color = Color::from_u8(255, 255, 100, 128);     // Yellow
    pub const STATUS_BAR: Color = Color::from_u8(255, 100, 255, 128); // Magenta
    pub const PANEL: Color = Color::from_u8(100, 255, 255, 128);      // Cyan
    pub const SCISSOR: Color = Color::from_u8(255, 165, 0, 200);      // Orange (brighter)
}

/// Represents a simple rectangle for debug overlay rendering.
pub struct DebugRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

impl DebugRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Color) -> Self {
        Self { x, y, width, height, color }
    }
}

/// Builds border rectangles for visualizing a component's bounds.
/// Returns 4 rectangles forming a border around the given bounds.
pub fn build_bounds_border(bounds: Bounds, color: Color, thickness: f32) -> Vec<DebugRect> {
    vec![
        // Top border
        DebugRect::new(bounds.x, bounds.y, bounds.width, thickness, color),
        // Right border
        DebugRect::new(bounds.x + bounds.width - thickness, bounds.y, thickness, bounds.height, color),
        // Bottom border
        DebugRect::new(bounds.x, bounds.y + bounds.height - thickness, bounds.width, thickness, color),
        // Left border
        DebugRect::new(bounds.x, bounds.y, thickness, bounds.height, color),
    ]
}

/// Configuration for what debug overlays to show.
#[derive(Clone, Copy)]
pub struct DebugOverlayConfig {
    pub show_bounds: bool,
    pub show_scissor: bool,
    pub show_hit_areas: bool,
    pub border_thickness: f32,
}

impl Default for DebugOverlayConfig {
    fn default() -> Self {
        Self {
            show_bounds: true,
            show_scissor: false,  // Scissor overlay can be noisy
            show_hit_areas: false,
            border_thickness: 2.0,
        }
    }
}

/// Debug overlay state for the renderer.
pub struct DebugOverlay {
    pub config: DebugOverlayConfig,
    pub enabled: bool,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            config: DebugOverlayConfig::default(),
            enabled: false, // Off by default, toggle with key
        }
    }

    /// Toggle debug overlay visibility.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        if self.enabled {
            log::info!("Debug overlay ENABLED");
        } else {
            log::info!("Debug overlay DISABLED");
        }
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}
