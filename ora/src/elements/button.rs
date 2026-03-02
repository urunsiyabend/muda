use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;
use crate::theme::{ColorToken, Theme};

/// Shared size tiers used by all Phase 8 widgets (buttons, inputs, checkboxes, etc.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetSize {
    Sm,        // compact: height=24px, font=12px, padding_h=8px, padding_v=4px, border_radius=3px
    #[default]
    Md,        // standard: height=32px, font=13px, padding_h=12px, padding_v=6px, border_radius=4px
    Lg,        // prominent: height=40px, font=14px, padding_h=16px, padding_v=8px, border_radius=5px
}

impl WidgetSize {
    pub fn height(&self) -> f32 {
        match self { Self::Sm => 24.0, Self::Md => 32.0, Self::Lg => 40.0 }
    }
    pub fn font_size(&self) -> f32 {
        match self { Self::Sm => 12.0, Self::Md => 13.0, Self::Lg => 14.0 }
    }
    pub fn padding_h(&self) -> f32 {
        match self { Self::Sm => 8.0, Self::Md => 12.0, Self::Lg => 16.0 }
    }
    pub fn padding_v(&self) -> f32 {
        match self { Self::Sm => 4.0, Self::Md => 6.0, Self::Lg => 8.0 }
    }
    pub fn border_radius(&self) -> f32 {
        match self { Self::Sm => 3.0, Self::Md => 4.0, Self::Lg => 5.0 }
    }
}

/// Button visual variants
#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

/// Button interaction states for styling
#[derive(Clone, Copy, Debug)]
pub enum ButtonState {
    Enabled,
    Hover,
    Active,
    Disabled,
}

/// Internal styling configuration for a button variant in a given state
struct ButtonStyle {
    bg: Color,
    text: Color,
    border_color: Option<Color>,
    border_width: f32,
}

impl ButtonVariant {
    /// Get the style configuration for this variant in the given state
    pub fn style(&self, state: ButtonState, theme: &Theme) -> ButtonStyle {
        match (self, state) {
            // Primary - uses accent colors from theme
            (Self::Primary, ButtonState::Enabled) => ButtonStyle {
                bg: theme.color(ColorToken::Accent),
                text: theme.color(ColorToken::BgPrimary),  // Contrast with accent
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Hover) => ButtonStyle {
                bg: theme.color(ColorToken::AccentHover),
                text: theme.color(ColorToken::BgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Active) => ButtonStyle {
                bg: theme.color(ColorToken::AccentActive),
                text: theme.color(ColorToken::BgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Disabled) => {
                let mut bg = theme.color(ColorToken::Accent);
                bg.a = 0.5;
                let mut text = theme.color(ColorToken::BgPrimary);
                text.a = 0.6;
                ButtonStyle { bg, text, border_color: None, border_width: 0.0 }
            },

            // Secondary - uses bg/fg colors from theme
            (Self::Secondary, ButtonState::Enabled) => ButtonStyle {
                bg: theme.color(ColorToken::BgSecondary),
                text: theme.color(ColorToken::FgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Hover) => ButtonStyle {
                bg: theme.color(ColorToken::BgElevated),
                text: theme.color(ColorToken::FgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Active) => ButtonStyle {
                bg: theme.color(ColorToken::Border),
                text: theme.color(ColorToken::FgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Disabled) => {
                let mut bg = theme.color(ColorToken::BgSecondary);
                bg.a = 0.5;
                let mut text = theme.color(ColorToken::FgPrimary);
                text.a = 0.4;
                ButtonStyle { bg, text, border_color: None, border_width: 0.0 }
            },

            // Ghost - transparent with hover
            (Self::Ghost, ButtonState::Enabled) => ButtonStyle {
                bg: Color::transparent(),
                text: theme.color(ColorToken::FgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Hover) => {
                let mut bg = theme.color(ColorToken::FgPrimary);
                bg.a = 0.1;
                ButtonStyle {
                    bg,
                    text: theme.color(ColorToken::FgPrimary),
                    border_color: None,
                    border_width: 0.0,
                }
            },
            (Self::Ghost, ButtonState::Active) => {
                let mut bg = theme.color(ColorToken::FgPrimary);
                bg.a = 0.2;
                ButtonStyle {
                    bg,
                    text: theme.color(ColorToken::FgPrimary),
                    border_color: None,
                    border_width: 0.0,
                }
            },
            (Self::Ghost, ButtonState::Disabled) => {
                let mut text = theme.color(ColorToken::FgPrimary);
                text.a = 0.4;
                ButtonStyle {
                    bg: Color::transparent(),
                    text,
                    border_color: None,
                    border_width: 0.0,
                }
            },

            // Destructive - uses error color from theme
            (Self::Destructive, ButtonState::Enabled) => ButtonStyle {
                bg: theme.color(ColorToken::Error),
                text: theme.color(ColorToken::BgPrimary),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Hover) => {
                // Slightly darker error for hover
                let mut bg = theme.color(ColorToken::Error);
                bg.r *= 0.9;
                bg.g *= 0.9;
                bg.b *= 0.9;
                ButtonStyle {
                    bg,
                    text: theme.color(ColorToken::BgPrimary),
                    border_color: None,
                    border_width: 0.0,
                }
            },
            (Self::Destructive, ButtonState::Active) => {
                let mut bg = theme.color(ColorToken::Error);
                bg.r *= 0.8;
                bg.g *= 0.8;
                bg.b *= 0.8;
                ButtonStyle {
                    bg,
                    text: theme.color(ColorToken::BgPrimary),
                    border_color: None,
                    border_width: 0.0,
                }
            },
            (Self::Destructive, ButtonState::Disabled) => {
                let mut bg = theme.color(ColorToken::Error);
                bg.a = 0.5;
                let mut text = theme.color(ColorToken::BgPrimary);
                text.a = 0.6;
                ButtonStyle { bg, text, border_color: None, border_width: 0.0 }
            },
        }
    }
}

/// Interactive button element with variant-based styling
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: WidgetSize,
    disabled: bool,
    on_click: Option<Box<dyn Fn() + 'static>>,
    focus_handle: Option<FocusHandle>,
    text_element: Option<TextElement>,
}

/// Constructor function for button
pub fn button(label: impl Into<String>) -> Button {
    Button::new(label.into())
}

impl Button {
    /// Create a new button with the given label (defaults to Primary variant, Md size)
    pub fn new(label: String) -> Self {
        Self {
            label,
            variant: ButtonVariant::Primary,
            size: WidgetSize::Md,
            disabled: false,
            on_click: None,
            focus_handle: None,
            text_element: None,
        }
    }

    /// Set button to primary variant (blue)
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// Set button to secondary variant (gray)
    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    /// Set button to ghost variant (transparent)
    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    /// Set button to destructive variant (red)
    pub fn destructive(mut self) -> Self {
        self.variant = ButtonVariant::Destructive;
        self
    }

    /// Set the size tier (sm/md/lg)
    pub fn size(mut self, size: WidgetSize) -> Self {
        self.size = size;
        self
    }

    /// Enable or disable the button
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Attach a click handler
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Make the button focusable
    pub fn focusable(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }
}

/// State persisted through the rendering lifecycle
pub struct ButtonElementState {
    layout_id: LayoutId,
    text_layout_id: LayoutId,
    text_state: TextState,
    hitbox_id: Option<HitboxId>,
    focus_id: Option<FocusId>,
}

impl Element for Button {
    type RequestLayoutState = ButtonElementState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Create style for button container using WidgetSize values
        let mut style = Style::default();
        style.padding = Edges::xy(self.size.padding_h(), self.size.padding_v());
        style.border_radius = Corners::all(self.size.border_radius());
        style.display = Display::Flex;
        style.justify_content = JustifyContent::Center;
        style.align_items = AlignItems::Center;
        style.min_height = Length::Px(self.size.height());

        let layout_id = cx.request_layout(&style);

        // Create text element with a placeholder color
        // Note: Text color will be set during paint when we have access to theme
        let mut text_element = TextElement::new(self.label.clone())
            .size(self.size.font_size())
            .color(Color::white()); // Placeholder, will be updated in paint

        // Request layout for text child
        let (text_layout_id, text_state) = text_element.request_layout(cx);
        cx.add_child(layout_id, text_layout_id);

        // Store text element for paint phase
        self.text_element = Some(text_element);

        (layout_id, ButtonElementState {
            layout_id,
            text_layout_id,
            text_state,
            hitbox_id: None,
            focus_id: None,
        })
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true); // opaque
        state.hitbox_id = Some(hitbox_id);

        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
            state.focus_id = Some(focus_handle.id);
        }

        // Prepaint text element
        if let Some(text_element) = &mut self.text_element {
            text_element.prepaint(&mut state.text_state, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Determine button state from interaction state
        let button_state = if self.disabled {
            ButtonState::Disabled
        } else if let Some(hitbox_id) = state.hitbox_id {
            if cx.is_active(hitbox_id) {
                ButtonState::Active
            } else if cx.is_hovered(hitbox_id) {
                ButtonState::Hover
            } else {
                ButtonState::Enabled
            }
        } else {
            ButtonState::Enabled
        };

        // Get variant-specific style WITH THEME
        let button_style = self.variant.style(button_state, cx.theme());

        // Build Style for rendering
        let mut style = Style::default();
        style.background = Background::Solid(button_style.bg);
        style.border_radius = Corners::all(self.size.border_radius());
        if let Some(border_color) = button_style.border_color {
            style.border.color = border_color;
            style.border.widths = Edges::all(button_style.border_width);
        }

        // Paint button background
        cx.paint_styled_rect(&style, &bounds);

        // Update text color based on theme before painting
        if let Some(text_element) = &mut self.text_element {
            text_element.set_color(button_style.text);
            text_element.paint(&mut state.text_state, cx);
        }
    }
}
