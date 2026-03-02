use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::button::WidgetSize;
use crate::elements::text::{TextElement, TextState};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;
use crate::theme::ColorToken;

/// Text input element with placeholder, focus ring, and themed colors.
///
/// Follows the composite element pattern (like Button): stores a TextElement
/// internally for lifecycle control across request_layout/prepaint/paint.
pub struct Input {
    value: String,
    placeholder: String,
    size: WidgetSize,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    on_change: Option<Box<dyn Fn(&str) + 'static>>,
    text_element: Option<TextElement>,
}

/// Constructor function for creating an Input element.
pub fn input(placeholder: impl Into<String>) -> Input {
    Input::new(placeholder.into())
}

impl Input {
    /// Create a new Input with the given placeholder text.
    /// Defaults to empty value, Md size, not disabled.
    pub fn new(placeholder: String) -> Self {
        Self {
            value: String::new(),
            placeholder,
            size: WidgetSize::Md,
            disabled: false,
            focus_handle: None,
            on_change: None,
            text_element: None,
        }
    }

    /// Set the current value displayed in the input.
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }

    /// Set the size tier (Sm/Md/Lg).
    pub fn size(mut self, s: WidgetSize) -> Self {
        self.size = s;
        self
    }

    /// Enable or disable the input.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    /// Make the input focusable with the given handle.
    pub fn focusable(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Attach a change callback invoked when value changes.
    pub fn on_change(mut self, handler: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

/// State persisted through the rendering lifecycle for a single frame.
pub struct InputState {
    layout_id: LayoutId,
    text_layout_id: LayoutId,
    text_state: TextState,
    hitbox_id: Option<HitboxId>,
    focus_id: Option<FocusId>,
}

impl Element for Input {
    type RequestLayoutState = InputState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Build container style using WidgetSize configuration
        let mut style = Style::default();
        style.padding = Edges::xy(self.size.padding_h(), self.size.padding_v());
        style.border_radius = Corners::all(self.size.border_radius());
        style.border.widths = Edges::all(1.0);
        // Use a placeholder border color; actual color is applied in paint
        style.border.color = Color::transparent();
        style.display = Display::Flex;
        style.align_items = AlignItems::Center;
        style.min_height = Length::Px(self.size.height());

        let layout_id = cx.request_layout(&style);

        // Determine display text: value if set, otherwise placeholder
        let display_text = if self.value.is_empty() {
            self.placeholder.clone()
        } else {
            self.value.clone()
        };

        // Create text element with placeholder color; color updated during paint
        let mut text_element = TextElement::new(display_text)
            .size(self.size.font_size())
            .color(Color::white()); // Will be overridden in paint

        // Request layout for text child
        let (text_layout_id, text_state) = text_element.request_layout(cx);
        cx.add_child(layout_id, text_layout_id);

        // Store text element for use in prepaint/paint
        self.text_element = Some(text_element);

        (
            layout_id,
            InputState {
                layout_id,
                text_layout_id,
                text_state,
                hitbox_id: None,
                focus_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        // Opaque hitbox so the input captures mouse events
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Register as focusable if a focus handle was provided
        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
            state.focus_id = Some(focus_handle.id);
        }

        // Prepaint text child
        if let Some(text_element) = &mut self.text_element {
            text_element.prepaint(&mut state.text_state, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Determine if the input is keyboard-focused
        let is_focused = state
            .focus_id
            .map(|id| cx.is_focused(id))
            .unwrap_or(false);

        // Resolve interaction state priority: disabled > focused > normal
        // Complete all mutable cx operations (is_hovered, is_active, etc.) before
        // calling cx.theme() to avoid aliased borrows (borrow-safe pattern).
        let is_hovered = state
            .hitbox_id
            .map(|id| cx.is_hovered(id))
            .unwrap_or(false);

        // --- Resolve colors from theme ---
        // Apply borrow-safe pattern: obtain theme colors into owned values first,
        // then perform all mutable paint operations.
        let bg_token = if self.disabled {
            ColorToken::BgSecondary
        } else {
            ColorToken::BgPrimary
        };

        let border_token = if self.disabled {
            ColorToken::BgSecondary
        } else if is_focused {
            ColorToken::Accent
        } else if is_hovered {
            ColorToken::FgMuted
        } else {
            ColorToken::Border
        };

        let text_token = if self.value.is_empty() {
            ColorToken::FgMuted
        } else {
            ColorToken::FgPrimary
        };

        // Collect theme colors into owned values before any mutable borrow of cx
        let mut bg_color = cx.theme().color(bg_token);
        let border_color = cx.theme().color(border_token);
        let mut text_color = cx.theme().color(text_token);

        // Apply alpha reduction for disabled state
        if self.disabled {
            bg_color.a *= 0.5;
            text_color.a *= 0.5;
        }

        // Build Style for background and border
        let mut style = Style::default();
        style.background = Background::Solid(bg_color);
        style.border_radius = Corners::all(self.size.border_radius());
        style.border.color = border_color;
        style.border.widths = Edges::all(1.0);

        // Paint background with border
        cx.paint_styled_rect(&style, &bounds);

        // Update text color based on value/placeholder status and paint text child
        if let Some(text_element) = &mut self.text_element {
            text_element.set_color(text_color);
            text_element.paint(&mut state.text_state, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_creation() {
        let inp = Input::new("Enter text...".to_string());
        assert_eq!(inp.placeholder, "Enter text...");
        assert_eq!(inp.value, "");
        assert_eq!(inp.size, WidgetSize::Md);
        assert!(!inp.disabled);
        assert!(inp.focus_handle.is_none());
        assert!(inp.on_change.is_none());
        assert!(inp.text_element.is_none());
    }

    #[test]
    fn test_input_builder() {
        let inp = input("Search...")
            .value("hello")
            .size(WidgetSize::Lg)
            .disabled(true);

        assert_eq!(inp.placeholder, "Search...");
        assert_eq!(inp.value, "hello");
        assert_eq!(inp.size, WidgetSize::Lg);
        assert!(inp.disabled);
    }
}
