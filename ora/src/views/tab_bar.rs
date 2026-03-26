//! Tab bar view showing open document tabs.
//!
//! Renders a horizontal bar with tabs for each open document.
//! Consumes `TabBarPresentation` from ora::editor_adapter for tab data.

use crate::context::ViewContext;
use crate::editor_adapter::{EditorCommand, TabBarPresentation};
use crate::element::AnyElement;
use crate::elements::{tab, Div};
use crate::style::{pct, px};
use crate::theme::ColorToken;
use crate::view::View;
use std::rc::Rc;

/// Tab bar height in logical pixels (matches sidebar header for alignment).
pub const TAB_BAR_HEIGHT: f32 = 36.0;

/// Gap between tabs.
const TAB_GAP: f32 = 1.0;

/// Tab bar view for displaying open document tabs.
///
/// Consumes `TabBarPresentation` from core_editor and renders
/// a horizontal row of tabs with:
/// - Dot dirty indicator (dot before filename for unsaved changes)
/// - Title text
/// - Active/inactive background styling
/// - Accent bottom border for active tab
/// - Click handlers for switching and closing tabs
///
/// # Example
///
/// ```ignore
/// let tab_bar = TabBarView::new(presentation)
///     .with_dispatch(Rc::new(move |cmd| adapter.borrow_mut().dispatch_command(cmd)));
/// // In a parent view's render():
/// Div::new().child(tab_bar.render(cx))
/// ```
pub struct TabBarView {
    /// The presentation data for the tab bar.
    presentation: TabBarPresentation,
    /// Optional command dispatch callback for wiring tab click handlers.
    dispatch: Option<Rc<dyn Fn(EditorCommand)>>,
}

impl TabBarView {
    /// Creates a new tab bar view with the given presentation data.
    pub fn new(presentation: TabBarPresentation) -> Self {
        Self { presentation, dispatch: None }
    }

    /// Attaches a command dispatch callback for tab click handlers.
    ///
    /// When provided, clicking a tab dispatches `SwitchTab(view_id)` and
    /// clicking the close button dispatches `CloseTab`.
    pub fn with_dispatch(mut self, dispatch: Rc<dyn Fn(EditorCommand)>) -> Self {
        self.dispatch = Some(dispatch);
        self
    }

    /// Updates the presentation data.
    pub fn set_presentation(&mut self, presentation: TabBarPresentation) {
        self.presentation = presentation;
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

        let theme = cx.theme();
        let mut tab_elements: Vec<AnyElement> = Vec::with_capacity(self.presentation.tabs.len());

        for tab_data in &self.presentation.tabs {
            let view_id = tab_data.view_id;
            let dispatch = self.dispatch.clone();

            let mut t = tab(&tab_data.title)
                .active(tab_data.is_active)
                .dirty(tab_data.is_dirty);

            // Wire click handler: dispatch SwitchTab(view_id)
            if let Some(d) = dispatch.clone() {
                t = t.on_click(move || {
                    d(EditorCommand::SwitchTab(view_id));
                });
            }

            // Wire close handler: dispatch CloseTab
            // Note: Tab element uses Rc<dyn Fn()> internally so this closure
            // is shared between the close button and middle-click on tab body.
            if let Some(d) = dispatch.clone() {
                t = t.on_close(move || {
                    d(EditorCommand::CloseTab);
                });
            }

            tab_elements.push(t.into());
        }

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
    }

    #[test]
    fn test_tab_bar_with_dispatch() {
        use std::cell::Cell;
        use std::rc::Rc;

        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();

        let presentation = TabBarPresentation {
            tabs: vec![TabPresentation::new(42, "test.rs".to_string(), true, false)],
            visible: true,
        };

        let view = TabBarView::new(presentation)
            .with_dispatch(Rc::new(move |cmd| {
                if matches!(cmd, EditorCommand::SwitchTab(42)) {
                    called_clone.set(true);
                }
            }));

        assert!(view.dispatch.is_some());
    }
}
