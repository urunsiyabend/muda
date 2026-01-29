use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;

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
    pub fn style(&self, state: ButtonState) -> ButtonStyle {
        match (self, state) {
            // Primary - blue tones
            (Self::Primary, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.25, 0.45, 0.85),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.20, 0.38, 0.75),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.15, 0.30, 0.65),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Primary, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.25, 0.45, 0.85, 0.5),
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border_color: None,
                border_width: 0.0,
            },

            // Secondary - gray tones
            (Self::Secondary, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.90, 0.90, 0.91),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.85, 0.85, 0.86),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.78, 0.78, 0.80),
                text: Color::rgb(0.1, 0.1, 0.1),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Secondary, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.90, 0.90, 0.91, 0.5),
                text: Color::rgba(0.1, 0.1, 0.1, 0.4),
                border_color: None,
                border_width: 0.0,
            },

            // Ghost - transparent with hover
            (Self::Ghost, ButtonState::Enabled) => ButtonStyle {
                bg: Color::transparent(),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgba(1.0, 1.0, 1.0, 0.1),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Active) => ButtonStyle {
                bg: Color::rgba(1.0, 1.0, 1.0, 0.2),
                text: Color::rgb(0.9, 0.9, 0.9),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Ghost, ButtonState::Disabled) => ButtonStyle {
                bg: Color::transparent(),
                text: Color::rgba(0.9, 0.9, 0.9, 0.4),
                border_color: None,
                border_width: 0.0,
            },

            // Destructive - red tones
            (Self::Destructive, ButtonState::Enabled) => ButtonStyle {
                bg: Color::rgb(0.85, 0.20, 0.20),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Hover) => ButtonStyle {
                bg: Color::rgb(0.75, 0.15, 0.15),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Active) => ButtonStyle {
                bg: Color::rgb(0.65, 0.10, 0.10),
                text: Color::white(),
                border_color: None,
                border_width: 0.0,
            },
            (Self::Destructive, ButtonState::Disabled) => ButtonStyle {
                bg: Color::rgba(0.85, 0.20, 0.20, 0.5),
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border_color: None,
                border_width: 0.0,
            },
        }
    }
}

/// Interactive button element with variant-based styling
pub struct Button {
    label: String,
    variant: ButtonVariant,
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
    /// Create a new button with the given label (defaults to Primary variant)
    pub fn new(label: String) -> Self {
        Self {
            label,
            variant: ButtonVariant::Primary,
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
        // Create style for button container
        let mut style = Style::default();
        style.padding = Edges::xy(16.0, 8.0); // px(16) horizontal, px(8) vertical
        style.border_radius = Corners::all(4.0);
        style.display = Display::Flex;
        style.justify_content = JustifyContent::Center;
        style.align_items = AlignItems::Center;

        let layout_id = cx.request_layout(&style);

        // Create text element with appropriate color for the current variant
        // Note: We use default enabled state color here since we can't update color dynamically
        let default_style = self.variant.style(ButtonState::Enabled);
        let mut text_element = TextElement::new(self.label.clone())
            .size(14.0)
            .color(default_style.text);

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

        // Get variant-specific style
        let button_style = self.variant.style(button_state);

        // Build Style for rendering
        let mut style = Style::default();
        style.background = Background::Solid(button_style.bg);
        style.border_radius = Corners::all(4.0);
        if let Some(border_color) = button_style.border_color {
            style.border.color = border_color;
            style.border.widths = Edges::all(button_style.border_width);
        }

        // Paint button background
        cx.paint_styled_rect(&style, &bounds);

        // Paint text element
        if let Some(text_element) = &mut self.text_element {
            text_element.paint(&mut state.text_state, cx);
        }
    }
}
