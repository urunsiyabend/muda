//! Keyboard-to-EditorCommand translation for the editor backend.
//!
//! Translates ora keyboard events (produced by `translate_key_event`) into
//! `EditorCommand` variants that can be dispatched to an `EditorDataSource`.
//!
//! This is the pure-ora version of the translation logic that was previously
//! only available in `wgpu_client/src/input.rs`. The logic is ported here so
//! ora's event loop can dispatch editor commands without depending on wgpu_client.

use crate::editor_adapter::{CursorDirection, EditorCommand, MoveScope};
use crate::events::keyboard::{Key, KeyboardEvent, NamedKey};
use crate::events::types::Modifiers;

/// Translate an ora keyboard event into an EditorCommand, if applicable.
///
/// Returns `None` if the key event does not map to any editor command
/// (e.g., modifier-only keypresses or keys handled by the action system).
///
/// Caller should invoke this AFTER the existing Tab-navigation and action-system
/// handling, so that editor commands only consume keypresses not claimed upstream.
pub fn translate_editor_command(event: &KeyboardEvent, modifiers: Modifiers) -> Option<EditorCommand> {
    let ctrl = modifiers.ctrl;
    let shift = modifiers.shift;

    match &event.keystroke.key {
        // --- Navigation ---
        Key::Named(NamedKey::ArrowLeft) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Left,
            scope: if ctrl { MoveScope::Word } else { MoveScope::Char },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowRight) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Right,
            scope: if ctrl { MoveScope::Word } else { MoveScope::Char },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowUp) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Up,
            scope: MoveScope::Char,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowDown) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Down,
            scope: MoveScope::Char,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::Home) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Left,
            scope: if ctrl { MoveScope::Document } else { MoveScope::Line },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::End) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Right,
            scope: if ctrl { MoveScope::Document } else { MoveScope::Line },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::PageUp) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Up,
            scope: MoveScope::Page,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::PageDown) => Some(EditorCommand::MoveCursor {
            direction: CursorDirection::Down,
            scope: MoveScope::Page,
            extend_selection: shift,
        }),

        // --- Editing ---
        Key::Named(NamedKey::Backspace) => Some(EditorCommand::Backspace),
        Key::Named(NamedKey::Delete) => Some(EditorCommand::Delete),
        Key::Named(NamedKey::Enter) => Some(EditorCommand::InsertNewline),
        Key::Named(NamedKey::Tab) => {
            // Insert 4 spaces (matching wgpu_client behaviour).
            Some(EditorCommand::InsertText("    ".to_string()))
        }

        // --- Ctrl shortcuts ---
        Key::Character(c) if ctrl => match c.as_str() {
            "a" | "A" => Some(EditorCommand::SelectAll),
            "c" | "C" => Some(EditorCommand::Copy),
            "x" | "X" => Some(EditorCommand::Cut),
            "v" | "V" => Some(EditorCommand::Paste),
            "z" | "Z" => Some(EditorCommand::Undo),
            "y" | "Y" => Some(EditorCommand::Redo),
            "s" | "S" => Some(EditorCommand::Save),
            "l" | "L" => Some(EditorCommand::ToggleLineNumbers),
            _ => None,
        },

        // --- Printable character input ---
        Key::Character(c) if !ctrl => {
            let ch = c.chars().next()?;
            Some(EditorCommand::InsertChar(ch))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::keyboard::{Key, KeyboardEvent, Keystroke, NamedKey};

    fn make_event(key: Key) -> KeyboardEvent {
        KeyboardEvent {
            keystroke: Keystroke { key, modifiers: Modifiers::none() },
            repeat: false,
            text: None,
        }
    }

    #[test]
    fn test_arrow_left_char() {
        let ev = make_event(Key::Named(NamedKey::ArrowLeft));
        let cmd = translate_editor_command(&ev, Modifiers::none()).unwrap();
        assert!(matches!(cmd, EditorCommand::MoveCursor {
            direction: CursorDirection::Left,
            scope: MoveScope::Char,
            extend_selection: false,
        }));
    }

    #[test]
    fn test_ctrl_arrow_left_word() {
        let ev = make_event(Key::Named(NamedKey::ArrowLeft));
        let mods = Modifiers { ctrl: true, ..Modifiers::none() };
        let cmd = translate_editor_command(&ev, mods).unwrap();
        assert!(matches!(cmd, EditorCommand::MoveCursor {
            direction: CursorDirection::Left,
            scope: MoveScope::Word,
            ..
        }));
    }

    #[test]
    fn test_enter_inserts_newline() {
        let ev = make_event(Key::Named(NamedKey::Enter));
        let cmd = translate_editor_command(&ev, Modifiers::none()).unwrap();
        assert!(matches!(cmd, EditorCommand::InsertNewline));
    }

    #[test]
    fn test_backspace() {
        let ev = make_event(Key::Named(NamedKey::Backspace));
        let cmd = translate_editor_command(&ev, Modifiers::none()).unwrap();
        assert!(matches!(cmd, EditorCommand::Backspace));
    }

    #[test]
    fn test_ctrl_s_save() {
        let ev = make_event(Key::Character("s".to_string()));
        let mods = Modifiers { ctrl: true, ..Modifiers::none() };
        let cmd = translate_editor_command(&ev, mods).unwrap();
        assert!(matches!(cmd, EditorCommand::Save));
    }

    #[test]
    fn test_char_input() {
        let ev = make_event(Key::Character("a".to_string()));
        let cmd = translate_editor_command(&ev, Modifiers::none()).unwrap();
        assert!(matches!(cmd, EditorCommand::InsertChar('a')));
    }

    #[test]
    fn test_tab_inserts_spaces() {
        let ev = make_event(Key::Named(NamedKey::Tab));
        let cmd = translate_editor_command(&ev, Modifiers::none()).unwrap();
        assert!(matches!(cmd, EditorCommand::InsertText(ref s) if s == "    "));
    }
}
