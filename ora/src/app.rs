use crate::context::WindowContext;
use crate::platform::event_loop::OraApp;
use winit::event_loop::EventLoop;

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
