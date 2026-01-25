//! Context menu composite widget.
//!
//! A popup menu that appears at a specific position, containing
//! a list of selectable items with optional shortcuts and submenus.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextAlign,
    LayoutConstraints, CornerRadii, Radius, Elevation,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Context menu item types.
#[derive(Clone, Debug)]
pub enum ContextMenuItem {
    /// Regular action item
    Action {
        id: String,
        label: String,
        shortcut: Option<String>,
        disabled: bool,
    },
    /// Separator line
    Separator,
    /// Submenu (not fully implemented)
    Submenu {
        label: String,
        items: Vec<ContextMenuItem>,
    },
}

impl ContextMenuItem {
    /// Create a new action item.
    pub fn action(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Action {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            disabled: false,
        }
    }

    /// Add a keyboard shortcut display.
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Action { shortcut: ref mut s, .. } = self {
            *s = Some(shortcut.into());
        }
        self
    }

    /// Set disabled state.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        if let Self::Action { disabled: ref mut d, .. } = self {
            *d = disabled;
        }
        self
    }

    /// Create a separator.
    pub fn separator() -> Self {
        Self::Separator
    }
}

/// A context menu widget.
pub struct ContextMenu {
    id: WidgetId,
    items: Vec<ContextMenuItem>,
    bounds: Bounds,
    palette: ColorPalette,
    visible: bool,
    position: (f32, f32),

    // Hover state
    hovered_index: Option<usize>,
}

impl ContextMenu {
    const ITEM_HEIGHT: f32 = 28.0;
    const SEPARATOR_HEIGHT: f32 = 9.0;
    const PADDING_V: f32 = 4.0;
    const PADDING_H: f32 = 4.0;
    const MIN_WIDTH: f32 = 180.0;
    const MAX_WIDTH: f32 = 300.0;

    pub fn new(items: Vec<ContextMenuItem>) -> Self {
        Self {
            id: WidgetId::new(),
            items,
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            visible: false,
            position: (0.0, 0.0),
            hovered_index: None,
        }
    }

    /// Show the menu at the given position.
    pub fn show(&mut self, x: f32, y: f32) {
        self.visible = true;
        self.position = (x, y);
        self.hovered_index = None;
    }

    /// Hide the menu.
    pub fn hide(&mut self) {
        self.visible = false;
        self.hovered_index = None;
    }

    /// Check if the menu is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Calculate the height of all items.
    fn calculate_height(&self) -> f32 {
        let content_height: f32 = self.items.iter().map(|item| {
            match item {
                ContextMenuItem::Separator => Self::SEPARATOR_HEIGHT,
                _ => Self::ITEM_HEIGHT,
            }
        }).sum();

        content_height + Self::PADDING_V * 2.0
    }

    /// Calculate the required width.
    fn calculate_width(&self) -> f32 {
        let max_label_width: f32 = self.items.iter().filter_map(|item| {
            match item {
                ContextMenuItem::Action { label, shortcut, .. } => {
                    let label_width = label.len() as f32 * 8.0;
                    let shortcut_width = shortcut.as_ref().map(|s| s.len() as f32 * 7.0 + 20.0).unwrap_or(0.0);
                    Some(label_width + shortcut_width + 24.0) // padding
                }
                ContextMenuItem::Submenu { label, .. } => {
                    Some(label.len() as f32 * 8.0 + 40.0) // padding + arrow
                }
                ContextMenuItem::Separator => None,
            }
        }).fold(0.0_f32, |a, b| a.max(b));

        max_label_width.clamp(Self::MIN_WIDTH, Self::MAX_WIDTH) + Self::PADDING_H * 2.0
    }

    /// Get the Y position for an item at the given index.
    fn item_y(&self, index: usize) -> f32 {
        let mut y = self.bounds.y + Self::PADDING_V;
        for (i, item) in self.items.iter().enumerate() {
            if i == index {
                return y;
            }
            y += match item {
                ContextMenuItem::Separator => Self::SEPARATOR_HEIGHT,
                _ => Self::ITEM_HEIGHT,
            };
        }
        y
    }

    /// Find which item is at the given Y position.
    fn item_at_y(&self, y: f32) -> Option<usize> {
        let mut current_y = self.bounds.y + Self::PADDING_V;
        for (i, item) in self.items.iter().enumerate() {
            let height = match item {
                ContextMenuItem::Separator => Self::SEPARATOR_HEIGHT,
                _ => Self::ITEM_HEIGHT,
            };

            if y >= current_y && y < current_y + height {
                // Don't return separators
                if matches!(item, ContextMenuItem::Separator) {
                    return None;
                }
                return Some(i);
            }

            current_y += height;
        }
        None
    }
}

impl Widget for ContextMenu {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        if self.visible {
            InteractionState::Focused
        } else {
            InteractionState::Idle
        }
    }

    fn update(&mut self, _dt: f32) -> bool {
        false
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        if !self.visible {
            return (0.0, 0.0);
        }

        let width = self.calculate_width();
        let height = self.calculate_height();

        // Adjust position to fit within viewport
        let mut x = self.position.0;
        let mut y = self.position.1;

        if x + width > constraints.max_width {
            x = (constraints.max_width - width).max(0.0);
        }
        if y + height > constraints.max_height {
            y = (constraints.max_height - height).max(0.0);
        }

        self.bounds = Bounds { x, y, width, height };
        (width, height)
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn build(&mut self) -> WidgetOutput {
        let mut output = WidgetOutput::new();

        if !self.visible {
            return output;
        }

        // Menu background with shadow
        let bg = StyledRect::new(self.bounds)
            .with_fill(self.palette.bg_elevated)
            .with_border(1.0, self.palette.border_default)
            .with_corner_radii(CornerRadii::all(Radius::Md.px()))
            .with_elevation(Elevation::Medium);

        output.add_rect(bg);

        // Render items
        let mut y = self.bounds.y + Self::PADDING_V;
        let item_width = self.bounds.width - Self::PADDING_H * 2.0;

        for (i, item) in self.items.iter().enumerate() {
            match item {
                ContextMenuItem::Action { label, shortcut, disabled, .. } => {
                    let is_hovered = self.hovered_index == Some(i) && !*disabled;

                    // Item background
                    if is_hovered {
                        let item_bg = StyledRect::new(Bounds {
                            x: self.bounds.x + Self::PADDING_H,
                            y,
                            width: item_width,
                            height: Self::ITEM_HEIGHT,
                        })
                        .with_fill(self.palette.selection)
                        .with_corner_radii(CornerRadii::all(Radius::Xs.px()));
                        output.add_rect(item_bg);
                    }

                    // Label
                    let fg_color = if *disabled {
                        self.palette.fg_muted
                    } else if is_hovered {
                        self.palette.fg_inverse
                    } else {
                        self.palette.fg_primary
                    };

                    let label_bounds = Bounds {
                        x: self.bounds.x + Self::PADDING_H + 8.0,
                        y,
                        width: item_width - 16.0,
                        height: Self::ITEM_HEIGHT,
                    };

                    let label_text = TextBlock::new(label, label_bounds)
                        .with_color(fg_color)
                        .with_font_size(13.0);
                    output.add_text(label_text);

                    // Shortcut
                    if let Some(shortcut) = shortcut {
                        let shortcut_bounds = Bounds {
                            x: self.bounds.x + self.bounds.width - Self::PADDING_H - 80.0,
                            y,
                            width: 72.0,
                            height: Self::ITEM_HEIGHT,
                        };

                        let shortcut_color = if *disabled {
                            self.palette.fg_muted.with_alpha(0.5)
                        } else {
                            self.palette.fg_muted
                        };

                        let shortcut_text = TextBlock::new(shortcut, shortcut_bounds)
                            .with_color(shortcut_color)
                            .with_font_size(12.0)
                            .with_align(TextAlign::Right);
                        output.add_text(shortcut_text);
                    }

                    y += Self::ITEM_HEIGHT;
                }
                ContextMenuItem::Separator => {
                    let sep_y = y + Self::SEPARATOR_HEIGHT / 2.0;
                    let separator = StyledRect::new(Bounds {
                        x: self.bounds.x + Self::PADDING_H + 4.0,
                        y: sep_y - 0.5,
                        width: item_width - 8.0,
                        height: 1.0,
                    }).with_fill(self.palette.border_subtle);
                    output.add_rect(separator);

                    y += Self::SEPARATOR_HEIGHT;
                }
                ContextMenuItem::Submenu { label, .. } => {
                    let is_hovered = self.hovered_index == Some(i);

                    if is_hovered {
                        let item_bg = StyledRect::new(Bounds {
                            x: self.bounds.x + Self::PADDING_H,
                            y,
                            width: item_width,
                            height: Self::ITEM_HEIGHT,
                        })
                        .with_fill(self.palette.selection)
                        .with_corner_radii(CornerRadii::all(Radius::Xs.px()));
                        output.add_rect(item_bg);
                    }

                    let fg_color = if is_hovered {
                        self.palette.fg_inverse
                    } else {
                        self.palette.fg_primary
                    };

                    let label_bounds = Bounds {
                        x: self.bounds.x + Self::PADDING_H + 8.0,
                        y,
                        width: item_width - 24.0,
                        height: Self::ITEM_HEIGHT,
                    };

                    let label_text = TextBlock::new(label, label_bounds)
                        .with_color(fg_color)
                        .with_font_size(13.0);
                    output.add_text(label_text);

                    // Arrow indicator
                    let arrow = StyledRect::new(Bounds {
                        x: self.bounds.x + self.bounds.width - Self::PADDING_H - 12.0,
                        y: y + Self::ITEM_HEIGHT / 2.0 - 3.0,
                        width: 2.0,
                        height: 6.0,
                    }).with_fill(fg_color);
                    output.add_rect(arrow);

                    y += Self::ITEM_HEIGHT;
                }
            }
        }

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        if !self.visible {
            return false;
        }

        match event {
            PointerEvent::Move { x, y } => {
                let new_hovered = self.item_at_y(y);
                if new_hovered != self.hovered_index {
                    self.hovered_index = new_hovered;
                    return true;
                }
                false
            }
            PointerEvent::Click { button: PointerButton::Left, x, y } => {
                if let Some(index) = self.item_at_y(y) {
                    if let Some(ContextMenuItem::Action { id, disabled, .. }) = self.items.get(index) {
                        if !*disabled {
                            self.hide();
                            return true;
                        }
                    }
                }
                false
            }
            PointerEvent::Leave => {
                self.hovered_index = None;
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, event: KeyEvent) -> bool {
        if !self.visible || !event.pressed {
            return false;
        }

        match event.key {
            KeyCode::Escape => {
                self.hide();
                true
            }
            KeyCode::ArrowUp => {
                // Find previous non-separator item
                let current = self.hovered_index.unwrap_or(self.items.len());
                for i in (0..current).rev() {
                    if !matches!(self.items.get(i), Some(ContextMenuItem::Separator)) {
                        self.hovered_index = Some(i);
                        return true;
                    }
                }
                false
            }
            KeyCode::ArrowDown => {
                // Find next non-separator item
                let current = self.hovered_index.map(|i| i + 1).unwrap_or(0);
                for i in current..self.items.len() {
                    if !matches!(self.items.get(i), Some(ContextMenuItem::Separator)) {
                        self.hovered_index = Some(i);
                        return true;
                    }
                }
                false
            }
            KeyCode::Enter | KeyCode::Space => {
                if let Some(index) = self.hovered_index {
                    if let Some(ContextMenuItem::Action { disabled, .. }) = self.items.get(index) {
                        if !*disabled {
                            self.hide();
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn on_focus(&mut self, gained: bool) {
        if !gained {
            self.hide();
        }
    }

    fn hit_test(&self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        x >= self.bounds.x && x < self.bounds.x + self.bounds.width &&
        y >= self.bounds.y && y < self.bounds.y + self.bounds.height
    }
}
