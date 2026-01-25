//! Split view component for editor panes.
//!
//! Supports horizontal and vertical splits with draggable dividers
//! for resizing panes.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, CornerRadii,
    tokens::ColorPalette,
};

/// Split direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitDirection {
    #[default]
    Horizontal, // Side by side
    Vertical,   // Top and bottom
}

/// A split pane.
#[derive(Clone, Debug)]
pub struct SplitPane {
    pub id: usize,
    pub ratio: f32, // 0.0 to 1.0, proportion of space
    pub min_size: f32,
}

impl SplitPane {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            ratio: 0.5,
            min_size: 100.0,
        }
    }

    pub fn with_ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }
}

/// A split view managing multiple panes.
pub struct SplitView {
    direction: SplitDirection,
    panes: Vec<SplitPane>,
    pane_bounds: Vec<Bounds>,
    divider_bounds: Vec<Bounds>,
    bounds: Bounds,
    palette: ColorPalette,
    active_divider: Option<usize>,
    divider_hovered: Option<usize>,
    drag_start: f32,
    drag_start_ratios: Vec<f32>,
}

impl SplitView {
    const DIVIDER_SIZE: f32 = 6.0;
    const DIVIDER_VISIBLE_SIZE: f32 = 2.0;

    pub fn new(direction: SplitDirection) -> Self {
        Self {
            direction,
            panes: vec![SplitPane::new(0)],
            pane_bounds: Vec::new(),
            divider_bounds: Vec::new(),
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            active_divider: None,
            divider_hovered: None,
            drag_start: 0.0,
            drag_start_ratios: Vec::new(),
        }
    }

    pub fn horizontal() -> Self {
        Self::new(SplitDirection::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::new(SplitDirection::Vertical)
    }

    /// Add a pane with the given ratio.
    pub fn add_pane(&mut self, ratio: f32) -> usize {
        let id = self.panes.len();
        self.panes.push(SplitPane::new(id).with_ratio(ratio));
        self.normalize_ratios();
        id
    }

    /// Split an existing pane.
    pub fn split(&mut self, pane_id: usize) -> Option<usize> {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == pane_id) {
            let half_ratio = pane.ratio / 2.0;
            pane.ratio = half_ratio;

            let new_id = self.panes.len();
            self.panes.push(SplitPane::new(new_id).with_ratio(half_ratio));
            return Some(new_id);
        }
        None
    }

    /// Remove a pane.
    pub fn remove_pane(&mut self, pane_id: usize) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }

        if let Some(idx) = self.panes.iter().position(|p| p.id == pane_id) {
            let ratio = self.panes[idx].ratio;
            self.panes.remove(idx);

            // Distribute ratio to remaining panes
            if !self.panes.is_empty() {
                let extra = ratio / self.panes.len() as f32;
                for pane in &mut self.panes {
                    pane.ratio += extra;
                }
            }
            self.normalize_ratios();
            return true;
        }
        false
    }

    fn normalize_ratios(&mut self) {
        let total: f32 = self.panes.iter().map(|p| p.ratio).sum();
        if total > 0.0 {
            for pane in &mut self.panes {
                pane.ratio /= total;
            }
        }
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.calculate_pane_bounds();
    }

    fn calculate_pane_bounds(&mut self) {
        self.pane_bounds.clear();
        self.divider_bounds.clear();

        if self.panes.is_empty() {
            return;
        }

        let available_size = if self.direction == SplitDirection::Horizontal {
            self.bounds.width - Self::DIVIDER_SIZE * (self.panes.len() - 1) as f32
        } else {
            self.bounds.height - Self::DIVIDER_SIZE * (self.panes.len() - 1) as f32
        };

        let mut pos = if self.direction == SplitDirection::Horizontal {
            self.bounds.x
        } else {
            self.bounds.y
        };

        for (i, pane) in self.panes.iter().enumerate() {
            let size = (available_size * pane.ratio).max(pane.min_size);

            let pane_bounds = if self.direction == SplitDirection::Horizontal {
                Bounds {
                    x: pos,
                    y: self.bounds.y,
                    width: size,
                    height: self.bounds.height,
                }
            } else {
                Bounds {
                    x: self.bounds.x,
                    y: pos,
                    width: self.bounds.width,
                    height: size,
                }
            };

            self.pane_bounds.push(pane_bounds);
            pos += size;

            // Add divider after each pane except the last
            if i < self.panes.len() - 1 {
                let divider_bounds = if self.direction == SplitDirection::Horizontal {
                    Bounds {
                        x: pos,
                        y: self.bounds.y,
                        width: Self::DIVIDER_SIZE,
                        height: self.bounds.height,
                    }
                } else {
                    Bounds {
                        x: self.bounds.x,
                        y: pos,
                        width: self.bounds.width,
                        height: Self::DIVIDER_SIZE,
                    }
                };

                self.divider_bounds.push(divider_bounds);
                pos += Self::DIVIDER_SIZE;
            }
        }
    }

    /// Get bounds for a specific pane.
    pub fn pane_bounds(&self, pane_id: usize) -> Option<Bounds> {
        self.panes.iter().position(|p| p.id == pane_id)
            .and_then(|idx| self.pane_bounds.get(idx).copied())
    }

    /// Get all pane bounds.
    pub fn all_pane_bounds(&self) -> &[Bounds] {
        &self.pane_bounds
    }

    /// Get pane count.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        let mut rects = Vec::new();

        // Dividers
        for (i, divider) in self.divider_bounds.iter().enumerate() {
            let is_hovered = self.divider_hovered == Some(i);
            let is_active = self.active_divider == Some(i);

            // Divider hit area (invisible)
            // Visible divider line
            let visible_bounds = if self.direction == SplitDirection::Horizontal {
                Bounds {
                    x: divider.x + (Self::DIVIDER_SIZE - Self::DIVIDER_VISIBLE_SIZE) / 2.0,
                    y: divider.y,
                    width: Self::DIVIDER_VISIBLE_SIZE,
                    height: divider.height,
                }
            } else {
                Bounds {
                    x: divider.x,
                    y: divider.y + (Self::DIVIDER_SIZE - Self::DIVIDER_VISIBLE_SIZE) / 2.0,
                    width: divider.width,
                    height: Self::DIVIDER_VISIBLE_SIZE,
                }
            };

            let color = if is_active {
                self.palette.accent_primary
            } else if is_hovered {
                self.palette.accent_secondary
            } else {
                self.palette.border_default
            };

            rects.push(
                StyledRect::new(visible_bounds)
                    .with_fill(color)
            );
        }

        rects
    }

    /// Check if point is on a divider. Returns divider index.
    pub fn divider_at(&self, x: f32, y: f32) -> Option<usize> {
        for (i, divider) in self.divider_bounds.iter().enumerate() {
            if x >= divider.x && x < divider.x + divider.width &&
               y >= divider.y && y < divider.y + divider.height {
                return Some(i);
            }
        }
        None
    }

    /// Handle pointer move.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        // If dragging, resize panes
        if let Some(divider_idx) = self.active_divider {
            let delta = if self.direction == SplitDirection::Horizontal {
                x - self.drag_start
            } else {
                y - self.drag_start
            };

            let total_size = if self.direction == SplitDirection::Horizontal {
                self.bounds.width
            } else {
                self.bounds.height
            };

            let ratio_delta = delta / total_size;

            // Apply delta to the panes adjacent to the divider
            if divider_idx < self.panes.len() - 1 {
                let new_ratio1 = (self.drag_start_ratios[divider_idx] + ratio_delta)
                    .clamp(0.1, 0.9);
                let new_ratio2 = (self.drag_start_ratios[divider_idx + 1] - ratio_delta)
                    .clamp(0.1, 0.9);

                self.panes[divider_idx].ratio = new_ratio1;
                self.panes[divider_idx + 1].ratio = new_ratio2;
                self.calculate_pane_bounds();
            }

            return true;
        }

        // Update hover state
        let new_hover = self.divider_at(x, y);
        if new_hover != self.divider_hovered {
            self.divider_hovered = new_hover;
            return true;
        }

        false
    }

    /// Check if on a divider for cursor change.
    pub fn is_on_divider(&self, x: f32, y: f32) -> bool {
        self.divider_at(x, y).is_some()
    }

    /// Start dragging a divider.
    pub fn start_resize(&mut self, x: f32, y: f32) -> bool {
        if let Some(idx) = self.divider_at(x, y) {
            self.active_divider = Some(idx);
            self.drag_start = if self.direction == SplitDirection::Horizontal { x } else { y };
            self.drag_start_ratios = self.panes.iter().map(|p| p.ratio).collect();
            return true;
        }
        false
    }

    /// Stop dragging.
    pub fn stop_resize(&mut self) {
        self.active_divider = None;
        self.drag_start_ratios.clear();
    }

    /// Check if currently resizing.
    pub fn is_resizing(&self) -> bool {
        self.active_divider.is_some()
    }

    /// Get the split direction.
    pub fn direction(&self) -> SplitDirection {
        self.direction
    }

    /// Set the split direction.
    pub fn set_direction(&mut self, direction: SplitDirection) {
        self.direction = direction;
        self.calculate_pane_bounds();
    }
}

impl Default for SplitView {
    fn default() -> Self {
        Self::horizontal()
    }
}
