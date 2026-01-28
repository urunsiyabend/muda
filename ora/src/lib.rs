pub mod app;
pub mod effect;
pub mod element;
pub mod elements;
pub mod entity;
pub mod platform;
pub mod subscription;
pub mod view;
pub(crate) mod window;
mod context;

pub mod style;
pub mod layout;
pub mod rendering;

pub use app::App;
pub use context::{AppContext, ViewContext, WindowContext};
pub use element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
pub use entity::Model;
pub use subscription::Subscription;
pub use view::View;

// Re-export commonly used style and layout types
pub use style::{Style, Color, Background, Overflow, FlexDirection};
pub use layout::{AvailableSpace, LayoutInput, LayoutOutput};
pub use rendering::TextSystem;

// Re-export primitive elements
pub use elements::{Div, TextElement};

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
