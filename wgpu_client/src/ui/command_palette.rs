//! Command palette for quick access to IDE commands.
//!
//! A searchable overlay that provides access to all commands,
//! files, and settings with keyboard-driven navigation.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextAlign, TextOverflow, CornerRadii,
    InteractionState, InteractionTracker,
    Space, Radius, Elevation,
    Tween, Easing,
    tokens::ColorPalette,
};

/// Command entry types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandKind {
    /// Editor command
    Command,
    /// File to open
    File,
    /// Settings
    Setting,
    /// Recent file
    Recent,
}

/// A command palette entry.
#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub shortcut: Option<String>,
    pub kind: CommandKind,
    pub score: i32, // For fuzzy matching
}

impl CommandEntry {
    pub fn command(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            shortcut: None,
            kind: CommandKind::Command,
            score: 0,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Entry state with interaction tracking.
struct EntryState {
    entry: CommandEntry,
    interaction: InteractionTracker,
    bounds: Bounds,
}

/// The command palette component.
pub struct CommandPalette {
    entries: Vec<EntryState>,
    filtered_entries: Vec<usize>, // Indices into entries
    bounds: Bounds,
    palette: ColorPalette,
    visible: bool,
    query: String,
    selected_index: usize,
    scroll_offset: f32,

    // Animation
    opacity_tween: Tween<f32>,
    scale_tween: Tween<f32>,
}

impl CommandPalette {
    const WIDTH: f32 = 600.0;
    const MAX_HEIGHT: f32 = 400.0;
    const INPUT_HEIGHT: f32 = 44.0;
    const ITEM_HEIGHT: f32 = 36.0;
    const PADDING: f32 = 8.0;
    const MAX_VISIBLE_ITEMS: usize = 10;

    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filtered_entries: Vec::new(),
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            visible: false,
            query: String::new(),
            selected_index: 0,
            scroll_offset: 0.0,
            opacity_tween: Tween::instant(0.0),
            scale_tween: Tween::instant(0.95),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the command palette.
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0.0;
        self.filter_entries();

        // Animate in
        self.opacity_tween = Tween::new(0.0, 1.0, std::time::Duration::from_millis(150), Easing::EaseOut);
        self.opacity_tween.start();
        self.scale_tween = Tween::new(0.95, 1.0, std::time::Duration::from_millis(150), Easing::EaseOut);
        self.scale_tween.start();
    }

    /// Hide the command palette.
    pub fn hide(&mut self) {
        self.visible = false;
        self.opacity_tween.finish();
        self.scale_tween.finish();
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Register commands.
    pub fn register_commands(&mut self, commands: Vec<CommandEntry>) {
        self.entries = commands.into_iter().map(|e| EntryState {
            entry: e,
            interaction: InteractionTracker::new(),
            bounds: Bounds::default(),
        }).collect();
        self.filter_entries();
    }

    /// Set the search query.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.selected_index = 0;
        self.scroll_offset = 0.0;
        self.filter_entries();
    }

    /// Add character to query.
    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.selected_index = 0;
        self.filter_entries();
    }

    /// Backspace in query.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.filter_entries();
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.filtered_entries.len() {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    /// Get the currently selected entry.
    pub fn selected_entry(&self) -> Option<&CommandEntry> {
        self.filtered_entries.get(self.selected_index)
            .and_then(|&idx| self.entries.get(idx))
            .map(|e| &e.entry)
    }

    /// Execute the selected entry and hide.
    pub fn execute(&mut self) -> Option<CommandEntry> {
        let entry = self.selected_entry().cloned();
        self.hide();
        entry
    }

    fn filter_entries(&mut self) {
        if self.query.is_empty() {
            self.filtered_entries = (0..self.entries.len()).collect();
        } else {
            let query_lower = self.query.to_lowercase();
            self.filtered_entries = self.entries.iter().enumerate()
                .filter(|(_, e)| {
                    e.entry.label.to_lowercase().contains(&query_lower) ||
                    e.entry.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Update selection bounds
        if self.selected_index >= self.filtered_entries.len() && !self.filtered_entries.is_empty() {
            self.selected_index = self.filtered_entries.len() - 1;
        }
    }

    fn ensure_visible(&mut self) {
        let item_top = self.selected_index as f32 * Self::ITEM_HEIGHT;
        let item_bottom = item_top + Self::ITEM_HEIGHT;
        let visible_height = Self::MAX_VISIBLE_ITEMS as f32 * Self::ITEM_HEIGHT;

        if item_top < self.scroll_offset {
            self.scroll_offset = item_top;
        } else if item_bottom > self.scroll_offset + visible_height {
            self.scroll_offset = item_bottom - visible_height;
        }
    }

    /// Calculate bounds centered on screen.
    pub fn calculate_bounds(&mut self, screen_width: f32, screen_height: f32) {
        let items_height = (self.filtered_entries.len().min(Self::MAX_VISIBLE_ITEMS) as f32 * Self::ITEM_HEIGHT).min(Self::MAX_HEIGHT - Self::INPUT_HEIGHT);
        let height = Self::INPUT_HEIGHT + items_height + Self::PADDING * 2.0;

        let x = (screen_width - Self::WIDTH) / 2.0;
        let y = screen_height * 0.15; // Near top of screen

        self.bounds = Bounds {
            x,
            y,
            width: Self::WIDTH,
            height,
        };

        // Layout entry bounds
        let content_y = self.bounds.y + Self::INPUT_HEIGHT + Self::PADDING - self.scroll_offset;
        for (i, &entry_idx) in self.filtered_entries.iter().take(Self::MAX_VISIBLE_ITEMS * 2).enumerate() {
            if let Some(entry) = self.entries.get_mut(entry_idx) {
                entry.bounds = Bounds {
                    x: self.bounds.x + Self::PADDING,
                    y: content_y + i as f32 * Self::ITEM_HEIGHT,
                    width: self.bounds.width - Self::PADDING * 2.0,
                    height: Self::ITEM_HEIGHT,
                };
            }
        }
    }

    /// Update animations.
    pub fn update(&mut self) -> bool {
        let o = self.opacity_tween.update();
        let s = self.scale_tween.update();
        o || s
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        if !self.visible {
            return Vec::new();
        }

        let mut rects = Vec::new();
        let opacity = *self.opacity_tween.value();
        let scale = *self.scale_tween.value();

        // Scale from center
        let scaled_width = self.bounds.width * scale;
        let scaled_height = self.bounds.height * scale;
        let scaled_x = self.bounds.x + (self.bounds.width - scaled_width) / 2.0;
        let scaled_y = self.bounds.y + (self.bounds.height - scaled_height) / 2.0;

        let scaled_bounds = Bounds {
            x: scaled_x,
            y: scaled_y,
            width: scaled_width,
            height: scaled_height,
        };

        // Backdrop
        rects.push(
            StyledRect::new(Bounds::new(0.0, 0.0, 10000.0, 10000.0))
                .with_fill(self.palette.bg_overlay.with_alpha(0.5 * opacity))
        );

        // Main container with shadow
        rects.push(
            StyledRect::new(scaled_bounds)
                .with_fill(self.palette.bg_elevated.with_alpha(opacity))
                .with_border(1.0, self.palette.border_default.with_alpha(opacity))
                .with_corner_radii(CornerRadii::all(Radius::Lg.px()))
                .with_elevation(Elevation::Highest)
        );

        // Input area background
        let input_bounds = Bounds {
            x: scaled_bounds.x + Self::PADDING,
            y: scaled_bounds.y + Self::PADDING,
            width: scaled_bounds.width - Self::PADDING * 2.0,
            height: Self::INPUT_HEIGHT - Self::PADDING,
        };

        rects.push(
            StyledRect::new(input_bounds)
                .with_fill(self.palette.bg_tertiary.with_alpha(opacity))
                .with_border(1.0, self.palette.border_focus.with_alpha(opacity))
                .with_corner_radii(CornerRadii::all(Radius::Sm.px()))
        );

        // Cursor in input
        let cursor_x = input_bounds.x + 12.0 + self.query.len() as f32 * 8.0;
        rects.push(
            StyledRect::new(Bounds {
                x: cursor_x,
                y: input_bounds.y + 8.0,
                width: 2.0,
                height: input_bounds.height - 16.0,
            })
            .with_fill(self.palette.fg_primary.with_alpha(opacity))
        );

        // Entry backgrounds
        let content_y = scaled_bounds.y + Self::INPUT_HEIGHT;
        for (i, &entry_idx) in self.filtered_entries.iter().enumerate().take(Self::MAX_VISIBLE_ITEMS) {
            let y = content_y + i as f32 * Self::ITEM_HEIGHT - self.scroll_offset;

            // Skip entries outside visible area
            if y + Self::ITEM_HEIGHT < content_y || y > scaled_bounds.y + scaled_bounds.height {
                continue;
            }

            let is_selected = i == self.selected_index;

            if is_selected {
                rects.push(
                    StyledRect::new(Bounds {
                        x: scaled_bounds.x + Self::PADDING,
                        y,
                        width: scaled_bounds.width - Self::PADDING * 2.0,
                        height: Self::ITEM_HEIGHT,
                    })
                    .with_fill(self.palette.selection.with_alpha(opacity))
                    .with_corner_radii(CornerRadii::all(Radius::Sm.px()))
                );
            }
        }

        // Separator between input and list
        rects.push(
            StyledRect::new(Bounds {
                x: scaled_bounds.x + Self::PADDING,
                y: content_y - 1.0,
                width: scaled_bounds.width - Self::PADDING * 2.0,
                height: 1.0,
            })
            .with_fill(self.palette.border_subtle.with_alpha(opacity))
        );

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        if !self.visible {
            return Vec::new();
        }

        let mut texts = Vec::new();
        let opacity = *self.opacity_tween.value();
        let scale = *self.scale_tween.value();

        let scaled_width = self.bounds.width * scale;
        let scaled_x = self.bounds.x + (self.bounds.width - scaled_width) / 2.0;
        let scaled_y = self.bounds.y;

        // Query text or placeholder
        let input_text = if self.query.is_empty() {
            "Type to search commands..."
        } else {
            &self.query
        };

        let text_color = if self.query.is_empty() {
            self.palette.fg_muted
        } else {
            self.palette.fg_primary
        };

        texts.push(
            TextBlock::new(input_text, Bounds {
                x: scaled_x + Self::PADDING + 12.0,
                y: scaled_y + Self::PADDING,
                width: scaled_width - Self::PADDING * 2.0 - 24.0,
                height: Self::INPUT_HEIGHT - Self::PADDING,
            })
            .with_color(text_color.with_alpha(opacity))
            .with_font_size(14.0)
        );

        // Entry texts
        let content_y = scaled_y + Self::INPUT_HEIGHT;
        for (i, &entry_idx) in self.filtered_entries.iter().enumerate().take(Self::MAX_VISIBLE_ITEMS) {
            let y = content_y + i as f32 * Self::ITEM_HEIGHT - self.scroll_offset;

            if y + Self::ITEM_HEIGHT < content_y || y > self.bounds.y + self.bounds.height * scale {
                continue;
            }

            let entry = &self.entries[entry_idx].entry;
            let is_selected = i == self.selected_index;

            let fg_color = if is_selected {
                self.palette.fg_primary
            } else {
                self.palette.fg_secondary
            };

            // Label
            texts.push(
                TextBlock::new(&entry.label, Bounds {
                    x: scaled_x + Self::PADDING + 12.0,
                    y,
                    width: scaled_width * 0.5,
                    height: Self::ITEM_HEIGHT,
                })
                .with_color(fg_color.with_alpha(opacity))
                .with_font_size(13.0)
                .with_overflow(TextOverflow::Ellipsis)
            );

            // Shortcut (right-aligned)
            if let Some(shortcut) = &entry.shortcut {
                texts.push(
                    TextBlock::new(shortcut, Bounds {
                        x: scaled_x + scaled_width - Self::PADDING - 100.0,
                        y,
                        width: 88.0,
                        height: Self::ITEM_HEIGHT,
                    })
                    .with_color(self.palette.fg_muted.with_alpha(opacity))
                    .with_font_size(12.0)
                    .with_align(TextAlign::Right)
                );
            }
        }

        texts
    }

    /// Handle pointer move.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }

        // Update hover state on entries
        let content_y = self.bounds.y + Self::INPUT_HEIGHT;
        for (i, &entry_idx) in self.filtered_entries.iter().enumerate().take(Self::MAX_VISIBLE_ITEMS) {
            let item_y = content_y + i as f32 * Self::ITEM_HEIGHT - self.scroll_offset;

            if y >= item_y && y < item_y + Self::ITEM_HEIGHT &&
               x >= self.bounds.x && x < self.bounds.x + self.bounds.width {
                if self.selected_index != i {
                    self.selected_index = i;
                    return true;
                }
            }
        }

        false
    }

    /// Handle click.
    pub fn on_click(&mut self, x: f32, y: f32) -> Option<CommandEntry> {
        if !self.visible {
            return None;
        }

        // Check if clicking outside - close
        if x < self.bounds.x || x > self.bounds.x + self.bounds.width ||
           y < self.bounds.y || y > self.bounds.y + self.bounds.height {
            self.hide();
            return None;
        }

        // Check if clicking on an entry
        let content_y = self.bounds.y + Self::INPUT_HEIGHT;
        if y >= content_y {
            return self.execute();
        }

        None
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
