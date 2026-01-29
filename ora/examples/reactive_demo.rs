//! Reactive Demo: Model -> Notify -> Observer -> Re-render
//! Shows the complete reactive pipeline:
//! 1. Model<CounterState> stores application state
//! 2. CounterView observes the model via cx.observe()
//! 3. State changes via model.update() queue notify effects
//! 4. Observer fires, marks entities dirty
//! 5. View re-renders with updated state

use ora::{AnyElement, App, Div, TextElement, View, ViewContext, Color, Model, Subscription};

#[derive(Clone)]
struct CounterState {
    count: u32,
}

struct CounterView {
    model: Model<CounterState>,
    _subscription: Subscription, // Keep observation alive
}

impl View for CounterView {
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
                TextElement::new("ora Reactive Demo")
                    .size(24.0)
                    .color(Color::rgb(0.9, 0.9, 1.0))
            )
            .child(
                Div::new()
                    .flex_col()
                    .gap(8.0)
                    .child(
                        TextElement::new(&format!("Count: {}", count))
                            .size(48.0)
                            .color(Color::rgb(0.4, 0.8, 1.0))
                    )
                    .child(
                        TextElement::new("Model -> observe() -> notify() -> re-render")
                            .size(14.0)
                            .color(Color::rgb(0.5, 0.5, 0.6))
                    )
            )
            .child(
                TextElement::new("Reactive state system working!")
                    .size(16.0)
                    .color(Color::rgb(0.3, 0.9, 0.5))
            )
            .into()
    }
}

fn main() {
    env_logger::init();

    App::new()
        .title("ora - Reactive Demo")
        .size(600, 400)
        .on_open(|cx| {
            // 1. Create reactive model with initial state
            let model = cx.new_model(CounterState { count: 0 });
            log::info!("Created model with count=0");

            // 2. Set up observation - fires when model calls notify()
            let sub = cx.observe(&model, |_cx| {
                log::info!("Observer fired: model changed!");
            });

            // 3. Create view with model handle and subscription
            let view = CounterView {
                model: model.clone(),
                _subscription: sub,
            };
            cx.set_root_view(view);
            log::info!("Root view set");

            // 4. Trigger reactive update - this should:
            //    a. Update model state (count = 42)
            //    b. Queue notify effect
            //    c. Flush effects (since update_depth returns to 0)
            //    d. Observer callback fires
            //    e. Entity marked dirty
            //    f. Next render shows count=42
            model.update(cx.app_context_mut(), |state, mcx| {
                state.count = 42;
                mcx.notify();
                log::info!("Model updated to count={}", state.count);
            });
        })
        .run();
}
