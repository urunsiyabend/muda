pub mod app;
pub mod element;
pub mod entity;
pub mod platform;
pub mod view;
pub(crate) mod window;
mod context;

pub mod style;
pub mod layout;
// pub mod rendering; // Temporarily disabled - incomplete code from future phase

pub use app::App;
pub use context::{AppContext, ViewContext, WindowContext};
pub use element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
pub use view::View;

// Re-export commonly used style and layout types
pub use style::{Style, Color, Background, Overflow, FlexDirection};
pub use layout::{AvailableSpace, LayoutInput, LayoutOutput};
// pub use rendering::TextSystem; // Temporarily disabled - incomplete code from future phase

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
