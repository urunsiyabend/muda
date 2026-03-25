use super::types::Modifiers;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event::KeyEvent;

/// Named keyboard keys (non-character keys)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Space,
}

/// Key representation - either a named key or a character
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Named(NamedKey),
    Character(String),
}

/// A keystroke combines a key and modifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keystroke {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Keystroke {
    /// Create a keystroke from a character with optional modifiers
    pub fn char(c: impl Into<String>) -> Self {
        Self {
            key: Key::Character(c.into()),
            modifiers: Modifiers::none(),
        }
    }

    /// Create a keystroke from a named key with optional modifiers
    pub fn named(key: NamedKey) -> Self {
        Self {
            key: Key::Named(key),
            modifiers: Modifiers::none(),
        }
    }

    /// Set modifiers on this keystroke (builder pattern)
    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Add ctrl modifier
    pub fn ctrl(mut self) -> Self {
        self.modifiers.ctrl = true;
        self
    }

    /// Add alt modifier
    pub fn alt(mut self) -> Self {
        self.modifiers.alt = true;
        self
    }

    /// Add shift modifier
    pub fn shift(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }

    /// Add meta/cmd modifier
    pub fn meta(mut self) -> Self {
        self.modifiers.meta = true;
        self
    }
}

/// Keyboard event with keystroke and additional metadata
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub keystroke: Keystroke,
    pub repeat: bool,
    pub text: Option<String>,
}

/// Translate winit KeyEvent to ora KeyboardEvent
pub fn translate_key_event(event: &KeyEvent, modifiers: Modifiers) -> Option<KeyboardEvent> {
    // Only handle key presses for now
    if !event.state.is_pressed() {
        return None;
    }

    // Extract key from physical key
    let key = match event.physical_key {
        PhysicalKey::Code(code) => {
            match code {
                KeyCode::Enter => Key::Named(NamedKey::Enter),
                KeyCode::Escape => Key::Named(NamedKey::Escape),
                KeyCode::Tab => Key::Named(NamedKey::Tab),
                KeyCode::Backspace => Key::Named(NamedKey::Backspace),
                KeyCode::Delete => Key::Named(NamedKey::Delete),
                KeyCode::Insert => Key::Named(NamedKey::Insert),
                KeyCode::Home => Key::Named(NamedKey::Home),
                KeyCode::End => Key::Named(NamedKey::End),
                KeyCode::PageUp => Key::Named(NamedKey::PageUp),
                KeyCode::PageDown => Key::Named(NamedKey::PageDown),
                KeyCode::ArrowUp => Key::Named(NamedKey::ArrowUp),
                KeyCode::ArrowDown => Key::Named(NamedKey::ArrowDown),
                KeyCode::ArrowLeft => Key::Named(NamedKey::ArrowLeft),
                KeyCode::ArrowRight => Key::Named(NamedKey::ArrowRight),
                KeyCode::F1 => Key::Named(NamedKey::F1),
                KeyCode::F2 => Key::Named(NamedKey::F2),
                KeyCode::F3 => Key::Named(NamedKey::F3),
                KeyCode::F4 => Key::Named(NamedKey::F4),
                KeyCode::F5 => Key::Named(NamedKey::F5),
                KeyCode::F6 => Key::Named(NamedKey::F6),
                KeyCode::F7 => Key::Named(NamedKey::F7),
                KeyCode::F8 => Key::Named(NamedKey::F8),
                KeyCode::F9 => Key::Named(NamedKey::F9),
                KeyCode::F10 => Key::Named(NamedKey::F10),
                KeyCode::F11 => Key::Named(NamedKey::F11),
                KeyCode::F12 => Key::Named(NamedKey::F12),
                KeyCode::Space => Key::Named(NamedKey::Space),
                // For character keys, try to get the text representation
                _ => {
                    // Try to get text from the logical key
                    if let Some(text) = event.text.as_ref() {
                        Key::Character(text.to_string())
                    } else {
                        // Fallback: try to get a simple character from the key code
                        if let Some(c) = keycode_to_char(code) {
                            Key::Character(c.to_string())
                        } else {
                            return None; // Skip unhandled keys
                        }
                    }
                }
            }
        }
        PhysicalKey::Unidentified(_) => return None,
    };

    Some(KeyboardEvent {
        keystroke: Keystroke { key, modifiers },
        repeat: event.repeat,
        text: event.text.as_ref().map(|s| s.to_string()),
    })
}

/// Convert simple key codes to characters (for keys without text)
fn keycode_to_char(code: KeyCode) -> Option<char> {
    match code {
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        _ => None,
    }
}
