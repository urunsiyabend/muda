use crate::app::App;
use crate::context::{AppContext, WindowContext};
use crate::editor_adapter::{EditorDataSource, PendingFileOp};
use crate::element::{LayoutContext, PaintContext, PrepaintContext};
use crate::elements::{BLINK_RATE, ACTIVITY_TIMEOUT};
use crate::entity::EntityStorage;
use crate::events::editor_input::translate_editor_command;
use crate::events::mouse::{hit_test, Hitbox, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use crate::events::types::{MouseButton, Modifiers, Point};
use crate::events::dispatch::{dispatch_mouse_down, dispatch_mouse_move, dispatch_mouse_scroll, dispatch_mouse_up, EventHandlers};
use crate::events::keyboard::{translate_key_event, Key, NamedKey};
use crate::events::actions::KeyContext;
use crate::platform::gpu::GpuState;
use crate::views::SharedAdapter;
use crate::window::OraWindow;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

/// Global flag to enable FPS overlay. Set via `ora::enable_fps_counter()`.
static SHOW_FPS: AtomicBool = AtomicBool::new(false);

/// Global flag to disable the glyph buffer LRU cache (for A/B comparison).
/// Set by parsing --no-glyph-cache from CLI args at startup.
static NO_GLYPH_CACHE: AtomicBool = AtomicBool::new(false);

/// Global flag to disable the layout dirty-flag cache (for A/B comparison).
/// Set by parsing --no-layout-cache from CLI args at startup.
static NO_LAYOUT_CACHE: AtomicBool = AtomicBool::new(false);

/// Enable the FPS counter (call before `run_with_editor`).
pub fn enable_fps_counter() {
    SHOW_FPS.store(true, Ordering::Relaxed);
}

/// Parse performance-related CLI flags from process args.
/// Called once at startup before the event loop runs.
pub fn parse_perf_flags() {
    for arg in std::env::args() {
        if arg == "--show-fps" {
            SHOW_FPS.store(true, Ordering::Relaxed);
        }
        if arg == "--no-glyph-cache" {
            NO_GLYPH_CACHE.store(true, Ordering::Relaxed);
        }
        if arg == "--no-layout-cache" {
            NO_LAYOUT_CACHE.store(true, Ordering::Relaxed);
        }
        if arg == "--no-cache" {
            // Umbrella flag: disables all caching subsystems for A/B comparison.
            NO_GLYPH_CACHE.store(true, Ordering::Relaxed);
            NO_LAYOUT_CACHE.store(true, Ordering::Relaxed);
            log::info!("--no-cache: all caches disabled (glyph + layout)");
        }
    }
}

/// Frame degradation guard.
///
/// Tracks consecutive slow frames (>4ms) and enters degraded mode when 3+
/// consecutive frames exceed budget. In degraded mode, transition animations
/// are suppressed to reduce per-frame cost and allow the renderer to recover.
/// Automatically exits degraded mode when a fast frame (<= 4ms) is observed.
struct FrameDegradation {
    /// Number of consecutive frames that exceeded the 4ms budget.
    consecutive_slow: u32,
    /// Whether the renderer is currently in degraded mode.
    pub degraded: bool,
}

impl FrameDegradation {
    fn new() -> Self {
        Self { consecutive_slow: 0, degraded: false }
    }

    /// Update degradation state based on the most recent frame time.
    ///
    /// - If `frame_ms > 4.0` and 3+ consecutive slow frames have occurred,
    ///   sets `degraded = true`.
    /// - If `frame_ms <= 4.0`, resets the counter and clears `degraded`.
    fn update(&mut self, frame_ms: f64) {
        if frame_ms > 4.0 {
            self.consecutive_slow += 1;
            if self.consecutive_slow >= 3 {
                self.degraded = true;
            }
        } else {
            self.consecutive_slow = 0;
            self.degraded = false;
        }
    }
}

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
    /// True while the user is dragging in the text area (mouse pressed + moved).
    /// Used to dispatch DragTo commands on CursorMoved events.
    text_area_drag: bool,
    /// Snap mode for drag selection: 0=char, 1=word-snap, 2=line-snap.
    /// Set from click_count when drag begins.
    drag_snap_mode: u32,
    /// The pixel position where the drag started (for 3px threshold).
    drag_start_pos: Point,
    /// Whether the 3px drag threshold has been met.
    drag_threshold_met: bool,
    /// Whether the editor window currently has OS-level focus.
    editor_has_focus: bool,
    /// Shared focus state, read by EditorRootView for selection dimming.
    shared_focus_state: Option<crate::app::SharedFocusState>,
    /// FPS tracking: frame count in current second.
    fps_frame_count: u32,
    /// FPS tracking: start of current measurement second.
    fps_last_report: Instant,
    /// FPS tracking: last computed FPS value.
    fps_display: u32,
    /// Layout dirty flag. When true, the full layout pipeline runs this frame
    /// (request_layout + compute). When false, cached_layout_outputs are reused.
    /// Defaults to true. Reset to false after each frame. Set to true for any
    /// event that changes element sizes or positions (resize, keyboard, tab switch).
    /// Set to false (or left false) for paint-only events (hover, blink, scroll).
    needs_layout: bool,
    /// Cached layout_outputs from the last full layout pass. Reused when
    /// needs_layout is false and layout_cache_enabled is true.
    cached_layout_outputs: Vec<crate::layout::LayoutOutput>,
    /// Number of frames where layout was skipped (cache hit).
    layout_hits: u64,
    /// Number of frames where full layout was computed (cache miss).
    layout_misses: u64,
    /// Whether layout caching is enabled. False when --no-layout-cache is passed.
    layout_cache_enabled: bool,
    /// Frame degradation guard. Tracks consecutive slow frames and suppresses
    /// transition animations when 3+ frames exceed the 4ms budget.
    frame_degradation: FrameDegradation,
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
            text_area_drag: false,
            drag_snap_mode: 0,
            drag_start_pos: Point::new(0.0, 0.0),
            drag_threshold_met: false,
            editor_has_focus: true,
            shared_focus_state: None,
            fps_frame_count: 0,
            fps_last_report: Instant::now(),
            fps_display: 0,
            needs_layout: true,
            cached_layout_outputs: Vec::new(),
            layout_hits: 0,
            layout_misses: 0,
            layout_cache_enabled: true,
            frame_degradation: FrameDegradation::new(),
        }
    }

    /// Create a new OraApp that includes an editor backend adapter.
    pub fn new_with_editor(
        app: App,
        adapter: SharedAdapter,
        scroll_offset: crate::app::SharedScrollOffset,
        focus_state: crate::app::SharedFocusState,
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
            text_area_drag: false,
            drag_snap_mode: 0,
            drag_start_pos: Point::new(0.0, 0.0),
            drag_threshold_met: false,
            editor_has_focus: true,
            shared_focus_state: Some(focus_state),
            fps_frame_count: 0,
            fps_last_report: Instant::now(),
            fps_display: 0,
            needs_layout: true,
            cached_layout_outputs: Vec::new(),
            layout_hits: 0,
            layout_misses: 0,
            layout_cache_enabled: true,
            frame_degradation: FrameDegradation::new(),
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

    /// Convert a pixel position to document (line, col) coordinates.
    ///
    /// Returns `Some((line, col))` if the click is inside the text area,
    /// `None` if outside (e.g., clicking on sidebar, gutter, tab bar, status bar).
    fn pixel_to_doc(&self, px: Point) -> Option<(usize, usize)> {
        let adapter = self.editor_adapter.as_ref()?;
        let adapter_ref = adapter.borrow();

        const LINE_HEIGHT: f32 = 21.0;
        const TAB_BAR_H: f32 = 36.0;
        const STATUS_BAR_H: f32 = 28.0;

        let char_w = crate::rendering::measured_char_width();
        let sidebar_px = adapter_ref.sidebar_width_px();
        let gutter_chars = adapter_ref.gutter_width_chars();
        let gutter_px = gutter_chars as f32 * char_w;
        let text_area_x = sidebar_px + gutter_px;
        let text_area_y = TAB_BAR_H;

        // Check bounds: click must be inside text area region.
        if px.x < text_area_x || px.y < text_area_y {
            return None;
        }

        // Get window height from GPU state for status bar exclusion.
        let window_h = self.gpu_state.as_ref()
            .map(|g| g.size.1 as f32)
            .unwrap_or(600.0);
        if px.y > window_h - STATUS_BAR_H {
            return None;
        }

        let scroll_y = adapter_ref.scroll_y();
        let scroll_x = adapter_ref.scroll_x();
        let scroll_y_offset_px = self.scroll_top_px - (scroll_y as f32 * LINE_HEIGHT);

        let local_y = (px.y - text_area_y) + scroll_y_offset_px;
        let local_x = px.x - text_area_x;

        let line_in_viewport = (local_y / LINE_HEIGHT).floor().max(0.0) as usize;
        let line_idx = scroll_y + line_in_viewport;

        // Mid-character snap: +0.5 means past midpoint snaps right.
        let col_raw = ((local_x / char_w) + 0.5).floor().max(0.0) as usize;
        let col_idx = scroll_x + col_raw;

        Some((line_idx, col_idx))
    }

    /// Convert a pixel position to a gutter line number.
    ///
    /// Returns `Some(line)` if the click is inside the gutter region,
    /// `None` if outside.
    fn pixel_to_gutter_line(&self, px: Point) -> Option<usize> {
        let adapter = self.editor_adapter.as_ref()?;
        let adapter_ref = adapter.borrow();

        const LINE_HEIGHT: f32 = 21.0;
        const TAB_BAR_H: f32 = 36.0;
        const STATUS_BAR_H: f32 = 28.0;

        let char_w = crate::rendering::measured_char_width();
        let sidebar_px = adapter_ref.sidebar_width_px();
        let gutter_chars = adapter_ref.gutter_width_chars();
        let gutter_px = gutter_chars as f32 * char_w;

        // Gutter occupies x from sidebar_px to sidebar_px + gutter_px
        if px.x < sidebar_px || px.x >= sidebar_px + gutter_px {
            return None;
        }
        if px.y < TAB_BAR_H {
            return None;
        }
        let window_h = self.gpu_state.as_ref()
            .map(|g| g.size.1 as f32)
            .unwrap_or(600.0);
        if px.y > window_h - STATUS_BAR_H {
            return None;
        }

        let scroll_y = adapter_ref.scroll_y();
        let scroll_y_offset_px = self.scroll_top_px - (scroll_y as f32 * LINE_HEIGHT);
        let local_y = (px.y - TAB_BAR_H) + scroll_y_offset_px;
        let line_in_viewport = (local_y / LINE_HEIGHT).floor().max(0.0) as usize;

        Some(scroll_y + line_in_viewport)
    }

    /// Polls the adapter for a pending file operation and dispatches it.
    ///
    /// Called after every `dispatch_command` invocation. If the adapter has
    /// queued a file op (e.g. from Ctrl+O), this method spawns the appropriate
    /// async dialog future on the local executor.
    fn poll_pending_file_ops(&mut self) {
        let adapter = match &self.editor_adapter {
            Some(a) => a,
            None => return,
        };
        let op = adapter.borrow_mut().take_pending_file_op();
        let op = match op {
            Some(op) => op,
            None => return,
        };
        match op {
            PendingFileOp::Open => self.spawn_open_dialog(),
            PendingFileOp::SaveAs => self.spawn_save_as_dialog(),
            PendingFileOp::OpenFolder => self.spawn_open_folder_dialog(),
            PendingFileOp::Save => {
                // If the active document has a path, save silently.
                // Otherwise fall through to the Save As dialog.
                let has_path = self.editor_adapter
                    .as_ref()
                    .map(|a| a.borrow().active_doc_has_path())
                    .unwrap_or(false);
                if has_path {
                    if let Some(adapter) = &self.editor_adapter {
                        let _ = adapter.borrow_mut().save_active_doc();
                    }
                    if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.window.request_redraw();
                    }
                } else {
                    self.spawn_save_as_dialog();
                }
            }
        }
    }

    /// Spawns an async future that opens the native save dialog.
    ///
    /// The future runs on the `LocalExecutor`. It opens `rfd::AsyncFileDialog::save_file`,
    /// and on confirmation calls `handle_file_saved` with the chosen path so the
    /// adapter writes the file to disk, updates the document's file path, clears
    /// the dirty flag, and registers the path in the buffer registry.
    fn spawn_save_as_dialog(&mut self) {
        let adapter = Rc::clone(self.editor_adapter.as_ref().unwrap());
        adapter.borrow_mut().set_dialog_open(true);
        let window = self.gpu_state.as_ref().unwrap().window.clone();
        let last_dir = adapter.borrow().last_opened_directory();

        self.app_context.spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .set_directory(&last_dir)
                .set_file_name("")
                .save_file()
                .await;

            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                adapter.borrow_mut().handle_file_saved(path);
            }
            adapter.borrow_mut().set_dialog_open(false);
            window.request_redraw();
        })
        .detach();
    }

    /// Spawns an async future that opens the native file picker and loads files.
    ///
    /// The future runs on the `LocalExecutor` (single-threaded, !Send safe).
    /// It opens `rfd::AsyncFileDialog`, reads each selected file on a background
    /// thread, validates UTF-8, strips BOM, and delivers results to the adapter
    /// via `handle_file_loaded` / `handle_file_error`.
    fn spawn_open_dialog(&mut self) {
        let adapter = Rc::clone(self.editor_adapter.as_ref().unwrap());
        adapter.borrow_mut().set_dialog_open(true);
        let window = self.gpu_state.as_ref().unwrap().window.clone();
        let last_dir = adapter.borrow().last_opened_directory();

        self.app_context.spawn(async move {
            let handles = rfd::AsyncFileDialog::new()
                .set_directory(&last_dir)
                .pick_files()
                .await;

            if let Some(handles) = handles {
                for handle in handles {
                    let path = handle.path().to_path_buf();
                    let (tx, rx) = std::sync::mpsc::channel::<(std::path::PathBuf, std::io::Result<Vec<u8>>)>();
                    let path_clone = path.clone();
                    std::thread::spawn(move || {
                        let result = std::fs::read(&path_clone);
                        let _ = tx.send((path_clone, result));
                    });
                    loop {
                        if let Ok((recv_path, result)) = rx.try_recv() {
                            match result {
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(mut text) => {
                                        // Strip UTF-8 BOM if present.
                                        if text.starts_with('\u{FEFF}') {
                                            text = text['\u{FEFF}'.len_utf8()..].to_string();
                                        }
                                        adapter.borrow_mut().handle_file_loaded(recv_path, text);
                                    }
                                    Err(_) => {
                                        adapter.borrow_mut().handle_file_error(
                                            "Cannot open: file is not valid UTF-8".to_string(),
                                        );
                                    }
                                },
                                Err(e) => {
                                    adapter.borrow_mut().handle_file_error(
                                        format!("Cannot open file: {}", e),
                                    );
                                }
                            }
                            window.request_redraw();
                            break;
                        }
                        futures_lite::future::yield_now().await;
                    }
                }
            }

            adapter.borrow_mut().set_dialog_open(false);
            window.request_redraw();
        })
        // Detach the task — we don't need to await its completion.
        .detach();
    }

    /// Spawns an async future that opens the native folder picker dialog.
    ///
    /// The future runs on the `LocalExecutor`. It opens `rfd::AsyncFileDialog::pick_folder`,
    /// and on confirmation calls `handle_folder_opened` with the chosen path so the
    /// adapter updates the sidebar's base directory, persists the workspace path, and
    /// requests a re-render.
    fn spawn_open_folder_dialog(&mut self) {
        let adapter = Rc::clone(self.editor_adapter.as_ref().unwrap());
        adapter.borrow_mut().set_dialog_open(true);
        let window = self.gpu_state.as_ref().unwrap().window.clone();

        self.app_context.spawn(async move {
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Open Folder")
                .pick_folder()
                .await;

            if let Some(handle) = handle {
                let path = handle.path().to_path_buf();
                adapter.borrow_mut().handle_folder_opened(path);
            }
            adapter.borrow_mut().set_dialog_open(false);
            window.request_redraw();
        })
        .detach();
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

        // Parse CLI performance flags (--show-fps, --no-glyph-cache).
        parse_perf_flags();

        // Initialize GPU state
        let mut gpu_state = pollster::block_on(GpuState::new(window.clone()));

        // Apply --no-glyph-cache flag if set
        if NO_GLYPH_CACHE.load(Ordering::Relaxed) {
            gpu_state.text_system.set_cache_enabled(false);
            log::info!("Glyph buffer cache disabled via --no-glyph-cache");
        }

        // Apply --no-layout-cache flag if set
        if NO_LAYOUT_CACHE.load(Ordering::Relaxed) {
            self.layout_cache_enabled = false;
            log::info!("Layout dirty-flag cache disabled via --no-layout-cache");
        }

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
                // Window size changed — element sizes will change, full layout required.
                self.needs_layout = true;
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
                if let Some(adapter) = &self.editor_adapter {
                    if adapter.borrow().is_dialog_open() {
                        return;
                    }
                }
                let point = Point::new(position.x as f32, position.y as f32);
                self.cursor_position = point;

                // Update cursor icon based on hover region
                if let Some(gpu_state) = &self.gpu_state {
                    let icon = if self.pixel_to_doc(point).is_some() {
                        winit::window::CursorIcon::Text
                    } else {
                        winit::window::CursorIcon::Default
                    };
                    gpu_state.window.set_cursor(winit::window::Cursor::Icon(icon));
                }

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

                // Text area drag: if dragging, dispatch DragTo to extend selection.
                if self.text_area_drag {
                    // 3px drag threshold: don't start selecting until mouse moves enough.
                    if !self.drag_threshold_met {
                        let dx = point.x - self.drag_start_pos.x;
                        let dy = point.y - self.drag_start_pos.y;
                        let distance = (dx * dx + dy * dy).sqrt();
                        if distance < 3.0 {
                            // Threshold not met yet -- skip DragTo.
                        } else {
                            self.drag_threshold_met = true;
                        }
                    }

                    if self.drag_threshold_met {
                        // Scroll-while-drag: when cursor is near viewport edges,
                        // scroll the document and then extend selection.
                        const EDGE_PX: f32 = 20.0;
                        const TAB_BAR_H: f32 = 36.0;
                        const STATUS_BAR_H: f32 = 28.0;
                        let window_h = self.gpu_state.as_ref()
                            .map(|g| g.size.1 as f32)
                            .unwrap_or(600.0);
                        let text_top = TAB_BAR_H;
                        let text_bottom = window_h - STATUS_BAR_H;

                        if point.y < text_top + EDGE_PX && point.y >= text_top {
                            // Near top edge: scroll up
                            if let Some(adapter) = &self.editor_adapter {
                                use crate::editor_adapter::EditorCommand as Cmd;
                                adapter.borrow_mut().dispatch_command(Cmd::Scroll(-1));
                                let new_scroll_y = adapter.borrow().scroll_y();
                                self.sync_scroll_from_core(new_scroll_y);
                            }
                        } else if point.y > text_bottom - EDGE_PX && point.y <= text_bottom {
                            // Near bottom edge: scroll down
                            if let Some(adapter) = &self.editor_adapter {
                                use crate::editor_adapter::EditorCommand as Cmd;
                                adapter.borrow_mut().dispatch_command(Cmd::Scroll(1));
                                let new_scroll_y = adapter.borrow().scroll_y();
                                self.sync_scroll_from_core(new_scroll_y);
                            }
                        }

                        if let Some((line, col)) = self.pixel_to_doc(point) {
                            if let Some(adapter) = &self.editor_adapter {
                                use crate::editor_adapter::EditorCommand;
                                adapter.borrow_mut().dispatch_command(
                                    EditorCommand::DragTo { line, col, snap_mode: self.drag_snap_mode },
                                );
                                crate::elements::notify_caret_activity();
                                self.next_blink_instant = Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE);
                                let new_scroll_y = adapter.borrow().scroll_y();
                                self.sync_scroll_from_core(new_scroll_y);
                            }
                        }
                    }
                }

                // Request redraw to update hover state visuals.
                // Hover changes are paint-only: element bounds don't change.
                // needs_layout stays false unless drag dispatched a layout-dirty command.
                if self.text_area_drag && self.drag_threshold_met {
                    // DragTo dispatched above changes selection, which is paint-only too.
                    // Layout is not needed for selection rendering (overlay rects).
                    // needs_layout remains unchanged (false unless already set true).
                }
                // Do NOT set needs_layout = true here; hover/drag redraws are paint-only.
                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(adapter) = &self.editor_adapter {
                    if adapter.borrow().is_dialog_open() {
                        return;
                    }
                }
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

                            // Text area click-to-cursor: if left click lands in
                            // the text area, dispatch ClickAt to the adapter.
                            if mouse_button == MouseButton::Left {
                                if let Some((line, col)) = self.pixel_to_doc(self.cursor_position) {
                                    if let Some(adapter) = &self.editor_adapter {
                                        use crate::editor_adapter::EditorCommand;
                                        adapter.borrow_mut().dispatch_command(
                                            EditorCommand::ClickAt {
                                                line,
                                                col,
                                                extend_selection: self.modifiers.shift,
                                                click_count: self.click_count,
                                            },
                                        );
                                        crate::elements::notify_caret_activity();
                                        self.next_blink_instant = Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE);
                                        let new_scroll_y = adapter.borrow().scroll_y();
                                        self.sync_scroll_from_core(new_scroll_y);
                                        self.text_area_drag = true;
                                        self.drag_snap_mode = match self.click_count {
                                            2 => 1, // word-snap
                                            3 => 2, // line-snap
                                            _ => 0, // char
                                        };
                                        self.drag_start_pos = self.cursor_position;
                                        self.drag_threshold_met = false;
                                    }
                                } else if let Some(gutter_line) = self.pixel_to_gutter_line(self.cursor_position) {
                                    // Gutter click: select entire line
                                    if let Some(adapter) = &self.editor_adapter {
                                        use crate::editor_adapter::EditorCommand;
                                        adapter.borrow_mut().dispatch_command(
                                            EditorCommand::GutterClickAt { line: gutter_line },
                                        );
                                        crate::elements::notify_caret_activity();
                                        self.next_blink_instant = Some(Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE);
                                        if let Some(gpu_state) = &self.gpu_state {
                                            gpu_state.window.request_redraw();
                                        }
                                    }
                                    self.text_area_drag = false;
                                } else {
                                    self.text_area_drag = false;
                                }
                            }
                        }
                        winit::event::ElementState::Released => {
                            // Clear active state and release capture if present
                            self.app_context.interaction_state.clear_active();
                            self.app_context.interaction_state.release_mouse_capture();
                            self.text_area_drag = false;
                            self.drag_snap_mode = 0;
                            self.drag_threshold_met = false;
                            log::info!("Active state: Mouse button {:?} released", mouse_button);

                            let event = MouseUpEvent {
                                position: self.cursor_position,
                                button: mouse_button,
                                modifiers: self.modifiers,
                            };
                            dispatch_mouse_up(&mut self.event_handlers, &event, hit_id);
                        }
                    }

                    // Mouse clicks can trigger tab switches, UI state changes, or command
                    // dispatch — mark layout dirty to be safe.
                    self.needs_layout = true;
                    // Request redraw to update active state visuals
                    if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(adapter) = &self.editor_adapter {
                    if adapter.borrow().is_dialog_open() {
                        return;
                    }
                }
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

                // Dispatch scroll to hitbox handlers first (ScrollArea etc.)
                let scroll_event = crate::events::mouse::MouseScrollEvent {
                    delta: Point::new(0.0, delta_px),
                    modifiers: self.modifiers,
                };
                let hit = hit_test(&self.hitboxes, self.cursor_position);
                let consumed = if let Some(hit_id) = hit {
                    let path = crate::events::dispatch::build_dispatch_path(&self.event_handlers.parent_map, hit_id);
                    log::info!("Scroll: hit={}, path={:?}, scroll_handler_count={}", hit_id.0, path.iter().map(|h| h.0).collect::<Vec<_>>(), self.event_handlers.has_scroll_handlers());
                    dispatch_mouse_scroll(&mut self.event_handlers, &scroll_event, hit_id)
                } else {
                    false
                };

                // If no scroll handler consumed it, fall back to editor scroll
                if !consumed {
                if let Some(adapter) = &self.editor_adapter {
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
                } // end if !consumed

                // Both sidebar scroll and editor scroll change visible content,
                // which changes element count and LayoutId assignment order.
                // Stale cached layout outputs produce wrong positions → flicker.
                // Layout cache still helps for non-scroll frames (hover, blink).
                self.needs_layout = true;
                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let Some(adapter) = &self.editor_adapter {
                    if adapter.borrow().is_dialog_open() {
                        return;
                    }
                }
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

                            // Tab focus change can alter visible focus indicators — mark layout dirty.
                            self.needs_layout = true;
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

                                // Theme change alters all element colors — full layout required.
                                self.needs_layout = true;
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
                                    // Actions (tab switch, file ops, command palette) change layout.
                                    self.needs_layout = true;
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
                                        // Keyboard input modifies buffer content — layout required.
                                        self.needs_layout = true;
                                        if let Some(gpu_state) = &self.gpu_state {
                                            gpu_state.window.request_redraw();
                                        }
                                        self.poll_pending_file_ops();
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
                                // Actions (tab switch, file ops, command palette) change layout.
                                self.needs_layout = true;
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
                                    // Keyboard input modifies buffer content — layout required.
                                    self.needs_layout = true;
                                    if let Some(gpu_state) = &self.gpu_state {
                                        gpu_state.window.request_redraw();
                                    }
                                    self.poll_pending_file_ops();
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
                self.editor_has_focus = focused;
                if let Some(ref state) = self.shared_focus_state {
                    state.set(focused);
                }
                // Focus change affects selection dimming style — mark layout dirty.
                self.needs_layout = true;
                // Redraw to update selection dimming
                if let Some(gpu_state) = &self.gpu_state {
                    gpu_state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    // Determine whether to run full layout or reuse cached layout_outputs.
                    // Conservative rule: when in doubt, mark needs_layout = true.
                    // A false cache hit produces visual bugs; a missed cache is just a normal frame.
                    let run_full_layout = self.needs_layout
                        || !self.layout_cache_enabled
                        || self.cached_layout_outputs.is_empty();

                    log::trace!(
                        "Redraw: dirty={}, needs_layout={}, cache_enabled={}, run_full={}",
                        self.app_context.has_dirty_entities(),
                        self.needs_layout,
                        self.layout_cache_enabled,
                        run_full_layout
                    );

                    // Reset needs_layout for next frame — any event that requires layout
                    // will set it to true again before the next RedrawRequested.
                    self.needs_layout = false;

                    let frame_start = Instant::now();

                    // Render the root view to get element tree
                    if let Some(mut element_tree) = self.ora_window.render(&mut self.app_context) {
                        let t_view_tree = frame_start.elapsed();
                        let window_size = gpu_state.size;

                        // Phase 1: Layout (full or cached)
                        let layout_outputs: Vec<crate::layout::LayoutOutput>;
                        let t_layout;

                        // Always create LayoutContext and run request_layout so the
                        // fresh element tree gets LayoutId assignments. Without this,
                        // elements can't look up bounds during prepaint/paint.
                        let mut layout_cx = LayoutContext::new(
                            &mut self.app_context.entity_storage,
                            window_size,
                        );

                        // Pass TextSystem for text measurement during layout
                        layout_cx.set_text_system(&mut gpu_state.text_system as *mut _);

                        element_tree.request_layout(&mut layout_cx);
                        let new_layout_count = layout_cx.next_layout_id();

                        // If the element tree grew beyond the cached outputs,
                        // force full layout — otherwise out-of-range LayoutIds
                        // return Rect::zero and elements become invisible.
                        let run_full_layout = run_full_layout
                            || new_layout_count > self.cached_layout_outputs.len();

                        if run_full_layout {
                            // Full pipeline: compute_flexbox on top of request_layout
                            layout_cx.compute();
                            layout_outputs = layout_cx.layout_outputs.clone();

                            // Cache for next paint-only frames
                            self.cached_layout_outputs = layout_outputs.clone();
                            self.layout_misses += 1;
                        } else {
                            // Paint-only frame: skip the expensive compute_flexbox.
                            // request_layout already ran (LayoutIds assigned, intrinsic
                            // sizes measured). Reuse cached flexbox results.
                            layout_outputs = self.cached_layout_outputs.clone();
                            self.layout_hits += 1;
                        }
                        t_layout = frame_start.elapsed();

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
                        let t_prepaint = frame_start.elapsed();

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
                        let t_paint = frame_start.elapsed();

                        // Extract paint commands and render
                        let commands = paint_cx.take_commands();
                        let cmd_count = commands.len();
                        match gpu_state.render_frame(&commands) {
                            Ok(_) => {
                                // Return shaped buffers to the glyph LRU cache for next frame reuse.
                                // drain_buffers_for_cache() drains pending_buffers/layers and
                                // calls atlas.trim(). return_buffers_to_cache() reinserts keyed buffers.
                                let returned = gpu_state.text_system.drain_buffers_for_cache();
                                gpu_state.text_system.return_buffers_to_cache(returned);

                                // Clear dirty entities after rendering
                                self.app_context.clear_dirty();

                                // Frame timing display
                                let t_gpu = frame_start.elapsed();
                                let total_ms = t_gpu.as_secs_f64() * 1000.0;

                                // Update frame degradation guard with this frame's total time.
                                self.frame_degradation.update(total_ms);

                                if SHOW_FPS.load(Ordering::Relaxed) {
                                    let view_ms = t_view_tree.as_secs_f64() * 1000.0;
                                    let layout_ms = (t_layout - t_view_tree).as_secs_f64() * 1000.0;
                                    let prepaint_ms = (t_prepaint - t_layout).as_secs_f64() * 1000.0;
                                    let paint_ms = (t_paint - t_prepaint).as_secs_f64() * 1000.0;
                                    let gpu_ms = (t_gpu - t_paint).as_secs_f64() * 1000.0;
                                    let glyph_hit_pct = (gpu_state.text_system.cache_hit_rate() * 100.0) as u32;
                                    let total_layout_frames = self.layout_hits + self.layout_misses;
                                    let layout_hit_pct = if total_layout_frames > 0 {
                                        ((self.layout_hits as f64 / total_layout_frames as f64) * 100.0) as u32
                                    } else {
                                        0
                                    };

                                    self.fps_frame_count += 1;
                                    let now = Instant::now();
                                    let elapsed = now.duration_since(self.fps_last_report);
                                    if elapsed.as_secs_f32() >= 1.0 {
                                        self.fps_display = self.fps_frame_count;
                                        self.fps_frame_count = 0;
                                        self.fps_last_report = now;
                                    }

                                    let degraded_tag = if self.frame_degradation.degraded { " [DEGRADED]" } else { "" };

                                    log::info!(
                                        "FRAME {:.1}ms | view {:.1} layout {:.1} prepaint {:.1} paint {:.1} gpu {:.1} | glyph {}% layout {}% | {} cmds{}",
                                        total_ms, view_ms, layout_ms, prepaint_ms, paint_ms, gpu_ms, glyph_hit_pct, layout_hit_pct, cmd_count, degraded_tag
                                    );

                                    gpu_state.window.set_title(
                                        &format!(
                                            "Muda [{:.0}ms | view {:.0} layout {:.0} prepaint {:.0} paint {:.0} gpu {:.0} | glyph {}% layout {}% | {} cmds | {}fps{}]",
                                            total_ms, view_ms, layout_ms, prepaint_ms, paint_ms, gpu_ms, glyph_hit_pct, layout_hit_pct, cmd_count, self.fps_display, degraded_tag
                                        )
                                    );
                                }
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

        // Check for dirty entities and request redraw if needed.
        // Dirty entities indicate data model changes (file load, tab switch, etc.)
        // that require full layout recomputation — not just a paint.
        if self.app_context.has_dirty_entities() {
            self.needs_layout = true;
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Tick async executor
        while self.app_context.tick_executor() {}

        // Poll filesystem watcher for sidebar refresh (non-blocking, zero overhead when idle).
        let watcher_had_events = if let Some(adapter) = &self.editor_adapter {
            adapter.borrow_mut().poll_watcher_events()
        } else {
            false
        };
        if watcher_had_events {
            // Process external changes to open buffers (auto-reload, deleted indicator, prompt).
            if let Some(adapter) = &self.editor_adapter {
                adapter.borrow_mut().handle_external_file_changes();
            }
            self.needs_layout = true;
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        }

        // Check if transitions are still running (need continuous frames).
        // When degraded (3+ consecutive slow frames), suppress transition animation
        // redraws to reduce per-frame cost and allow the renderer to recover.
        // Dirty entity redraws are never suppressed — only animation-only frames.
        let has_active_animations = self.app_context.has_active_transitions();
        let animations_suppressed = self.frame_degradation.degraded && has_active_animations && !self.app_context.has_dirty_entities();

        if self.app_context.has_dirty_entities() || (has_active_animations && !animations_suppressed) {
            // Need another frame immediately.
            // Dirty entities require full layout; active animations are paint-only.
            if self.app_context.has_dirty_entities() {
                self.needs_layout = true;
            }
            // Transitions (hover, color animation) are paint-only — needs_layout stays false.
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
        } else {
            // Sleep until next caret blink or status message expiry, whichever is sooner.
            let mut earliest_wake: Option<Instant> = self.next_blink_instant;
            if let Some(adapter) = &self.editor_adapter {
                if let Some(expiry) = adapter.borrow().status_message_expiry() {
                    earliest_wake = Some(match earliest_wake {
                        Some(existing) => existing.min(expiry),
                        None => expiry,
                    });
                }
            }
            match earliest_wake {
                Some(instant) if instant > Instant::now() => {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(instant));
                }
                Some(_) => {
                    // Wake instant already passed — request redraw and reschedule
                    if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.window.request_redraw();
                    }
                    if self.next_blink_instant.map_or(false, |i| i <= Instant::now()) {
                        self.next_blink_instant = Some(Instant::now() + BLINK_RATE);
                    }
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        earliest_wake.unwrap_or_else(|| Instant::now() + BLINK_RATE)
                    ));
                }
                None => {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
        }
    }
}
