pub mod types;
pub mod mouse;
pub mod focus;
pub mod keyboard;
pub mod dispatch;
pub mod actions;
pub mod interaction;

pub use types::{Point, Modifiers, MouseButton};
pub use mouse::{
    MouseMoveEvent, MouseDownEvent, MouseUpEvent, MouseScrollEvent,
    Hitbox, HitboxId, hit_test,
};
pub use focus::{FocusHandle, FocusId, FocusSource, FocusState};
pub use keyboard::{Key, NamedKey, Keystroke, KeyboardEvent};
pub use dispatch::{
    DispatchPhase, EventContext, EventHandlers,
    dispatch_mouse_down, dispatch_mouse_up, dispatch_mouse_move,
};
pub use actions::{Action, Keymap, KeyBinding, KeyContext, ActionRegistry};
pub use interaction::{InteractionState, MouseCapture};
