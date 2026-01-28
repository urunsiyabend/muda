use crate::app::App;
use crate::context::{AppContext, WindowContext};
use crate::element::{LayoutContext, PaintContext, PrepaintContext};
use crate::entity::EntityStorage;
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
    entity_storage: EntityStorage,
}

impl OraApp {
    pub fn new(app: App) -> Self {
        Self {
            app_config: Some(app),
            gpu_state: None,
            ora_window: OraWindow::new(),
            entity_storage: EntityStorage::new(),
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
            let mut app_context = AppContext::new(std::mem::replace(
                &mut self.entity_storage,
                EntityStorage::new(),
            ));
            let winit_window = &self.gpu_state.as_ref().unwrap().window;
            let mut window_context = WindowContext::new(
                &mut app_context,
                &mut self.ora_window,
                winit_window,
            );
            on_open(&mut window_context);
            self.entity_storage = app_context.into_storage();
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
            WindowEvent::RedrawRequested => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    // Create AppContext to access entity storage
                    let mut app_context = AppContext::new(std::mem::replace(
                        &mut self.entity_storage,
                        EntityStorage::new(),
                    ));

                    // Render the root view to get element tree
                    if let Some(mut element_tree) = self.ora_window.render(&mut app_context) {
                        let window_size = gpu_state.size;

                        // Phase 1: Request layout
                        let mut layout_cx = LayoutContext::new(
                            &mut app_context.entity_storage,
                            window_size,
                        );
                        element_tree.request_layout(&mut layout_cx);

                        // Phase 2: Prepaint
                        let mut prepaint_cx = PrepaintContext::new(
                            &mut app_context.entity_storage,
                            window_size,
                        );
                        element_tree.prepaint(&mut prepaint_cx);

                        // Phase 3: Paint
                        let mut paint_cx = PaintContext::new(
                            &mut app_context.entity_storage,
                            window_size,
                        );
                        element_tree.paint(&mut paint_cx);

                        // Extract paint commands and present
                        let commands = paint_cx.take_commands();
                        match gpu_state.present(commands) {
                            Ok(_) => {
                                // Request continuous redraw
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
                        match gpu_state.present(Vec::new()) {
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

                    // Restore entity storage
                    self.entity_storage = app_context.into_storage();
                }
            }
            _ => {}
        }
    }
}
