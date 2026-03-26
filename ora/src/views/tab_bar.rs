//! Tab bar view showing open document tabs.
//!
//! Renders a horizontal bar with tabs for each open document.
//! Consumes `TabBarPresentation` from ora::editor_adapter for tab data.

use crate::animation::transition::TransitionId;
use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::editor_adapter::{TabBarPresentation, TabPresentation};
use crate::style::{pct, px};
use crate::theme::ColorToken;
use crate::view::View;

/// Base TransitionId namespace for tab elements.
/// Tab IDs = TAB_TRANSITION_BASE + tab.view_id  (no collision with other views).
const TAB_TRANSITION_BASE: u64 = 20_000;

/// Tab bar height in logical pixels (matches sidebar header for alignment).
pub const TAB_BAR_HEIGHT: f32 = 36.0;

/// Horizontal padding within each tab.
const TAB_PADDING_H: f32 = 12.0;

/// Vertical padding within each tab.
const TAB_PADDING_V: f32 = 8.0;

/// Font size for tab titles (readable size).
const TAB_FONT_SIZE: f32 = 13.0;

/// Gap between tabs.
const TAB_GAP: f32 = 1.0;

/// Close button size.
const CLOSE_BUTTON_SIZE: f32 = 16.0;

/// Tab bar view for displaying open document tabs.
///
/// Consumes `TabBarPresentation` from core_editor and renders
/// a horizontal row of tabs with:
/// - Dirty indicator ("*" prefix for unsaved changes)
/// - Title text
/// - Active/inactive background styling
/// - Hover highlight
///
/// # Example
///
/// ```ignore
/// let tab_bar = TabBarView::new(presentation);
/// // In a parent view's render():
/// Div::new().child(tab_bar.render(cx))
/// ```
pub struct TabBarView {
    /// The presentation data for the tab bar.
    presentation: TabBarPresentation,
}

impl TabBarView {
    /// Creates a new tab bar view with the given presentation data.
    pub fn new(presentation: TabBarPresentation) -> Self {
        Self { presentation }
    }

    /// Updates the presentation data.
    pub fn set_presentation(&mut self, presentation: TabBarPresentation) {
        self.presentation = presentation;
    }

    /// Renders a single tab element.
    fn render_tab(&self, tab: &TabPresentation, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Determine background color based on active state
        let bg_color = if tab.is_active {
            theme.color(ColorToken::BgPrimary)
        } else {
            theme.color(ColorToken::BgSecondary)
        };

        // Hover color for inactive tabs (subtle highlight)
        let hover_color = theme.color(ColorToken::BgElevated);

        // Create title text element
        let title_text = tab.title.clone();
        let text = TextElement::new(&title_text)
            .size(TAB_FONT_SIZE)
            .color(theme.color(ColorToken::FgPrimary));

        // Create close button (visible on hover via parent hover state)
        // Using "x" character as close icon
        let close_button = Div::new()
            .flex_row()
            .align_center()
            .justify_center()
            .w(px(CLOSE_BUTTON_SIZE))
            .h(px(CLOSE_BUTTON_SIZE))
            .border_radius(3.0)
            .hover_bg(theme.color(ColorToken::BgElevated))
            .child(
                TextElement::new("x")
                    .size(TAB_FONT_SIZE - 2.0)
                    .color(theme.color(ColorToken::FgMuted))
            );

        // Stable TransitionId per tab: base + view_id (avoids collisions with other views)
        let tid = TransitionId(TAB_TRANSITION_BASE + tab.view_id);

        // Build tab container. For dirty tabs, render a small dot before the title.
        // Order: [dot if dirty] [title] [close button]
        let mut tab_div = Div::new()
            .flex_row()
            .align_center()
            .gap(8.0)
            .px(TAB_PADDING_H)
            .py(TAB_PADDING_V)
            .bg(bg_color)
            .transition_id(tid)
            .transition_bg(150);

        // Dot dirty indicator: small filled circle rendered before the title
        if tab.is_dirty {
            let dot = Div::new()
                .w(px(6.0))
                .h(px(6.0))
                .border_radius(3.0)
                .bg(theme.color(ColorToken::FgMuted))
                .shrink(0.0);
            tab_div = tab_div.child(dot);
        }

        tab_div = tab_div.child(text).child(close_button);

        // Add hover effect for inactive tabs only
        if !tab.is_active {
            tab_div = tab_div.hover_bg(hover_color);
        }

        // Active tab accent border: 2px bottom border using AccentPrimary color
        // Wrap content in column to add accent bar at bottom
        if tab.is_active {
            let accent_bar = Div::new()
                .w(pct(100.0))
                .h(px(2.0))
                .shrink(0.0)
                .bg(theme.color(ColorToken::Accent));

            return Div::new()
                .flex_col()
                .child(tab_div)
                .child(accent_bar);
        }

        // Inactive tabs: transparent 2px bar at bottom for height alignment
        let spacer = Div::new()
            .w(pct(100.0))
            .h(px(2.0))
            .shrink(0.0);

        Div::new()
            .flex_col()
            .child(tab_div)
            .child(spacer)
    }
}

impl View for TabBarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // If not visible or no tabs, render empty placeholder
        if !self.presentation.visible || self.presentation.tabs.is_empty() {
            return Div::new()
                .h(px(0.0))
                .into();
        }

        // Build tab elements (render_tab borrows cx mutably, so build first)
        let mut tab_elements: Vec<AnyElement> = Vec::with_capacity(self.presentation.tabs.len());
        for tab in &self.presentation.tabs {
            tab_elements.push(self.render_tab(tab, cx).into());
        }

        // Now get theme for container styling
        let theme = cx.theme();

        // Build the tab bar container (explicit height and width for consistent alignment)
        Div::new()
            .flex_row()
            .w(pct(100.0))  // Full width of parent container
            .h(px(TAB_BAR_HEIGHT))
            .shrink(0.0)  // Don't shrink below fixed height
            .align_center()
            .bg(theme.color(ColorToken::BgSecondary))
            .border(1.0, theme.color(ColorToken::Border))
            .gap(TAB_GAP)
            .children(tab_elements)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_adapter::{TabBarPresentation, TabPresentation};

    #[test]
    fn test_tab_bar_view_creation() {
        let presentation = TabBarPresentation {
            tabs: vec![
                TabPresentation::new(1, "main.rs".to_string(), true, false),
                TabPresentation::new(2, "lib.rs".to_string(), false, true),
            ],
            visible: true,
        };

        let view = TabBarView::new(presentation.clone());
        assert_eq!(view.presentation.tabs.len(), 2);
        assert!(view.presentation.visible);
    }

    #[test]
    fn test_tab_bar_view_empty() {
        let presentation = TabBarPresentation {
            tabs: vec![],
            visible: true,
        };

        let view = TabBarView::new(presentation);
        assert!(view.presentation.tabs.is_empty());
    }

    #[test]
    fn test_tab_bar_constants() {
        assert_eq!(TAB_BAR_HEIGHT, 36.0);
        assert_eq!(TAB_FONT_SIZE, 13.0);
    }
}
