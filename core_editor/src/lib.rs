//! Muda Text Editor - Core editor library.
//!
//! This crate contains the platform-agnostic core of the editor:
//! - Domain models (Document, TextBuffer, Workspace)
//! - View state (EditorView, Viewport, Caret, Selection)
//! - ViewModel (RenderModel, StyledSpan, TextStyle)
//! - Command system
//! - Syntax highlighting
//!
//! Rendering backends (TUI, GPU) live in separate crates.

pub mod app;
pub mod commands;
pub mod domain;
pub mod events;
pub mod syntax;
pub mod view;
pub mod view_model;
