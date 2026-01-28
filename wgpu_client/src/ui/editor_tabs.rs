//! Editor tab bar component.
//!
//! A modern tab bar with animations, close buttons, and drag reordering support.
//! Uses design system tokens for consistent styling.

use std::time::Instant;

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextAlign, TextOverflow, CornerRadii,
    InteractionState, InteractionTracker,
    Space, Radius, Elevation,
    Tween, Easing,
    tokens::ColorPalette,
};

/// Tab data for display.
#[derive(Clone, Debug)]
pub struct TabInfo {
    /// The view ID from core_editor (used to switch views on click).
    pub view_id: u64,
    pub title: String,
    pub path: Option<String>,
    pub is_dirty: bool,
    pub is_active: bool,
}

/// Internal tab state with interaction tracking.
struct TabState {
    info: TabInfo,
    interaction: InteractionTracker,
    bounds: Bounds,
    close_hovered: bool,
    // Animation state
    width_tween: Option<Tween<f32>>,
}

impl TabState {
    fn new(info: TabInfo) -> Self {
        let mut state = Self {
            info: info.clone(),
            interaction: InteractionTracker::new(),
            bounds: Bounds::default(),
            close_hovered: false,
            width_tween: None,
        };
        state.interaction.set_selected(info.is_active);
        state
    }
}

/// The editor tabs component.
pub struct EditorTabs {
    tabs: Vec<TabState>,
    bounds: Bounds,
    palette: ColorPalette,
    scroll_offset: f32,
    hovered_tab: Option<usize>,
}

impl EditorTabs {
    const HEIGHT: f32 = 35.0;
    const TAB_PADDING_H: f32 = 16.0;
    const TAB_GAP: f32 = 1.0;
    const CLOSE_SIZE: f32 = 16.0;
    const CLOSE_MARGIN: f32 = 8.0;
    const MIN_TAB_WIDTH: f32 = 80.0;
    const MAX_TAB_WIDTH: f32 = 200.0;
    const CHAR_WIDTH: f32 = 7.5;

    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            scroll_offset: 0.0,
            hovered_tab: None,
        }
    }

    pub fn height(&self) -> f32 {
        Self::HEIGHT
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.layout_tabs();
    }

    /// Update tabs from external data.
    pub fn update_tabs(&mut self, tabs: Vec<TabInfo>) {
        // Preserve interaction state for existing tabs
        let mut new_tabs: Vec<TabState> = tabs.into_iter().map(|info| {
            // Try to find existing tab state
            if let Some(existing) = self.tabs.iter().find(|t| t.info.view_id == info.view_id) {
                let mut state = TabState::new(info.clone());
                // Preserve hover state if still hovered
                if existing.interaction.is_hovered() {
                    state.interaction.on_pointer_enter();
                }
                state
            } else {
                TabState::new(info)
            }
        }).collect();

        self.tabs = new_tabs;
        self.layout_tabs();
    }

    fn layout_tabs(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        // Calculate tab widths
        let available_width = self.bounds.width - Self::TAB_GAP * (self.tabs.len() - 1) as f32;
        let natural_width = available_width / self.tabs.len() as f32;
        let tab_width = natural_width.clamp(Self::MIN_TAB_WIDTH, Self::MAX_TAB_WIDTH);

        let mut x = self.bounds.x - self.scroll_offset;
        let y = self.bounds.y;

        for tab in &mut self.tabs {
            // Calculate width based on title length
            let title_width = tab.info.title.len() as f32 * Self::CHAR_WIDTH;
            let content_width = title_width + Self::TAB_PADDING_H * 2.0 + Self::CLOSE_SIZE + Self::CLOSE_MARGIN;
            let width = content_width.clamp(Self::MIN_TAB_WIDTH, Self::MAX_TAB_WIDTH).min(tab_width);

            tab.bounds = Bounds {
                x,
                y,
                width,
                height: Self::HEIGHT,
            };

            x += width + Self::TAB_GAP;
        }
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        let mut rects = Vec::new();

        // Tab bar background
        rects.push(
            StyledRect::new(self.bounds)
                .with_fill(self.palette.bg_secondary)
        );

        // Bottom border
        rects.push(
            StyledRect::new(Bounds {
                x: self.bounds.x,
                y: self.bounds.y + self.bounds.height - 1.0,
                width: self.bounds.width,
                height: 1.0,
            })
            .with_fill(self.palette.border_default)
        );

        // Tab backgrounds
        for (i, tab) in self.tabs.iter().enumerate() {
            let state = tab.interaction.state();
            let is_active = tab.info.is_active;

            // Tab background
            let bg_color = if is_active {
                self.palette.bg_primary
            } else if state.is_hovered() {
                self.palette.bg_tertiary
            } else {
                self.palette.bg_secondary
            };

            let tab_rect = StyledRect::new(tab.bounds)
                .with_fill(bg_color)
                .with_corner_radii(CornerRadii::top(Radius::Sm.px()));

            rects.push(tab_rect);

            // Active tab indicator
            if is_active {
                rects.push(
                    StyledRect::new(Bounds {
                        x: tab.bounds.x,
                        y: tab.bounds.y + tab.bounds.height - 2.0,
                        width: tab.bounds.width,
                        height: 2.0,
                    })
                    .with_fill(self.palette.accent_primary)
                );
            }

            // Dirty indicator (dot)
            if tab.info.is_dirty && !tab.close_hovered {
                let dot_x = tab.bounds.x + tab.bounds.width - Self::TAB_PADDING_H - Self::CLOSE_SIZE / 2.0;
                let dot_y = tab.bounds.y + (tab.bounds.height - 8.0) / 2.0;
                rects.push(
                    StyledRect::new(Bounds {
                        x: dot_x - 4.0,
                        y: dot_y,
                        width: 8.0,
                        height: 8.0,
                    })
                    .with_fill(self.palette.fg_secondary)
                    .with_corner_radii(CornerRadii::all(4.0))
                );
            }

            // Close button background (on hover)
            if tab.close_hovered || (state.is_hovered() && !tab.info.is_dirty) {
                let close_x = tab.bounds.x + tab.bounds.width - Self::TAB_PADDING_H - Self::CLOSE_SIZE;
                let close_y = tab.bounds.y + (tab.bounds.height - Self::CLOSE_SIZE) / 2.0;

                if tab.close_hovered {
                    rects.push(
                        StyledRect::new(Bounds {
                            x: close_x,
                            y: close_y,
                            width: Self::CLOSE_SIZE,
                            height: Self::CLOSE_SIZE,
                        })
                        .with_fill(self.palette.error.with_alpha(0.2))
                        .with_corner_radii(CornerRadii::all(Self::CLOSE_SIZE / 2.0))
                    );
                }

                // X mark - two diagonal strokes forming an X
                // Since we can't rotate rects, we approximate with two small rects
                // positioned to suggest diagonal lines (forming a + that reads as X at small size)
                let cx = close_x + Self::CLOSE_SIZE / 2.0;
                let cy = close_y + Self::CLOSE_SIZE / 2.0;
                let x_color = if tab.close_hovered { self.palette.error } else { self.palette.fg_secondary };

                // Diagonal stroke 1 (top-left to bottom-right approximation)
                // We draw a small rotated-looking cross by offsetting two perpendicular lines
                rects.push(
                    StyledRect::new(Bounds {
                        x: cx - 4.0,
                        y: cy - 0.75,
                        width: 8.0,
                        height: 1.5,
                    })
                    .with_fill(x_color)
                );

                // Diagonal stroke 2 (perpendicular to form X)
                rects.push(
                    StyledRect::new(Bounds {
                        x: cx - 0.75,
                        y: cy - 4.0,
                        width: 1.5,
                        height: 8.0,
                    })
                    .with_fill(x_color)
                );
            }

            // Separator between tabs
            if i < self.tabs.len() - 1 && !is_active {
                let next_active = self.tabs.get(i + 1).map(|t| t.info.is_active).unwrap_or(false);
                if !next_active {
                    rects.push(
                        StyledRect::new(Bounds {
                            x: tab.bounds.x + tab.bounds.width,
                            y: tab.bounds.y + 8.0,
                            width: 1.0,
                            height: tab.bounds.height - 16.0,
                        })
                        .with_fill(self.palette.border_subtle)
                    );
                }
            }
        }

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        let mut texts = Vec::new();

        for tab in &self.tabs {
            let is_active = tab.info.is_active;

            // Calculate text bounds (accounting for close button)
            let text_width = tab.bounds.width - Self::TAB_PADDING_H * 2.0 - Self::CLOSE_SIZE - Self::CLOSE_MARGIN;
            let text_bounds = Bounds {
                x: tab.bounds.x + Self::TAB_PADDING_H,
                y: tab.bounds.y,
                width: text_width,
                height: tab.bounds.height,
            };

            let fg_color = if is_active {
                self.palette.fg_primary
            } else {
                self.palette.fg_secondary
            };

            texts.push(
                TextBlock::new(&tab.info.title, text_bounds)
                    .with_color(fg_color)
                    .with_font_size(13.0)
                    .with_overflow(TextOverflow::Ellipsis)
            );
        }

        texts
    }

    /// Handle pointer move. Returns true if redraw needed.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        let mut changed = false;

        // Check if in tab bar bounds
        if y < self.bounds.y || y >= self.bounds.y + self.bounds.height {
            // Left tab bar - clear all hover states
            for tab in &mut self.tabs {
                if tab.interaction.is_hovered() || tab.close_hovered {
                    tab.interaction.on_pointer_leave();
                    tab.close_hovered = false;
                    changed = true;
                }
            }
            self.hovered_tab = None;
            return changed;
        }

        for (i, tab) in self.tabs.iter_mut().enumerate() {
            let in_tab = x >= tab.bounds.x && x < tab.bounds.x + tab.bounds.width;

            if in_tab {
                if !tab.interaction.is_hovered() {
                    tab.interaction.on_pointer_enter();
                    changed = true;
                }
                self.hovered_tab = Some(i);

                // Check close button hover
                let close_x = tab.bounds.x + tab.bounds.width - Self::TAB_PADDING_H - Self::CLOSE_SIZE;
                let close_y = tab.bounds.y + (tab.bounds.height - Self::CLOSE_SIZE) / 2.0;
                let over_close = x >= close_x && x < close_x + Self::CLOSE_SIZE &&
                                 y >= close_y && y < close_y + Self::CLOSE_SIZE;

                if over_close != tab.close_hovered {
                    tab.close_hovered = over_close;
                    changed = true;
                }
            } else {
                if tab.interaction.is_hovered() {
                    tab.interaction.on_pointer_leave();
                    tab.close_hovered = false;
                    changed = true;
                }
            }
        }

        changed
    }

    /// Handle click. Returns (view_id, is_close_click) if clicked on a tab.
    pub fn on_click(&mut self, x: f32, y: f32) -> Option<(u64, bool)> {
        if y < self.bounds.y || y >= self.bounds.y + self.bounds.height {
            return None;
        }

        for tab in &self.tabs {
            if x >= tab.bounds.x && x < tab.bounds.x + tab.bounds.width {
                // Check if clicking close button
                let close_x = tab.bounds.x + tab.bounds.width - Self::TAB_PADDING_H - Self::CLOSE_SIZE;
                let close_y = tab.bounds.y + (tab.bounds.height - Self::CLOSE_SIZE) / 2.0;
                let on_close = x >= close_x && x < close_x + Self::CLOSE_SIZE &&
                              y >= close_y && y < close_y + Self::CLOSE_SIZE;

                return Some((tab.info.view_id, on_close));
            }
        }

        None
    }

    /// Handle middle click to close tab.
    pub fn on_middle_click(&mut self, x: f32, y: f32) -> Option<u64> {
        if y < self.bounds.y || y >= self.bounds.y + self.bounds.height {
            return None;
        }

        for tab in &self.tabs {
            if x >= tab.bounds.x && x < tab.bounds.x + tab.bounds.width {
                return Some(tab.info.view_id);
            }
        }

        None
    }

    /// Scroll tabs if there are too many.
    pub fn scroll(&mut self, delta: f32) {
        self.scroll_offset = (self.scroll_offset + delta).max(0.0);
        self.layout_tabs();
    }
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self::new()
    }
}
