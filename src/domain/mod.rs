//! Core domain types for the Muda Editor.
//!
//! This module contains the fundamental data structures that represent
//! the editor's domain model, separated from view and UI concerns.
//!
//! # Module Structure
//!
//! - `text_buffer`: Low-level text storage and coordinate types
//! - `document`: Document with identity, persistence, and derived state
//! - `workspace`: Top-level container for multi-document editing sessions

pub mod document;
pub mod text_buffer;
pub mod workspace;

pub use document::{Document, DocumentId, DocumentMetadata, LineEnding};
pub use text_buffer::{DocumentRevision, TextBuffer, TextOffset, TextPosition, TextRange};
pub use workspace::Workspace;
