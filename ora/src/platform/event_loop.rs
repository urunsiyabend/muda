use crate::app::App;
use crate::context::{AppContext, WindowContext};
use crate::element::{LayoutContext, PaintContext, PrepaintContext};
use crate::entity::EntityStorage;
use crate::events::mouse::{hit_test, Hitbox, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use crate::events::types::{MouseButton, Modifiers, Point};
use crate::events::dispatch::{dispatch_mouse_down, dispatch_mouse_move, dispatch_mouse_up, EventHandlers};
use crate::events::keyboard::{translate_key_event, Key, NamedKey};
use crate::events::actions::KeyContext;
use crate::platform::gpu::GpuState;
use crate::window::OraWindow;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

/// Application handler driving the winit event loop.
pub struct OraApp {
    app_config: Option<App>,
    gpu_state: Option<GpuState>,
    ora_window: OraWindow,
    app_context: AppContext,
    hitboxes: Vec<Hitbox>,
    cursor_position: Point,
    event_handlers: EventHandlers,
    modifiers: Modifiers,
}

impl OraApp {
    pub fn new(app: App) -> Self {
        Self {
            app_config: Some(app),
            gpu_state: None,
            ora_window: OraWindow::new(),
            app_context: AppContext::new(EntityStorage::new()),
            hitboxes: Vec::new(),
            cursor_position: Point::new(0.0, 0.0),
            event_handlers: EventHandlers::new(),
            modifiers: Modifiers::none(),
        }
    }
}

impl ApplicationHandler for OraApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_state.is_some() {
            return; // Already initialized
        }

        let app_config = self.app_config.take().expect("App config missing");

        // Create window with title and size
        let window_attributes = Window::default_attributes()
            .with_title(app_config.title)
            .with_inner_size(winit::dpi::PhysicalSize::new(
                app_config.size.0,
                app_config.size.1,
            ));

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");
        let window = Arc::new(window);

        // Initialize GPU state
        let gpu_state = pollster::block_on(GpuState::new(window.clone()));
        self.gpu_state = Some(gpu_state);

        // Call on_open callback if present
        if let Some(on_open) = app_config.on_open {
            let winit_window = &self.gpu_state.as_ref().unwrap().window;
            let mut window_context = WindowContext::new(
                &mut self.app_context,
                &mut self.ora_window,
                winit_window,
            );
            on_open(&mut window_context);
        }

        // Request initial redraw
        if let Some(gpu_state) = &self.gpu_state {
            gpu_state.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    gpu_state.resize(physical_size.width, physical_size.height);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(modifiers_state) => {
                // Update modifier state
                self.modifiers = Modifiers {
                    ctrl: modifiers_state.state().control_key(),
                    alt: modifiers_state.state().alt_key(),
                    shift: modifiers_state.state().shift_key(),
                    meta: modifiers_state.state().super_key(),
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x as f32, position.y as f32);
                self.cursor_position = point;

                // Check for mouse capture - if captured, route to captured element
                let hit_id = if let Some(capture) = self.app_context.interaction_state.mouse_capture() {
                    Some(capture.hitbox_id)
                } else {
                    hit_test(&self.hitboxes, point)
                };

                // Update hover state
                let prev_hover = self.app_context.interaction_state.hovered_hitbox();
                self.app_context.interaction_state.update_hover(hit_id);

                // Log hover state changes
                if prev_hover != hit_id {
                    match (prev_hover, hit_id) {
                        (None, Some(id)) => log::info!("Hover entered: HitboxId({:?})", id.0),
                        (Some(_), None) => log::info!("Hover exited"),
                        (Some(old_id), Some(new_id)) => log::info!("Hover changed: {:?} -> {:?}", old_id.0, new_id.0),
                        (None, None) => {},
                    }
                }

                // Dispatch mouse move event
                if let Some(id) = hit_id {
                    let event = MouseMoveEvent {
                        position: point,
                        modifiers: self.modifiers,
                    };
                    dispatch_mouse_move(&mut self.event_handlers, &event, id);
                }

                // Request redraw to update hover state visuals
                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let mouse_button = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Back => MouseButton::Back,
                    winit::event::MouseButton::Forward => MouseButton::Forward,
                    winit::event::MouseButton::Other(n) => MouseButton::Other(n),
                };

                if let Some(hit_id) = hit_test(&self.hitboxes, self.cursor_position) {
                    match state {
                        winit::event::ElementState::Pressed => {
                            // Set active state
                            self.app_context.interaction_state.set_active(hit_id);
                            log::info!("Active state: Mouse button {:?} pressed on HitboxId({:?})", mouse_button, hit_id.0);

                            let event = MouseDownEvent {
                                position: self.cursor_position,
                                button: mouse_button,
                                modifiers: self.modifiers,
                            };
                            dispatch_mouse_down(&mut self.event_handlers, &event, hit_id);
                        }
                        winit::event::ElementState::Released => {
                            // Clear active state and release capture if present
                            self.app_context.interaction_state.clear_active();
                            self.app_context.interaction_state.release_mouse_capture();
                            log::info!("Active state: Mouse button {:?} released", mouse_button);

                            let event = MouseUpEvent {
                                position: self.cursor_position,
                                button: mouse_button,
                                modifiers: self.modifiers,
                            };
                            dispatch_mouse_up(&mut self.event_handlers, &event, hit_id);
                        }
                    }

                    // Request redraw to update active state visuals
                    if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                // Translate winit key event to ora keyboard event
                if let Some(keyboard_event) = translate_key_event(&key_event, self.modifiers) {
                    // Handle Tab navigation first (before action matching)
                    if key_event.state.is_pressed() {
                        if let Key::Named(NamedKey::Tab) = keyboard_event.keystroke.key {
                            let prev_focused = self.app_context.focus_state.focused_id();
                            if self.modifiers.shift {
                                log::info!("Tab navigation: Shift+Tab pressed, focusing previous");
                                self.app_context.focus_prev();
                            } else {
                                log::info!("Tab navigation: Tab pressed, focusing next");
                                self.app_context.focus_next();
                            }
                            let new_focused = self.app_context.focus_state.focused_id();
                            log::info!("Focus changed: {:?} -> {:?}", prev_focused, new_focused);

                            // Request redraw to show new focus state
                            if let Some(gpu_state) = &self.gpu_state {
                                gpu_state.window.request_redraw();
                            }
                            // Tab handled, skip action matching
                        } else if let Key::Character(c) = &keyboard_event.keystroke.key {
                            // DEMO HACK: Handle 'T' key for theme toggle
                            // This is a temporary workaround for demos until action handlers get context access
                            if c == "t" || c == "T" {
                                use crate::theme::{Theme, ThemeMode};
                                let current_mode = self.app_context.theme().mode();
                                let new_theme = match current_mode {
                                    ThemeMode::Dark => Theme::light(),
                                    ThemeMode::Light => Theme::dark(),
                                };
                                let new_mode = new_theme.mode();
                                self.app_context.set_theme(new_theme);
                                log::info!("Theme toggled to: {:?}", new_mode);

                                // Request redraw to show new theme
                                if let Some(gpu_state) = &self.gpu_state {
                                    gpu_state.window.request_redraw();
                                }
                            } else {
                                // Match keystroke against keymap
                                let context = KeyContext::new(); // TODO: Build context from focus stack
                                if let Some(action) = self.app_context.match_action(&keyboard_event.keystroke, &context) {
                                    // Clone the action so we can dispatch it with mutable context
                                    let action_clone = action.boxed_clone();
                                    self.app_context.dispatch_action(&*action_clone);

                                    // Request redraw after action dispatch
                                    if let Some(gpu_state) = &self.gpu_state {
                                        gpu_state.window.request_redraw();
                                    }
                                }
                            }
                        } else {
                            // Match keystroke against keymap
                            let context = KeyContext::new(); // TODO: Build context from focus stack
                            if let Some(action) = self.app_context.match_action(&keyboard_event.keystroke, &context) {
                                // Clone the action so we can dispatch it with mutable context
                                let action_clone = action.boxed_clone();
                                self.app_context.dispatch_action(&*action_clone);

                                // Request redraw after action dispatch
                                if let Some(gpu_state) = &self.gpu_state {
                                    gpu_state.window.request_redraw();
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                // Clear all interaction state when window loses focus
                if !focused {
                    self.app_context.interaction_state.clear_all();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    // Log dirty state for debugging
                    log::trace!("Redraw: dirty={}", self.app_context.has_dirty_entities());

                    // Render the root view to get element tree
                    if let Some(mut element_tree) = self.ora_window.render(&mut self.app_context) {
                        let window_size = gpu_state.size;

                        // Phase 1: Request layout with text measurement
                        let mut layout_cx = LayoutContext::new(
                            &mut self.app_context.entity_storage,
                            window_size,
                        );

                        // Pass TextSystem for text measurement during layout
                        layout_cx.set_text_system(&mut gpu_state.text_system as *mut _);

                        element_tree.request_layout(&mut layout_cx);

                        // Compute layout using flexbox algorithm
                        layout_cx.compute();
                        let layout_outputs = layout_cx.layout_outputs.clone();

                        // Phase 2: Prepaint
                        let mut prepaint_cx = PrepaintContext::new(
                            &mut self.app_context.entity_storage,
                            window_size,
                            &layout_outputs,
                        );
                        element_tree.prepaint(&mut prepaint_cx);

                        // Store hitboxes and event handlers for mouse event routing
                        self.hitboxes = prepaint_cx.take_hitboxes();
                        self.event_handlers = prepaint_cx.take_event_handlers();

                        // Transfer focus order to FocusState
                        let focusables = prepaint_cx.take_focusables();
                        self.app_context.focus_state.clear_focus_order();
                        for focus_id in focusables {
                            self.app_context.focus_state.register_focusable(focus_id);
                        }

                        // Phase 3: Paint
                        // SAFETY: We use a raw pointer for app_context to avoid aliasing issues.
                        // The app_context reference is valid for the entire paint phase and not mutated.
                        let app_context_ptr = &self.app_context as *const AppContext;
                        let mut paint_cx = PaintContext::new(
                            unsafe { &*app_context_ptr },
                            &mut self.app_context.entity_storage,
                            window_size,
                            &layout_outputs,
                            &self.app_context.interaction_state,
                            &self.app_context.focus_state,
                        );
                        element_tree.paint(&mut paint_cx);

                        // Extract paint commands and render
                        let commands = paint_cx.take_commands();
                        match gpu_state.render_frame(&commands) {
                            Ok(_) => {
                                // Clear dirty entities after rendering
                                self.app_context.clear_dirty();
                                // Request continuous redraw for now (don't break existing demos)
                                gpu_state.window.request_redraw();
                            }
                            Err(wgpu::SurfaceError::Lost) => {
                                // Reconfigure the surface if lost
                                gpu_state.resize(gpu_state.size.0, gpu_state.size.1);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("Out of memory");
                                event_loop.exit();
                            }
                            Err(e) => {
                                log::warn!("Surface error: {:?}", e);
                            }
                        }
                    } else {
                        // No root view, just clear
                        match gpu_state.render_frame(&[]) {
                            Ok(_) => {
                                gpu_state.window.request_redraw();
                            }
                            Err(wgpu::SurfaceError::Lost) => {
                                gpu_state.resize(gpu_state.size.0, gpu_state.size.1);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("Out of memory");
                                event_loop.exit();
                            }
                            Err(e) => {
                                log::warn!("Surface error: {:?}", e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // After processing any event, tick the async executor
        while self.app_context.tick_executor() {}

        // Check for dirty entities and request redraw if needed
        if self.app_context.has_dirty_entities() {
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Tick executor when idle
        while self.app_context.tick_executor() {}

        // Request redraw if entities are dirty
        if self.app_context.has_dirty_entities() {
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        }
    }
}
