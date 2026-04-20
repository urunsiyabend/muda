//! Root view for the editor application.
//!
//! `EditorRootView` holds a shared reference to an `EditorDataSource` adapter
//! and rebuilds the full AppLayout from fresh `RenderModel` data each frame.
//! This is the bridge between the adapter boundary and the ora view tree.

use std::cell::RefCell;
use std::rc::Rc;

use crate::context::ViewContext;
use crate::editor_adapter::{
    EditorCommand, EditorDataSource, FileTreeNode, FileTreePresentation, RenderModel,
    SidebarPresentation,
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
    /// Shared smooth-scroll pixel offset from the event loop accumulator.
    scroll_offset: crate::app::SharedScrollOffset,
    /// Shared focus state from the event loop (window focus tracking).
    focus_state: crate::app::SharedFocusState,
    /// Persistent sidebar scroll state (survives across frames).
    sidebar_scroll: crate::elements::SharedScrollState,
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
    pub fn new(
        adapter: SharedAdapter,
        scroll_offset: crate::app::SharedScrollOffset,
        focus_state: crate::app::SharedFocusState,
        cx: &mut ViewContext,
    ) -> Self {
        Self {
            adapter,
            scroll_offset,
            focus_state,
            sidebar_scroll: crate::elements::scroll_state(),
            dialog_save_focus: cx.focus_handle(),
            dialog_dont_save_focus: cx.focus_handle(),
            dialog_cancel_focus: cx.focus_handle(),
            palette_focus: cx.focus_handle(),
        }
    }

    /// Build a FileTreePresentation from the sidebar's recursive tree.
    fn build_file_tree(sidebar: &SidebarPresentation) -> FileTreePresentation {
        if !sidebar.visible || sidebar.tree.is_empty() {
            return FileTreePresentation::default();
        }

        FileTreePresentation {
            roots: sidebar.tree.clone(),
            // Don't highlight any entry by default — keyboard selection
            // is a TUI concept, not relevant for mouse-driven tree view
            selected_index: None,
        }
    }

    /// Build the AppLayout from a fresh RenderModel.
    fn build_layout(&self, model: &RenderModel) -> AppLayout {
        // Build shared dispatch closure for click handlers (tabs, sidebar, etc.).
        let adapter_for_dispatch = self.adapter.clone();
        let dispatch: Rc<dyn Fn(EditorCommand)> = Rc::new(move |cmd: EditorCommand| {
            adapter_for_dispatch.borrow_mut().dispatch_command(cmd);
        });

        // Get window height from viewport_lines (reverse the event loop calculation)
        const LINE_HEIGHT: f32 = 21.0;
        const TAB_BAR_H: f32 = 36.0;
        const STATUS_BAR_H: f32 = 28.0;
        let viewport_lines = {
            let adapter = self.adapter.borrow();
            adapter.viewport_lines()
        };
        let window_h = (viewport_lines as f32 * LINE_HEIGHT) + TAB_BAR_H + STATUS_BAR_H;

        let file_tree = Self::build_file_tree(&model.sidebar);
        let sidebar = SidebarView::new(model.sidebar.clone(), file_tree)
            .with_dispatch(dispatch.clone())
            .with_scroll_state(self.sidebar_scroll.clone())
            .with_content_height(window_h);

        let tab_bar = TabBarView::new(model.tab_bar.clone()).with_dispatch(dispatch.clone());
        let gutter = GutterView::new_with_scroll_offset(
            model.gutter.clone(),
            model.visible_lines.clone(),
            model.scroll_y_offset_px,
        );
        // Always hand TextAreaView the fixed viewport row count so the
        // element tree shape stays constant across scroll frames — the
        // invariant the cached layout depends on. `viewport_lines` from
        // the adapter is driven by the window height, not by scroll
        // position, so it is stable frame-to-frame.
        let text_area = TextAreaView::with_viewport(model, viewport_lines);
        let status_bar = StatusBarView::new(model.status.clone());
        let panel_manager = PanelManagerView::new();
        let command_palette = CommandPaletteView::new(
            vec![
                CommandItem::new("palette:toggle-theme", "Toggle Theme"),
                CommandItem::new("palette:toggle-sidebar", "Toggle Sidebar"),
                CommandItem::new("palette:toggle-panel", "Toggle Panel"),
                CommandItem::new("file.open-folder", "Open Folder").with_shortcut("Ctrl+Shift+O"),
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
        let mut model = {
            let adapter = self.adapter.borrow();
            adapter.build_render_model(viewport_lines)
        };

        // Compute the partial-line pixel offset from the absolute scroll position.
        //
        // scroll_offset carries the full pixel distance from the document top
        // (scroll_top_px). The fractional part `scroll_top_px % LINE_HEIGHT`
        // is the amount by which the topmost visible line is shifted upward.
        // This is passed to TextAreaView and GutterView for sub-line rendering.
        // The container's overflow_hidden() + GPU scissor clips partial lines.
        const LINE_HEIGHT: f32 = 21.0;
        let scroll_top_px = self.scroll_offset.get();
        model.scroll_y_offset_px = scroll_top_px % LINE_HEIGHT;
        model.editor_focused = self.focus_state.get();

        // Construct the AppLayout from fresh data and render it
        let layout = self.build_layout(&model);
        layout.render(_cx)
    }
}
