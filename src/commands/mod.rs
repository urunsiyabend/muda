//! Commands module for the editor.
//!
//! This module provides:
//! - `EditorCommand`: High-level user intents (UI-agnostic)
//! - `EditOperation`: Low-level reversible text operations
//! - `UndoTransaction`: Groups of operations for atomic undo/redo
//! - `CommandHistory`: Transaction-based undo/redo stack

pub mod edit_operation;
pub mod editor_command;
pub mod history;
pub mod transaction;

pub use edit_operation::EditOperation;
pub use editor_command::EditorCommand;
pub use history::CommandHistory;
pub use transaction::UndoTransaction;
