//! View module containing editor view state types.
//!
//! This module separates view-specific state (cursor, selection, scroll)
//! from document state, enabling multiple views of the same document.

pub mod caret;
pub mod editor_view;
pub mod selection;
pub mod sidebar;
pub mod viewport;

pub use caret::{Caret, CaretSet};
pub use editor_view::{EditorView, ViewId, ViewOptions};
pub use selection::{Selection, SelectionSet};
pub use sidebar::{FileEntry, FocusState, Sidebar};
pub use viewport::Viewport;
