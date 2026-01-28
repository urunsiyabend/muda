pub mod app;
pub mod element;
pub mod entity;
pub mod platform;
pub mod view;
mod context;

pub use app::App;
pub use context::AppContext;
pub use element::{AnyElement, Element};
pub use view::View;

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
