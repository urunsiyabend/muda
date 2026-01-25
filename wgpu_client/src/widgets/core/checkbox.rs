//! Checkbox widget with check animation.

use std::time::Duration;

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, CornerRadii, Radius,
    LayoutConstraints,
    Tween, Easing,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent, WidgetValue,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Checkbox widget configuration.
#[derive(Clone, Debug)]
pub struct CheckboxProps {
    /// Whether the checkbox is checked
    pub checked: bool,
    /// Optional label text
    pub label: Option<String>,
    /// Whether the checkbox is disabled
    pub disabled: bool,
    /// Whether the checkbox is in indeterminate state
    pub indeterminate: bool,
}

impl Default for CheckboxProps {
    fn default() -> Self {
        Self {
            checked: false,
            label: None,
            disabled: false,
            indeterminate: false,
        }
    }
}

/// A checkbox widget.
pub struct Checkbox {
    id: WidgetId,
    props: CheckboxProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Animation
    check_tween: Tween<f32>, // 0.0 = unchecked, 1.0 = checked
}

impl Checkbox {
    const BOX_SIZE: f32 = 18.0;
    const LABEL_GAP: f32 = 8.0;

    pub fn new(checked: bool) -> Self {
        let initial_progress = if checked { 1.0 } else { 0.0 };

        let mut checkbox = Self {
            id: WidgetId::new(),
            props: CheckboxProps { checked, ..Default::default() },
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette: ColorPalette::dark(),
            check_tween: Tween::instant(initial_progress),
        };

        checkbox.interaction.set_selected(checked);
        checkbox
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    /// Get the current checked state.
    pub fn is_checked(&self) -> bool {
        self.props.checked
    }

    /// Set the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        if self.props.checked != checked {
            self.props.checked = checked;
            self.props.indeterminate = false;
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

    /// Set indeterminate state.
    pub fn set_indeterminate(&mut self, indeterminate: bool) {
        self.props.indeterminate = indeterminate;
    }

    fn update_animation_targets(&mut self) {
        let target = if self.props.checked { 1.0 } else { 0.0 };
        if (self.check_tween.target() - target).abs() > 0.001 {
            self.check_tween.retarget(target);
            self.check_tween.start();
        }
    }
}

impl Widget for Checkbox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, _dt: f32) -> bool {
        self.check_tween.update()
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let mut width = Self::BOX_SIZE;
        let height = Self::BOX_SIZE;

        // Add label width if present
        if let Some(label) = &self.props.label {
            let label_width = label.len() as f32 * 8.0; // Rough estimate
            width += Self::LABEL_GAP + label_width;
        }

        (
            width.clamp(constraints.min_width, constraints.max_width),
            height.clamp(constraints.min_height, constraints.max_height),
        )
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
        let progress = *self.check_tween.value();

        // Box background
        let box_bounds = Bounds {
            x: self.bounds.x,
            y: self.bounds.y + (self.bounds.height - Self::BOX_SIZE) / 2.0,
            width: Self::BOX_SIZE,
            height: Self::BOX_SIZE,
        };

        let bg_color = if progress > 0.01 {
            self.palette.accent_primary.lerp(self.palette.accent_primary, progress)
        } else if state.is_hovered() {
            self.palette.interactive_hover
        } else {
            self.palette.interactive
        };

        let bg_color = if state.is_disabled() {
            bg_color.with_alpha(0.5)
        } else {
            bg_color
        };

        let border_color = if state.is_focused() {
            self.palette.focus
        } else if progress > 0.01 {
            self.palette.accent_primary
        } else {
            self.palette.border_default
        };

        let radius = Radius::Xs.px();
        let checkbox_box = StyledRect::new(box_bounds)
            .with_fill(if progress > 0.01 { bg_color } else { self.palette.bg_secondary })
            .with_border(if state.is_focused() { 2.0 } else { 1.0 }, border_color)
            .with_corner_radii(CornerRadii::all(radius));

        output.add_rect(checkbox_box);

        // Check mark (rendered as two rectangles forming a checkmark)
        if progress > 0.0 {
            // Scale the checkmark based on animation progress
            let check_color = self.palette.fg_inverse;
            let center_x = box_bounds.x + box_bounds.width / 2.0;
            let center_y = box_bounds.y + box_bounds.height / 2.0;

            if self.props.indeterminate {
                // Dash for indeterminate state
                let dash_width = 10.0 * progress;
                let dash = StyledRect::new(Bounds {
                    x: center_x - dash_width / 2.0,
                    y: center_y - 1.0,
                    width: dash_width,
                    height: 2.0,
                }).with_fill(check_color);
                output.add_rect(dash);
            } else {
                // Checkmark (simplified as two small rectangles)
                // Short leg
                let short_width = 4.0 * progress;
                let short = StyledRect::new(Bounds {
                    x: center_x - 4.0,
                    y: center_y,
                    width: short_width,
                    height: 2.0,
                }).with_fill(check_color);

                // Long leg
                let long_height = 8.0 * progress;
                let long = StyledRect::new(Bounds {
                    x: center_x,
                    y: center_y - long_height / 2.0 + 1.0,
                    width: 2.0,
                    height: long_height,
                }).with_fill(check_color);

                output.add_rect(short);
                output.add_rect(long);
            }
        }

        // Label
        if let Some(label) = &self.props.label {
            let label_x = box_bounds.x + Self::BOX_SIZE + Self::LABEL_GAP;
            let label_bounds = Bounds {
                x: label_x,
                y: self.bounds.y,
                width: self.bounds.width - label_x + self.bounds.x,
                height: self.bounds.height,
            };

            let label_color = if state.is_disabled() {
                self.palette.fg_muted
            } else {
                self.palette.fg_primary
            };

            let text = TextBlock::new(label, label_bounds)
                .with_color(label_color)
                .with_font_size(14.0);

            output.add_text(text);
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
