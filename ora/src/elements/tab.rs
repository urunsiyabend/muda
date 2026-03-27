use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::mouse::HitboxId;
use crate::events::MouseButton;
use crate::style::*;
use crate::theme::ColorToken;
use std::rc::Rc;

const TAB_HEIGHT: f32 = 36.0;
const TAB_PADDING_H: f32 = 12.0;
const TAB_FONT_SIZE: f32 = 13.0;
const CLOSE_BUTTON_SIZE: f32 = 16.0;

/// Individual tab widget with active/inactive styling and an optional close button.
pub struct Tab {
    label: String,
    is_active: bool,
    is_dirty: bool,
    /// Whether the underlying file was deleted externally.
    ///
    /// When `true`, the tab title is rendered with a muted/red-ish color to
    /// indicate the file no longer exists on disk.
    is_deleted: bool,
    show_close: bool,
    on_click: Option<Rc<dyn Fn() + 'static>>,
    on_close: Option<Rc<dyn Fn() + 'static>>,
    // Internal text elements (created during request_layout)
    label_element: Option<TextElement>,
    close_element: Option<TextElement>,
}

/// Constructor function for Tab
pub fn tab(label: impl Into<String>) -> Tab {
    Tab {
        label: label.into(),
        is_active: false,
        is_dirty: false,
        is_deleted: false,
        show_close: true,
        on_click: None,
        on_close: None,
        label_element: None,
        close_element: None,
    }
}

impl Tab {
    /// Set the active state of this tab
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Show or hide the dirty indicator (dot before label)
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.is_dirty = dirty;
        self
    }

    /// Mark tab as deleted (file no longer exists on disk).
    ///
    /// When `true`, the tab title renders with a muted/warning color and a
    /// "(deleted)" suffix to inform the user that the file is gone.
    pub fn deleted(mut self, deleted: bool) -> Self {
        self.is_deleted = deleted;
        self
    }

    /// Show or hide the close button (default: true)
    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Attach a click handler for tab selection
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Attach a handler for the close button
    pub fn on_close(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

/// State persisted through the rendering lifecycle
pub struct TabElementState {
    layout_id: LayoutId,
    label_state: TextState,
    close_state: Option<TextState>,
    hitbox_id: Option<HitboxId>,
    close_hitbox_id: Option<HitboxId>,
}

impl Element for Tab {
    type RequestLayoutState = TabElementState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Build container style: flex row, fixed height, horizontal padding
        let mut style = Style::default();
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Row;
        style.align_items = AlignItems::Center;
        style.min_height = Length::Px(TAB_HEIGHT);
        style.padding = Edges {
            top: 0.0,
            bottom: 0.0,
            left: TAB_PADDING_H,
            right: TAB_PADDING_H,
        };
        style.gap = 4.0;

        let layout_id = cx.request_layout(&style);

        // Build label text.
        // Deleted takes precedence over dirty for the suffix/prefix display.
        let label_text = if self.is_deleted {
            format!("{} (deleted)", self.label)
        } else if self.is_dirty {
            format!("* {}", self.label)
        } else {
            self.label.clone()
        };

        let mut label_element = TextElement::new(label_text)
            .size(TAB_FONT_SIZE)
            .color(Color::white());
        let (label_layout_id, label_state) = label_element.request_layout(cx);
        cx.add_child(layout_id, label_layout_id);
        self.label_element = Some(label_element);

        // Optionally create close button "x"
        let close_state = if self.show_close {
            let mut close_element = TextElement::new("x")
                .size(CLOSE_BUTTON_SIZE * 0.75)
                .color(Color::white());
            let (close_layout_id, cs) = close_element.request_layout(cx);
            cx.add_child(layout_id, close_layout_id);
            self.close_element = Some(close_element);
            Some(cs)
        } else {
            None
        };

        (
            layout_id,
            TabElementState {
                layout_id,
                label_state,
                close_state,
                hitbox_id: None,
                close_hitbox_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Compute close button bounds (right side) before registering hitboxes
        let close_bounds = if self.show_close {
            Some(Rect {
                origin: Point {
                    x: bounds.origin.x + bounds.size.width - CLOSE_BUTTON_SIZE - TAB_PADDING_H,
                    y: bounds.origin.y + (TAB_HEIGHT - CLOSE_BUTTON_SIZE) / 2.0,
                },
                size: Size {
                    width: CLOSE_BUTTON_SIZE,
                    height: CLOSE_BUTTON_SIZE,
                },
            })
        } else {
            None
        };

        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        if let Some(cb) = close_bounds {
            let close_hitbox_id = cx.register_hitbox(cb, true);
            state.close_hitbox_id = Some(close_hitbox_id);

            if let Some(on_close) = self.on_close.clone() {
                cx.on_mouse_down(close_hitbox_id, move |event, ctx| {
                    if ctx.phase() != crate::events::dispatch::DispatchPhase::Bubble {
                        return;
                    }
                    if matches!(event.button, MouseButton::Left | MouseButton::Middle) {
                        on_close();
                    }
                });
            }
        }

        let on_click = self.on_click.clone();
        let on_close_for_middle = self.on_close.clone();
        cx.on_mouse_down(hitbox_id, move |event, ctx| {
            if ctx.phase() != crate::events::dispatch::DispatchPhase::Bubble {
                return;
            }
            match event.button {
                MouseButton::Left => {
                    if let Some(handler) = &on_click {
                        handler();
                    }
                }
                MouseButton::Middle => {
                    if let Some(handler) = &on_close_for_middle {
                        handler();
                    }
                }
                _ => {}
            }
        });

        if let Some(label_el) = &mut self.label_element {
            label_el.prepaint(&mut state.label_state, cx);
        }
        if let (Some(close_el), Some(close_st)) = (&mut self.close_element, &mut state.close_state)
        {
            close_el.prepaint(close_st, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        let is_hovered = state.hitbox_id.map(|id| cx.is_hovered(id)).unwrap_or(false);
        let close_hovered = state.close_hitbox_id.map(|id| cx.is_hovered(id)).unwrap_or(false);

        let bg_color = if self.is_active {
            cx.theme().color(ColorToken::BgPrimary)
        } else if is_hovered {
            cx.theme().color(ColorToken::BgElevated)
        } else {
            cx.theme().color(ColorToken::BgSecondary)
        };

        let text_color = if self.is_deleted {
            // Deleted files show with a muted warning tone to indicate the file is gone.
            cx.theme().color(ColorToken::Warning)
        } else if self.is_active {
            cx.theme().color(ColorToken::FgPrimary)
        } else {
            cx.theme().color(ColorToken::FgSecondary)
        };

        let accent_color = cx.theme().color(ColorToken::Accent);
        let muted_color = cx.theme().color(ColorToken::FgMuted);
        let fg_primary = cx.theme().color(ColorToken::FgPrimary);
        let border_color = cx.theme().color(ColorToken::Border);

        let mut bg_style = Style::default();
        bg_style.background = Background::Solid(bg_color);
        bg_style.border.widths = Edges {
            top: 0.0,
            right: 1.0,
            bottom: 0.0,
            left: 0.0,
        };
        bg_style.border.color = border_color;

        if self.is_active {
            bg_style.border.widths = Edges {
                top: 0.0,
                right: 1.0,
                bottom: 2.0,
                left: 0.0,
            };
            bg_style.border.color = accent_color;
        }

        cx.paint_styled_rect(&bg_style, &bounds);

        if let Some(label_el) = &mut self.label_element {
            label_el.set_color(text_color);
            label_el.paint(&mut state.label_state, cx);
        }

        if self.show_close {
            let close_color = if close_hovered { fg_primary } else { muted_color };

            if let Some(close_el) = &mut self.close_element {
                close_el.set_color(close_color);
                if let Some(close_st) = &mut state.close_state {
                    close_el.paint(close_st, cx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_creation() {
        let t = tab("main.rs");
        assert_eq!(t.label, "main.rs");
        assert!(!t.is_active);
        assert!(!t.is_dirty);
        assert!(t.show_close);
        assert!(t.on_click.is_none());
        assert!(t.on_close.is_none());
    }

    #[test]
    fn test_tab_active_state() {
        let t = tab("Test").active(true).dirty(true);
        assert!(t.is_active);
        assert!(t.is_dirty);
        assert_eq!(t.label, "Test");
    }

    #[test]
    fn test_tab_callbacks() {
        let clicked = std::cell::Cell::new(false);
        let t = tab("test.rs").on_click(move || {
            clicked.set(true);
        });
        assert!(t.on_click.is_some());
    }
}
