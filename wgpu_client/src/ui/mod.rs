//! Modern IDE UI layer built on the design system.
//!
//! This module provides the high-level UI infrastructure for the MUDA IDE,
//! connecting the widget system with the application renderer.

pub mod app_layout;
pub mod toolbar;
pub mod editor_tabs;
pub mod file_tree;
pub mod panels;
pub mod status_line;
pub mod command_palette;
pub mod split_view;

pub use app_layout::{AppLayout, LayoutRegion};
pub use toolbar::Toolbar;
pub use editor_tabs::EditorTabs;
pub use file_tree::FileTree;
pub use panels::{PanelManager, PanelKind};
pub use status_line::StatusLine;
pub use command_palette::{CommandPalette, CommandEntry, CommandKind};
pub use split_view::SplitView;
