//! Application layout manager for IDE chrome.
//!
//! Manages the overall IDE layout using flexbox-like semantics,
//! handling the toolbar, sidebars, editor area, panels, and status bar.

use crate::components::Bounds;
use crate::design_system::{
    Space, Spacing, FlexLayout, FlexDirection, AlignItems, JustifyContent,
    LayoutConstraints,
};

/// Layout regions for hit testing and event routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutRegion {
    Toolbar,
    LeftSidebar,
    RightSidebar,
    EditorTabs,
    EditorArea,
    Gutter,
    BottomPanel,
    StatusBar,
    CommandPalette,
    Dialog,
    None,
}

/// Calculated bounds for all UI regions.
#[derive(Clone, Debug, Default)]
pub struct LayoutBounds {
    pub toolbar: Bounds,
    pub left_sidebar: Bounds,
    pub right_sidebar: Bounds,
    pub editor_tabs: Bounds,
    pub editor_area: Bounds,
    pub gutter: Bounds,
    pub text_area: Bounds,
    pub bottom_panel: Bounds,
    pub status_bar: Bounds,
}

/// Configuration for the application layout.
#[derive(Clone, Debug)]
pub struct LayoutConfig {
    /// Toolbar height
    pub toolbar_height: f32,
    /// Tab bar height
    pub tab_bar_height: f32,
    /// Status bar height
    pub status_bar_height: f32,
    /// Left sidebar width (0 = hidden)
    pub left_sidebar_width: f32,
    /// Right sidebar width (0 = hidden)
    pub right_sidebar_width: f32,
    /// Bottom panel height (0 = hidden)
    pub bottom_panel_height: f32,
    /// Gutter width (dynamic based on line count)
    pub gutter_width: f32,
    /// Minimum sidebar width when visible
    pub min_sidebar_width: f32,
    /// Minimum editor width
    pub min_editor_width: f32,
    /// Whether toolbar is visible
    pub show_toolbar: bool,
    /// Whether left sidebar is visible
    pub show_left_sidebar: bool,
    /// Whether right sidebar is visible
    pub show_right_sidebar: bool,
    /// Whether bottom panel is visible
    pub show_bottom_panel: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            toolbar_height: 0.0, // No toolbar by default (like VS Code)
            tab_bar_height: 35.0,
            status_bar_height: 22.0,
            left_sidebar_width: 240.0,
            right_sidebar_width: 0.0,
            bottom_panel_height: 0.0,
            gutter_width: 60.0,
            min_sidebar_width: 150.0,
            min_editor_width: 200.0,
            show_toolbar: false,
            show_left_sidebar: true,
            show_right_sidebar: false,
            show_bottom_panel: false,
        }
    }
}

/// Main application layout manager.
pub struct AppLayout {
    config: LayoutConfig,
    bounds: LayoutBounds,
    total_bounds: Bounds,
}

impl AppLayout {
    pub fn new() -> Self {
        Self {
            config: LayoutConfig::default(),
            bounds: LayoutBounds::default(),
            total_bounds: Bounds::default(),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }

    /// Get mutable config for updates.
    pub fn config_mut(&mut self) -> &mut LayoutConfig {
        &mut self.config
    }

    /// Get the calculated bounds.
    pub fn bounds(&self) -> &LayoutBounds {
        &self.bounds
    }

    /// Set the gutter width (call after measuring line number width).
    pub fn set_gutter_width(&mut self, width: f32) {
        self.config.gutter_width = width;
    }

    /// Toggle left sidebar visibility.
    pub fn toggle_left_sidebar(&mut self) {
        self.config.show_left_sidebar = !self.config.show_left_sidebar;
    }

    /// Toggle right sidebar visibility.
    pub fn toggle_right_sidebar(&mut self) {
        self.config.show_right_sidebar = !self.config.show_right_sidebar;
    }

    /// Toggle bottom panel visibility.
    pub fn toggle_bottom_panel(&mut self) {
        self.config.show_bottom_panel = !self.config.show_bottom_panel;
    }

    /// Set left sidebar width.
    pub fn set_left_sidebar_width(&mut self, width: f32) {
        self.config.left_sidebar_width = width.max(self.config.min_sidebar_width);
    }

    /// Set bottom panel height.
    pub fn set_bottom_panel_height(&mut self, height: f32) {
        self.config.bottom_panel_height = height.max(100.0);
    }

    /// Calculate layout for the given screen dimensions.
    ///
    /// Layout structure:
    /// ```text
    /// ┌─────────────────────────────────────────────┐
    /// │                  Toolbar                     │ (optional)
    /// ├──────────┬──────────────────────┬───────────┤
    /// │          │     Editor Tabs      │           │
    /// │  Left    ├───────┬──────────────┤  Right    │
    /// │  Sidebar │Gutter │  Text Area   │  Sidebar  │
    /// │          │       │              │           │
    /// │          │       │              │           │
    /// │          ├───────┴──────────────┤           │
    /// │          │    Bottom Panel      │           │
    /// ├──────────┴──────────────────────┴───────────┤
    /// │                 Status Bar                   │
    /// └─────────────────────────────────────────────┘
    /// ```
    pub fn calculate(&mut self, width: f32, height: f32) {
        self.total_bounds = Bounds::new(0.0, 0.0, width, height);
        let mut remaining = self.total_bounds;

        // 1. Status bar at bottom
        let (main_area, status_bar) = remaining.split_vertical(
            remaining.height - self.config.status_bar_height
        );
        self.bounds.status_bar = status_bar;
        remaining = main_area;

        // 2. Toolbar at top (if visible)
        if self.config.show_toolbar && self.config.toolbar_height > 0.0 {
            let (toolbar, rest) = remaining.split_vertical(self.config.toolbar_height);
            self.bounds.toolbar = toolbar;
            remaining = rest;
        } else {
            self.bounds.toolbar = Bounds::new(0.0, 0.0, 0.0, 0.0);
        }

        // 3. Left sidebar
        if self.config.show_left_sidebar && self.config.left_sidebar_width > 0.0 {
            let (left, rest) = remaining.split_horizontal(self.config.left_sidebar_width);
            self.bounds.left_sidebar = left;
            remaining = rest;
        } else {
            self.bounds.left_sidebar = Bounds::new(0.0, 0.0, 0.0, 0.0);
        }

        // 4. Right sidebar
        if self.config.show_right_sidebar && self.config.right_sidebar_width > 0.0 {
            let (center, right) = remaining.split_horizontal(
                remaining.width - self.config.right_sidebar_width
            );
            self.bounds.right_sidebar = right;
            remaining = center;
        } else {
            self.bounds.right_sidebar = Bounds::new(0.0, 0.0, 0.0, 0.0);
        }

        // 5. Bottom panel (if visible)
        if self.config.show_bottom_panel && self.config.bottom_panel_height > 0.0 {
            let (editor, panel) = remaining.split_vertical(
                remaining.height - self.config.bottom_panel_height
            );
            self.bounds.bottom_panel = panel;
            remaining = editor;
        } else {
            self.bounds.bottom_panel = Bounds::new(0.0, 0.0, 0.0, 0.0);
        }

        // 6. Tab bar at top of editor area
        let (tabs, editor) = remaining.split_vertical(self.config.tab_bar_height);
        self.bounds.editor_tabs = tabs;
        self.bounds.editor_area = editor;

        // 7. Split editor area into gutter and text area
        let (gutter, text_area) = editor.split_horizontal(self.config.gutter_width);
        self.bounds.gutter = gutter;
        self.bounds.text_area = text_area;
    }

    /// Determine which region a point falls into.
    pub fn hit_test(&self, x: f32, y: f32) -> LayoutRegion {
        // Check regions in z-order (front to back)
        if self.contains(self.bounds.status_bar, x, y) {
            return LayoutRegion::StatusBar;
        }
        if self.contains(self.bounds.toolbar, x, y) {
            return LayoutRegion::Toolbar;
        }
        if self.contains(self.bounds.left_sidebar, x, y) {
            return LayoutRegion::LeftSidebar;
        }
        if self.contains(self.bounds.right_sidebar, x, y) {
            return LayoutRegion::RightSidebar;
        }
        if self.contains(self.bounds.bottom_panel, x, y) {
            return LayoutRegion::BottomPanel;
        }
        if self.contains(self.bounds.editor_tabs, x, y) {
            return LayoutRegion::EditorTabs;
        }
        if self.contains(self.bounds.gutter, x, y) {
            return LayoutRegion::Gutter;
        }
        if self.contains(self.bounds.text_area, x, y) {
            return LayoutRegion::EditorArea;
        }
        LayoutRegion::None
    }

    fn contains(&self, bounds: Bounds, x: f32, y: f32) -> bool {
        bounds.width > 0.0 && bounds.height > 0.0 &&
        x >= bounds.x && x < bounds.x + bounds.width &&
        y >= bounds.y && y < bounds.y + bounds.height
    }

    /// Get responsive layout adjustments for narrow screens.
    pub fn apply_responsive(&mut self, width: f32) {
        // On narrow screens, hide sidebars automatically
        if width < 800.0 {
            self.config.show_right_sidebar = false;
        }
        if width < 600.0 {
            self.config.show_left_sidebar = false;
        }

        // Reduce sidebar width on medium screens
        if width < 1200.0 && self.config.left_sidebar_width > 200.0 {
            self.config.left_sidebar_width = 200.0;
        }
    }
}

impl Default for AppLayout {
    fn default() -> Self {
        Self::new()
    }
}
