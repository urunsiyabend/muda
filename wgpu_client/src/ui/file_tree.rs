//! File tree component for the sidebar.
//!
//! A modern file explorer with expand/collapse animations,
//! file icons, and keyboard navigation.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextOverflow, CornerRadii,
    InteractionState, InteractionTracker,
    Space, Radius,
    tokens::ColorPalette,
};

/// File tree entry data.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub id: usize,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: u8,
    pub is_selected: bool,
}

/// Internal entry state with interaction tracking.
struct EntryState {
    entry: FileEntry,
    interaction: InteractionTracker,
    bounds: Bounds,
    chevron_hovered: bool,
}

impl EntryState {
    fn new(entry: FileEntry) -> Self {
        let mut state = Self {
            entry: entry.clone(),
            interaction: InteractionTracker::new(),
            bounds: Bounds::default(),
            chevron_hovered: false,
        };
        state.interaction.set_selected(entry.is_selected);
        state
    }
}

/// The file tree component.
pub struct FileTree {
    entries: Vec<EntryState>,
    bounds: Bounds,
    palette: ColorPalette,
    scroll_offset: f32,
    focused: bool,
    hovered_entry: Option<usize>,
}

impl FileTree {
    const ITEM_HEIGHT: f32 = 24.0;
    const INDENT_WIDTH: f32 = 16.0;
    const CHEVRON_SIZE: f32 = 16.0;
    const ICON_SIZE: f32 = 16.0;
    const ICON_GAP: f32 = 6.0;
    const PADDING_H: f32 = 8.0;
    const PADDING_V: f32 = 4.0;

    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            scroll_offset: 0.0,
            focused: false,
            hovered_entry: None,
        }
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.layout_entries();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Update entries from external data.
    pub fn update_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries.into_iter().map(EntryState::new).collect();
        self.layout_entries();
    }

    fn layout_entries(&mut self) {
        let start_y = self.bounds.y + Self::PADDING_V - self.scroll_offset;

        for (i, entry) in self.entries.iter_mut().enumerate() {
            let y = start_y + i as f32 * Self::ITEM_HEIGHT;
            entry.bounds = Bounds {
                x: self.bounds.x,
                y,
                width: self.bounds.width,
                height: Self::ITEM_HEIGHT,
            };
        }
    }

    /// Get visible entry count.
    pub fn visible_count(&self) -> usize {
        ((self.bounds.height - Self::PADDING_V * 2.0) / Self::ITEM_HEIGHT) as usize
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        let mut rects = Vec::new();

        // Background
        rects.push(
            StyledRect::new(self.bounds)
                .with_fill(self.palette.bg_secondary)
        );

        // Right border
        rects.push(
            StyledRect::new(Bounds {
                x: self.bounds.x + self.bounds.width - 1.0,
                y: self.bounds.y,
                width: 1.0,
                height: self.bounds.height,
            })
            .with_fill(self.palette.border_default)
        );

        // Entry backgrounds
        for entry in &self.entries {
            // Skip entries outside visible area
            if entry.bounds.y + entry.bounds.height < self.bounds.y ||
               entry.bounds.y > self.bounds.y + self.bounds.height {
                continue;
            }

            let is_selected = entry.entry.is_selected;
            let is_hovered = entry.interaction.is_hovered();

            // Selection/hover background
            let bg_color = if is_selected {
                if self.focused {
                    self.palette.selection
                } else {
                    self.palette.selection.with_alpha(0.5)
                }
            } else if is_hovered {
                self.palette.interactive_hover.with_alpha(0.5)
            } else {
                Color::TRANSPARENT
            };

            if bg_color.a > 0.0 {
                rects.push(
                    StyledRect::new(Bounds {
                        x: entry.bounds.x + 2.0,
                        y: entry.bounds.y,
                        width: entry.bounds.width - 4.0,
                        height: entry.bounds.height,
                    })
                    .with_fill(bg_color)
                    .with_corner_radii(CornerRadii::all(Radius::Xs.px()))
                );
            }

            let indent = entry.entry.depth as f32 * Self::INDENT_WIDTH;
            let content_x = entry.bounds.x + Self::PADDING_H + indent;

            // Chevron (for directories)
            if entry.entry.is_dir {
                let chevron_y = entry.bounds.y + (entry.bounds.height - Self::CHEVRON_SIZE) / 2.0;

                // Chevron hover background
                if entry.chevron_hovered {
                    rects.push(
                        StyledRect::new(Bounds {
                            x: content_x,
                            y: chevron_y,
                            width: Self::CHEVRON_SIZE,
                            height: Self::CHEVRON_SIZE,
                        })
                        .with_fill(self.palette.interactive_hover)
                        .with_corner_radii(CornerRadii::all(2.0))
                    );
                }

                // Chevron arrow
                let arrow_color = self.palette.fg_secondary;
                let cx = content_x + Self::CHEVRON_SIZE / 2.0;
                let cy = entry.bounds.y + entry.bounds.height / 2.0;

                if entry.entry.is_expanded {
                    // Down arrow
                    rects.push(
                        StyledRect::new(Bounds {
                            x: cx - 3.0,
                            y: cy - 1.0,
                            width: 6.0,
                            height: 2.0,
                        })
                        .with_fill(arrow_color)
                    );
                } else {
                    // Right arrow
                    rects.push(
                        StyledRect::new(Bounds {
                            x: cx - 1.0,
                            y: cy - 3.0,
                            width: 2.0,
                            height: 6.0,
                        })
                        .with_fill(arrow_color)
                    );
                }
            }

            // File/folder icon
            let icon_x = content_x + Self::CHEVRON_SIZE + 2.0;
            let icon_y = entry.bounds.y + (entry.bounds.height - Self::ICON_SIZE) / 2.0;

            let icon_color = if entry.entry.is_dir {
                self.palette.warning // Yellow for folders
            } else {
                self.palette.fg_secondary
            };

            rects.push(
                StyledRect::new(Bounds {
                    x: icon_x,
                    y: icon_y + 2.0,
                    width: Self::ICON_SIZE - 4.0,
                    height: Self::ICON_SIZE - 4.0,
                })
                .with_fill(icon_color)
                .with_corner_radii(CornerRadii::all(2.0))
            );
        }

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        let mut texts = Vec::new();

        for entry in &self.entries {
            // Skip entries outside visible area
            if entry.bounds.y + entry.bounds.height < self.bounds.y ||
               entry.bounds.y > self.bounds.y + self.bounds.height {
                continue;
            }

            let indent = entry.entry.depth as f32 * Self::INDENT_WIDTH;
            let label_x = entry.bounds.x + Self::PADDING_H + indent + Self::CHEVRON_SIZE + 2.0 + Self::ICON_SIZE + Self::ICON_GAP;
            let label_width = entry.bounds.width - label_x + entry.bounds.x - Self::PADDING_H;

            let fg_color = if entry.entry.is_selected {
                self.palette.fg_primary
            } else {
                self.palette.fg_primary
            };

            texts.push(
                TextBlock::new(&entry.entry.name, Bounds {
                    x: label_x,
                    y: entry.bounds.y,
                    width: label_width,
                    height: entry.bounds.height,
                })
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

        // Check if in tree bounds
        if x < self.bounds.x || x >= self.bounds.x + self.bounds.width ||
           y < self.bounds.y || y >= self.bounds.y + self.bounds.height {
            for entry in &mut self.entries {
                if entry.interaction.is_hovered() || entry.chevron_hovered {
                    entry.interaction.on_pointer_leave();
                    entry.chevron_hovered = false;
                    changed = true;
                }
            }
            self.hovered_entry = None;
            return changed;
        }

        for (i, entry) in self.entries.iter_mut().enumerate() {
            let in_entry = y >= entry.bounds.y && y < entry.bounds.y + entry.bounds.height;

            if in_entry {
                if !entry.interaction.is_hovered() {
                    entry.interaction.on_pointer_enter();
                    changed = true;
                }
                self.hovered_entry = Some(i);

                // Check chevron hover
                if entry.entry.is_dir {
                    let indent = entry.entry.depth as f32 * Self::INDENT_WIDTH;
                    let chevron_x = entry.bounds.x + Self::PADDING_H + indent;
                    let chevron_y = entry.bounds.y + (entry.bounds.height - Self::CHEVRON_SIZE) / 2.0;
                    let over_chevron = x >= chevron_x && x < chevron_x + Self::CHEVRON_SIZE &&
                                      y >= chevron_y && y < chevron_y + Self::CHEVRON_SIZE;

                    if over_chevron != entry.chevron_hovered {
                        entry.chevron_hovered = over_chevron;
                        changed = true;
                    }
                }
            } else {
                if entry.interaction.is_hovered() {
                    entry.interaction.on_pointer_leave();
                    entry.chevron_hovered = false;
                    changed = true;
                }
            }
        }

        changed
    }

    /// Handle click. Returns (entry_id, is_chevron_click).
    pub fn on_click(&mut self, x: f32, y: f32) -> Option<(usize, bool)> {
        for entry in &self.entries {
            if y >= entry.bounds.y && y < entry.bounds.y + entry.bounds.height {
                // Check chevron click
                if entry.entry.is_dir {
                    let indent = entry.entry.depth as f32 * Self::INDENT_WIDTH;
                    let chevron_x = entry.bounds.x + Self::PADDING_H + indent;
                    let chevron_y = entry.bounds.y + (entry.bounds.height - Self::CHEVRON_SIZE) / 2.0;
                    let on_chevron = x >= chevron_x && x < chevron_x + Self::CHEVRON_SIZE &&
                                    y >= chevron_y && y < chevron_y + Self::CHEVRON_SIZE;

                    if on_chevron {
                        return Some((entry.entry.id, true));
                    }
                }

                return Some((entry.entry.id, false));
            }
        }

        None
    }

    /// Handle scroll.
    pub fn scroll(&mut self, delta: f32) {
        let max_scroll = ((self.entries.len() as f32 * Self::ITEM_HEIGHT) - self.bounds.height + Self::PADDING_V * 2.0).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
        self.layout_entries();
    }

    /// Get entry at index.
    pub fn get_entry(&self, index: usize) -> Option<&FileEntry> {
        self.entries.get(index).map(|e| &e.entry)
    }

    /// Get selected entry.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.iter().find(|e| e.entry.is_selected).map(|e| &e.entry)
    }
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}
