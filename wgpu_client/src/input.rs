//! Input handling: winit events → EditorCommand mapping.

use winit::event::{ElementState, KeyEvent, Modifiers};
use winit::keyboard::{Key, NamedKey};

use core_editor::commands::editor_command::{Direction, MoveScope};
use core_editor::commands::EditorCommand;

/// Translates a winit key event to an EditorCommand.
pub fn translate_key(event: &KeyEvent, modifiers: &Modifiers) -> Option<EditorCommand> {
    // Only handle key presses, not releases
    if event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.state().control_key();
    let shift = modifiers.state().shift_key();
    let _alt = modifiers.state().alt_key();

    match &event.logical_key {
        // Navigation
        Key::Named(NamedKey::ArrowLeft) => Some(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: if ctrl { MoveScope::Word } else { MoveScope::Char },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowRight) => Some(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: if ctrl { MoveScope::Word } else { MoveScope::Char },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowUp) => Some(EditorCommand::MoveCursor {
            direction: Direction::Up,
            scope: MoveScope::Char,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::ArrowDown) => Some(EditorCommand::MoveCursor {
            direction: Direction::Down,
            scope: MoveScope::Char,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::Home) => Some(EditorCommand::MoveCursor {
            direction: Direction::Left,
            scope: if ctrl { MoveScope::Document } else { MoveScope::Line },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::End) => Some(EditorCommand::MoveCursor {
            direction: Direction::Right,
            scope: if ctrl { MoveScope::Document } else { MoveScope::Line },
            extend_selection: shift,
        }),
        Key::Named(NamedKey::PageUp) => Some(EditorCommand::MoveCursor {
            direction: Direction::Up,
            scope: MoveScope::Page,
            extend_selection: shift,
        }),
        Key::Named(NamedKey::PageDown) => Some(EditorCommand::MoveCursor {
            direction: Direction::Down,
            scope: MoveScope::Page,
            extend_selection: shift,
        }),

        // Editing
        Key::Named(NamedKey::Backspace) => Some(EditorCommand::Backspace),
        Key::Named(NamedKey::Delete) => Some(EditorCommand::Delete),
        Key::Named(NamedKey::Enter) => Some(EditorCommand::InsertNewline),
        Key::Named(NamedKey::Tab) => {
            // Insert 4 spaces for tab
            Some(EditorCommand::InsertText("    ".to_string()))
        }

        // Ctrl shortcuts
        Key::Character(c) if ctrl => match c.as_str() {
            "a" => Some(EditorCommand::SelectAll),
            "c" => Some(EditorCommand::Copy),
            "x" => Some(EditorCommand::Cut),
            "v" => Some(EditorCommand::Paste),
            "z" => Some(EditorCommand::Undo),
            "y" => Some(EditorCommand::Redo),
            "s" => Some(EditorCommand::Save),
            "l" => Some(EditorCommand::ToggleLineNumbers),
            _ => None,
        },

        // Character input
        Key::Character(c) if !ctrl => {
            let ch = c.chars().next()?;
            Some(EditorCommand::InsertChar(ch))
        }

        _ => None,
    }
}

/// Input action that requires app-level handling (not a direct EditorCommand).
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Request to quit the application.
    RequestQuit,
    /// Toggle sidebar visibility.
    ToggleSidebar,
    /// Focus the sidebar.
    FocusSidebar,
    /// Focus the editor.
    FocusEditor,
    /// Confirm pending action (Y in dialog).
    ConfirmPendingAction,
    /// Cancel pending action (N or Esc in dialog).
    CancelPendingAction,
    /// Sidebar navigation: move up.
    SidebarUp,
    /// Sidebar navigation: move down.
    SidebarDown,
    /// Sidebar navigation: go back (parent directory).
    SidebarBack,
    /// Sidebar: open selected item.
    SidebarOpen,
}

/// Translates a winit key event to an app-level action.
pub fn translate_app_action(
    event: &KeyEvent,
    modifiers: &Modifiers,
    has_pending_action: bool,
    sidebar_focused: bool,
) -> Option<AppAction> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.state().control_key();

    // Dialog handling takes priority
    if has_pending_action {
        return match &event.logical_key {
            Key::Character(c) if c == "y" || c == "Y" => Some(AppAction::ConfirmPendingAction),
            Key::Character(c) if c == "n" || c == "N" => Some(AppAction::CancelPendingAction),
            Key::Named(NamedKey::Escape) => Some(AppAction::CancelPendingAction),
            _ => None,
        };
    }

    // Global shortcuts
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            return if sidebar_focused {
                Some(AppAction::FocusEditor)
            } else {
                Some(AppAction::RequestQuit)
            };
        }
        Key::Character(c) if ctrl && c == "b" => return Some(AppAction::ToggleSidebar),
        Key::Character(c) if ctrl && c == "h" => return Some(AppAction::FocusSidebar),
        _ => {}
    }

    // Sidebar-specific shortcuts
    if sidebar_focused {
        return match &event.logical_key {
            Key::Named(NamedKey::ArrowUp) => Some(AppAction::SidebarUp),
            Key::Character(c) if c == "k" => Some(AppAction::SidebarUp),
            Key::Named(NamedKey::ArrowDown) => Some(AppAction::SidebarDown),
            Key::Character(c) if c == "j" => Some(AppAction::SidebarDown),
            Key::Named(NamedKey::ArrowLeft) => Some(AppAction::SidebarBack),
            Key::Named(NamedKey::Backspace) => Some(AppAction::SidebarBack),
            Key::Character(c) if c == "h" && !ctrl => Some(AppAction::SidebarBack),
            Key::Named(NamedKey::Enter) => Some(AppAction::SidebarOpen),
            _ => None,
        };
    }

    None
}
