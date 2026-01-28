pub mod app;
pub mod entity;
pub mod platform;
mod context;

pub use app::App;
pub use context::AppContext;

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
