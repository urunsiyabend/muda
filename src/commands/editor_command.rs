//! EditorCommand: High-level user intents.
//!
//! These represent what the user wants to do, not how to do it.
//! Commands are UI-agnostic and can be triggered by keyboard, mouse, or API.

use std::path::PathBuf;

/// Direction for movement commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Scope for movement/selection commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveScope {
    /// Single character/line.
    Char,
    /// Word boundary.
    Word,
    /// Line boundary (beginning/end of line).
    Line,
    /// Page (viewport height).
    Page,
    /// Document boundary (beginning/end of document).
    Document,
}

/// High-level editor commands representing user intents.
///
/// These are UI-agnostic and will be translated to `EditOperation`s.
#[derive(Clone, Debug)]
pub enum EditorCommand {
    // === Navigation ===
    /// Move cursor in a direction.
    MoveCursor {
        direction: Direction,
        scope: MoveScope,
        extend_selection: bool,
    },
    /// Go to a specific line number (1-indexed).
    GotoLine(usize),

    // === Editing ===
    /// Insert a single character.
    InsertChar(char),
    /// Insert a string of text.
    InsertText(String),
    /// Insert a newline with auto-indent.
    InsertNewline,
    /// Delete selection or character before cursor.
    Backspace,
    /// Delete selection or character after cursor.
    Delete,
    /// Delete the current selection.
    DeleteSelection,

    // === Selection ===
    /// Select all text.
    SelectAll,
    /// Clear the current selection.
    ClearSelection,

    // === Clipboard ===
    /// Copy selection to clipboard.
    Copy,
    /// Cut selection to clipboard.
    Cut,
    /// Paste from clipboard.
    Paste,

    // === Undo/Redo ===
    /// Undo the last transaction.
    Undo,
    /// Redo the last undone transaction.
    Redo,

    // === File Operations ===
    /// Save the current document.
    Save,
    /// Save the document to a new path.
    SaveAs(PathBuf),
    /// Open a file.
    Open(PathBuf),
    /// Create a new document.
    New,

    // === View ===
    /// Toggle line number display.
    ToggleLineNumbers,
    /// Scroll the viewport.
    Scroll { lines: i32 },

    // === Application ===
    /// Request to quit the application.
    Quit,
    /// Force quit without saving.
    ForceQuit,
    /// Cancel the current operation (e.g., dismiss dialog).
    Cancel,
}

impl EditorCommand {
    /// Returns true if this command modifies the document.
    pub fn modifies_document(&self) -> bool {
        matches!(
            self,
            EditorCommand::InsertChar(_)
                | EditorCommand::InsertText(_)
                | EditorCommand::InsertNewline
                | EditorCommand::Backspace
                | EditorCommand::Delete
                | EditorCommand::DeleteSelection
                | EditorCommand::Cut
                | EditorCommand::Paste
                | EditorCommand::Undo
                | EditorCommand::Redo
        )
    }

    /// Returns true if this command affects the selection.
    pub fn affects_selection(&self) -> bool {
        matches!(
            self,
            EditorCommand::MoveCursor { .. }
                | EditorCommand::SelectAll
                | EditorCommand::ClearSelection
                | EditorCommand::DeleteSelection
                | EditorCommand::Cut
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifies_document() {
        assert!(EditorCommand::InsertChar('a').modifies_document());
        assert!(EditorCommand::Backspace.modifies_document());
        assert!(!EditorCommand::Copy.modifies_document());
        assert!(!EditorCommand::SelectAll.modifies_document());
    }

    #[test]
    fn test_affects_selection() {
        assert!(EditorCommand::SelectAll.affects_selection());
        assert!(
            EditorCommand::MoveCursor {
                direction: Direction::Left,
                scope: MoveScope::Char,
                extend_selection: true,
            }
            .affects_selection()
        );
        assert!(!EditorCommand::Save.affects_selection());
    }
}
