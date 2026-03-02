use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::mouse::HitboxId;
use crate::style::*;
use crate::theme::ColorToken;

const LIST_ITEM_HEIGHT_MD: f32 = 32.0;
const LIST_ITEM_PADDING_H: f32 = 12.0;
const LIST_ITEM_FONT_SIZE: f32 = 13.0;

/// Selectable list row widget used in command palettes, context menus, etc.
pub struct ListItem {
    label: String,
    secondary_text: Option<String>,
    is_selected: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn() + 'static>>,
    // Internal text elements (created during request_layout)
    label_element: Option<TextElement>,
    secondary_element: Option<TextElement>,
}

/// Constructor function for ListItem
pub fn list_item(label: impl Into<String>) -> ListItem {
    ListItem {
        label: label.into(),
        secondary_text: None,
        is_selected: false,
        disabled: false,
        on_click: None,
        label_element: None,
        secondary_element: None,
    }
}

impl ListItem {
    /// Set optional right-aligned secondary text (e.g., keyboard shortcut)
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary_text = Some(text.into());
        self
    }

    /// Highlight this item as selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.is_selected = selected;
        self
    }

    /// Dim the item and make it non-interactive
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Attach a click handler
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

/// State persisted through the rendering lifecycle
pub struct ListItemState {
    layout_id: LayoutId,
    label_state: TextState,
    secondary_state: Option<TextState>,
    hitbox_id: Option<HitboxId>,
}

impl Element for ListItem {
    type RequestLayoutState = ListItemState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Container: flex row, space-between, center-aligned, fixed height
        let mut style = Style::default();
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Row;
        style.justify_content = JustifyContent::SpaceBetween;
        style.align_items = AlignItems::Center;
        style.min_height = Length::Px(LIST_ITEM_HEIGHT_MD);
        style.padding = Edges {
            top: 0.0,
            bottom: 0.0,
            left: LIST_ITEM_PADDING_H,
            right: LIST_ITEM_PADDING_H,
        };

        let layout_id = cx.request_layout(&style);

        // Label text (left-aligned, grows to fill space)
        let mut label_element = TextElement::new(self.label.clone())
            .size(LIST_ITEM_FONT_SIZE)
            .color(Color::white())
            .grow(1.0);
        let (label_layout_id, label_state) = label_element.request_layout(cx);
        cx.add_child(layout_id, label_layout_id);
        self.label_element = Some(label_element);

        // Optional secondary text (right-aligned)
        let secondary_state = if let Some(ref sec_text) = self.secondary_text {
            let mut sec_element = TextElement::new(sec_text.clone())
                .size(LIST_ITEM_FONT_SIZE)
                .color(Color::white());
            let (sec_layout_id, ss) = sec_element.request_layout(cx);
            cx.add_child(layout_id, sec_layout_id);
            self.secondary_element = Some(sec_element);
            Some(ss)
        } else {
            None
        };

        (
            layout_id,
            ListItemState {
                layout_id,
                label_state,
                secondary_state,
                hitbox_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Prepaint child text elements
        if let Some(label_el) = &mut self.label_element {
            label_el.prepaint(&mut state.label_state, cx);
        }
        if let (Some(sec_el), Some(sec_st)) =
            (&mut self.secondary_element, &mut state.secondary_state)
        {
            sec_el.prepaint(sec_st, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Determine interaction state before borrowing theme
        let is_hovered = state.hitbox_id.map(|id| cx.is_hovered(id)).unwrap_or(false);

        // Extract colors from theme up front
        let bg_color = if self.disabled {
            Color::transparent()
        } else if self.is_selected {
            let mut c = cx.theme().color(ColorToken::Accent);
            c.a = 0.15; // low-alpha accent for selected state
            c
        } else if is_hovered {
            cx.theme().color(ColorToken::BgElevated)
        } else {
            Color::transparent()
        };

        let label_color = if self.disabled {
            let mut c = cx.theme().color(ColorToken::FgPrimary);
            c.a = 0.5; // 50% alpha for disabled
            c
        } else {
            cx.theme().color(ColorToken::FgPrimary)
        };

        let secondary_color = if self.disabled {
            let mut c = cx.theme().color(ColorToken::FgMuted);
            c.a = 0.5;
            c
        } else {
            cx.theme().color(ColorToken::FgMuted)
        };

        // Paint background if non-transparent
        if bg_color.a > 0.0 {
            let mut bg_style = Style::default();
            bg_style.background = Background::Solid(bg_color);
            cx.paint_styled_rect(&bg_style, &bounds);
        }

        // Paint label text
        if let Some(label_el) = &mut self.label_element {
            label_el.set_color(label_color);
            label_el.paint(&mut state.label_state, cx);
        }

        // Paint secondary text (muted, right-aligned)
        if let Some(sec_el) = &mut self.secondary_element {
            sec_el.set_color(secondary_color);
            if let Some(sec_st) = &mut state.secondary_state {
                sec_el.paint(sec_st, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_item_selected() {
        let item = list_item("Test").selected(true);
        assert!(item.is_selected);
        assert_eq!(item.label, "Test");
        assert!(!item.disabled);
    }

    #[test]
    fn test_list_item_disabled() {
        let item = list_item("Test").disabled(true);
        assert!(item.disabled);
        assert_eq!(item.label, "Test");
        assert!(!item.is_selected);
    }
}
