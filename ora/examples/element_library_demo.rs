//! Element Library Demo: Comprehensive showcase of Phase 05 + Phase 06 elements
//! Demonstrates:
//! 1. Builder API with px() and pct() unit functions
//! 2. Button variants (Primary, Secondary, Ghost, Destructive)
//! 3. Stack container for z-layering
//! 4. Image element with placeholder rendering
//! 5. Runtime theme switching with theme-aware components

use ora::{
    AnyElement, App, Color, Div, TextElement, View, ViewContext,
    px, pct, button, stack, img,
    FocusHandle,
};

struct ElementLibraryView {
    // Persistent focus handles for buttons
    button_primary: FocusHandle,
    button_secondary: FocusHandle,
    button_ghost: FocusHandle,
    button_destructive: FocusHandle,
}

impl View for ElementLibraryView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();
        let bg_color = theme.color(ora::ColorToken::BgPrimary);
        let title_color = theme.color(ora::ColorToken::FgPrimary);
        let subtitle_color = theme.color(ora::ColorToken::FgSecondary);

        Div::new()
            .flex_col()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(bg_color)
            .p(32.0)
            .gap(32.0)
            // Header
            .child(
                Div::new()
                    .flex_col()
                    .gap(8.0)
                    .child(
                        TextElement::new("Element Library Demo")
                            .size(32.0)
                            .color(title_color)
                    )
                    .child(
                        TextElement::new(format!(
                            "Phase 05 + Theme System - Current theme: {:?} (Press 'T' to toggle)",
                            theme.mode()
                        ))
                            .size(14.0)
                            .color(subtitle_color)
                    )
            )
            // Section 1: Builder API with unit functions
            .child(self.builder_api_section(cx))
            // Section 2: Button variants
            .child(self.button_variants_section(cx))
            // Section 3: Stack container
            .child(self.stack_section(cx))
            // Section 4: Image elements
            .child(self.image_section(cx))
            .into()
    }
}

impl ElementLibraryView {
    /// Section 1: Builder API with px() and pct() unit functions
    fn builder_api_section(&self, cx: &mut ViewContext) -> Div {
        let section_title_color = cx.theme().color(ora::ColorToken::FgPrimary);

        Div::new()
            .flex_col()
            .gap(16.0)
            .child(
                TextElement::new("1. Builder API - Unit Functions")
                    .size(20.0)
                    .color(section_title_color)
            )
            .child(
                Div::new()
                    .flex_row()
                    .gap(16.0)
                    // Fixed width with px()
                    .child(
                        Div::new()
                            .w(px(120.0))
                            .h(px(80.0))
                            .bg(Color::rgb(0.3, 0.4, 0.6))
                            .border_radius(8.0)
                            .flex_col()
                            .justify_center()
                            .align_center()
                            .child(
                                TextElement::new("px(120)")
                                    .size(14.0)
                                    .color(Color::white())
                            )
                    )
                    // Percentage width with pct()
                    .child(
                        Div::new()
                            .w(pct(30.0))
                            .h(px(80.0))
                            .bg(Color::rgb(0.4, 0.5, 0.3))
                            .border_radius(8.0)
                            .flex_col()
                            .justify_center()
                            .align_center()
                            .child(
                                TextElement::new("pct(30)")
                                    .size(14.0)
                                    .color(Color::white())
                            )
                    )
                    // Alignment demonstration
                    .child(
                        Div::new()
                            .flex_row()
                            .w(px(200.0))
                            .h(px(80.0))
                            .bg(Color::rgb(0.5, 0.3, 0.4))
                            .border_radius(8.0)
                            .p(12.0)
                            .justify(ora::JustifyContent::SpaceBetween)
                            .items(ora::AlignItems::Center)
                            .child(
                                TextElement::new("Start")
                                    .size(12.0)
                                    .color(Color::white())
                            )
                            .child(
                                TextElement::new("End")
                                    .size(12.0)
                                    .color(Color::white())
                            )
                    )
            )
    }

    /// Section 2: Button variants with hover/active states
    fn button_variants_section(&self, cx: &mut ViewContext) -> Div {
        let section_title_color = cx.theme().color(ora::ColorToken::FgPrimary);
        let hint_color = cx.theme().color(ora::ColorToken::FgMuted);

        Div::new()
            .flex_col()
            .gap(16.0)
            .child(
                TextElement::new("2. Button Variants - Hover/Active/Focus States")
                    .size(20.0)
                    .color(section_title_color)
            )
            .child(
                TextElement::new("Hover over buttons (lighten), click and hold (darken), Tab for focus ring")
                    .size(12.0)
                    .color(hint_color)
            )
            .child(
                Div::new()
                    .flex_row()
                    .gap(12.0)
                    // Primary button
                    .child(
                        button("Primary Button")
                            .primary()
                            .focusable(self.button_primary.clone())
                            .on_click(|| {
                                log::info!("Primary button clicked!");
                            })
                    )
                    // Secondary button
                    .child(
                        button("Secondary")
                            .secondary()
                            .focusable(self.button_secondary.clone())
                            .on_click(|| {
                                log::info!("Secondary button clicked!");
                            })
                    )
                    // Ghost button
                    .child(
                        button("Ghost")
                            .ghost()
                            .focusable(self.button_ghost.clone())
                            .on_click(|| {
                                log::info!("Ghost button clicked!");
                            })
                    )
                    // Destructive button
                    .child(
                        button("Destructive")
                            .destructive()
                            .focusable(self.button_destructive.clone())
                            .on_click(|| {
                                log::info!("Destructive button clicked!");
                            })
                    )
            )
            .child(
                Div::new()
                    .flex_row()
                    .gap(12.0)
                    .m(8.0)
                    // Disabled button example
                    .child(
                        button("Disabled Button")
                            .primary()
                            .disabled(true)
                    )
            )
    }

    /// Section 3: Stack container with z-layering
    fn stack_section(&self, cx: &mut ViewContext) -> Div {
        let section_title_color = cx.theme().color(ora::ColorToken::FgPrimary);
        let hint_color = cx.theme().color(ora::ColorToken::FgMuted);

        Div::new()
            .flex_col()
            .gap(16.0)
            .child(
                TextElement::new("3. Stack Container - Z-Layering")
                    .size(20.0)
                    .color(section_title_color)
            )
            .child(
                TextElement::new("Three overlapping rectangles - gray (bottom), purple (middle), gold (top)")
                    .size(12.0)
                    .color(hint_color)
            )
            .child(
                stack()
                    .w(300.0)
                    .h(200.0)
                    // Bottom layer - gray (largest)
                    .child(
                        Div::new()
                            .w(px(280.0))
                            .h(px(180.0))
                            .bg(Color::rgb(0.3, 0.3, 0.35))
                            .border_radius(12.0)
                    )
                    // Middle layer - purple (medium)
                    .child(
                        Div::new()
                            .w(px(200.0))
                            .h(px(120.0))
                            .bg(Color::rgb(0.5, 0.3, 0.6))
                            .border_radius(12.0)
                    )
                    // Top layer - gold (smallest)
                    .child(
                        Div::new()
                            .w(px(120.0))
                            .h(px(80.0))
                            .bg(Color::rgb(0.8, 0.6, 0.2))
                            .border_radius(12.0)
                            .flex_col()
                            .justify_center()
                            .align_center()
                            .child(
                                TextElement::new("Top")
                                    .size(14.0)
                                    .color(Color::white())
                            )
                    )
            )
    }

    /// Section 4: Image elements with ObjectFit
    fn image_section(&self, cx: &mut ViewContext) -> Div {
        let section_title_color = cx.theme().color(ora::ColorToken::FgPrimary);
        let hint_color = cx.theme().color(ora::ColorToken::FgMuted);

        Div::new()
            .flex_col()
            .gap(16.0)
            .child(
                TextElement::new("4. Image Elements - ObjectFit Placeholder")
                    .size(20.0)
                    .color(section_title_color)
            )
            .child(
                TextElement::new("Image placeholders (actual texture rendering deferred to future phase)")
                    .size(12.0)
                    .color(hint_color)
            )
            .child(
                Div::new()
                    .flex_row()
                    .gap(16.0)
                    // Contain
                    .child(
                        Div::new()
                            .flex_col()
                            .gap(8.0)
                            .child(
                                img("placeholder.png")
                                    .object_fit(ora::ObjectFit::Contain)
                                    .w(px(150.0))
                                    .h(px(150.0))
                                    .placeholder_color(Color::rgb(0.2, 0.3, 0.4))
                            )
                            .child(
                                TextElement::new("Contain")
                                    .size(12.0)
                                    .color(hint_color)
                            )
                    )
                    // Cover
                    .child(
                        Div::new()
                            .flex_col()
                            .gap(8.0)
                            .child(
                                img("placeholder.png")
                                    .object_fit(ora::ObjectFit::Cover)
                                    .w(px(150.0))
                                    .h(px(150.0))
                                    .placeholder_color(Color::rgb(0.3, 0.4, 0.2))
                            )
                            .child(
                                TextElement::new("Cover")
                                    .size(12.0)
                                    .color(hint_color)
                            )
                    )
                    // Fill
                    .child(
                        Div::new()
                            .flex_col()
                            .gap(8.0)
                            .child(
                                img("placeholder.png")
                                    .object_fit(ora::ObjectFit::Fill)
                                    .w(px(150.0))
                                    .h(px(150.0))
                                    .placeholder_color(Color::rgb(0.4, 0.2, 0.3))
                            )
                            .child(
                                TextElement::new("Fill")
                                    .size(12.0)
                                    .color(hint_color)
                            )
                    )
            )
    }
}

fn main() {
    env_logger::init();

    App::new()
        .title("ora - Element Library Demo")
        .size(900, 1000)
        .on_open(|cx| {
            // Create persistent focus handles for buttons
            let button_primary = cx.focus_handle();
            let button_secondary = cx.focus_handle();
            let button_ghost = cx.focus_handle();
            let button_destructive = cx.focus_handle();

            log::info!("Element library demo initialized");
            log::info!("Theme starts in dark mode. Press 'T' to toggle between dark and light themes.");

            let view = ElementLibraryView {
                button_primary,
                button_secondary,
                button_ghost,
                button_destructive,
            };
            cx.set_root_view(view);
            log::info!("Element library demo ready! Buttons use theme-aware colors.");
        })
        .run();
}
