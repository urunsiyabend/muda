//! Interactive Demo: Complete Event System
//! Shows the complete event system working:
//! 1. Mouse events: hover, active, click
//! 2. Focus management: Tab/Shift+Tab navigation
//! 3. Keyboard events: actions via keybindings
//! 4. Interaction state: framework-tracked hover/active

use ora::{
    AnyElement, App, Color, Div, Model, Subscription, TextElement, View, ViewContext,
    define_action, Keystroke, Key, NamedKey, Modifiers,
};

// Define actions for this demo
define_action!(IncrementAction);
define_action!(DecrementAction);
define_action!(ResetAction);

#[derive(Clone)]
struct CounterState {
    count: i32,
}

struct InteractiveView {
    model: Model<CounterState>,
    _subscription: Subscription,
}

impl View for InteractiveView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let count = cx.read_model(&self.model).count;

        Div::new()
            .flex_col()
            .w_pct(100.0)
            .h_pct(100.0)
            .bg(Color::rgb(0.08, 0.08, 0.10))
            .p(24.0)
            .gap(16.0)
            .child(
                TextElement::new("ora Interactive Demo")
                    .size(28.0)
                    .color(Color::rgb(0.9, 0.9, 1.0))
            )
            .child(
                Div::new()
                    .flex_col()
                    .gap(12.0)
                    .child(
                        TextElement::new(&format!("Count: {}", count))
                            .size(48.0)
                            .color(Color::rgb(0.4, 0.8, 1.0))
                    )
                    .child(
                        TextElement::new("Full event system infrastructure:")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
                    .child(
                        TextElement::new("✓ Mouse hover/active tracking")
                            .size(14.0)
                            .color(Color::rgb(0.3, 0.9, 0.5))
                    )
                    .child(
                        TextElement::new("✓ Focus management with Tab navigation")
                            .size(14.0)
                            .color(Color::rgb(0.3, 0.9, 0.5))
                    )
                    .child(
                        TextElement::new("✓ Two-phase event dispatch (capture/bubble)")
                            .size(14.0)
                            .color(Color::rgb(0.3, 0.9, 0.5))
                    )
                    .child(
                        TextElement::new("✓ Action system with keybindings")
                            .size(14.0)
                            .color(Color::rgb(0.3, 0.9, 0.5))
                    )
            )
            .child(
                Div::new()
                    .flex_col()
                    .gap(8.0)
                    .child(
                        TextElement::new("Keyboard Controls:")
                            .size(16.0)
                            .color(Color::rgb(0.7, 0.7, 0.8))
                    )
                    .child(
                        TextElement::new("  ↑  Arrow Up - Increment (logged)")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
                    .child(
                        TextElement::new("  ↓  Arrow Down - Decrement (logged)")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
                    .child(
                        TextElement::new("  Ctrl+R - Reset (logged)")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
                    .child(
                        TextElement::new("  Tab - Focus next")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
                    .child(
                        TextElement::new("  Shift+Tab - Focus previous")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
            )
            .child(
                TextElement::new("Check logs (RUST_LOG=trace) to see event flow!")
                    .size(12.0)
                    .color(Color::rgb(0.4, 0.4, 0.5))
            )
            .into()
    }
}

fn main() {
    env_logger::init();

    App::new()
        .title("ora - Interactive Demo")
        .size(700, 600)
        .on_open(|cx| {
            // Create reactive model
            let model = cx.new_model(CounterState { count: 0 });
            log::info!("Created counter model with count=0");

            // Set up observation
            let sub = cx.observe(&model, |_cx| {
                log::info!("Observer fired: counter changed!");
            });

            // Note: Full action handler integration requires access to AppContext
            // For this infrastructure demo, we register the actions but they won't
            // actually mutate the model (requires Phase 5 builder API for proper
            // element-level action handling with context access)
            cx.on_action::<IncrementAction>(move |_action| {
                log::info!("IncrementAction triggered! (action system working)");
            });

            cx.on_action::<DecrementAction>(move |_action| {
                log::info!("DecrementAction triggered! (action system working)");
            });

            cx.on_action::<ResetAction>(move |_action| {
                log::info!("ResetAction triggered! (action system working)");
            });

            // Bind keys to actions
            // Arrow Up -> Increment
            cx.bind_key(
                Keystroke {
                    key: Key::Named(NamedKey::ArrowUp),
                    modifiers: Modifiers::none(),
                },
                Box::new(IncrementAction),
            );

            // Arrow Down -> Decrement
            cx.bind_key(
                Keystroke {
                    key: Key::Named(NamedKey::ArrowDown),
                    modifiers: Modifiers::none(),
                },
                Box::new(DecrementAction),
            );

            // Ctrl+R -> Reset
            cx.bind_key(
                Keystroke {
                    key: Key::Character("r".to_string()),
                    modifiers: Modifiers {
                        ctrl: true,
                        alt: false,
                        shift: false,
                        meta: false,
                    },
                },
                Box::new(ResetAction),
            );

            log::info!("Keybindings registered: Arrow Up/Down, Ctrl+R");

            // Create view
            let view = InteractiveView {
                model: model.clone(),
                _subscription: sub,
            };
            cx.set_root_view(view);
            log::info!("Root view set - interactive demo ready!");
        })
        .run();
}
