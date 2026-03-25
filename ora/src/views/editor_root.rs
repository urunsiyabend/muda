//! Root view for the editor application.
//!
//! `EditorRootView` holds a shared reference to an `EditorDataSource` adapter
//! and rebuilds the full AppLayout from fresh `RenderModel` data each frame.
//! This is the bridge between the adapter boundary and the ora view tree.

use std::cell::RefCell;
use std::rc::Rc;

use crate::context::ViewContext;
use crate::editor_adapter::{
    EditorDataSource, FileTreeNode, FileTreePresentation, RenderModel, SidebarPresentation,
};
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

    /// Build a FileTreePresentation from the sidebar's flat entry list.
    ///
    /// core_editor provides a flat list of file/directory entries via
    /// SidebarPresentation. We convert these into FileTreeNode roots so
    /// the FileTreeView in SidebarView can render a proper file explorer.
    fn build_file_tree(sidebar: &SidebarPresentation) -> FileTreePresentation {
        if !sidebar.visible || sidebar.entries.is_empty() {
            return FileTreePresentation::default();
        }

        let roots: Vec<FileTreeNode> = sidebar
            .entries
            .iter()
            .map(|entry| {
                if entry.is_dir {
                    FileTreeNode::dir(&entry.name, false, vec![])
                } else {
                    let ext = std::path::Path::new(&entry.name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    FileTreeNode::file(&entry.name, ext)
                }
            })
            .collect();

        let selected_index = sidebar.entries.iter().position(|e| e.is_selected);

        FileTreePresentation {
            roots,
            selected_index,
        }
    }

    /// Build the AppLayout from a fresh RenderModel.
    fn build_layout(&self, model: &RenderModel) -> AppLayout {
        let file_tree = Self::build_file_tree(&model.sidebar);
        let sidebar = SidebarView::new(model.sidebar.clone(), file_tree);
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
        let dialog = DialogView::with_focus_handles(
            model.dialog.clone(),
            self.dialog_save_focus.clone(),
            self.dialog_dont_save_focus.clone(),
            self.dialog_cancel_focus.clone(),
        );

        let mut layout = AppLayout::new(
            sidebar, tab_bar, gutter, text_area, status_bar,
            panel_manager, command_palette, dialog,
        );

        // Sync sidebar visibility from the model data.
        // AppLayout defaults sidebar_visible to true, but if the core_editor
        // sidebar is not visible (no directory opened), we must hide it so
        // AppLayout::render_root_row skips the 0-width sidebar div entirely.
        layout.sidebar_visible = model.sidebar.visible;

        layout
    }
}

impl View for EditorRootView {
    fn render(&self, _cx: &mut ViewContext) -> AnyElement {
        // Query the adapter for the current viewport line count.
        // The event loop updates this on WindowEvent::Resized so the core
        // editor's viewport stays in sync with the actual window height.
        let viewport_lines = {
            let adapter = self.adapter.borrow();
            adapter.viewport_lines()
        };

        // Build fresh RenderModel from the adapter
        let model = {
            let adapter = self.adapter.borrow();
            adapter.build_render_model(viewport_lines)
        };

        // Construct the AppLayout from fresh data and render it
        let layout = self.build_layout(&model);
        layout.render(_cx)
    }
}
