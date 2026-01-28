//! GPU renderer orchestrating all components.

#[cfg(debug_assertions)]
mod debug;

use crate::components::{Bounds, Caret, Dialog, Gutter, SidebarComponent, StatusBar, TabBar, TextArea, UITextRenderer, safe_scissor_rect};
use crate::theme::{Theme, ColorRole};
use crate::ui::{AppLayout, LayoutRegion, CommandPalette, CommandEntry, CommandKind, EditorTabs, TabInfo, PanelManager, PanelKind, StatusLine, FileTree, FileEntry};
use crate::design_system::StyledRectRenderer;
use crate::design_system::primitives::TextBlock;
use core_editor::view_model::RenderModel;

/// Main GPU renderer that coordinates all UI components.
pub struct GpuRenderer {
    // Components
    text_area: TextArea,
    gutter: Gutter,
    caret: Caret,
    status_bar: StatusBar,
    sidebar: SidebarComponent,
    tab_bar: TabBar,
    dialog: Dialog,

    // New UI components
    app_layout: AppLayout,
    command_palette: CommandPalette,
    editor_tabs: EditorTabs,
    panel_manager: PanelManager,
    status_line: StatusLine,
    file_tree: FileTree,

    // New UI renderers (design system)
    styled_rect_renderer: StyledRectRenderer,
    ui_text_renderer: UITextRenderer,

    // State
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    theme: Theme,
    /// Cached measured character width for accurate positioning.
    measured_char_width: f32,

    /// Debug overlay state (only in debug builds).
    #[cfg(debug_assertions)]
    debug_overlay: debug::DebugOverlay,
}

impl GpuRenderer {
    /// Creates a new GPU renderer for the given window.
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("muda_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Initialize theme
        let theme = Theme::dark();

        // Create components
        let mut text_area = TextArea::new(&device, &queue, surface_format);
        let gutter = Gutter::new(&device, &queue, surface_format);
        let caret = Caret::new(&device, surface_format);
        let status_bar = StatusBar::new(&device, &queue, surface_format);
        let sidebar = SidebarComponent::new(&device, &queue, surface_format);
        let tab_bar = TabBar::new(&device, &queue, surface_format);
        let dialog = Dialog::new(&device, &queue, surface_format);

        // New UI components
        let app_layout = AppLayout::new();
        let mut command_palette = CommandPalette::new();
        let editor_tabs = EditorTabs::new();
        let panel_manager = PanelManager::new();
        let status_line = StatusLine::new();
        let file_tree = FileTree::new();

        // New UI renderers (design system)
        let styled_rect_renderer = StyledRectRenderer::new(&device, surface_format);
        let ui_text_renderer = UITextRenderer::new(&device, &queue, surface_format);

        // Register default commands
        command_palette.register_commands(vec![
            CommandEntry::command("file.new", "New File").with_shortcut("Ctrl+N"),
            CommandEntry::command("file.open", "Open File").with_shortcut("Ctrl+O"),
            CommandEntry::command("file.save", "Save").with_shortcut("Ctrl+S"),
            CommandEntry::command("file.save_as", "Save As").with_shortcut("Ctrl+Shift+S"),
            CommandEntry::command("edit.undo", "Undo").with_shortcut("Ctrl+Z"),
            CommandEntry::command("edit.redo", "Redo").with_shortcut("Ctrl+Y"),
            CommandEntry::command("edit.cut", "Cut").with_shortcut("Ctrl+X"),
            CommandEntry::command("edit.copy", "Copy").with_shortcut("Ctrl+C"),
            CommandEntry::command("edit.paste", "Paste").with_shortcut("Ctrl+V"),
            CommandEntry::command("edit.find", "Find").with_shortcut("Ctrl+F"),
            CommandEntry::command("edit.replace", "Find and Replace").with_shortcut("Ctrl+H"),
            CommandEntry::command("view.sidebar", "Toggle Sidebar").with_shortcut("Ctrl+B"),
            CommandEntry::command("view.panel", "Toggle Panel").with_shortcut("Ctrl+J"),
            CommandEntry::command("view.command_palette", "Command Palette").with_shortcut("Ctrl+Shift+P"),
            CommandEntry::command("go.line", "Go to Line").with_shortcut("Ctrl+G"),
            CommandEntry::command("go.file", "Go to File").with_shortcut("Ctrl+P"),
        ]);

        // Measure actual character width from the font
        let measured_char_width = text_area.measure_char_width(theme.font_size);

        Ok(Self {
            text_area,
            gutter,
            caret,
            status_bar,
            sidebar,
            tab_bar,
            dialog,
            app_layout,
            command_palette,
            editor_tabs,
            panel_manager,
            status_line,
            file_tree,
            styled_rect_renderer,
            ui_text_renderer,
            surface,
            device,
            queue,
            config,
            theme,
            measured_char_width,
            #[cfg(debug_assertions)]
            debug_overlay: debug::DebugOverlay::new(),
        })
    }

    /// Returns a reference to the theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Returns the measured character width in pixels.
    pub fn char_width(&self) -> f32 {
        self.measured_char_width
    }

    /// Returns the tab bar height in logical pixels.
    pub fn tab_bar_height(&self) -> f32 {
        self.editor_tabs.height()
    }

    /// Sets the theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Handles window resize.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Notifies the renderer of user activity (for caret blinking).
    pub fn on_activity(&mut self) {
        self.caret.on_activity();
    }

    /// Updates animation state. Returns true if a redraw is needed.
    pub fn update(&mut self) -> bool {
        let caret_update = self.caret.update();
        let palette_update = self.command_palette.update();
        caret_update || palette_update
    }

    /// Toggle the command palette.
    pub fn toggle_command_palette(&mut self) {
        self.command_palette.toggle();
    }

    /// Show the command palette.
    pub fn show_command_palette(&mut self) {
        self.command_palette.show();
    }

    /// Hide the command palette.
    pub fn hide_command_palette(&mut self) {
        self.command_palette.hide();
    }

    /// Check if command palette is visible.
    pub fn is_command_palette_visible(&self) -> bool {
        self.command_palette.is_visible()
    }

    /// Type a character into the command palette.
    pub fn command_palette_type(&mut self, c: char) {
        self.command_palette.type_char(c);
    }

    /// Backspace in the command palette.
    pub fn command_palette_backspace(&mut self) {
        self.command_palette.backspace();
    }

    /// Move selection up in command palette.
    pub fn command_palette_up(&mut self) {
        self.command_palette.select_previous();
    }

    /// Move selection down in command palette.
    pub fn command_palette_down(&mut self) {
        self.command_palette.select_next();
    }

    /// Execute selected command in palette.
    pub fn command_palette_execute(&mut self) -> Option<String> {
        self.command_palette.execute().map(|e| e.id)
    }

    /// Handle mouse movement over editor tabs. Returns true if redraw needed.
    pub fn editor_tabs_on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        self.editor_tabs.on_pointer_move(x, y)
    }

    /// Handle click on editor tabs. Returns (view_id, is_close_click) if clicked.
    pub fn editor_tabs_on_click(&mut self, x: f32, y: f32) -> Option<(u64, bool)> {
        self.editor_tabs.on_click(x, y)
    }

    /// Handle middle click on editor tabs. Returns view_id if clicked.
    pub fn editor_tabs_on_middle_click(&mut self, x: f32, y: f32) -> Option<u64> {
        self.editor_tabs.on_middle_click(x, y)
    }

    /// Toggle the bottom panel visibility.
    pub fn toggle_panel(&mut self) {
        self.panel_manager.toggle();
    }

    /// Show the bottom panel.
    pub fn show_panel(&mut self) {
        self.panel_manager.show();
    }

    /// Hide the bottom panel.
    pub fn hide_panel(&mut self) {
        self.panel_manager.hide();
    }

    /// Check if the bottom panel is visible.
    pub fn is_panel_visible(&self) -> bool {
        self.panel_manager.is_visible()
    }

    /// Get the current panel height (0 if hidden).
    pub fn panel_height(&self) -> f32 {
        self.panel_manager.height()
    }

    /// Handle mouse movement over panel. Returns true if redraw needed.
    pub fn panel_on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        self.panel_manager.on_pointer_move(x, y)
    }

    /// Handle click on panel. Returns clicked panel kind if any.
    pub fn panel_on_click(&mut self, x: f32, y: f32) -> Option<PanelKind> {
        self.panel_manager.on_click(x, y)
    }

    /// Check if pointer is on panel resize handle.
    pub fn is_on_panel_resize_handle(&self, x: f32, y: f32) -> bool {
        self.panel_manager.is_on_resize_handle(x, y)
    }

    /// Start panel resize.
    pub fn start_panel_resize(&mut self, y: f32) {
        self.panel_manager.start_resize(y);
    }

    /// Stop panel resize.
    pub fn stop_panel_resize(&mut self) {
        self.panel_manager.stop_resize();
    }

    /// Handle mouse movement over file tree. Returns true if redraw needed.
    pub fn file_tree_on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        self.file_tree.on_pointer_move(x, y)
    }

    /// Handle click on file tree. Returns clicked entry info if any.
    /// Returns (entry_id, is_chevron_click) where is_chevron_click means expand/collapse.
    pub fn file_tree_on_click(&mut self, x: f32, y: f32) -> Option<(usize, bool)> {
        self.file_tree.on_click(x, y)
    }

    /// Hit test for layout region.
    pub fn hit_test(&self, x: f32, y: f32) -> LayoutRegion {
        self.app_layout.hit_test(x, y)
    }

    /// Returns the time until the next animation frame is needed.
    pub fn time_until_next_frame(&self) -> std::time::Duration {
        self.caret.time_until_next_blink()
    }

    /// Toggle debug overlay visibility (debug builds only).
    #[cfg(debug_assertions)]
    pub fn toggle_debug_overlay(&mut self) {
        self.debug_overlay.toggle();
    }

    /// Toggle debug overlay visibility (no-op in release builds).
    #[cfg(not(debug_assertions))]
    pub fn toggle_debug_overlay(&mut self) {
        // No-op in release builds
    }

    /// Collects all UI text blocks from visible components for batched rendering.
    ///
    /// This method gathers text from all UI components that need to be rendered
    /// via the shared UITextRenderer. By collecting all text first and making a
    /// single prepare() call, we avoid the glyphon issue where multiple prepare()
    /// calls overwrite the glyph vertex buffer.
    fn collect_all_ui_texts(&self, model: &RenderModel) -> Vec<TextBlock> {
        let mut all_texts = Vec::with_capacity(256);

        // Collect sidebar/file tree texts
        if model.sidebar.visible {
            all_texts.extend(self.file_tree.build_texts());
        }

        // Collect tab bar texts
        if model.tab_bar.visible {
            all_texts.extend(self.editor_tabs.build_texts());
        }

        // Collect status line texts
        all_texts.extend(self.status_line.build_texts());

        // Collect panel texts (if visible)
        if self.panel_manager.is_visible() {
            all_texts.extend(self.panel_manager.build_texts());
        }

        // Collect command palette texts (if visible)
        if self.command_palette.is_visible() {
            all_texts.extend(self.command_palette.build_texts());
        }

        all_texts
    }

    /// Renders a frame from the given RenderModel.
    pub fn render(&mut self, model: &RenderModel, scale_factor: f32) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen_width = self.config.width as f32;
        let screen_height = self.config.height as f32;
        let logical_width = screen_width / scale_factor;
        let logical_height = screen_height / scale_factor;

        // Update app layout for new frame
        self.app_layout.calculate(logical_width, logical_height);

        // Update command palette bounds
        self.command_palette.calculate_bounds(logical_width, logical_height);

        // Calculate layout (existing system)
        let layout = self.calculate_layout(model, scale_factor);

        // Prepare all components (pass full screen resolution for glyphon viewport)
        self.sidebar.prepare(
            &self.device,
            &self.queue,
            &model.sidebar,
            layout.sidebar,
            &self.theme,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        // Update new FileTree with sidebar entries
        // IMPORTANT: set_bounds MUST be called BEFORE update_entries because
        // update_entries calls layout_entries() which depends on self.bounds
        if model.sidebar.visible {
            self.file_tree.set_bounds(layout.sidebar);
            self.file_tree.set_focused(model.sidebar.focused);
            let file_entries: Vec<FileEntry> = model.sidebar.entries.iter().enumerate().map(|(i, entry)| {
                FileEntry {
                    id: i,
                    name: entry.name.clone(),
                    path: String::new(), // Path not available in presentation
                    is_dir: entry.is_dir,
                    is_expanded: false, // Expansion state not tracked in presentation
                    depth: 0,           // Flat list in current presentation
                    is_selected: entry.is_selected,
                }
            }).collect();
            self.file_tree.update_entries(file_entries);
        }

        self.gutter.prepare(
            &self.device,
            &self.queue,
            model,
            layout.gutter,
            &self.theme,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        self.text_area.prepare(
            &self.device,
            &self.queue,
            model,
            layout.text_area,
            &self.theme,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        self.status_bar.prepare(
            &self.device,
            &self.queue,
            &model.status,
            layout.status_bar,
            &self.theme,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        // Update new StatusLine with status data
        self.status_line.set_bounds(layout.status_bar);
        self.status_line.set_position(model.status.cursor_line, model.status.cursor_column);
        self.status_line.set_language(&model.status.language);

        self.tab_bar.prepare(
            &self.device,
            &self.queue,
            &model.tab_bar,
            layout.tab_bar,
            &self.theme,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        // Update new EditorTabs with tab data
        if model.tab_bar.visible {
            let tab_infos: Vec<TabInfo> = model.tab_bar.tabs.iter().map(|tab| {
                TabInfo {
                    view_id: tab.view_id,
                    title: tab.title.clone(),
                    path: None,
                    is_dirty: tab.is_dirty,
                    is_active: tab.is_active,
                }
            }).collect();
            self.editor_tabs.update_tabs(tab_infos);
            self.editor_tabs.set_bounds(layout.tab_bar);
        }

        // =======================================================================
        // CRITICAL FIX: Collect ALL UI text blocks FIRST, then prepare ONCE
        // =======================================================================
        // Glyphon requires a single prepare() call per frame per TextRenderer.
        // Multiple prepare() calls overwrite the glyph vertex buffer, causing
        // earlier text to vanish. We collect all text blocks from all components
        // before making the single prepare() call.
        let all_ui_texts = self.collect_all_ui_texts(model);

        // Single prepare() call for ALL UI text (sidebar, tabs, status, panel, palette)
        self.ui_text_renderer.prepare(
            &self.device,
            &self.queue,
            &all_ui_texts,
            scale_factor,
            self.config.width,
            self.config.height,
        );

        // Build all rectangles
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Clear the screen
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.theme.palette.get(ColorRole::BgPrimary).to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // =======================================================================
        // PHASE 1: Render all UI backgrounds/rectangles in a single pass
        // =======================================================================
        // CRITICAL: We must collect ALL rects first, then prepare ONCE, then render.
        // This is because queue.write_buffer() executes immediately, but render passes
        // execute later when the command buffer is submitted. Multiple prepare() calls
        // would overwrite the GPU buffer before previous passes can use it.

        // Collect all UI rects from all components
        let mut all_ui_rects: Vec<crate::design_system::StyledRect> = Vec::new();

        // Sidebar/file tree rects
        if model.sidebar.visible {
            all_ui_rects.extend(self.file_tree.build_rects());
        }

        // Tab bar rects
        if model.tab_bar.visible {
            all_ui_rects.extend(self.editor_tabs.build_rects());
        }

        // Status line rects
        all_ui_rects.extend(self.status_line.build_rects());

        // Render all UI background rects in a single pass
        if !all_ui_rects.is_empty() {
            self.styled_rect_renderer.clear();
            self.styled_rect_renderer.push_all(&all_ui_rects, logical_width, logical_height);
            self.styled_rect_renderer.prepare(&self.queue);

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ui_rect_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                // No scissor needed - rects are already positioned within their bounds
                self.styled_rect_renderer.render(&mut pass);
            }
        }

        // Render gutter background
        if model.gutter.visible {
            self.gutter.render_background(&mut encoder, &view, &self.queue, layout.gutter, &self.theme, screen_width, screen_height, scale_factor);
        }

        // Render current line highlight first (below selection)
        // Use scissor rect to prevent overflow into tab bar
        let text_area_scissor = safe_scissor_rect(layout.text_area, scale_factor, self.config.width, self.config.height);
        let (current_line_rects, selection_rects) = self.text_area.build_selection_rects(model, layout.text_area, &self.theme, self.measured_char_width);
        self.text_area.render_selections(&mut encoder, &view, &self.queue, &current_line_rects, screen_width, screen_height, scale_factor, Some(text_area_scissor));

        // Render selection backgrounds on top
        self.text_area.render_selections(&mut encoder, &view, &self.queue, &selection_rects, screen_width, screen_height, scale_factor, Some(text_area_scissor));

        // Render caret (with scissor to prevent overflow into tab bar)
        self.caret.render(
            &mut encoder,
            &view,
            &self.queue,
            &model.caret,
            layout.text_area,
            &self.theme,
            self.measured_char_width,
            screen_width,
            screen_height,
            scale_factor,
            Some(text_area_scissor),
        );

        // Render gutter text (separate pass with gutter scissor)
        if model.gutter.visible {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gutter_text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (sx, sy, sw, sh) = safe_scissor_rect(layout.gutter, scale_factor, self.config.width, self.config.height);
            pass.set_scissor_rect(sx, sy, sw, sh);
            self.gutter.render(&mut pass);
        }

        // Render text area (requires a render pass with text_area scissor)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (sx, sy, sw, sh) = safe_scissor_rect(layout.text_area, scale_factor, self.config.width, self.config.height);
            pass.set_scissor_rect(sx, sy, sw, sh);
            self.text_area.render(&mut pass);
        }

        // Render bottom panel background rects (if visible)
        if self.panel_manager.is_visible() {
            let panel_rects = self.panel_manager.build_rects();
            self.styled_rect_renderer.clear();
            self.styled_rect_renderer.push_all(&panel_rects, logical_width, logical_height);
            self.styled_rect_renderer.prepare(&self.queue);

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("panel_rect_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let (sx, sy, sw, sh) = safe_scissor_rect(layout.panel, scale_factor, self.config.width, self.config.height);
                pass.set_scissor_rect(sx, sy, sw, sh);
                self.styled_rect_renderer.render(&mut pass);
            }
        }

        // Render dialog overlay (on top of everything)
        if Dialog::is_visible(&model.dialog) {
            let logical_width = screen_width / scale_factor;
            let logical_height = screen_height / scale_factor;

            self.dialog.prepare(
                &self.device,
                &self.queue,
                &model.dialog,
                logical_width,
                logical_height,
                &self.theme,
                scale_factor,
            );

            let dialog_rects = self.dialog.build_rects(&model.dialog, logical_width, logical_height, &self.theme);
            self.dialog.render_background(&mut encoder, &view, &self.queue, &dialog_rects, screen_width, screen_height, scale_factor);

            // Dialog text pass (full screen scissor for overlay)
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("dialog_text_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                self.dialog.render(&mut pass);
            }
        }

        // Render command palette background rects (if visible)
        if self.command_palette.is_visible() {
            let palette_rects = self.command_palette.build_rects();
            self.styled_rect_renderer.clear();
            self.styled_rect_renderer.push_all(&palette_rects, logical_width, logical_height);
            self.styled_rect_renderer.prepare(&self.queue);

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("command_palette_rect_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                self.styled_rect_renderer.render(&mut pass);
            }
        }

        // =======================================================================
        // PHASE 2: Render ALL UI text in a SINGLE pass
        // =======================================================================
        // The ui_text_renderer was prepared once at the start with all text blocks.
        // Now we render them all together.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.ui_text_renderer.render(&mut pass);
        }

        // =======================================================================
        // PHASE 3: Debug overlay (only in debug builds)
        // =======================================================================
        #[cfg(debug_assertions)]
        if self.debug_overlay.enabled {
            let debug_rects = self.build_debug_overlay_rects(&layout);
            // Render debug rects using existing rect renderer
            self.styled_rect_renderer.clear();
            self.styled_rect_renderer.push_all(&debug_rects, logical_width, logical_height);
            self.styled_rect_renderer.prepare(&self.queue);

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("debug_overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.styled_rect_renderer.render(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Calculates the layout bounds for all components.
    fn calculate_layout(&mut self, model: &RenderModel, scale_factor: f32) -> Layout {
        let screen_width = self.config.width as f32 / scale_factor;
        let screen_height = self.config.height as f32 / scale_factor;
        let status_height = self.status_bar.height();
        let tab_bar_height = if model.tab_bar.visible { self.editor_tabs.height() } else { 0.0 };
        let panel_height = self.panel_manager.height();

        // Total bounds
        let total = Bounds::new(0.0, 0.0, screen_width, screen_height);

        // Split off status bar at bottom
        let (main_area, status_bar) = total.split_vertical(screen_height - status_height);

        // Split off panel above status bar (if visible)
        let (main_area, panel) = if panel_height > 0.0 {
            let (upper, lower) = main_area.split_vertical(main_area.height - panel_height);
            // Update panel bounds
            self.panel_manager.set_bounds(lower);
            (upper, lower)
        } else {
            (main_area, Bounds::default())
        };

        // Split sidebar if visible
        let (sidebar, right_area) = if model.sidebar.visible {
            main_area.split_horizontal(self.sidebar.width(&model.sidebar))
        } else {
            (Bounds::default(), main_area)
        };

        // Split tab bar at top of editor area
        let (tab_bar, editor_area) = if model.tab_bar.visible {
            right_area.split_vertical(tab_bar_height)
        } else {
            (Bounds::default(), right_area)
        };

        // Split gutter if visible
        let (gutter, text_area) = if model.gutter.visible {
            let gutter_width = self.gutter.width(&model.gutter, self.measured_char_width);
            editor_area.split_horizontal(gutter_width)
        } else {
            (Bounds::default(), editor_area)
        };

        Layout {
            sidebar,
            tab_bar,
            gutter,
            text_area,
            panel,
            status_bar,
        }
    }

    /// Build debug overlay rectangles showing component bounds.
    #[cfg(debug_assertions)]
    fn build_debug_overlay_rects(&self, layout: &Layout) -> Vec<crate::design_system::StyledRect> {
        use crate::design_system::StyledRect;

        let mut rects = Vec::new();
        let thickness = self.debug_overlay.config.border_thickness;

        if self.debug_overlay.config.show_bounds {
            // Sidebar bounds (red)
            if layout.sidebar.width > 0.0 {
                for rect in debug::build_bounds_border(layout.sidebar, debug::colors::SIDEBAR, thickness) {
                    rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                        .with_fill(rect.color));
                }
            }

            // Tab bar bounds (green)
            if layout.tab_bar.height > 0.0 {
                for rect in debug::build_bounds_border(layout.tab_bar, debug::colors::TAB_BAR, thickness) {
                    rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                        .with_fill(rect.color));
                }
            }

            // Gutter bounds (yellow)
            if layout.gutter.width > 0.0 {
                for rect in debug::build_bounds_border(layout.gutter, debug::colors::GUTTER, thickness) {
                    rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                        .with_fill(rect.color));
                }
            }

            // Text area bounds (blue)
            for rect in debug::build_bounds_border(layout.text_area, debug::colors::TEXT_AREA, thickness) {
                rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                    .with_fill(rect.color));
            }

            // Panel bounds (cyan)
            if layout.panel.height > 0.0 {
                for rect in debug::build_bounds_border(layout.panel, debug::colors::PANEL, thickness) {
                    rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                        .with_fill(rect.color));
                }
            }

            // Status bar bounds (magenta)
            for rect in debug::build_bounds_border(layout.status_bar, debug::colors::STATUS_BAR, thickness) {
                rects.push(StyledRect::new(Bounds::new(rect.x, rect.y, rect.width, rect.height))
                    .with_fill(rect.color));
            }
        }

        rects
    }
}

/// Layout bounds for all components.
struct Layout {
    sidebar: Bounds,
    tab_bar: Bounds,
    gutter: Bounds,
    text_area: Bounds,
    panel: Bounds,
    status_bar: Bounds,
}
