//! Application state and winit integration.

use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use core_editor::app::App as EditorApp;

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
        let viewport_height = (window.inner_size().height as f32 / scale_factor
            / renderer.theme().line_height_px()) as usize;

        let model = self.editor.build_render_model(viewport_height);

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
                    }
                }
            }
        }
        self.request_redraw();
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
                }
                self.request_redraw();
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // Notify renderer of activity (for caret blinking)
                if let Some(renderer) = &mut self.renderer {
                    renderer.on_activity();
                }

                // Try app-level action first
                if let Some(action) = translate_app_action(
                    &event,
                    &self.modifiers,
                    self.editor.has_pending_action(),
                    self.editor.is_sidebar_focused(),
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
                        // Handle selection state for navigation commands
                        use core_editor::commands::EditorCommand;
                        match &cmd {
                            EditorCommand::MoveCursor { extend_selection, .. } => {
                                if *extend_selection && !self.editor.has_selection() {
                                    self.editor.begin_selection();
                                } else if !extend_selection {
                                    self.editor.clear_selection();
                                }
                            }
                            _ => {}
                        }

                        // Handle delete selection for editing commands
                        match &cmd {
                            EditorCommand::InsertChar(_)
                            | EditorCommand::InsertText(_)
                            | EditorCommand::InsertNewline
                            | EditorCommand::Backspace
                            | EditorCommand::Delete => {
                                if self.editor.has_selection()
                                    && !matches!(cmd, EditorCommand::Backspace | EditorCommand::Delete)
                                {
                                    self.editor.delete_selection();
                                }
                            }
                            _ => {}
                        }

                        // Dispatch the command
                        let handled = self.editor.dispatch(cmd);

                        // Update selection for move commands
                        if let Some(EditorCommand::MoveCursor { extend_selection: true, .. }) =
                            translate_key(&event, &self.modifiers)
                        {
                            let offset = self.editor.cursor_to_char_idx();
                            self.editor.extend_selection_to(offset);
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
