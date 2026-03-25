//! Root view for the editor application.
//!
//! `EditorRootView` holds a shared reference to an `EditorDataSource` adapter
//! and rebuilds the full AppLayout from fresh `RenderModel` data each frame.
//! This is the bridge between the adapter boundary and the ora view tree.

use std::cell::RefCell;
use std::rc::Rc;

use crate::context::ViewContext;
use crate::editor_adapter::{EditorDataSource, FileTreePresentation, RenderModel};
use crate::element::AnyElement;
use crate::events::FocusHandle;
use crate::view::View;
use crate::views::{
    AppLayout, CommandItem, CommandPaletteView, DialogView, GutterView,
    PanelManagerView, SidebarView, StatusBarView, TabBarView, TextAreaView,
};

/// Shared adapter handle used by both the root view (for build_render_model)
/// and the event loop (for dispatch_command).
pub type SharedAdapter = Rc<RefCell<Box<dyn EditorDataSource>>>;

/// Root view that drives the full editor UI from an EditorDataSource.
///
/// On each frame, `render()` calls `adapter.build_render_model()` to get
/// fresh presentation data and constructs the complete AppLayout tree.
pub struct EditorRootView {
    adapter: SharedAdapter,
    /// Persistent focus handles (created once, reused across frames).
    dialog_save_focus: FocusHandle,
    dialog_dont_save_focus: FocusHandle,
    dialog_cancel_focus: FocusHandle,
    palette_focus: FocusHandle,
}

impl EditorRootView {
    /// Create a new editor root view with the given shared adapter.
    ///
    /// Focus handles are allocated once and reused across frames to avoid
    /// FocusId explosion (per Phase 4 decision: persistent FocusHandles).
    pub fn new(adapter: SharedAdapter, cx: &mut ViewContext) -> Self {
        Self {
            adapter,
            dialog_save_focus: cx.focus_handle(),
            dialog_dont_save_focus: cx.focus_handle(),
            dialog_cancel_focus: cx.focus_handle(),
            palette_focus: cx.focus_handle(),
        }
    }

    /// Build the AppLayout from a fresh RenderModel.
    fn build_layout(&self, model: &RenderModel, cx: &mut ViewContext) -> AppLayout {
        let sidebar = SidebarView::new(
            model.sidebar.clone(),
            FileTreePresentation::default(),
        );
        let tab_bar = TabBarView::new(model.tab_bar.clone());
        let gutter = GutterView::new(model.gutter.clone(), model.visible_lines.clone());
        let text_area = TextAreaView::new(model);
        let status_bar = StatusBarView::new(model.status.clone());
        let panel_manager = PanelManagerView::new();
        let command_palette = CommandPaletteView::new(
            vec![
                CommandItem::new("palette:toggle-theme", "Toggle Theme"),
                CommandItem::new("palette:toggle-sidebar", "Toggle Sidebar"),
                CommandItem::new("palette:toggle-panel", "Toggle Panel"),
            ],
            self.palette_focus.clone(),
        );
        let dialog = DialogView::new(model.dialog.clone(), cx);

        AppLayout::new(
            sidebar, tab_bar, gutter, text_area, status_bar,
            panel_manager, command_palette, dialog,
        )
    }
}

impl View for EditorRootView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Get the current window size to compute viewport lines
        // Use a reasonable default if we can't determine it
        let viewport_lines = 40; // TODO: compute from window height / LINE_HEIGHT

        // Build fresh RenderModel from the adapter
        let model = {
            let adapter = self.adapter.borrow();
            adapter.build_render_model(viewport_lines)
        };

        // Construct the AppLayout from fresh data and render it
        let layout = self.build_layout(&model, cx);
        layout.render(cx)
    }
}
