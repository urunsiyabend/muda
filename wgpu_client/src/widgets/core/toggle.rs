//! Toggle/switch widget with smooth animation.

use std::time::Duration;

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, CornerRadii, Radius,
    LayoutConstraints,
    Tween, Easing,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent, WidgetValue,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Toggle widget configuration.
#[derive(Clone, Debug)]
pub struct ToggleProps {
    /// Whether the toggle is on
    pub checked: bool,
    /// Whether the toggle is disabled
    pub disabled: bool,
}

impl Default for ToggleProps {
    fn default() -> Self {
        Self {
            checked: false,
            disabled: false,
        }
    }
}

/// A toggle/switch widget.
pub struct Toggle {
    id: WidgetId,
    props: ToggleProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Animation
    position_tween: Tween<f32>, // 0.0 = off, 1.0 = on
    bg_color_tween: Tween<Color>,
}

impl Toggle {
    const WIDTH: f32 = 44.0;
    const HEIGHT: f32 = 24.0;
    const KNOB_SIZE: f32 = 20.0;
    const KNOB_MARGIN: f32 = 2.0;

    pub fn new(checked: bool) -> Self {
        let palette = ColorPalette::dark();
        let initial_pos = if checked { 1.0 } else { 0.0 };
        let initial_color = if checked { palette.accent_primary } else { palette.interactive };

        let mut toggle = Self {
            id: WidgetId::new(),
            props: ToggleProps { checked, disabled: false },
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette,
            position_tween: Tween::instant(initial_pos),
            bg_color_tween: Tween::instant(initial_color),
        };

        toggle.interaction.set_selected(checked);
        toggle
    }

    /// Get the current checked state.
    pub fn is_checked(&self) -> bool {
        self.props.checked
    }

    /// Set the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        if self.props.checked != checked {
            self.props.checked = checked;
            self.interaction.set_selected(checked);
            self.update_animation_targets();
        }
    }

    /// Toggle the checked state.
    pub fn toggle(&mut self) {
        self.set_checked(!self.props.checked);
    }

    /// Set disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.props.disabled = disabled;
        self.interaction.set_disabled(disabled);
    }

    fn update_animation_targets(&mut self) {
        let target_pos = if self.props.checked { 1.0 } else { 0.0 };
        let target_color = if self.props.checked {
            self.palette.accent_primary
        } else {
            self.palette.interactive
        };

        if (self.position_tween.target() - target_pos).abs() > 0.001 {
            self.position_tween.retarget(target_pos);
            self.position_tween.start();
        }

        if self.bg_color_tween.target() != &target_color {
            self.bg_color_tween.retarget(target_color);
            self.bg_color_tween.start();
        }
    }
}

impl Widget for Toggle {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, _dt: f32) -> bool {
        let pos_changed = self.position_tween.update();
        let color_changed = self.bg_color_tween.update();
        pos_changed || color_changed
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let width = Self::WIDTH.clamp(constraints.min_width, constraints.max_width);
        let height = Self::HEIGHT.clamp(constraints.min_height, constraints.max_height);
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

        let bg_color = if state.is_disabled() {
            self.bg_color_tween.value().with_alpha(0.5)
        } else {
            *self.bg_color_tween.value()
        };

        // Track background
        let track_radius = Self::HEIGHT / 2.0;
        let track = StyledRect::new(self.bounds)
            .with_fill(bg_color)
            .with_corner_radii(CornerRadii::all(track_radius));

        output.add_rect(track);

        // Focus ring
        if state.is_focused() {
            let focus_ring = StyledRect::new(Bounds {
                x: self.bounds.x - 2.0,
                y: self.bounds.y - 2.0,
                width: self.bounds.width + 4.0,
                height: self.bounds.height + 4.0,
            })
            .with_border(2.0, self.palette.focus)
            .with_corner_radii(CornerRadii::all(track_radius + 2.0));

            output.rects.insert(0, focus_ring);
        }

        // Knob
        let pos = *self.position_tween.value();
        let travel = Self::WIDTH - Self::KNOB_SIZE - Self::KNOB_MARGIN * 2.0;
        let knob_x = self.bounds.x + Self::KNOB_MARGIN + pos * travel;
        let knob_y = self.bounds.y + Self::KNOB_MARGIN;

        let knob_color = if state.is_disabled() {
            self.palette.fg_muted
        } else if state.is_hovered() {
            self.palette.fg_inverse
        } else {
            self.palette.fg_inverse.darken(0.05)
        };

        let knob = StyledRect::new(Bounds {
            x: knob_x,
            y: knob_y,
            width: Self::KNOB_SIZE,
            height: Self::KNOB_SIZE,
        })
        .with_fill(knob_color)
        .with_corner_radii(CornerRadii::all(Self::KNOB_SIZE / 2.0));

        output.add_rect(knob);

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
                self.toggle();
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
                self.toggle();
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
