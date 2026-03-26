use crate::app::App;
use crate::context::{AppContext, WindowContext};
use crate::editor_adapter::EditorDataSource;
use crate::element::{LayoutContext, PaintContext, PrepaintContext};
use crate::elements::{BLINK_RATE, ACTIVITY_TIMEOUT};
use crate::entity::EntityStorage;
use crate::events::editor_input::translate_editor_command;
use crate::events::mouse::{hit_test, Hitbox, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use crate::events::types::{MouseButton, Modifiers, Point};
use crate::events::dispatch::{dispatch_mouse_down, dispatch_mouse_move, dispatch_mouse_up, EventHandlers};
use crate::events::keyboard::{translate_key_event, Key, NamedKey};
use crate::events::actions::KeyContext;
use crate::platform::gpu::GpuState;
use crate::views::SharedAdapter;
use crate::window::OraWindow;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
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
    /// Optional editor adapter. When present, keyboard events that are not
    /// handled by Tab navigation or the action system are translated to
    /// `EditorCommand` and dispatched through the adapter.
    /// Shared with the `EditorRootView` via `Rc<RefCell<>>`.
    editor_adapter: Option<SharedAdapter>,
    /// Absolute pixel scroll position (distance from document top in pixels).
    ///
    /// This is the primary scroll state. It is incremented by mouse wheel
    /// deltas and clamped to `[0, (total_lines - 1) * LINE_HEIGHT]`.
    ///
    /// `floor(scroll_top_px / LINE_HEIGHT)` gives the first visible line index,
    /// which is kept in sync with `core_editor`'s `viewport.scroll_y` by
    /// dispatching `Scroll(delta_lines)` commands.
    ///
    /// `scroll_top_px % LINE_HEIGHT` is the partial-line visual offset passed
    /// to `TextAreaView` / `GutterView` for sub-line smooth rendering.
    scroll_top_px: f32,
    /// Shared absolute pixel scroll offset, read by EditorRootView each frame.
    /// `None` when running without an editor adapter.
    shared_scroll_offset: Option<crate::app::SharedScrollOffset>,
    /// Next scheduled caret blink wakeup. When `Some(instant)`, the event loop
    /// sleeps until that instant and then requests a redraw for the blink toggle.
    /// Reset to `Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE` on each keystroke.
    /// `None` means no caret is active and the loop sleeps indefinitely.
    next_blink_instant: Option<Instant>,
    /// Double-click tracking: timestamp and position of last mouse press.
    last_click_time: Option<Instant>,
    last_click_pos: Point,
    click_count: u32,
    /// Sidebar scroll offset in pixels (separate from editor scroll).
    sidebar_scroll_px: f32,
    /// Shared sidebar scroll offset, read by EditorRootView each frame.
    shared_sidebar_scroll: Option<Rc<Cell<f32>>>,
}

impl OraApp {
    /// Create a new OraApp from an app configuration.
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
            editor_adapter: None,
            scroll_top_px: 0.0,
            shared_scroll_offset: None,
            next_blink_instant: Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE),
            last_click_time: None,
            last_click_pos: Point::new(0.0, 0.0),
            click_count: 0,
            sidebar_scroll_px: 0.0,
            shared_sidebar_scroll: None,
        }
    }

    /// Create a new OraApp that includes an editor backend adapter.
    ///
    /// The adapter is shared via `Rc<RefCell<>>` with the `EditorRootView`
    /// so both the view (for `build_render_model`) and the event loop
    /// (for `dispatch_command`) can access it.
    ///
    /// `scroll_offset` is the shared absolute pixel scroll offset, also read
    /// by `EditorRootView` for pixel-based scroll rendering.
    pub fn new_with_editor(
        app: App,
        adapter: SharedAdapter,
        scroll_offset: crate::app::SharedScrollOffset,
        sidebar_scroll: Rc<Cell<f32>>,
    ) -> Self {
        Self {
            app_config: Some(app),
            gpu_state: None,
            ora_window: OraWindow::new(),
            app_context: AppContext::new(EntityStorage::new()),
            hitboxes: Vec::new(),
            cursor_position: Point::new(0.0, 0.0),
            event_handlers: EventHandlers::new(),
            modifiers: Modifiers::none(),
            editor_adapter: Some(adapter),
            scroll_top_px: 0.0,
            shared_scroll_offset: Some(scroll_offset),
            next_blink_instant: Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE),
            last_click_time: None,
            last_click_pos: Point::new(0.0, 0.0),
            click_count: 0,
            sidebar_scroll_px: 0.0,
            shared_sidebar_scroll: Some(sidebar_scroll),
        }
    }
}

impl OraApp {
    /// Synchronize `scroll_top_px` after core_editor updates its `scroll_y`.
    ///
    /// Called after keyboard-driven caret movement triggers
    /// `ensure_caret_visible` in core_editor. Snaps to the exact line
    /// boundary (no fractional offset) so the viewport doesn't judder
    /// when typing or navigating with arrow keys.
    fn sync_scroll_from_core(&mut self, new_scroll_y: usize) {
        const LINE_HEIGHT: f32 = 21.0;
        self.scroll_top_px = new_scroll_y as f32 * LINE_HEIGHT;
        if let Some(ref offset) = self.shared_scroll_offset {
            offset.set(self.scroll_top_px);
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
        let mut gpu_state = pollster::block_on(GpuState::new(window.clone()));

        // Measure actual monospace character width for accurate caret positioning.
        // This must happen after GPU/font init so glyphon can shape real glyphs.
        {
            let char_width = gpu_state.text_system.measure_monospace_char_width(14.0, 21.0);
            crate::rendering::set_measured_char_width(char_width);
            log::info!("Measured monospace char width: {:.2}px", char_width);
        }

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
                // Update the editor viewport so core_editor knows the new
                // visible line count and can scroll correctly.
                if let Some(adapter) = &self.editor_adapter {
                    const TAB_BAR_HEIGHT: f32 = 36.0;
                    const STATUS_BAR_HEIGHT: f32 = 28.0;
                    const LINE_HEIGHT: f32 = 21.0;
                    let chrome = TAB_BAR_HEIGHT + STATUS_BAR_HEIGHT;
                    let available = (physical_size.height as f32 - chrome).max(0.0);
                    let lines = (available / LINE_HEIGHT).floor() as usize;
                    let lines = lines.max(1);
                    adapter.borrow_mut().resize_viewport(200, lines);
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

                            // Track click count for double-click detection
                            let now = Instant::now();
                            let double_click_threshold = std::time::Duration::from_millis(400);
                            let distance_threshold = 5.0_f32;
                            let dx = self.cursor_position.x - self.last_click_pos.x;
                            let dy = self.cursor_position.y - self.last_click_pos.y;
                            let distance = (dx * dx + dy * dy).sqrt();

                            if let Some(last_time) = self.last_click_time {
                                if now.duration_since(last_time) < double_click_threshold
                                    && distance < distance_threshold
                                {
                                    self.click_count += 1;
                                } else {
                                    self.click_count = 1;
                                }
                            } else {
                                self.click_count = 1;
                            }
                            self.last_click_time = Some(now);
                            self.last_click_pos = self.cursor_position;

                            let event = MouseDownEvent {
                                position: self.cursor_position,
                                button: mouse_button,
                                modifiers: self.modifiers,
                                click_count: self.click_count,
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
            WindowEvent::MouseWheel { delta, .. } => {
                // Pixel-based scrolling.
                //
                // scroll_top_px is the absolute pixel distance from the top of
                // the document to the top of the visible viewport. It is
                // clamped to [0, max_scroll_px] and shared with EditorRootView.
                //
                // EditorRootView computes:
                //   first_line = floor(scroll_top_px / LINE_HEIGHT)
                //   partial_offset = scroll_top_px % LINE_HEIGHT  (0..LINE_HEIGHT)
                //
                // core_editor's viewport.scroll_y is kept in sync with
                // first_line so build_render_model returns the right lines.
                const LINE_HEIGHT: f32 = 21.0;
                // Pixels per notch for LineDelta (one "line" in OS terms).
                const PIXELS_PER_LINE: f32 = 40.0;

                let delta_px = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_x, y) => {
                        // y > 0 = scroll up (content moves down in viewport),
                        // y < 0 = scroll down (content moves up).
                        // Negate so positive scroll_top_px = further down doc.
                        -y * PIXELS_PER_LINE
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        // Precision touchpad: already in pixels, just negate.
                        -(pos.y as f32)
                    }
                };

                // Determine if cursor is over the sidebar area
                const SIDEBAR_WIDTH: f32 = 480.0; // matches SIDEBAR_DEFAULT_WIDTH
                let cursor_over_sidebar = self.cursor_position.x < SIDEBAR_WIDTH
                    && self.editor_adapter.is_some();

                if cursor_over_sidebar {
                    // Sidebar scroll — simple pixel offset, clamped to [0, max]
                    // Max is not known precisely, so use a large upper bound;
                    // overflow_hidden clips visually.
                    self.sidebar_scroll_px = (self.sidebar_scroll_px + delta_px).max(0.0);
                    if let Some(ref ss) = self.shared_sidebar_scroll {
                        ss.set(self.sidebar_scroll_px);
                    }
                } else if let Some(adapter) = &self.editor_adapter {
                    use crate::editor_adapter::EditorCommand;

                    let total_lines = adapter.borrow().total_lines().max(1);
                    let viewport_lines = adapter.borrow().viewport_lines().max(1);
                    let half_viewport = viewport_lines / 2;
                    let scrollable_lines = total_lines.saturating_sub(viewport_lines.saturating_sub(half_viewport));
                    let max_scroll_px = scrollable_lines as f32 * LINE_HEIGHT;

                    let new_top = (self.scroll_top_px + delta_px).clamp(0.0, max_scroll_px);
                    let new_first_line = (new_top / LINE_HEIGHT).floor() as usize;

                    let current_scroll_y = adapter.borrow().scroll_y();
                    let line_delta = new_first_line as i32 - current_scroll_y as i32;
                    if line_delta != 0 {
                        adapter.borrow_mut().dispatch_command(EditorCommand::Scroll(line_delta));
                    }

                    let actual_scroll_y = adapter.borrow().scroll_y();
                    if actual_scroll_y == new_first_line {
                        self.scroll_top_px = new_top;
                    } else {
                        self.scroll_top_px = actual_scroll_y as f32 * LINE_HEIGHT;
                    }

                    if let Some(ref offset) = self.shared_scroll_offset {
                        offset.set(self.scroll_top_px);
                    }
                }

                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
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
                            // DEMO HACK: Handle 'T' key for theme toggle when no adapter is present.
                            // When an adapter is present, 't' is a regular character input.
                            if (c == "t" || c == "T") && self.editor_adapter.is_none() {
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
                                // Try action system first
                                let context = KeyContext::new(); // TODO: Build context from focus stack
                                let action_matched = if let Some(action) = self.app_context.match_action(&keyboard_event.keystroke, &context) {
                                    let action_clone = action.boxed_clone();
                                    self.app_context.dispatch_action(&*action_clone);
                                    true
                                } else {
                                    false
                                };

                                if action_matched {
                                    if let Some(gpu_state) = &self.gpu_state {
                                        gpu_state.window.request_redraw();
                                    }
                                } else if let Some(adapter) = &self.editor_adapter {
                                    // No action matched: try editor command translation
                                    if let Some(cmd) = translate_editor_command(&keyboard_event, self.modifiers) {
                                        adapter.borrow_mut().dispatch_command(cmd);
                                        crate::elements::notify_caret_activity();
                                        self.next_blink_instant = Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE);
                                        // Sync pixel scroll offset: core_editor may have moved
                                        // viewport.scroll_y via ensure_caret_visible. Snap
                                        // scroll_top_px to the new line boundary, preserving
                                        // any fractional offset within the current line.
                                        let new_scroll_y = adapter.borrow().scroll_y();
                                        self.sync_scroll_from_core(new_scroll_y);
                                        log::debug!("Editor command dispatched via adapter");
                                        if let Some(gpu_state) = &self.gpu_state {
                                            gpu_state.window.request_redraw();
                                        }
                                    }
                                }
                            }
                        } else {
                            // Named keys (non-Tab): try action system first
                            let context = KeyContext::new(); // TODO: Build context from focus stack
                            let action_matched = if let Some(action) = self.app_context.match_action(&keyboard_event.keystroke, &context) {
                                let action_clone = action.boxed_clone();
                                self.app_context.dispatch_action(&*action_clone);
                                true
                            } else {
                                false
                            };

                            if action_matched {
                                if let Some(gpu_state) = &self.gpu_state {
                                    gpu_state.window.request_redraw();
                                }
                            } else if let Some(adapter) = &self.editor_adapter {
                                // No action matched: try editor command translation
                                if let Some(cmd) = translate_editor_command(&keyboard_event, self.modifiers) {
                                    adapter.borrow_mut().dispatch_command(cmd);
                                    crate::elements::notify_caret_activity();
                                    self.next_blink_instant = Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE);
                                    // Sync pixel scroll offset: core_editor may have moved
                                    // viewport.scroll_y via ensure_caret_visible. Snap
                                    // scroll_top_px to the new line boundary, preserving
                                    // any fractional offset within the current line.
                                    let new_scroll_y = adapter.borrow().scroll_y();
                                    self.sync_scroll_from_core(new_scroll_y);
                                    log::debug!("Editor command dispatched via adapter");
                                    if let Some(gpu_state) = &self.gpu_state {
                                        gpu_state.window.request_redraw();
                                    }
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
                                // No root view — no redraw needed until next event
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Tick async executor
        while self.app_context.tick_executor() {}

        // Check if transitions are still running (need continuous frames)
        let has_active_animations = self.app_context.has_active_transitions();

        if self.app_context.has_dirty_entities() || has_active_animations {
            // Need another frame immediately
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        } else {
            // Sleep until next caret blink or indefinitely
            match self.next_blink_instant {
                Some(instant) if instant > Instant::now() => {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(instant));
                }
                Some(_) => {
                    // Blink instant already passed — request redraw for blink toggle
                    // and schedule next blink
                    if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.window.request_redraw();
                    }
                    self.next_blink_instant = Some(Instant::now() + BLINK_RATE);
                    event_loop.set_control_flow(
                        ControlFlow::WaitUntil(self.next_blink_instant.unwrap())
                    );
                }
                None => {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
        }
    }
}
