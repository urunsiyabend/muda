//! Widget system for composable UI components.
//!
//! This module provides a trait-based widget architecture for building
//! interactive UI components with consistent event handling and rendering.
//!
//! # Architecture
//!
//! ```text
//! widgets/
//! ├── core/           # Core widgets (Button, Input, Toggle, etc.)
//! └── navigation/     # Navigation widgets (Tab, TreeItem, ListItem)
//! ```
//!
//! # Widget Lifecycle
//!
//! 1. **Create**: Widget is instantiated with initial properties
//! 2. **Layout**: Widget receives constraints and returns its size
//! 3. **Update**: Widget updates internal state (animations, etc.)
//! 4. **Build**: Widget produces render output
//! 5. **Events**: Widget handles pointer/keyboard events

mod traits;
pub mod core;
pub mod navigation;
pub mod composites;
pub mod widget_renderer;

pub use traits::{
    Widget, WidgetId, WidgetEvent, WidgetOutput, WidgetValue,
    PointerEvent, PointerButton, KeyEvent, KeyCode, Modifiers,
};

// Re-export core widgets
pub use core::button::{Button, ButtonVariant, ButtonProps};
pub use core::input::{Input, InputProps};
pub use core::toggle::Toggle;
pub use core::checkbox::Checkbox;

// Re-export navigation widgets
pub use navigation::tab::{Tab, TabProps};
pub use navigation::list_item::{ListItem, ListItemProps};
pub use navigation::tree_item::{TreeItem, TreeItemProps, TreeItemKind};
