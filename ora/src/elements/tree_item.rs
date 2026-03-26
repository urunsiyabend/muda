use std::rc::Rc;

use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::mouse::HitboxId;
use crate::events::MouseButton;
use crate::style::*;
use crate::theme::ColorToken;

const TREE_ITEM_HEIGHT: f32 = 24.0;
const INDENT_WIDTH: f32 = 16.0;
const CHEVRON_WIDTH: f32 = 16.0;
const ICON_WIDTH: f32 = 16.0;
const TREE_FONT_SIZE: f32 = 12.0;

/// Tree node widget with expand/collapse chevron and depth-based indentation.
pub struct TreeItem {
    label: String,
    depth: usize,
    is_dir: bool,
    is_expanded: bool,
    is_selected: bool,
    is_generated: bool,
    icon_color: Option<Color>,
    on_click: Option<Rc<dyn Fn() + 'static>>,
    on_toggle: Option<Rc<dyn Fn() + 'static>>,
    // Internal text elements (created during request_layout)
    chevron_element: Option<TextElement>,
    icon_element: Option<TextElement>,
    label_element: Option<TextElement>,
}

/// Constructor function for TreeItem
pub fn tree_item(label: impl Into<String>) -> TreeItem {
    TreeItem {
        label: label.into(),
        depth: 0,
        is_dir: false,
        is_expanded: false,
        is_selected: false,
        is_generated: false,
        icon_color: None,
        on_click: None,
        on_toggle: None,
        chevron_element: None,
        icon_element: None,
        label_element: None,
    }
}

impl TreeItem {
    /// Set the indentation depth (each level adds INDENT_WIDTH pixels)
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Mark as a directory node with an initial expanded state
    pub fn directory(mut self, expanded: bool) -> Self {
        self.is_dir = true;
        self.is_expanded = expanded;
        self
    }

    /// Highlight this node as selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.is_selected = selected;
        self
    }

    /// Mark as a generated/build directory (muted styling)
    pub fn generated(mut self, generated: bool) -> Self {
        self.is_generated = generated;
        self
    }

    /// Set the file-type icon color
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Attach a click handler for selection
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Attach a handler for expand/collapse toggle
    pub fn on_toggle(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

/// State persisted through the rendering lifecycle
pub struct TreeItemState {
    layout_id: LayoutId,
    chevron_state: TextState,
    icon_state: TextState,
    label_state: TextState,
    hitbox_id: Option<HitboxId>,
    chevron_hitbox_id: Option<HitboxId>,
}

impl Element for TreeItem {
    type RequestLayoutState = TreeItemState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Container: flex row, center aligned, fixed height, left padding for depth
        let left_padding = self.depth as f32 * INDENT_WIDTH;

        let mut style = Style::default();
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Row;
        style.align_items = AlignItems::Center;
        style.min_height = Length::Px(TREE_ITEM_HEIGHT);
        style.padding = Edges {
            top: 0.0,
            bottom: 0.0,
            left: left_padding,
            right: 4.0,
        };
        style.gap = 2.0;

        let layout_id = cx.request_layout(&style);

        // Chevron: "v" for expanded dirs, ">" for collapsed dirs, " " spacer for files
        let chevron_text = if self.is_dir {
            if self.is_expanded { "v" } else { ">" }
        } else {
            " "
        };
        let mut chevron_el = TextElement::new(chevron_text)
            .size(TREE_FONT_SIZE)
            .color(Color::white())
            .w(CHEVRON_WIDTH);
        let (chevron_layout_id, chevron_state) = chevron_el.request_layout(cx);
        cx.add_child(layout_id, chevron_layout_id);
        self.chevron_element = Some(chevron_el);

        // Icon: folder or file dot indicator
        let icon_text = if self.is_dir { "D" } else { "." };
        let mut icon_el = TextElement::new(icon_text)
            .size(TREE_FONT_SIZE)
            .color(Color::white())
            .w(ICON_WIDTH);
        let (icon_layout_id, icon_state) = icon_el.request_layout(cx);
        cx.add_child(layout_id, icon_layout_id);
        self.icon_element = Some(icon_el);

        // Label text (grows to fill remaining space)
        let mut label_el = TextElement::new(self.label.clone())
            .size(TREE_FONT_SIZE)
            .color(Color::white())
            .grow(1.0);
        let (label_layout_id, label_state) = label_el.request_layout(cx);
        cx.add_child(layout_id, label_layout_id);
        self.label_element = Some(label_el);

        (
            layout_id,
            TreeItemState {
                layout_id,
                chevron_state,
                icon_state,
                label_state,
                hitbox_id: None,
                chevron_hitbox_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Opaque hitbox for entire row
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Wire click handlers — use clone (Rc) like Tab element, not take
        let on_click = self.on_click.clone();
        let on_toggle = self.on_toggle.clone();
        let is_dir = self.is_dir;
        cx.on_mouse_down(hitbox_id, move |event, _ctx| {
            if event.button != MouseButton::Left {
                return;
            }
            if is_dir {
                // Directories: click to expand/collapse
                if let Some(handler) = &on_toggle {
                    handler();
                }
            } else {
                // Files: double-click to open
                if event.click_count >= 2 {
                    if let Some(handler) = &on_click {
                        handler();
                    }
                }
            }
        });

        // Prepaint child elements
        if let Some(chevron_el) = &mut self.chevron_element {
            chevron_el.prepaint(&mut state.chevron_state, cx);
        }
        if let Some(icon_el) = &mut self.icon_element {
            icon_el.prepaint(&mut state.icon_state, cx);
        }
        if let Some(label_el) = &mut self.label_element {
            label_el.prepaint(&mut state.label_state, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Determine interaction state before borrowing theme
        let is_hovered = state.hitbox_id.map(|id| cx.is_hovered(id)).unwrap_or(false);

        // Extract all needed colors from theme up front
        let bg_color = if self.is_selected {
            let mut c = cx.theme().color(ColorToken::Accent);
            c.a = 0.15;
            c
        } else if is_hovered {
            cx.theme().color(ColorToken::BgElevated)
        } else {
            Color::transparent()
        };

        let chevron_color = cx.theme().color(ColorToken::FgMuted);
        let label_color = if self.is_generated {
            // Generated dirs: slightly dimmed but still readable
            cx.theme().color(ColorToken::FgSecondary)
        } else {
            cx.theme().color(ColorToken::FgPrimary)
        };
        let border_color = cx.theme().color(ColorToken::Border);

        let icon_color = if let Some(c) = self.icon_color {
            c
        } else if self.is_dir {
            cx.theme().color(ColorToken::FgMuted)
        } else {
            cx.theme().color(ColorToken::FgSecondary)
        };

        // Paint background (selection or hover)
        if bg_color.a > 0.0 {
            let mut bg_style = Style::default();
            bg_style.background = Background::Solid(bg_color);
            cx.paint_styled_rect(&bg_style, &bounds);
        }

        // Paint indent guides — 1px vertical lines for each ancestor level
        for level in 0..self.depth {
            let guide_x = bounds.origin.x + level as f32 * INDENT_WIDTH + INDENT_WIDTH / 2.0;
            let guide_bounds = Rect {
                origin: Point::new(guide_x, bounds.origin.y),
                size: Size::new(1.0, TREE_ITEM_HEIGHT),
            };
            let mut guide_style = Style::default();
            guide_style.background = Background::Solid(border_color);
            cx.paint_styled_rect(&guide_style, &guide_bounds);
        }

        // Paint chevron
        if let Some(chevron_el) = &mut self.chevron_element {
            chevron_el.set_color(chevron_color);
            chevron_el.paint(&mut state.chevron_state, cx);
        }

        // Paint icon
        if let Some(icon_el) = &mut self.icon_element {
            icon_el.set_color(icon_color);
            icon_el.paint(&mut state.icon_state, cx);
        }

        // Paint label
        if let Some(label_el) = &mut self.label_element {
            label_el.set_color(label_color);
            label_el.paint(&mut state.label_state, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_item_indent() {
        let item = tree_item("src").depth(2);
        assert_eq!(item.depth, 2);
        assert_eq!(item.label, "src");
        assert!(!item.is_dir);
        assert!(!item.is_selected);
    }

    #[test]
    fn test_tree_item_directory_state() {
        let item_collapsed = tree_item("src").directory(false);
        assert!(item_collapsed.is_dir);
        assert!(!item_collapsed.is_expanded);

        let item_expanded = tree_item("src").directory(true);
        assert!(item_expanded.is_dir);
        assert!(item_expanded.is_expanded);
    }
}
