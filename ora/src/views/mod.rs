//! Views module for editor chrome components.
//!
//! Views are stateful UI components that implement the View trait.
//! They produce element trees (Div, TextElement, etc.) during render(),
//! consuming presentation data from core_editor.
//!
//! # Architecture
//!
//! Views follow the GPUI pattern:
//! - Views own no rendering state (no FontSystem, TextAtlas, etc.)
//! - Views produce declarative element trees
//! - Framework handles text shaping, layout, and batched rendering
//!
//! # Components
//!
//! - `TabBarView`: Horizontal tab bar showing open documents
//! - `StatusBarView`: Bottom status bar with cursor position, language, etc.
//! - `DialogView`: Modal dialog for confirmations (unsaved changes, etc.)
//! - `SidebarView`: Collapsible file explorer panel
//! - `GutterView`: Line number gutter with current line highlighting
//! - `TextAreaView`: Syntax-highlighted text rendering with caret and selection

pub mod dialog;
pub mod gutter;
pub mod sidebar;
pub mod status_bar;
pub mod tab_bar;
pub mod text_area;

pub use dialog::DialogView;
pub use gutter::GutterView;
pub use sidebar::SidebarView;
pub use status_bar::StatusBarView;
pub use tab_bar::TabBarView;
pub use text_area::{TextAreaView, LINE_HEIGHT};
