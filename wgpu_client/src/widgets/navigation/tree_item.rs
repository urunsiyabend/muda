//! Tree item widget for hierarchical views.
//!
//! A tree item with expand/collapse support, ideal for file explorers
//! and hierarchical data.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextOverflow,
    LayoutConstraints, CornerRadii, Radius,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Tree item type (affects icon display).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeItemKind {
    /// A file (leaf node)
    #[default]
    File,
    /// A folder (can contain children)
    Folder,
    /// A folder that is currently expanded
    FolderOpen,
}

/// Tree item configuration.
#[derive(Clone, Debug)]
pub struct TreeItemProps {
    /// Item label
    pub label: String,
    /// Item type
    pub kind: TreeItemKind,
    /// Depth in the tree (for indentation)
    pub depth: u8,
    /// Whether the item is selected
    pub selected: bool,
    /// Whether the item is expanded (for folders)
    pub expanded: bool,
    /// Whether the item has children (shows expand arrow)
    pub has_children: bool,
}

impl Default for TreeItemProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            kind: TreeItemKind::File,
            depth: 0,
            selected: false,
            expanded: false,
            has_children: false,
        }
    }
}

impl TreeItemProps {
    pub fn file(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: TreeItemKind::File,
            ..Default::default()
        }
    }

    pub fn folder(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: TreeItemKind::Folder,
            has_children: true,
            ..Default::default()
        }
    }

    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        if expanded && self.kind == TreeItemKind::Folder {
            self.kind = TreeItemKind::FolderOpen;
        }
        self
    }
}

/// A tree item widget.
pub struct TreeItem {
    id: WidgetId,
    props: TreeItemProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Chevron hover state
    chevron_hovered: bool,
}

impl TreeItem {
    const HEIGHT: f32 = 22.0;
    const INDENT_WIDTH: f32 = 16.0;
    const CHEVRON_SIZE: f32 = 16.0;
    const ICON_SIZE: f32 = 16.0;
    const ICON_GAP: f32 = 6.0;
    const PADDING_H: f32 = 8.0;

    pub fn new(props: TreeItemProps) -> Self {
        let mut item = Self {
            id: WidgetId::new(),
            props: props.clone(),
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette: ColorPalette::dark(),
            chevron_hovered: false,
        };

        item.interaction.set_selected(props.selected);
        item
    }

    /// Check if selected.
    pub fn is_selected(&self) -> bool {
        self.props.selected
    }

    /// Set selected state.
    pub fn set_selected(&mut self, selected: bool) {
        self.props.selected = selected;
        self.interaction.set_selected(selected);
    }

    /// Check if expanded.
    pub fn is_expanded(&self) -> bool {
        self.props.expanded
    }

    /// Toggle expanded state.
    pub fn toggle_expanded(&mut self) {
        if self.props.has_children {
            self.props.expanded = !self.props.expanded;
            self.props.kind = if self.props.expanded {
                TreeItemKind::FolderOpen
            } else {
                TreeItemKind::Folder
            };
        }
    }

    /// Get the x position of the chevron.
    fn chevron_x(&self) -> f32 {
        self.bounds.x + Self::PADDING_H + self.props.depth as f32 * Self::INDENT_WIDTH
    }

    /// Check if a point is over the chevron.
    fn is_over_chevron(&self, x: f32, y: f32) -> bool {
        if !self.props.has_children {
            return false;
        }

        let cx = self.chevron_x();
        let cy = self.bounds.y;

        x >= cx && x <= cx + Self::CHEVRON_SIZE &&
        y >= cy && y <= cy + self.bounds.height
    }
}

impl Widget for TreeItem {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, _dt: f32) -> bool {
        false
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let height = Self::HEIGHT.clamp(constraints.min_height, constraints.max_height);
        let width = constraints.max_width;
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
        let state = self.interaction.state();

        // Background
        let bg_color = if state.is_selected() {
            self.palette.selection
        } else if state.is_hovered() {
            self.palette.interactive_hover.with_alpha(0.5)
        } else {
            Color::TRANSPARENT
        };

        if bg_color.a > 0.0 {
            let bg = StyledRect::new(self.bounds).with_fill(bg_color);
            output.add_rect(bg);
        }

        // Indentation + Chevron
        let indent_x = self.bounds.x + Self::PADDING_H + self.props.depth as f32 * Self::INDENT_WIDTH;

        if self.props.has_children {
            // Draw chevron (arrow)
            let chevron_y = self.bounds.y + (self.bounds.height - Self::CHEVRON_SIZE) / 2.0;

            // Chevron background on hover
            if self.chevron_hovered {
                let chevron_bg = StyledRect::new(Bounds {
                    x: indent_x,
                    y: chevron_y,
                    width: Self::CHEVRON_SIZE,
                    height: Self::CHEVRON_SIZE,
                })
                .with_fill(self.palette.interactive_hover)
                .with_corner_radii(CornerRadii::all(2.0));
                output.add_rect(chevron_bg);
            }

            // Arrow indicator (simplified as a small triangle-like shape)
            let arrow_color = self.palette.fg_secondary;
            let cx = indent_x + Self::CHEVRON_SIZE / 2.0;
            let cy = self.bounds.y + self.bounds.height / 2.0;

            if self.props.expanded {
                // Down arrow (horizontal bar)
                let arrow = StyledRect::new(Bounds {
                    x: cx - 4.0,
                    y: cy,
                    width: 8.0,
                    height: 2.0,
                }).with_fill(arrow_color);
                output.add_rect(arrow);

                // Additional bar for ">"
                let arrow2 = StyledRect::new(Bounds {
                    x: cx - 1.0,
                    y: cy - 2.0,
                    width: 2.0,
                    height: 6.0,
                }).with_fill(arrow_color);
                output.add_rect(arrow2);
            } else {
                // Right arrow (vertical bar)
                let arrow = StyledRect::new(Bounds {
                    x: cx,
                    y: cy - 4.0,
                    width: 2.0,
                    height: 8.0,
                }).with_fill(arrow_color);
                output.add_rect(arrow);

                let arrow2 = StyledRect::new(Bounds {
                    x: cx - 2.0,
                    y: cy - 1.0,
                    width: 6.0,
                    height: 2.0,
                }).with_fill(arrow_color);
                output.add_rect(arrow2);
            }
        }

        // Icon (simplified as colored square)
        let icon_x = indent_x + Self::CHEVRON_SIZE + 2.0;
        let icon_y = self.bounds.y + (self.bounds.height - Self::ICON_SIZE) / 2.0;

        let icon_color = match self.props.kind {
            TreeItemKind::File => self.palette.fg_secondary,
            TreeItemKind::Folder | TreeItemKind::FolderOpen => self.palette.warning, // Yellow for folders
        };

        let icon = StyledRect::new(Bounds {
            x: icon_x,
            y: icon_y + 2.0,
            width: Self::ICON_SIZE - 4.0,
            height: Self::ICON_SIZE - 4.0,
        })
        .with_fill(icon_color)
        .with_corner_radii(CornerRadii::all(2.0));

        output.add_rect(icon);

        // Label
        let label_x = icon_x + Self::ICON_SIZE + Self::ICON_GAP;
        let label_bounds = Bounds {
            x: label_x,
            y: self.bounds.y,
            width: self.bounds.width - label_x + self.bounds.x - Self::PADDING_H,
            height: self.bounds.height,
        };

        let fg_color = if state.is_selected() {
            self.palette.fg_primary
        } else {
            self.palette.fg_primary
        };

        let label = TextBlock::new(&self.props.label, label_bounds)
            .with_color(fg_color)
            .with_font_size(13.0)
            .with_overflow(TextOverflow::Ellipsis);

        output.add_text(label);

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        match event {
            PointerEvent::Enter => {
                self.interaction.on_pointer_enter();
                true
            }
            PointerEvent::Leave => {
                self.interaction.on_pointer_leave();
                self.chevron_hovered = false;
                true
            }
            PointerEvent::Move { x, y } => {
                let over_chevron = self.is_over_chevron(x, y);
                if over_chevron != self.chevron_hovered {
                    self.chevron_hovered = over_chevron;
                    return true;
                }
                false
            }
            PointerEvent::Click { button: PointerButton::Left, x, y } => {
                if self.is_over_chevron(x, y) {
                    self.toggle_expanded();
                    return true;
                }
                // Regular click - select item
                true
            }
            PointerEvent::Down { button: PointerButton::Left, x, y } => {
                // Double-click detection would go here
                self.interaction.on_pointer_down();
                true
            }
            PointerEvent::Up { button: PointerButton::Left, .. } => {
                self.interaction.on_pointer_up();
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, event: KeyEvent) -> bool {
        if !event.pressed {
            return false;
        }

        match event.key {
            KeyCode::Enter | KeyCode::Space => {
                // Activate item
                true
            }
            KeyCode::ArrowRight => {
                if self.props.has_children && !self.props.expanded {
                    self.toggle_expanded();
                    return true;
                }
                false
            }
            KeyCode::ArrowLeft => {
                if self.props.has_children && self.props.expanded {
                    self.toggle_expanded();
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn on_focus(&mut self, gained: bool) {
        if gained {
            self.interaction.on_focus();
        } else {
            self.interaction.on_blur();
        }
    }
}
