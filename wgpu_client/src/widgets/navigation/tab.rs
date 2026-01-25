//! Tab widget for tab bars.
//!
//! A single tab that can be part of a tab bar, with support for
//! active state, close button, and dirty indicator.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextAlign, TextOverflow,
    LayoutConstraints, CornerRadii, Radius, Space,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Tab widget configuration.
#[derive(Clone, Debug)]
pub struct TabProps {
    /// Tab label text
    pub label: String,
    /// Whether this tab is currently active/selected
    pub active: bool,
    /// Whether the tab has unsaved changes (shows dot indicator)
    pub dirty: bool,
    /// Whether to show close button
    pub closable: bool,
    /// Maximum width for the tab
    pub max_width: f32,
}

impl Default for TabProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            active: false,
            dirty: false,
            closable: true,
            max_width: 200.0,
        }
    }
}

impl TabProps {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn with_dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    pub fn with_closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

/// A tab widget.
pub struct Tab {
    id: WidgetId,
    props: TabProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Close button hover state
    close_hovered: bool,
}

impl Tab {
    const HEIGHT: f32 = 32.0;
    const PADDING_H: f32 = 12.0;
    const CLOSE_SIZE: f32 = 16.0;
    const CLOSE_MARGIN: f32 = 4.0;

    pub fn new(props: TabProps) -> Self {
        let mut tab = Self {
            id: WidgetId::new(),
            props: props.clone(),
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette: ColorPalette::dark(),
            close_hovered: false,
        };

        tab.interaction.set_selected(props.active);
        tab
    }

    /// Check if the tab is active.
    pub fn is_active(&self) -> bool {
        self.props.active
    }

    /// Set the active state.
    pub fn set_active(&mut self, active: bool) {
        self.props.active = active;
        self.interaction.set_selected(active);
    }

    /// Set the dirty state.
    pub fn set_dirty(&mut self, dirty: bool) {
        self.props.dirty = dirty;
    }

    /// Check if a point is over the close button.
    fn is_over_close_button(&self, x: f32, y: f32) -> bool {
        if !self.props.closable {
            return false;
        }

        let close_x = self.bounds.x + self.bounds.width - Self::PADDING_H - Self::CLOSE_SIZE;
        let close_y = self.bounds.y + (self.bounds.height - Self::CLOSE_SIZE) / 2.0;

        x >= close_x && x <= close_x + Self::CLOSE_SIZE &&
        y >= close_y && y <= close_y + Self::CLOSE_SIZE
    }
}

impl Widget for Tab {
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

        // Calculate width based on label
        let char_width = 8.0; // Rough estimate
        let label_width = self.props.label.len() as f32 * char_width;
        let close_width = if self.props.closable {
            Self::CLOSE_SIZE + Self::CLOSE_MARGIN
        } else {
            0.0
        };

        let natural_width = Self::PADDING_H * 2.0 + label_width + close_width;
        let width = natural_width.min(self.props.max_width).clamp(constraints.min_width, constraints.max_width);

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
        let bg_color = if self.props.active {
            self.palette.bg_primary
        } else if state.is_hovered() {
            self.palette.bg_secondary.lighten(0.05)
        } else {
            self.palette.bg_secondary
        };

        // Only round top corners for tabs
        let radius = Radius::Sm.px();
        let tab_bg = StyledRect::new(self.bounds)
            .with_fill(bg_color)
            .with_corner_radii(CornerRadii::top(radius));

        output.add_rect(tab_bg);

        // Active indicator (bottom border)
        if self.props.active {
            let indicator = StyledRect::new(Bounds {
                x: self.bounds.x,
                y: self.bounds.y + self.bounds.height - 2.0,
                width: self.bounds.width,
                height: 2.0,
            }).with_fill(self.palette.accent_primary);
            output.add_rect(indicator);
        }

        // Label
        let label_width = if self.props.closable {
            self.bounds.width - Self::PADDING_H * 2.0 - Self::CLOSE_SIZE - Self::CLOSE_MARGIN
        } else {
            self.bounds.width - Self::PADDING_H * 2.0
        };

        let label_bounds = Bounds {
            x: self.bounds.x + Self::PADDING_H,
            y: self.bounds.y,
            width: label_width,
            height: self.bounds.height,
        };

        let fg_color = if self.props.active {
            self.palette.fg_primary
        } else {
            self.palette.fg_secondary
        };

        let text = TextBlock::new(&self.props.label, label_bounds)
            .with_color(fg_color)
            .with_font_size(13.0)
            .with_overflow(TextOverflow::Ellipsis);

        output.add_text(text);

        // Dirty indicator (dot before close button or at end)
        if self.props.dirty {
            let dot_x = if self.props.closable {
                self.bounds.x + self.bounds.width - Self::PADDING_H - Self::CLOSE_SIZE - Self::CLOSE_MARGIN - 8.0
            } else {
                self.bounds.x + self.bounds.width - Self::PADDING_H - 8.0
            };

            let dot = StyledRect::new(Bounds {
                x: dot_x,
                y: self.bounds.y + (self.bounds.height - 6.0) / 2.0,
                width: 6.0,
                height: 6.0,
            })
            .with_fill(self.palette.fg_secondary)
            .with_corner_radii(CornerRadii::all(3.0));

            output.add_rect(dot);
        }

        // Close button
        if self.props.closable {
            let close_x = self.bounds.x + self.bounds.width - Self::PADDING_H - Self::CLOSE_SIZE;
            let close_y = self.bounds.y + (self.bounds.height - Self::CLOSE_SIZE) / 2.0;

            // Close button background on hover
            if self.close_hovered {
                let close_bg = StyledRect::new(Bounds {
                    x: close_x,
                    y: close_y,
                    width: Self::CLOSE_SIZE,
                    height: Self::CLOSE_SIZE,
                })
                .with_fill(self.palette.interactive_hover)
                .with_corner_radii(CornerRadii::all(Self::CLOSE_SIZE / 2.0));
                output.add_rect(close_bg);
            }

            // X mark (two small rectangles)
            let cx = close_x + Self::CLOSE_SIZE / 2.0;
            let cy = close_y + Self::CLOSE_SIZE / 2.0;
            let bar_len = 8.0;
            let bar_width = 1.5;

            // We'd ideally render an X here, but with rectangles we approximate
            // For now just show a simple marker
            let x_color = if self.close_hovered {
                self.palette.fg_primary
            } else {
                self.palette.fg_muted
            };

            let x_rect = StyledRect::new(Bounds {
                x: cx - 3.0,
                y: cy - 1.0,
                width: 6.0,
                height: 2.0,
            }).with_fill(x_color);

            output.add_rect(x_rect);
        }

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
                self.close_hovered = false;
                true
            }
            PointerEvent::Move { x, y } => {
                let over_close = self.is_over_close_button(x, y);
                if over_close != self.close_hovered {
                    self.close_hovered = over_close;
                    return true;
                }
                false
            }
            PointerEvent::Click { button: PointerButton::Left, x, y } => {
                if self.is_over_close_button(x, y) {
                    // Close button clicked - emit custom event
                    return true;
                }
                // Tab clicked - emit select event
                true
            }
            PointerEvent::Click { button: PointerButton::Middle, .. } => {
                // Middle click closes tab
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
                // Activate tab
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
}
