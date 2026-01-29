use ora::{AnyElement, Div, TextElement, View, ViewContext, Color, pct};

/// Mock editor layout demonstrating flexbox and styled elements
struct LayoutDemo;

impl LayoutDemo {
    fn new() -> Self {
        Self
    }
}

impl View for LayoutDemo {
    fn render(&self, _cx: &mut ViewContext) -> AnyElement {
        // Root container - full window with padding
        Div::new()
            .flex_col()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(Color::rgb(0.08, 0.08, 0.10))
            .p(16.0)
            .gap(8.0)
            .child(
                // Header bar
                Div::new()
                    .flex_row()
                    .w(pct(100.0))
                    .h(48.0)
                    .bg(Color::rgb(0.12, 0.12, 0.14))
                    .border(1.0, Color::rgb(0.2, 0.2, 0.22))
                    .border_radius(8.0)
                    .p(12.0)
                    .align_center()
                    .child(
                        TextElement::new("ora Layout Demo")
                            .size(18.0)
                            .color(Color::rgb(0.9, 0.9, 1.0))
                    )
            )
            .child(
                // Main content area - sidebar + editor
                Div::new()
                    .flex_row()
                    .grow(1.0)
                    .gap(8.0)
                    .child(
                        // Sidebar - fixed width file list
                        Div::new()
                            .flex_col()
                            .w(220.0)
                            .bg(Color::rgb(0.10, 0.10, 0.12))
                            .border(1.0, Color::rgb(0.18, 0.18, 0.20))
                            .border_radius(8.0)
                            .p(12.0)
                            .gap(8.0)
                            .child(
                                TextElement::new("FILES")
                                    .size(12.0)
                                    .color(Color::rgb(0.5, 0.5, 0.6))
                            )
                            .child(
                                TextElement::new("src/")
                                    .size(14.0)
                                    .color(Color::rgb(0.7, 0.7, 0.8))
                            )
                            .child(
                                TextElement::new("  main.rs")
                                    .size(14.0)
                                    .color(Color::rgb(0.8, 0.8, 0.9))
                            )
                            .child(
                                TextElement::new("  lib.rs")
                                    .size(14.0)
                                    .color(Color::rgb(0.8, 0.8, 0.9))
                            )
                            .child(
                                TextElement::new("  utils.rs")
                                    .size(14.0)
                                    .color(Color::rgb(0.8, 0.8, 0.9))
                            )
                            .child(
                                TextElement::new("Cargo.toml")
                                    .size(14.0)
                                    .color(Color::rgb(0.7, 0.7, 0.8))
                            )
                    )
                    .child(
                        // Main editor area - flex-grow to fill space
                        Div::new()
                            .flex_col()
                            .grow(1.0)
                            .bg(Color::rgb(0.06, 0.06, 0.08))
                            .border(1.0, Color::rgb(0.18, 0.18, 0.20))
                            .border_radius(8.0)
                            .p(16.0)
                            .gap(4.0)
                            .child(
                                TextElement::new("fn main() {")
                                    .size(16.0)
                                    .color(Color::rgb(0.95, 0.95, 1.0))
                            )
                            .child(
                                TextElement::new("    println!(\"Hello, ora!\");")
                                    .size(16.0)
                                    .color(Color::rgb(0.95, 0.95, 1.0))
                            )
                            .child(
                                TextElement::new("}")
                                    .size(16.0)
                                    .color(Color::rgb(0.95, 0.95, 1.0))
                            )
                    )
            )
            .child(
                // Status bar at bottom
                Div::new()
                    .flex_row()
                    .w(pct(100.0))
                    .h(32.0)
                    .bg(Color::rgb(0.12, 0.12, 0.14))
                    .border(1.0, Color::rgb(0.2, 0.2, 0.22))
                    .border_radius(6.0)
                    .px(16.0)
                    .align_center()
                    .gap(24.0)
                    .child(
                        TextElement::new("Ln 1, Col 1")
                            .size(13.0)
                            .color(Color::rgb(0.7, 0.7, 0.8))
                    )
                    .child(
                        TextElement::new("Rust")
                            .size(13.0)
                            .color(Color::rgb(0.7, 0.7, 0.8))
                    )
                    .child(
                        TextElement::new("UTF-8")
                            .size(13.0)
                            .color(Color::rgb(0.7, 0.7, 0.8))
                    )
            )
            .into()
    }
}

fn main() {
    ora::App::new()
        .title("ora - Layout Demo")
        .size(1200, 800)
        .on_open(|cx| {
            cx.set_root_view(LayoutDemo::new());
        })
        .run();
}
