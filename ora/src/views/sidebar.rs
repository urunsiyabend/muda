//! Sidebar view for file explorer panel.
//!
//! Renders a collapsible sidebar panel with:
//! - Header with "Explorer" title and toggle button
//! - File tree content area (placeholder for Phase 8)
//! - Icon rail when collapsed (48px wide)
//!
//! Consumes `SidebarPresentation` from core_editor for visibility and entries.

use std::rc::Rc;

use crate::animation::transition::TransitionId;
use crate::context::ViewContext;
use crate::editor_adapter::EditorCommand;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement, scroll_area, scroll_state, SharedScrollState};
use crate::events::focus::FocusHandle;
use crate::style::{pct, px};
use crate::theme::ColorToken;
use crate::editor_adapter::{FileTreePresentation, SidebarPresentation};
use crate::view::View;
use crate::views::file_tree::FileTreeView;

/// TransitionId for the sidebar toggle button (collapse/expand arrow in header).
const SIDEBAR_TOGGLE_TRANSITION_ID: TransitionId = TransitionId(10_000);

/// TransitionId for the sidebar expand button (shown when collapsed).
const SIDEBAR_EXPAND_TRANSITION_ID: TransitionId = TransitionId(10_001);

/// Default expanded sidebar width in logical pixels (professional IDE width).
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 480.0;

/// Icon rail width when collapsed (not completely hidden per CONTEXT decision).
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 48.0;

/// Header height in logical pixels.
const HEADER_HEIGHT: f32 = 36.0;

/// Horizontal padding in header.
const HEADER_PADDING_H: f32 = 12.0;

/// Font size for header title.
const TITLE_FONT_SIZE: f32 = 12.0;

/// Font size for file entries.
const ENTRY_FONT_SIZE: f32 = 13.0;

/// Horizontal padding for file entries.
const ENTRY_PADDING_H: f32 = 12.0;

/// Sidebar view for file explorer panel.
///
/// Implements a collapsible sidebar with:
/// - Expanded state: Full panel with header, toggle button, and content area
/// - Collapsed state: Narrow icon rail with expand button
///
/// # Architecture
///
/// The sidebar consumes `SidebarPresentation` from core_editor which provides:
/// - `visible`: Whether sidebar is shown at all
/// - `focused`: Whether sidebar has input focus
/// - `directory_name`: Base directory name for title
/// - `entries`: File/directory entries (used by Phase 8 FileTree)
///
/// # CONTEXT Decisions Applied
///
/// - Collapsed state shows icon rail (48px), not completely hidden
/// - Toggle via dedicated button, not header click
/// - Resize handle deferred (requires MouseCapture tracking)
///
/// # Example
///
/// ```ignore
/// let sidebar = SidebarView::new(presentation, Default::default());
/// // Toggle collapse state
/// sidebar.toggle();
/// // In a parent view's render():
/// Div::new().child(sidebar.render(cx))
/// ```
pub struct SidebarView {
    /// The presentation data from core_editor.
    presentation: SidebarPresentation,
    /// Whether the sidebar is collapsed to icon rail.
    is_collapsed: bool,
    /// Current width when expanded (default 220px).
    width: f32,
    /// Focus handle for the toggle button.
    toggle_focus: Option<FocusHandle>,
    /// File tree view for hierarchical file navigation.
    file_tree: FileTreeView,
    /// Command dispatch callback for sidebar actions.
    dispatch: Option<Rc<dyn Fn(EditorCommand)>>,
    /// Shared scroll state for the file tree scroll area.
    tree_scroll: SharedScrollState,
    /// Available height for the tree content (window height minus chrome).
    content_height: f32,
}

impl SidebarView {
    /// Creates a new sidebar view with the given presentation data.
    pub fn new(presentation: SidebarPresentation, tree: FileTreePresentation) -> Self {
        Self {
            presentation,
            is_collapsed: false,
            width: SIDEBAR_DEFAULT_WIDTH,
            toggle_focus: None,
            file_tree: FileTreeView::new(tree),
            dispatch: None,
            tree_scroll: scroll_state(),
            content_height: 600.0,
        }
    }

    /// Creates a new sidebar view with a focus handle for the toggle button.
    pub fn with_focus(presentation: SidebarPresentation, tree: FileTreePresentation, toggle_focus: FocusHandle) -> Self {
        Self {
            presentation,
            is_collapsed: false,
            width: SIDEBAR_DEFAULT_WIDTH,
            toggle_focus: Some(toggle_focus),
            file_tree: FileTreeView::new(tree),
            dispatch: None,
            tree_scroll: scroll_state(),
            content_height: 600.0,
        }
    }

    /// Attach a command dispatch callback for sidebar file actions.
    pub fn with_dispatch(mut self, dispatch: Rc<dyn Fn(EditorCommand)>) -> Self {
        self.dispatch = Some(dispatch);
        self
    }

    /// Set a shared scroll state for the file tree.
    pub fn with_scroll_state(mut self, scroll: SharedScrollState) -> Self {
        self.tree_scroll = scroll;
        self
    }

    /// Set available content height (window height minus chrome).
    pub fn with_content_height(mut self, h: f32) -> Self {
        self.content_height = h;
        self
    }

    /// Updates the file tree presentation data.
    pub fn set_tree(&mut self, tree: FileTreePresentation) {
        self.file_tree.set_presentation(tree);
    }

    /// Updates the presentation data.
    pub fn set_presentation(&mut self, presentation: SidebarPresentation) {
        self.presentation = presentation;
    }

    /// Returns whether the sidebar is currently collapsed.
    pub fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }

    /// Sets the collapsed state.
    pub fn set_collapsed(&mut self, collapsed: bool) {
        self.is_collapsed = collapsed;
    }

    /// Toggles the collapsed state.
    pub fn toggle(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    /// Returns the current sidebar width based on collapsed state.
    pub fn current_width(&self) -> f32 {
        if !self.presentation.visible {
            0.0
        } else if self.is_collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            self.width
        }
    }

    /// Sets the expanded width (minimum 100px, maximum 400px).
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(100.0, 400.0);
    }

    /// Renders the sidebar header with title and toggle button.
    fn render_header(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Title text
        let title = TextElement::new("Explorer")
            .size(TITLE_FONT_SIZE)
            .color(theme.color(ColorToken::FgSecondary));

        // Toggle button (collapse/expand arrow) - always visible ASCII character
        // Using clear ASCII arrows that are always visible (not hidden on hover)
        let toggle_icon = if self.is_collapsed { ">" } else { "<" };
        let toggle_button = Div::new()
            .flex_row()
            .align_center()
            .justify_center()
            .w(px(24.0))
            .h(px(24.0))
            .border_radius(4.0)
            .hover_bg(theme.color(ColorToken::BgElevated))
            .transition_id(SIDEBAR_TOGGLE_TRANSITION_ID)
            .transition_bg(150)
            .child(
                TextElement::new(toggle_icon)
                    .size(TITLE_FONT_SIZE + 2.0)
                    .color(theme.color(ColorToken::FgPrimary))
            );

        // Header container with space-between layout
        Div::new()
            .flex_row()
            .justify_between()
            .align_center()
            .h(px(HEADER_HEIGHT))
            .px(HEADER_PADDING_H)
            .bg(theme.color(ColorToken::BgSecondary))
            .border(1.0, theme.color(ColorToken::Border))
            .child(title)
            .child(toggle_button)
    }

    /// Renders the content area with FileTree or empty state.
    fn render_content(&self, cx: &mut ViewContext) -> AnyElement {
        if self.file_tree.is_empty() {
            let bg = cx.theme().color(ColorToken::BgSecondary);
            let fg = cx.theme().color(ColorToken::FgMuted);
            return Div::new()
                .flex_col()
                .grow(1.0)
                .p(ENTRY_PADDING_H)
                .bg(bg)
                .child(TextElement::new("No folder open")
                    .size(ENTRY_FONT_SIZE)
                    .color(fg))
                .into();
        }

        let bg = cx.theme().color(ColorToken::BgSecondary);

        // Render FileTree inside a ScrollArea
        let mut tree = FileTreeView::new(self.file_tree.presentation.clone());
        if let Some(ref dispatch) = self.dispatch {
            tree = tree.with_dispatch(dispatch.clone());
        }
        let tree_element = tree.render(cx);

        let available_h = (self.content_height - HEADER_HEIGHT).max(100.0);

        scroll_area(self.tree_scroll.clone())
            .bg(bg)
            .max_h(available_h)
            .child(tree_element)
            .into()
    }

    /// Renders the collapsed icon rail state.
    fn render_collapsed(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Expand button with visible ">" arrow at top
        let expand_button = Div::new()
            .flex_row()
            .align_center()
            .justify_center()
            .w(px(32.0))
            .h(px(32.0))
            .border_radius(4.0)
            .hover_bg(theme.color(ColorToken::BgElevated))
            .transition_id(SIDEBAR_EXPAND_TRANSITION_ID)
            .transition_bg(150)
            .child(
                TextElement::new(">")
                    .size(TITLE_FONT_SIZE + 2.0)
                    .color(theme.color(ColorToken::FgPrimary))
            );

        Div::new()
            .flex_col()
            .w(px(SIDEBAR_COLLAPSED_WIDTH))
            .h(pct(100.0))
            .shrink(0.0)  // Don't shrink below fixed width
            .bg(theme.color(ColorToken::BgSecondary))
            // No outer border for alignment with main area
            .align_center()
            .py(8.0)
            .child(expand_button)
    }

    /// Renders the expanded sidebar state.
    fn render_expanded(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        Div::new()
            .flex_col()
            .w(px(self.width))
            .h(pct(100.0))
            .shrink(0.0)  // Don't shrink below fixed width
            .bg(theme.color(ColorToken::BgSecondary))
            // No outer border - header has its own border for separation
            .child(self.render_header(cx))
            .child(self.render_content(cx))
    }
}

impl View for SidebarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // If not visible, render empty placeholder
        if !self.presentation.visible {
            return Div::new()
                .w(px(0.0))
                .h(px(0.0))
                .into();
        }

        // Render based on collapsed state
        if self.is_collapsed {
            self.render_collapsed(cx).into()
        } else {
            self.render_expanded(cx).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_adapter::SidebarPresentation;

    #[test]
    fn test_sidebar_view_creation() {
        let presentation = SidebarPresentation {
            visible: true,
            focused: false,
            directory_name: "my_project".to_string(),
            entries: vec![],
            tree: vec![],
            width: 480,
        };

        let view = SidebarView::new(presentation, FileTreePresentation::default());
        assert!(!view.is_collapsed());
        assert_eq!(view.width, SIDEBAR_DEFAULT_WIDTH);
    }

    #[test]
    fn test_sidebar_toggle() {
        let presentation = SidebarPresentation::default();
        let mut view = SidebarView::new(presentation, FileTreePresentation::default());

        assert!(!view.is_collapsed());
        view.toggle();
        assert!(view.is_collapsed());
        view.toggle();
        assert!(!view.is_collapsed());
    }

    #[test]
    fn test_sidebar_width_calculation() {
        let presentation = SidebarPresentation {
            visible: true,
            ..Default::default()
        };

        let mut view = SidebarView::new(presentation.clone(), FileTreePresentation::default());
        assert_eq!(view.current_width(), SIDEBAR_DEFAULT_WIDTH);

        view.set_collapsed(true);
        assert_eq!(view.current_width(), SIDEBAR_COLLAPSED_WIDTH);

        // Test not visible
        let hidden = SidebarPresentation {
            visible: false,
            ..Default::default()
        };
        let hidden_view = SidebarView::new(hidden, FileTreePresentation::default());
        assert_eq!(hidden_view.current_width(), 0.0);
    }

    #[test]
    fn test_sidebar_width_clamping() {
        let presentation = SidebarPresentation::default();
        let mut view = SidebarView::new(presentation, FileTreePresentation::default());

        view.set_width(50.0); // Below minimum
        assert_eq!(view.width, 100.0);

        view.set_width(500.0); // Above maximum
        assert_eq!(view.width, 400.0);

        view.set_width(250.0); // Within range
        assert_eq!(view.width, 250.0);
    }

    #[test]
    fn test_sidebar_constants() {
        assert_eq!(SIDEBAR_DEFAULT_WIDTH, 480.0);
        assert_eq!(SIDEBAR_COLLAPSED_WIDTH, 48.0);
        assert_eq!(HEADER_HEIGHT, 36.0);
    }
}
