pub mod app;
pub mod effect;
pub mod element;
pub mod elements;
pub mod entity;
pub mod events;
pub mod platform;
pub mod subscription;
pub mod view;
pub mod views;
pub(crate) mod window;
mod context;

pub mod animation;
pub mod editor_adapter;
pub mod style;
pub mod layout;
pub mod rendering;
pub mod theme;

pub use app::App;
pub use context::{AppContext, ViewContext, WindowContext, ThemeChanged};
pub use element::{AnyElement, Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
pub use entity::Model;
pub use subscription::Subscription;
pub use view::View;

// Re-export commonly used style and layout types
pub use style::{Style, Color, Background, Overflow, FlexDirection, JustifyContent, AlignItems, px, pct};
pub use layout::{AvailableSpace, LayoutInput, LayoutOutput};
pub use rendering::{TextSystem, TextureCache, TextureId, ImageSource};

// Re-export theme tokens
pub use theme::{Theme, ColorToken, ThemeMode, PaletteColor, sp, SpacingToken, TextSize, FontFamily};

// Re-export event types
pub use events::{
    FocusHandle, FocusId, FocusSource, MouseButton, Point, Modifiers,
    Action, Keymap, KeyBinding, Keystroke, Key, NamedKey, KeyContext,
};

// Re-export primitive elements
pub use elements::{
    button, Button, ButtonVariant, WidgetSize, Div, Stack, TextElement, stack, Image, img, img_from_bytes, ObjectFit,
    CaretElement, BLINK_RATE, ACTIVITY_TIMEOUT, CARET_WIDTH,
    input, Input,
    checkbox, Checkbox, CheckboxSize, toggle, Toggle,
    tab, Tab,
    list_item, ListItem,
    tree_item, TreeItem,
    ContextMenu, ContextMenuItem,
    Toast, ToastNotification, ToastSeverity,
};

// Re-export views
pub use views::{
    AppLayout,
    CommandPaletteView, CommandItem, fuzzy_match, FuzzyMatch,
    DialogView, FileTreeView, GutterView, PanelManagerView, PanelKind, PanelState,
    SidebarView, StatusBarView, TabBarView, TextAreaView, LINE_HEIGHT,
};

/// Run the application.
/// This is a convenience function that delegates to App::run().
pub fn run(app: App) -> ! {
    app.run()
}
