//! Core domain types for the Muda Editor.
//!
//! This module contains the fundamental data structures that represent
//! the editor's domain model, separated from view and UI concerns.

pub mod text_buffer;
pub mod document;

pub use text_buffer::{TextBuffer, TextOffset, TextPosition, TextRange, DocumentRevision};
pub use document::{Document, DocumentId, DocumentMetadata, LineEnding};
