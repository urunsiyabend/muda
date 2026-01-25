//! List item widget for menus and lists.
//!
//! A selectable list item with optional icon, label, and keyboard shortcut display.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextAlign,
    LayoutConstraints, CornerRadii, Radius, Space,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// List item configuration.
#[derive(Clone, Debug)]
pub struct ListItemProps {
    /// Primary label text
    pub label: String,
    /// Optional secondary text (e.g., description or shortcut)
    pub secondary: Option<String>,
    /// Whether the item is selected
    pub selected: bool,
    /// Whether the item is disabled
    pub disabled: bool,
    /// Whether to show a checkmark when selected
    pub checkable: bool,
    /// Indentation level (for nested lists)
    pub indent_level: u8,
}

impl Default for ListItemProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            secondary: None,
            selected: false,
            disabled: false,
            checkable: false,
            indent_level: 0,
        }
    }
}

impl ListItemProps {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn with_secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    pub fn with_indent(mut self, level: u8) -> Self {
        self.indent_level = level;
        self
    }
}

/// A list item widget.
pub struct ListItem {
    id: WidgetId,
    props: ListItemProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,
}

impl ListItem {
    const HEIGHT: f32 = 28.0;
    const PADDING_H: f32 = 12.0;
    const INDENT_WIDTH: f32 = 16.0;
    const CHECK_WIDTH: f32 = 20.0;

    pub fn new(props: ListItemProps) -> Self {
        let mut item = Self {
            id: WidgetId::new(),
            props: props.clone(),
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette: ColorPalette::dark(),
        };

        item.interaction.set_selected(props.selected);
        item.interaction.set_disabled(props.disabled);
        item
    }

    /// Check if item is selected.
    pub fn is_selected(&self) -> bool {
        self.props.selected
    }

    /// Set selected state.
    pub fn set_selected(&mut self, selected: bool) {
        self.props.selected = selected;
        self.interaction.set_selected(selected);
    }
}

impl Widget for ListItem {
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
        let bg_color = if state.is_disabled() {
            Color::TRANSPARENT
        } else if state.is_selected() {
            self.palette.selection
        } else if state.is_hovered() {
            self.palette.interactive_hover
        } else {
            Color::TRANSPARENT
        };

        if bg_color.a > 0.0 {
            let bg = StyledRect::new(self.bounds)
                .with_fill(bg_color)
                .with_corner_radii(CornerRadii::all(Radius::Xs.px()));
            output.add_rect(bg);
        }

        // Calculate content offset
        let indent = self.props.indent_level as f32 * Self::INDENT_WIDTH;
        let check_offset = if self.props.checkable { Self::CHECK_WIDTH } else { 0.0 };
        let content_x = self.bounds.x + Self::PADDING_H + indent + check_offset;

        // Checkmark (if checkable and selected)
        if self.props.checkable && self.props.selected {
            let check_x = self.bounds.x + Self::PADDING_H + indent;
            let check_y = self.bounds.y + (self.bounds.height - 12.0) / 2.0;

            // Simple checkmark indicator
            let check = StyledRect::new(Bounds {
                x: check_x + 2.0,
                y: check_y + 4.0,
                width: 8.0,
                height: 2.0,
            }).with_fill(self.palette.accent_primary);

            output.add_rect(check);
        }

        // Label
        let fg_color = if state.is_disabled() {
            self.palette.fg_muted
        } else if state.is_selected() {
            self.palette.fg_inverse
        } else {
            self.palette.fg_primary
        };

        let secondary_width = self.props.secondary.as_ref()
            .map(|s| s.len() as f32 * 7.0 + Self::PADDING_H)
            .unwrap_or(0.0);

        let label_bounds = Bounds {
            x: content_x,
            y: self.bounds.y,
            width: self.bounds.width - content_x + self.bounds.x - secondary_width - Self::PADDING_H,
            height: self.bounds.height,
        };

        let label = TextBlock::new(&self.props.label, label_bounds)
            .with_color(fg_color)
            .with_font_size(13.0);

        output.add_text(label);

        // Secondary text (shortcut, etc.)
        if let Some(secondary) = &self.props.secondary {
            let secondary_bounds = Bounds {
                x: self.bounds.x + self.bounds.width - secondary_width,
                y: self.bounds.y,
                width: secondary_width - Self::PADDING_H,
                height: self.bounds.height,
            };

            let secondary_color = if state.is_disabled() {
                self.palette.fg_muted.with_alpha(0.5)
            } else {
                self.palette.fg_muted
            };

            let secondary_text = TextBlock::new(secondary, secondary_bounds)
                .with_color(secondary_color)
                .with_font_size(12.0)
                .with_align(TextAlign::Right);

            output.add_text(secondary_text);
        }

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        if self.props.disabled {
            return false;
        }

        match event {
            PointerEvent::Enter => {
                self.interaction.on_pointer_enter();
                true
            }
            PointerEvent::Leave => {
                self.interaction.on_pointer_leave();
                true
            }
            PointerEvent::Click { button: PointerButton::Left, .. } => {
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, event: KeyEvent) -> bool {
        if self.props.disabled || !event.pressed {
            return false;
        }

        match event.key {
            KeyCode::Enter | KeyCode::Space => {
                true
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

    fn can_focus(&self) -> bool {
        !self.props.disabled
    }
}
