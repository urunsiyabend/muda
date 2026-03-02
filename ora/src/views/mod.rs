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
//! - `CommandPaletteView`: VS Code-style command palette overlay with fuzzy search
//! - `PanelManagerView`: Resizable bottom panel with tabs (Output, Problems, Terminal, Debug)
//! - `AppLayout`: Top-level layout orchestrator composing all regions with Stack overlays

pub mod app_layout;
pub mod command_palette;
pub mod dialog;
pub mod file_tree;
pub mod gutter;
pub mod panel_manager;
pub mod sidebar;
pub mod status_bar;
pub mod tab_bar;
pub mod text_area;

pub use app_layout::AppLayout;
pub use command_palette::{CommandPaletteView, CommandItem, fuzzy_match, FuzzyMatch};
pub use dialog::DialogView;
pub use file_tree::FileTreeView;
pub use gutter::GutterView;
pub use panel_manager::{PanelManagerView, PanelKind, PanelState};
pub use sidebar::SidebarView;
pub use status_bar::StatusBarView;
pub use tab_bar::TabBarView;
pub use text_area::{TextAreaView, LINE_HEIGHT};
