//! Button widget with multiple variants.
//!
//! Supports Primary, Secondary, Ghost, and Danger variants with
//! hover/press animations and disabled state.

use std::time::Duration;

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextAlign,
    LayoutConstraints, CornerRadii,
    ComponentSize, Radius, Elevation, Space,
    Tween, Easing,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Button visual variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Primary action button (accent color background)
    #[default]
    Primary,
    /// Secondary action button (subtle background)
    Secondary,
    /// Ghost button (transparent until hovered)
    Ghost,
    /// Danger/destructive action (red)
    Danger,
}

/// Button configuration properties.
#[derive(Clone, Debug)]
pub struct ButtonProps {
    /// Button text label
    pub label: String,
    /// Visual variant
    pub variant: ButtonVariant,
    /// Size preset
    pub size: ComponentSize,
    /// Whether button is disabled
    pub disabled: bool,
    /// Whether button should take full width
    pub full_width: bool,
    /// Optional icon (not implemented yet)
    pub icon: Option<String>,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            variant: ButtonVariant::Primary,
            size: ComponentSize::Md,
            disabled: false,
            full_width: false,
            icon: None,
        }
    }
}

impl ButtonProps {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn primary(label: impl Into<String>) -> Self {
        Self::new(label).with_variant(ButtonVariant::Primary)
    }

    pub fn secondary(label: impl Into<String>) -> Self {
        Self::new(label).with_variant(ButtonVariant::Secondary)
    }

    pub fn ghost(label: impl Into<String>) -> Self {
        Self::new(label).with_variant(ButtonVariant::Ghost)
    }

    pub fn danger(label: impl Into<String>) -> Self {
        Self::new(label).with_variant(ButtonVariant::Danger)
    }

    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_size(mut self, size: ComponentSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn with_full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }
}

/// A button widget.
pub struct Button {
    id: WidgetId,
    props: ButtonProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Animation state
    bg_color_tween: Tween<Color>,
    scale_tween: Tween<f32>,
}

impl Button {
    /// Create a new button with the given properties.
    pub fn new(props: ButtonProps) -> Self {
        let palette = ColorPalette::dark();
        let initial_color = Self::bg_color_for_state(&props, InteractionState::Idle, &palette);

        let mut button = Self {
            id: WidgetId::new(),
            props: props.clone(),
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette,
            bg_color_tween: Tween::instant(initial_color),
            scale_tween: Tween::instant(1.0),
        };

        button.interaction.set_disabled(props.disabled);
        button
    }

    /// Create a primary button.
    pub fn primary(label: impl Into<String>) -> Self {
        Self::new(ButtonProps::primary(label))
    }

    /// Create a secondary button.
    pub fn secondary(label: impl Into<String>) -> Self {
        Self::new(ButtonProps::secondary(label))
    }

    /// Create a ghost button.
    pub fn ghost(label: impl Into<String>) -> Self {
        Self::new(ButtonProps::ghost(label))
    }

    /// Create a danger button.
    pub fn danger(label: impl Into<String>) -> Self {
        Self::new(ButtonProps::danger(label))
    }

    /// Set the button label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.props.label = label.into();
    }

    /// Set the disabled state.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.props.disabled = disabled;
        self.interaction.set_disabled(disabled);
        self.update_animation_targets();
    }

    /// Get colors for a specific variant and state.
    fn bg_color_for_state(props: &ButtonProps, state: InteractionState, palette: &ColorPalette) -> Color {
        match props.variant {
            ButtonVariant::Primary => match state {
                InteractionState::Idle => palette.accent_primary,
                InteractionState::Hovered => palette.accent_primary.lighten(0.1),
                InteractionState::Pressed => palette.accent_primary.darken(0.1),
                InteractionState::Focused => palette.accent_primary,
                InteractionState::FocusedHovered => palette.accent_primary.lighten(0.1),
                InteractionState::Disabled => palette.accent_primary.with_alpha(0.5),
                _ => palette.accent_primary,
            },
            ButtonVariant::Secondary => match state {
                InteractionState::Idle => palette.interactive,
                InteractionState::Hovered => palette.interactive_hover,
                InteractionState::Pressed => palette.interactive_pressed,
                InteractionState::Focused => palette.interactive,
                InteractionState::FocusedHovered => palette.interactive_hover,
                InteractionState::Disabled => palette.interactive_disabled,
                _ => palette.interactive,
            },
            ButtonVariant::Ghost => match state {
                InteractionState::Idle => Color::TRANSPARENT,
                InteractionState::Hovered => palette.interactive_hover.with_alpha(0.3),
                InteractionState::Pressed => palette.interactive_pressed.with_alpha(0.4),
                InteractionState::Focused => Color::TRANSPARENT,
                InteractionState::FocusedHovered => palette.interactive_hover.with_alpha(0.3),
                InteractionState::Disabled => Color::TRANSPARENT,
                _ => Color::TRANSPARENT,
            },
            ButtonVariant::Danger => match state {
                InteractionState::Idle => palette.error,
                InteractionState::Hovered => palette.error.lighten(0.1),
                InteractionState::Pressed => palette.error.darken(0.1),
                InteractionState::Focused => palette.error,
                InteractionState::FocusedHovered => palette.error.lighten(0.1),
                InteractionState::Disabled => palette.error.with_alpha(0.5),
                _ => palette.error,
            },
        }
    }

    fn fg_color_for_state(&self) -> Color {
        if self.interaction.state().is_disabled() {
            return self.palette.fg_muted;
        }

        match self.props.variant {
            ButtonVariant::Primary | ButtonVariant::Danger => self.palette.fg_inverse,
            ButtonVariant::Secondary | ButtonVariant::Ghost => self.palette.fg_primary,
        }
    }

    fn update_animation_targets(&mut self) {
        let state = self.interaction.state();
        let target_color = Self::bg_color_for_state(&self.props, state, &self.palette);

        // Only start animation if target changed
        if self.bg_color_tween.target() != &target_color {
            self.bg_color_tween.retarget(target_color);
            self.bg_color_tween.start();
        }

        // Scale animation for press feedback
        let target_scale = if state.is_pressed() { 0.97 } else { 1.0 };
        if (self.scale_tween.target() - target_scale).abs() > 0.001 {
            self.scale_tween.retarget(target_scale);
            self.scale_tween.start();
        }
    }

    /// Calculate the natural size of this button.
    fn natural_size(&self) -> (f32, f32) {
        let height = self.props.size.height();
        let padding_x = self.props.size.padding_x();
        let font_size = self.props.size.font_size();

        // Estimate text width (rough approximation)
        let char_width = font_size * 0.6;
        let text_width = self.props.label.len() as f32 * char_width;

        let width = text_width + padding_x * 2.0;
        (width, height)
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, _dt: f32) -> bool {
        let color_changed = self.bg_color_tween.update();
        let scale_changed = self.scale_tween.update();
        color_changed || scale_changed
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let (natural_w, natural_h) = self.natural_size();

        let width = if self.props.full_width {
            constraints.max_width
        } else {
            natural_w.clamp(constraints.min_width, constraints.max_width)
        };

        let height = natural_h.clamp(constraints.min_height, constraints.max_height);
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

        let scale = *self.scale_tween.value();
        let bg_color = *self.bg_color_tween.value();
        let fg_color = self.fg_color_for_state();

        // Apply scale transform (scale from center)
        let scaled_width = self.bounds.width * scale;
        let scaled_height = self.bounds.height * scale;
        let offset_x = (self.bounds.width - scaled_width) / 2.0;
        let offset_y = (self.bounds.height - scaled_height) / 2.0;

        let scaled_bounds = Bounds {
            x: self.bounds.x + offset_x,
            y: self.bounds.y + offset_y,
            width: scaled_width,
            height: scaled_height,
        };

        // Background rectangle
        let radius = Radius::Md.px();
        let mut rect = StyledRect::new(scaled_bounds)
            .with_fill(bg_color)
            .with_corner_radii(CornerRadii::all(radius));

        // Add focus ring
        if self.interaction.state().is_focused() {
            rect = rect.with_border(2.0, self.palette.focus);
        }

        // Add subtle shadow for Primary variant
        if self.props.variant == ButtonVariant::Primary && !self.props.disabled {
            rect = rect.with_elevation(Elevation::Low);
        }

        output.add_rect(rect);

        // Text label
        let text_bounds = Bounds {
            x: scaled_bounds.x,
            y: scaled_bounds.y,
            width: scaled_bounds.width,
            height: scaled_bounds.height,
        };

        let text = TextBlock::new(&self.props.label, text_bounds)
            .with_color(fg_color)
            .with_font_size(self.props.size.font_size())
            .with_align(TextAlign::Center);

        output.add_text(text);

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        if self.props.disabled {
            return false;
        }

        match event {
            PointerEvent::Enter => {
                self.interaction.on_pointer_enter();
                self.update_animation_targets();
                true
            }
            PointerEvent::Leave => {
                self.interaction.on_pointer_leave();
                self.update_animation_targets();
                true
            }
            PointerEvent::Down { button: PointerButton::Left, .. } => {
                self.interaction.on_pointer_down();
                self.update_animation_targets();
                true
            }
            PointerEvent::Up { button: PointerButton::Left, .. } => {
                let was_pressed = self.interaction.on_pointer_up();
                self.update_animation_targets();
                was_pressed
            }
            PointerEvent::Click { button: PointerButton::Left, .. } => {
                // Click event handled in build() by checking interaction state
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
                // Simulate click on Enter/Space when focused
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
        self.update_animation_targets();
    }

    fn can_focus(&self) -> bool {
        !self.props.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_creation() {
        let button = Button::primary("Click me");
        assert_eq!(button.props.label, "Click me");
        assert_eq!(button.props.variant, ButtonVariant::Primary);
    }

    #[test]
    fn test_button_disabled() {
        let mut button = Button::primary("Click me");
        button.set_disabled(true);
        assert!(button.interaction.state().is_disabled());
    }

    #[test]
    fn test_button_layout() {
        let mut button = Button::primary("OK");
        let constraints = LayoutConstraints::loose(200.0, 50.0);
        let (w, h) = button.layout(constraints);
        assert!(w > 0.0);
        assert_eq!(h, ComponentSize::Md.height());
    }
}
