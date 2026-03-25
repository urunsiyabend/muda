use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::events::FocusHandle;
use crate::style::{px, Color};
use crate::theme::ColorToken;

const MENU_WIDTH: f32 = 200.0;
const MENU_ITEM_HEIGHT: f32 = 28.0;
const MENU_PADDING_V: f32 = 4.0;
const MENU_FONT_SIZE: f32 = 13.0;
const MENU_ITEM_PADDING_H: f32 = 12.0;
const MENU_BORDER_RADIUS: f32 = 4.0;
const SEPARATOR_HEIGHT: f32 = 1.0;
const SEPARATOR_MARGIN_V: f32 = 4.0;

/// A context menu item.
///
/// NOTE: Does not derive Clone or Debug because `on_click` is `Box<dyn Fn()>`
/// which implements neither. Items are created fresh each time.
pub enum ContextMenuItem {
    /// A clickable menu item with label and optional shortcut.
    Action {
        label: String,
        shortcut: Option<String>,
        disabled: bool,
        on_click: Option<Box<dyn Fn() + 'static>>,
    },
    /// A visual separator line.
    Separator,
}

impl ContextMenuItem {
    /// Create an action menu item with the given label.
    pub fn action(label: impl Into<String>) -> Self {
        Self::Action {
            label: label.into(),
            shortcut: None,
            disabled: false,
            on_click: None,
        }
    }

    /// Add a keyboard shortcut hint to this item.
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Action { shortcut: ref mut s, .. } = self {
            *s = Some(shortcut.into());
        }
        self
    }

    /// Mark this item as disabled (non-interactive, dimmed).
    pub fn disabled(mut self) -> Self {
        if let Self::Action { disabled: ref mut d, .. } = self {
            *d = true;
        }
        self
    }

    /// Attach a click handler to this item.
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        if let Self::Action { on_click: ref mut h, .. } = self {
            *h = Some(Box::new(handler));
        }
        self
    }

    /// Create a separator item.
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Returns true if this item is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }
}

/// Popup context menu widget that renders at a given screen position.
///
/// ContextMenu is not an Element in the three-phase lifecycle sense. Instead it is a
/// View-like struct with a `render()` method that returns an AnyElement. The parent
/// layout (e.g. Stack) handles absolute positioning using `position()`.
///
/// # Example
///
/// ```rust,ignore
/// let mut menu = ContextMenu::new(vec![
///     ContextMenuItem::action("Cut").with_shortcut("Ctrl+X"),
///     ContextMenuItem::action("Copy").with_shortcut("Ctrl+C"),
///     ContextMenuItem::separator(),
///     ContextMenuItem::action("Paste").with_shortcut("Ctrl+V"),
///     ContextMenuItem::action("Delete").disabled(),
/// ]);
/// menu.show(100.0, 200.0);
/// ```
pub struct ContextMenu {
    items: Vec<ContextMenuItem>,
    selected_index: usize,
    visible: bool,
    position_x: f32,
    position_y: f32,
    focus_handle: Option<FocusHandle>,
}

impl ContextMenu {
    /// Create a new context menu with the given items.
    pub fn new(items: Vec<ContextMenuItem>) -> Self {
        Self {
            items,
            selected_index: 0,
            visible: false,
            position_x: 0.0,
            position_y: 0.0,
            focus_handle: None,
        }
    }

    /// Make the menu visible at the given screen position.
    pub fn show(&mut self, x: f32, y: f32) {
        self.visible = true;
        self.position_x = x;
        self.position_y = y;
        // Reset selection to first non-separator item
        self.selected_index = self.first_action_index().unwrap_or(0);
    }

    /// Hide the menu.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Returns true if the menu is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns the current position as (x, y) for parent layout to use.
    pub fn position(&self) -> (f32, f32) {
        (self.position_x, self.position_y)
    }

    /// Set an optional focus handle for keyboard navigation.
    pub fn with_focus(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Move selection up, skipping separators. Wraps around.
    pub fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let mut idx = self.selected_index;
        loop {
            if idx == 0 {
                idx = self.items.len() - 1;
            } else {
                idx -= 1;
            }
            if !self.items[idx].is_separator() {
                break;
            }
            // Safety: if all items are separators, avoid infinite loop
            if idx == self.selected_index {
                return;
            }
        }
        self.selected_index = idx;
    }

    /// Move selection down, skipping separators. Wraps around.
    pub fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let mut idx = self.selected_index;
        loop {
            idx = (idx + 1) % self.items.len();
            if !self.items[idx].is_separator() {
                break;
            }
            // Safety: if all items are separators, avoid infinite loop
            if idx == self.selected_index {
                return;
            }
        }
        self.selected_index = idx;
    }

    /// Returns the currently selected item, if any.
    pub fn selected_item(&self) -> Option<&ContextMenuItem> {
        self.items.get(self.selected_index)
    }

    /// Clamp the menu position so it stays within the viewport.
    /// Call this after show() when you know the window dimensions.
    pub fn clamp_to_viewport(&mut self, window_width: f32, window_height: f32) {
        let menu_height = self.estimated_height();
        if self.position_x + MENU_WIDTH > window_width {
            self.position_x = (self.position_x - MENU_WIDTH).max(0.0);
        }
        if self.position_y + menu_height > window_height {
            self.position_y = (self.position_y - menu_height).max(0.0);
        }
    }

    /// Estimate the rendered height of the menu based on item count.
    pub fn estimated_height(&self) -> f32 {
        let mut h = MENU_PADDING_V * 2.0;
        for item in &self.items {
            h += match item {
                ContextMenuItem::Action { .. } => MENU_ITEM_HEIGHT,
                ContextMenuItem::Separator => SEPARATOR_HEIGHT + SEPARATOR_MARGIN_V * 2.0,
            };
        }
        h
    }

    /// Render the context menu as an AnyElement.
    ///
    /// Returns a zero-size element when not visible. When visible, renders the full
    /// menu popup with items, separators, hover highlights, and drop shadow.
    pub fn render(&self, cx: &mut ViewContext) -> AnyElement {
        if !self.visible {
            return Div::new().w(px(0.0)).h(px(0.0)).into();
        }

        let theme = cx.theme();

        // Color extraction before any mutable operations (borrow checker pattern)
        let bg_elevated = theme.color(ColorToken::BgElevated);
        let border_color = theme.color(ColorToken::Border);
        let bg_secondary = theme.color(ColorToken::BgSecondary);
        let fg_primary = theme.color(ColorToken::FgPrimary);
        let fg_muted = theme.color(ColorToken::FgMuted);
        let separator_color = theme.color(ColorToken::Border);

        let mut menu = Div::new()
            .flex_col()
            .w(px(MENU_WIDTH))
            .py(MENU_PADDING_V)
            .bg(bg_elevated)
            .border(1.0, border_color)
            .border_radius(MENU_BORDER_RADIUS)
            .shadow(0.0, 4.0, 16.0, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.4));

        let mut item_index = 0;
        for item in &self.items {
            match item {
                ContextMenuItem::Action { label, shortcut, disabled, .. } => {
                    let is_selected = item_index == self.selected_index;

                    let bg = if is_selected && !*disabled {
                        bg_secondary
                    } else {
                        Color::transparent()
                    };

                    let text_color = if *disabled {
                        let mut c = fg_muted;
                        c.a = 0.5;
                        c
                    } else {
                        fg_primary
                    };

                    let mut row = Div::new()
                        .flex_row()
                        .justify_between()
                        .align_center()
                        .h(px(MENU_ITEM_HEIGHT))
                        .px(MENU_ITEM_PADDING_H)
                        .bg(bg);

                    if !*disabled {
                        row = row.hover_bg(bg_secondary);
                    }

                    row = row.child(
                        TextElement::new(label)
                            .size(MENU_FONT_SIZE)
                            .color(text_color),
                    );

                    if let Some(sc) = shortcut {
                        row = row.child(
                            TextElement::new(sc)
                                .size(MENU_FONT_SIZE - 1.0)
                                .color(fg_muted),
                        );
                    }

                    menu = menu.child(row);
                    item_index += 1;
                }
                ContextMenuItem::Separator => {
                    // Separator: inset horizontal rule with top+bottom margin
                    // We simulate my() by using a wrapper div with padding
                    let separator_wrapper = Div::new()
                        .flex_col()
                        .py(SEPARATOR_MARGIN_V)
                        .px(4.0)
                        .child(
                            Div::new()
                                .h(px(SEPARATOR_HEIGHT))
                                .bg(separator_color),
                        );
                    menu = menu.child(separator_wrapper);
                }
            }
        }

        menu.into()
    }

    /// Find the index of the first non-separator item.
    fn first_action_index(&self) -> Option<usize> {
        self.items.iter().position(|i| !i.is_separator())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_menu() -> ContextMenu {
        ContextMenu::new(vec![
            ContextMenuItem::action("Cut"),
            ContextMenuItem::action("Copy"),
            ContextMenuItem::separator(),
            ContextMenuItem::action("Paste"),
            ContextMenuItem::action("Delete").disabled(),
        ])
    }

    #[test]
    fn test_context_menu_navigation() {
        let mut menu = make_menu();
        menu.show(0.0, 0.0);

        // Initial selection should be first non-separator (index 0)
        assert_eq!(menu.selected_index, 0);

        // Move down: 0 -> 1
        menu.move_down();
        assert_eq!(menu.selected_index, 1);

        // Move down: 1 -> skip separator(2) -> 3
        menu.move_down();
        assert_eq!(menu.selected_index, 3);

        // Move down: 3 -> 4
        menu.move_down();
        assert_eq!(menu.selected_index, 4);

        // Move down wraps: 4 -> 0
        menu.move_down();
        assert_eq!(menu.selected_index, 0);

        // Move up wraps: 0 -> 4
        menu.move_up();
        assert_eq!(menu.selected_index, 4);

        // Move up: 4 -> 3
        menu.move_up();
        assert_eq!(menu.selected_index, 3);

        // Move up: 3 -> skip separator(2) -> 1
        menu.move_up();
        assert_eq!(menu.selected_index, 1);
    }

    #[test]
    fn test_context_menu_viewport_clamping() {
        let mut menu = make_menu();

        // Position near right edge — should flip left
        menu.show(900.0, 100.0);
        menu.clamp_to_viewport(1000.0, 800.0);
        // 900 + 200 = 1100 > 1000, so x becomes 900 - 200 = 700
        assert_eq!(menu.position_x, 700.0);
        // y is fine: 100 + ~116 < 800, unchanged
        assert_eq!(menu.position_y, 100.0);

        // Position near bottom edge — should flip up
        menu.show(100.0, 750.0);
        let estimated = menu.estimated_height();
        menu.clamp_to_viewport(1000.0, 800.0);
        // 750 + estimated > 800, so y becomes 750 - estimated (clamped to 0)
        let expected_y = (750.0 - estimated).max(0.0);
        assert_eq!(menu.position_y, expected_y);
        assert_eq!(menu.position_x, 100.0);

        // Position in safe area — no change
        menu.show(100.0, 100.0);
        menu.clamp_to_viewport(1000.0, 800.0);
        assert_eq!(menu.position_x, 100.0);
        assert_eq!(menu.position_y, 100.0);
    }

    #[test]
    fn test_context_menu_estimated_height() {
        let menu = make_menu();
        // 4 actions + 1 separator
        // h = MENU_PADDING_V*2 + 4*MENU_ITEM_HEIGHT + (SEPARATOR_HEIGHT + SEPARATOR_MARGIN_V*2)
        let expected = MENU_PADDING_V * 2.0
            + 4.0 * MENU_ITEM_HEIGHT
            + (SEPARATOR_HEIGHT + SEPARATOR_MARGIN_V * 2.0);
        assert_eq!(menu.estimated_height(), expected);
    }

    #[test]
    fn test_context_menu_visibility() {
        let mut menu = make_menu();
        assert!(!menu.is_visible());

        menu.show(50.0, 50.0);
        assert!(menu.is_visible());
        assert_eq!(menu.position(), (50.0, 50.0));

        menu.hide();
        assert!(!menu.is_visible());
    }

    #[test]
    fn test_context_menu_selected_item() {
        let mut menu = make_menu();
        menu.show(0.0, 0.0);
        // First item selected by default
        if let Some(ContextMenuItem::Action { label, .. }) = menu.selected_item() {
            assert_eq!(label, "Cut");
        } else {
            panic!("Expected Action item");
        }
    }
}
