use super::keyboard::Keystroke;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Trait for typed actions that can be dispatched via keybindings
pub trait Action: 'static {
    /// Returns a unique name for this action
    fn name(&self) -> &'static str;

    /// Returns the TypeId for this action
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Clone this action into a Box<dyn Action>
    fn boxed_clone(&self) -> Box<dyn Action>;

    /// Downcast to &dyn Any for type-specific handling
    fn as_any(&self) -> &dyn Any;
}

/// Macro to define simple action structs that implement Action trait
#[macro_export]
macro_rules! define_action {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name;

        impl $crate::events::actions::Action for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn boxed_clone(&self) -> Box<dyn $crate::events::actions::Action> {
                Box::new(*self)
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

/// Context identifiers for conditional keybindings
#[derive(Debug, Clone, Default)]
pub struct KeyContext {
    identifiers: Vec<String>,
}

impl KeyContext {
    pub fn new() -> Self {
        Self {
            identifiers: Vec::new(),
        }
    }

    /// Add a context identifier
    pub fn add(&mut self, identifier: impl Into<String>) {
        self.identifiers.push(identifier.into());
    }

    /// Check if this context has a specific identifier
    pub fn has(&self, identifier: &str) -> bool {
        self.identifiers.iter().any(|id| id == identifier)
    }
}

/// A keybinding associates a keystroke with an action and optional context
pub struct KeyBinding {
    pub keystroke: Keystroke,
    pub action: Box<dyn Action>,
    pub context_predicate: Option<Box<dyn Fn(&KeyContext) -> bool>>,
}

impl KeyBinding {
    /// Create a new keybinding with keystroke and action
    pub fn new(keystroke: Keystroke, action: Box<dyn Action>) -> Self {
        Self {
            keystroke,
            action,
            context_predicate: None,
        }
    }

    /// Add a context predicate to this binding
    pub fn with_context(mut self, predicate: impl Fn(&KeyContext) -> bool + 'static) -> Self {
        self.context_predicate = Some(Box::new(predicate));
        self
    }

    /// Check if this binding matches the given keystroke and context
    pub fn matches(&self, keystroke: &Keystroke, context: &KeyContext) -> bool {
        if &self.keystroke != keystroke {
            return false;
        }

        if let Some(predicate) = &self.context_predicate {
            predicate(context)
        } else {
            true
        }
    }
}

/// Registry of keybindings with last-wins conflict resolution
pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Bind a keystroke to an action with optional context
    pub fn bind(&mut self, binding: KeyBinding) {
        self.bindings.push(binding);
    }

    /// Convenience method to bind a keystroke directly to an action
    pub fn bind_key(&mut self, keystroke: Keystroke, action: Box<dyn Action>) {
        self.bind(KeyBinding::new(keystroke, action));
    }

    /// Match a keystroke against bindings (last-wins)
    /// Returns the action if a matching binding is found
    pub fn match_action(&self, keystroke: &Keystroke, context: &KeyContext) -> Option<&dyn Action> {
        // Iterate in reverse for last-wins behavior
        for binding in self.bindings.iter().rev() {
            if binding.matches(keystroke, context) {
                return Some(&*binding.action);
            }
        }
        None
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for action handler callbacks
pub type ActionHandler = Box<dyn FnMut(&dyn Any)>;

/// Registry for action handlers
/// Maps TypeId -> Vec<handler> for typed action dispatch
pub struct ActionRegistry {
    handlers: HashMap<TypeId, Vec<ActionHandler>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a typed action handler
    pub fn on_action<A: Action>(&mut self, mut handler: impl FnMut(&A) + 'static) {
        let type_id = TypeId::of::<A>();

        let wrapped = Box::new(move |action: &dyn Any| {
            if let Some(typed_action) = action.downcast_ref::<A>() {
                handler(typed_action);
            }
        });

        self.handlers
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(wrapped);
    }

    /// Dispatch an action to all registered handlers
    pub fn dispatch(&mut self, action: &dyn Action) {
        let type_id = action.type_id();

        if let Some(handlers) = self.handlers.get_mut(&type_id) {
            let action_any = action.as_any();
            for handler in handlers.iter_mut() {
                handler(action_any);
            }
        }
    }

    /// Clear all handlers for a specific action type
    pub fn clear_handlers<A: Action>(&mut self) {
        let type_id = TypeId::of::<A>();
        self.handlers.remove(&type_id);
    }

    /// Clear all handlers
    pub fn clear_all(&mut self) {
        self.handlers.clear();
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
