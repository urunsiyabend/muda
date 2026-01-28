//! Application state and winit integration.

use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

use core_editor::app::App as EditorApp;
use core_editor::domain::TextPosition;

use crate::input::{translate_app_action, translate_key, AppAction};
use crate::renderer::GpuRenderer;

/// Main application state for the GPU client.
pub struct WgpuApp {
    /// The core editor state.
    editor: EditorApp,
    /// Window handle (created on resume).
    window: Option<Arc<Window>>,
    /// GPU renderer (created after window).
    renderer: Option<GpuRenderer>,
    /// Current keyboard modifiers state.
    modifiers: winit::event::Modifiers,
    /// Time of last frame for animation timing.
    last_frame: Instant,
    /// Whether a redraw is pending.
    redraw_pending: bool,
    /// Current cursor position (logical pixels).
    cursor_position: Option<(f64, f64)>,
    /// Last click time for double-click detection.
    last_click_time: Option<Instant>,
    /// Last click position for double-click detection.
    last_click_pos: Option<(f64, f64)>,
    /// Cached viewport dimensions to avoid recalculating every frame.
    cached_viewport: Option<(usize, usize)>,
    /// Current cursor icon.
    current_cursor: CursorIcon,
    /// Whether we're currently dragging to select text.
    is_dragging: bool,
}

impl WgpuApp {
    /// Creates a new application with an empty document.
    pub fn new() -> Self {
        Self {
            editor: EditorApp::new(),
            window: None,
            renderer: None,
            modifiers: winit::event::Modifiers::default(),
            last_frame: Instant::now(),
            redraw_pending: true,
            cursor_position: None,
            last_click_time: None,
            last_click_pos: None,
            cached_viewport: None,
            current_cursor: CursorIcon::Default,
            is_dragging: false,
        }
    }

    /// Creates an application by opening a file.
    pub fn open_file(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            editor: EditorApp::open_file(path)?,
            window: None,
            renderer: None,
            modifiers: winit::event::Modifiers::default(),
            last_frame: Instant::now(),
            redraw_pending: true,
            cursor_position: None,
            last_click_time: None,
            last_click_pos: None,
            cached_viewport: None,
            current_cursor: CursorIcon::Default,
            is_dragging: false,
        })
    }

    /// Creates an application by opening a directory.
    pub fn open_directory(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            editor: EditorApp::open_directory(path)?,
            window: None,
            renderer: None,
            modifiers: winit::event::Modifiers::default(),
            last_frame: Instant::now(),
            redraw_pending: true,
            cursor_position: None,
            last_click_time: None,
            last_click_pos: None,
            cached_viewport: None,
            current_cursor: CursorIcon::Default,
            is_dragging: false,
        })
    }

    /// Requests a redraw.
    fn request_redraw(&mut self) {
        self.redraw_pending = true;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Renders the current frame.
    fn render(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &mut self.renderer else { return };

        let scale_factor = window.scale_factor() as f32;
        let theme = renderer.theme();
        let line_height = theme.line_height_px();
        let char_width = renderer.char_width();

        // Calculate viewport dimensions in characters/lines
        let inner_size = window.inner_size();
        let logical_width = inner_size.width as f32 / scale_factor;
        let logical_height = inner_size.height as f32 / scale_factor;

        // Account for sidebar, gutter, and status bar
        let sidebar_width = self.editor.sidebar.width() as f32;
        let status_height = 24.0;
        let editor_width = ((logical_width - sidebar_width) / char_width) as usize;
        let editor_height = ((logical_height - status_height) / line_height) as usize;

        // Only update viewport if dimensions changed (performance optimization)
        let new_viewport = (editor_width, editor_height);
        if self.cached_viewport != Some(new_viewport) {
            self.editor.check_scrolling(editor_width, editor_height);
            self.cached_viewport = Some(new_viewport);
        }

        let model = self.editor.build_render_model(editor_height);

        match renderer.render(&model, scale_factor) {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Lost) => {
                renderer.resize(window.inner_size());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of GPU memory");
            }
            Err(e) => {
                log::warn!("Render error: {:?}", e);
            }
        }

        self.last_frame = Instant::now();
        self.redraw_pending = false;
    }

    /// Handles an app-level action.
    fn handle_app_action(&mut self, action: AppAction) {
        match action {
            AppAction::RequestQuit => {
                let _ = self.editor.request_exit();
            }
            AppAction::ToggleSidebar => {
                self.editor.toggle_sidebar();
            }
            AppAction::FocusSidebar => {
                self.editor.focus_sidebar();
            }
            AppAction::FocusEditor => {
                self.editor.focus_editor();
            }
            AppAction::ConfirmPendingAction => {
                if let Err(e) = self.editor.save() {
                    log::error!("Save on confirmation failed: {}", e);
                }
                self.editor.confirm_pending_action();
            }
            AppAction::CancelPendingAction => {
                self.editor.cancel_pending_action();
            }
            AppAction::SidebarUp => {
                self.editor.sidebar_move_up();
            }
            AppAction::SidebarDown => {
                self.editor.sidebar_move_down();
            }
            AppAction::SidebarBack => {
                self.editor.sidebar_go_back();
            }
            AppAction::SidebarOpen => {
                if let Some(entry) = self.editor.sidebar.selected_entry() {
                    if entry.is_dir {
                        let path = entry.path.clone();
                        self.editor.sidebar.set_base_directory(path);
                    } else {
                        let path = entry.path.clone();
                        let _ = self.editor.request_open_file(path);
                        // Invalidate cached viewport so the new view gets proper dimensions
                        self.cached_viewport = None;
                    }
                }
            }
            // Command palette actions
            AppAction::ToggleCommandPalette => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.toggle_command_palette();
                }
            }
            AppAction::CommandPaletteUp => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.command_palette_up();
                }
            }
            AppAction::CommandPaletteDown => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.command_palette_down();
                }
            }
            AppAction::CommandPaletteExecute => {
                if let Some(renderer) = &mut self.renderer {
                    if let Some(command_id) = renderer.command_palette_execute() {
                        self.execute_command(&command_id);
                    }
                }
            }
            AppAction::CommandPaletteClose => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.hide_command_palette();
                }
            }
            AppAction::CommandPaletteType(c) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.command_palette_type(c);
                }
            }
            AppAction::CommandPaletteBackspace => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.command_palette_backspace();
                }
            }
            // Panel actions
            AppAction::TogglePanel => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.toggle_panel();
                }
            }
        }
        self.request_redraw();
    }

    /// Execute a command by its ID.
    fn execute_command(&mut self, command_id: &str) {
        log::debug!("Executing command: {}", command_id);
        match command_id {
            "file.new" => {
                // TODO: Implement new file
            }
            "file.open" => {
                // TODO: Implement open file dialog
            }
            "file.save" => {
                if let Err(e) = self.editor.save() {
                    log::error!("Save failed: {}", e);
                }
            }
            "edit.undo" => {
                self.editor.dispatch(core_editor::commands::EditorCommand::Undo);
            }
            "edit.redo" => {
                self.editor.dispatch(core_editor::commands::EditorCommand::Redo);
            }
            "edit.cut" => {
                self.editor.dispatch(core_editor::commands::EditorCommand::Cut);
            }
            "edit.copy" => {
                self.editor.dispatch(core_editor::commands::EditorCommand::Copy);
            }
            "edit.paste" => {
                self.editor.dispatch(core_editor::commands::EditorCommand::Paste);
            }
            "view.sidebar" => {
                self.editor.toggle_sidebar();
            }
            _ => {
                log::warn!("Unknown command: {}", command_id);
            }
        }
    }

    /// Handles mouse click at the given position (logical pixels).
    fn handle_mouse_click(&mut self, x: f64, y: f64, is_double_click: bool) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &self.renderer else { return };

        let scale_factor = window.scale_factor() as f32;
        let theme = renderer.theme();
        let sidebar_width = self.editor.sidebar.width() as f32;
        let status_height = 24.0; // Status bar height
        let tab_bar_height = renderer.tab_bar_height();
        let screen_height = window.inner_size().height as f32 / scale_factor;

        // Convert from physical pixels (winit) to logical pixels (layout system)
        let x = x as f32 / scale_factor;
        let y = y as f32 / scale_factor;

        // Check if click is in status bar (ignore)
        if y > screen_height - status_height {
            return;
        }

        // Check if click is in sidebar - use FileTree's hit detection for correct bounds
        if self.editor.sidebar.visible && x < sidebar_width {
            drop(renderer);
            if let Some(renderer) = &mut self.renderer {
                if let Some((entry_id, _is_chevron)) = renderer.file_tree_on_click(x, y) {
                    // Select the entry (FileTree returns the correct entry index)
                    self.editor.sidebar.select_index(entry_id);
                    self.editor.focus_sidebar();

                    // On double-click, open the entry
                    if is_double_click {
                        if let Some(entry) = self.editor.sidebar.selected_entry() {
                            if entry.is_dir {
                                let path = entry.path.clone();
                                self.editor.sidebar.set_base_directory(path);
                            } else {
                                let path = entry.path.clone();
                                let _ = self.editor.request_open_file(path);
                                self.cached_viewport = None;
                            }
                        }
                    }
                    self.request_redraw();
                }
            }
            return;
        }

        // Check if click is in tab bar
        if y < tab_bar_height {
            // Handle tab clicks - we need to get the result and drop the renderer borrow first
            drop(renderer);
            if let Some(renderer) = &mut self.renderer {
                // x, y are already converted to logical pixels at the start of this function
                if let Some((view_id_raw, is_close)) = renderer.editor_tabs_on_click(x, y) {
                    // Convert raw u64 back to ViewId
                    let view_id = core_editor::ViewId::from_raw(view_id_raw);

                    if is_close {
                        // Close tab - close the view
                        let view_count = self.editor.workspace.views().count();
                        if view_count > 1 {
                            // Find the next view to switch to (must collect before mutating)
                            let next_id: Option<core_editor::ViewId> = self.editor.workspace.views()
                                .find(|(id, _)| **id != view_id)
                                .map(|(id, _)| *id);

                            // Switch to another view first
                            if let Some(next_id) = next_id {
                                self.editor.workspace.set_active_view(next_id);
                            }
                            self.editor.workspace.close_view(view_id);
                        }
                    } else {
                        // Switch to the clicked tab
                        self.editor.workspace.set_active_view(view_id);
                    }
                    self.request_redraw();
                }
            }
            return;
        }

        // Click is in editor area - adjust y for tab bar
        let char_width = renderer.char_width();
        self.handle_editor_click(x, y - tab_bar_height, sidebar_width, char_width, theme.line_height_px());
    }

    /// Handles drag selection - extends selection as mouse moves.
    fn handle_drag_selection(&mut self, x: f64, y: f64) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &self.renderer else { return };

        let scale_factor = window.scale_factor() as f32;
        let theme = renderer.theme();
        let sidebar_width = self.editor.sidebar.width() as f32;
        let status_height = 24.0;
        let tab_bar_height = renderer.tab_bar_height();
        let screen_height = window.inner_size().height as f32 / scale_factor;

        // Convert from physical pixels (winit) to logical pixels (layout system)
        let x = x as f32 / scale_factor;
        let y = y as f32 / scale_factor;

        // Only handle drag in editor area
        if y > screen_height - status_height {
            return;
        }
        if y < tab_bar_height {
            return;
        }
        if self.editor.sidebar.visible && x < sidebar_width {
            return;
        }

        // Calculate position in editor (adjust for tab bar)
        let char_width = renderer.char_width();
        let line_height = theme.line_height_px();
        let y = y - tab_bar_height;

        let show_line_numbers = self.editor.workspace.active_view()
            .map(|v| v.options.show_line_numbers)
            .unwrap_or(true);
        let gutter_width = if show_line_numbers {
            let total_lines = self.editor.workspace.active_document()
                .map(|d| d.len_lines())
                .unwrap_or(1);
            let digit_count = total_lines.to_string().len();
            8.0 + (digit_count + 2) as f32 * char_width // Include gutter padding
        } else {
            0.0
        };

        let text_x = (x - sidebar_width - gutter_width).max(0.0);

        let row = (y.max(0.0) / line_height) as usize;
        let col = (text_x / char_width) as usize;

        let scroll_y = self.editor.workspace.active_view()
            .map(|v| v.viewport.scroll_y)
            .unwrap_or(0);

        let document_row = row + scroll_y;

        // Calculate offset and extend selection
        let offset = if let Some(doc) = self.editor.workspace.active_document() {
            let max_row = doc.len_lines().saturating_sub(1);
            let actual_row = document_row.min(max_row);
            let line_len = doc.line_len(actual_row);
            let actual_col = col.min(line_len);
            let pos = TextPosition::new(actual_row, actual_col);
            doc.position_to_offset(pos)
        } else {
            return;
        };

        // Start selection if not already started
        if !self.editor.has_selection() {
            self.editor.begin_selection();
        }

        // Extend selection to new position
        self.editor.extend_selection_to(offset);
        self.request_redraw();

        // Notify renderer of activity
        if let Some(renderer) = &mut self.renderer {
            renderer.on_activity();
        }
    }

    /// Handles click in the editor area.
    fn handle_editor_click(&mut self, x: f32, y: f32, sidebar_width: f32, char_width: f32, line_height: f32) {
        // Adjust x for sidebar and gutter
        let show_line_numbers = self.editor.workspace.active_view()
            .map(|v| v.options.show_line_numbers)
            .unwrap_or(true);
        let gutter_width = if show_line_numbers {
            let total_lines = self.editor.workspace.active_document()
                .map(|d| d.len_lines())
                .unwrap_or(1);
            let digit_count = total_lines.to_string().len();
            8.0 + (digit_count + 2) as f32 * char_width // Include gutter left padding
        } else {
            0.0
        };

        let text_x = x - sidebar_width - gutter_width;
        let text_y = y;

        if text_x < 0.0 || text_y < 0.0 {
            return;
        }

        // Calculate row and column
        let row = (text_y / line_height) as usize;
        let col = (text_x / char_width) as usize;

        // Get scroll offset
        let scroll_y = self.editor.workspace.active_view()
            .map(|v| v.viewport.scroll_y)
            .unwrap_or(0);

        // Move cursor to clicked position
        let document_row = row + scroll_y;

        // Get document info and calculate offset
        let offset = if let Some(doc) = self.editor.workspace.active_document() {
            let max_row = doc.len_lines().saturating_sub(1);
            let actual_row = document_row.min(max_row);
            let line_len = doc.line_len(actual_row);
            let actual_col = col.min(line_len);
            let pos = TextPosition::new(actual_row, actual_col);
            doc.position_to_offset(pos)
        } else {
            return;
        };

        // Clear any existing selection and set the caret position
        self.editor.clear_selection();
        if let Some(view) = self.editor.workspace.active_view_mut() {
            view.carets.move_to(offset);
        }

        // Start selection anchor at this position (for drag selection)
        self.editor.begin_selection();

        self.editor.focus_editor();
        self.request_redraw();

        // Notify renderer of activity
        if let Some(renderer) = &mut self.renderer {
            renderer.on_activity();
        }
    }

    /// Updates the cursor icon based on mouse position.
    fn update_cursor_icon(&mut self, x: f64, y: f64) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &self.renderer else { return };

        let scale_factor = window.scale_factor() as f32;
        let sidebar_width = self.editor.sidebar.width() as f32;
        let status_height = 24.0;
        let screen_height = window.inner_size().height as f32 / scale_factor;

        let x = x as f32;
        let y = y as f32;

        let new_cursor = if y > screen_height - status_height {
            // Status bar
            CursorIcon::Default
        } else if self.editor.sidebar.visible && x < sidebar_width {
            // Sidebar
            CursorIcon::Default
        } else {
            // Editor area - use text cursor
            CursorIcon::Text
        };

        if new_cursor != self.current_cursor {
            self.current_cursor = new_cursor;
            window.set_cursor(new_cursor);
        }
    }

    /// Checks if a click is a double-click.
    fn is_double_click(&mut self, x: f64, y: f64) -> bool {
        const DOUBLE_CLICK_TIME_MS: u128 = 500;
        const DOUBLE_CLICK_DISTANCE: f64 = 5.0;

        let now = Instant::now();

        let is_double = if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
            let elapsed = now.duration_since(last_time).as_millis();
            let distance = ((x - last_pos.0).powi(2) + (y - last_pos.1).powi(2)).sqrt();
            elapsed < DOUBLE_CLICK_TIME_MS && distance < DOUBLE_CLICK_DISTANCE
        } else {
            false
        };

        self.last_click_time = Some(now);
        self.last_click_pos = Some((x, y));

        is_double
    }
}

impl Default for WgpuApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Create window
        let window_attrs = WindowAttributes::default()
            .with_title("Muda Editor")
            .with_inner_size(winit::dpi::LogicalSize::new(1200, 800));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        // Create renderer
        let renderer = pollster::block_on(GpuRenderer::new(window.clone()))
            .expect("Failed to create GPU renderer");

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.request_redraw();
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // Handle timer-based redraws (caret blinking)
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(renderer) = &mut self.renderer {
                if renderer.update() {
                    self.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.editor.request_exit();
                if self.editor.should_quit {
                    event_loop.exit();
                } else {
                    self.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                    // Invalidate cached viewport to force recalculation on next render
                    self.cached_viewport = None;
                }
                self.request_redraw();
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods;
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
                self.update_cursor_icon(position.x, position.y);

                // Block drag selection when dialog is open
                if self.editor.has_pending_action() {
                    return;
                }

                // Handle hover state for EditorTabs
                if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
                    // Convert from physical pixels (winit) to logical pixels (layout system)
                    let scale_factor = window.scale_factor() as f32;
                    let lx = position.x as f32 / scale_factor;
                    let ly = position.y as f32 / scale_factor;
                    if renderer.editor_tabs_on_pointer_move(lx, ly) {
                        self.request_redraw();
                    }
                }

                // Handle drag selection
                if self.is_dragging {
                    self.handle_drag_selection(position.x, position.y);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Block mouse input when dialog is open
                if self.editor.has_pending_action() {
                    return;
                }

                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if let Some((x, y)) = self.cursor_position {
                            let is_double = self.is_double_click(x, y);
                            self.handle_mouse_click(x, y, is_double);
                            // Start dragging for selection (only in editor area)
                            if !is_double {
                                self.is_dragging = true;
                            }
                        }
                    } else {
                        // Mouse released - stop dragging
                        self.is_dragging = false;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Block mouse wheel when dialog is open
                if self.editor.has_pending_action() {
                    return;
                }

                let scroll_lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        -(y as isize * 3) // 3 lines per scroll notch
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let line_height = self.renderer.as_ref()
                            .map(|r| r.theme().line_height_px())
                            .unwrap_or(21.0);
                        -(pos.y / line_height as f64) as isize
                    }
                };

                if scroll_lines != 0 {
                    // Get max_scroll first to avoid borrow conflict
                    let max_scroll = self.editor.workspace.active_document()
                        .map(|d| d.len_lines().saturating_sub(1))
                        .unwrap_or(0);

                    if let Some(view) = self.editor.workspace.active_view_mut() {
                        let current_scroll = view.viewport.scroll_y as isize;
                        let new_scroll = (current_scroll + scroll_lines).max(0) as usize;
                        view.viewport.scroll_y = new_scroll.min(max_scroll);
                    }
                    self.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // Notify renderer of activity (for caret blinking)
                if let Some(renderer) = &mut self.renderer {
                    renderer.on_activity();
                }

                // Handle dialog input first - block all other input when dialog is open
                if self.editor.has_pending_action() {
                    if event.state == ElementState::Pressed {
                        use winit::keyboard::{Key, NamedKey};
                        match &event.logical_key {
                            // Enter = Save and proceed
                            Key::Named(NamedKey::Enter) => {
                                if let Err(e) = self.editor.save() {
                                    log::error!("Save failed: {}", e);
                                }
                                self.editor.confirm_pending_action();
                                if self.editor.should_quit {
                                    event_loop.exit();
                                }
                                self.request_redraw();
                            }
                            // Y = Save and proceed
                            Key::Character(c) if c == "y" || c == "Y" => {
                                if let Err(e) = self.editor.save() {
                                    log::error!("Save failed: {}", e);
                                }
                                self.editor.confirm_pending_action();
                                if self.editor.should_quit {
                                    event_loop.exit();
                                }
                                self.request_redraw();
                            }
                            // N = Don't save, proceed anyway
                            Key::Character(c) if c == "n" || c == "N" => {
                                self.editor.confirm_pending_action();
                                if self.editor.should_quit {
                                    event_loop.exit();
                                }
                                self.request_redraw();
                            }
                            // Escape = Cancel
                            Key::Named(NamedKey::Escape) => {
                                self.editor.cancel_pending_action();
                                self.request_redraw();
                            }
                            _ => {}
                        }
                    }
                    return; // Block all other input when dialog is open
                }

                // Handle F12 to toggle debug overlay (debug builds only)
                if event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};
                    if let Key::Named(NamedKey::F12) = &event.logical_key {
                        if let Some(renderer) = &mut self.renderer {
                            renderer.toggle_debug_overlay();
                            self.request_redraw();
                        }
                        return;
                    }
                }

                // Check if command palette is visible
                let command_palette_visible = self.renderer
                    .as_ref()
                    .map(|r| r.is_command_palette_visible())
                    .unwrap_or(false);

                // Try app-level action first
                if let Some(action) = translate_app_action(
                    &event,
                    &self.modifiers,
                    self.editor.has_pending_action(),
                    self.editor.is_sidebar_focused(),
                    command_palette_visible,
                ) {
                    self.handle_app_action(action);

                    if self.editor.should_quit {
                        event_loop.exit();
                    }
                    return;
                }

                // Try editor command
                if !self.editor.is_sidebar_focused() {
                    if let Some(cmd) = translate_key(&event, &self.modifiers) {
                        use core_editor::commands::EditorCommand;

                        // Handle delete selection for editing commands (before dispatch)
                        match &cmd {
                            EditorCommand::InsertChar(_)
                            | EditorCommand::InsertText(_)
                            | EditorCommand::InsertNewline => {
                                if self.editor.has_selection() {
                                    self.editor.delete_selection();
                                }
                            }
                            _ => {}
                        }

                        // Dispatch the command - dispatcher handles selection for MoveCursor
                        let handled = self.editor.dispatch(cmd.clone());

                        // After cursor movement or editing, ensure cursor is visible
                        match &cmd {
                            EditorCommand::MoveCursor { .. }
                            | EditorCommand::InsertChar(_)
                            | EditorCommand::InsertText(_)
                            | EditorCommand::InsertNewline
                            | EditorCommand::Paste => {
                                self.cached_viewport = None;
                            }
                            _ => {}
                        }

                        if handled {
                            self.request_redraw();
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                self.render();

                // Schedule next frame for animations
                if let Some(renderer) = &self.renderer {
                    let next_frame = renderer.time_until_next_frame();
                    if let Some(window) = &self.window {
                        // Use set_request_redraw_expected for smooth animations
                        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                            Instant::now() + next_frame,
                        ));
                    }
                }
            }

            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("Application exiting");
    }
}
