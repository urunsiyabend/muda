use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::context::WindowContext;
use crate::editor_adapter::EditorDataSource;
use crate::platform::event_loop::OraApp;
use crate::views::EditorRootView;
use winit::event_loop::EventLoop;

/// Shared smooth-scroll pixel offset.
///
/// Written by the event loop's scroll accumulator, read by `EditorRootView`
/// to set `RenderModel.scroll_y_offset_px` for sub-line visual scrolling.
pub type SharedScrollOffset = Rc<Cell<f32>>;

/// Shared editor focus state.
///
/// Written by the event loop on `WindowEvent::Focused`, read by `EditorRootView`
/// to set `RenderModel.editor_focused` for selection dimming.
pub type SharedFocusState = Rc<Cell<bool>>;

/// Application builder for configuring and running the ora application.
pub struct App {
    pub(crate) title: String,
    pub(crate) size: (u32, u32),
    pub(crate) on_open: Option<Box<dyn FnOnce(&mut WindowContext)>>,
}

impl App {
    /// Create a new application with default settings.
    /// Default title: "ora"
    /// Default size: 800x600
    pub fn new() -> Self {
        Self {
            title: "ora".to_string(),
            size: (800, 600),
            on_open: None,
        }
    }

    /// Set the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Set the on_open callback, called when the application starts.
    /// The callback receives a WindowContext for setting up the root view.
    pub fn on_open<F: FnOnce(&mut WindowContext) + 'static>(mut self, f: F) -> Self {
        self.on_open = Some(Box::new(f));
        self
    }

    /// Run the application. This method never returns.
    pub fn run(self) -> ! {
        let event_loop = EventLoop::new().unwrap();
        let mut ora_app = OraApp::new(self);
        event_loop.run_app(&mut ora_app).unwrap();
        std::process::exit(0);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the ora application with an editor backend adapter.
///
/// This is the primary launch entry point for the full Muda editor.
/// The adapter is responsible for providing presentation data and
/// processing editor commands dispatched from keyboard events.
///
/// # Example
///
/// ```ignore
/// let adapter = CoreEditorAdapter::new();
/// ora::run_with_editor(adapter);  // never returns
/// ```
///
/// # Panics
///
/// Panics if the winit event loop cannot be created or if the application
/// exits abnormally. Under normal circumstances, this function never returns.
pub fn run_with_editor(adapter: impl EditorDataSource + 'static) -> ! {
    let event_loop = EventLoop::new().unwrap();

    // Wrap the adapter in Rc<RefCell<>> so it can be shared between the root
    // view (for build_render_model) and the event loop (for dispatch_command).
    let shared_adapter: Rc<RefCell<Box<dyn EditorDataSource>>> =
        Rc::new(RefCell::new(Box::new(adapter)));
    let view_adapter = shared_adapter.clone();

    // Shared smooth-scroll pixel offset: written by OraApp's scroll
    // accumulator, read by EditorRootView to shift text sub-line.
    let scroll_offset: SharedScrollOffset = Rc::new(Cell::new(0.0));
    let view_scroll_offset = scroll_offset.clone();

    // Shared focus state: written by OraApp on WindowEvent::Focused,
    // read by EditorRootView to dim selection when window is unfocused.
    let focus_state: SharedFocusState = Rc::new(Cell::new(true));
    let view_focus_state = focus_state.clone();

    let app = App::new()
        .title("Muda")
        .size(1280, 720)
        .on_open(move |cx| {
            let root_view = EditorRootView::new(
                view_adapter,
                view_scroll_offset,
                view_focus_state,
                &mut cx.as_view_context(),
            );
            cx.set_root_view(root_view);
        });
    let mut ora_app = OraApp::new_with_editor(app, shared_adapter, scroll_offset, focus_state);
    event_loop.run_app(&mut ora_app).unwrap();
    std::process::exit(0);
}
