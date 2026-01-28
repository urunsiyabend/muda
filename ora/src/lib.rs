pub mod app;
pub mod element;
pub mod entity;
pub mod platform;
pub mod view;
pub(crate) mod window;
mod context;

pub use app::App;
pub use context::{AppContext, ViewContext, WindowContext};
pub use element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
pub use view::View;

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
