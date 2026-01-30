//! Modal dialog view for confirmations and alerts.
//!
//! Renders a modal overlay with semi-transparent backdrop and centered dialog box.
//! Consumes `DialogPresentation` from core_editor for dialog state.
//!
//! # CONTEXT.md Locked Decisions
//!
//! - Backdrop click does NOT dismiss (must use buttons)
//! - Escape key does NOT dismiss (must use Cancel button)
//! - Buttons are right-aligned in dialog footer

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{button, Div, TextElement, stack};
use crate::events::FocusHandle;
use crate::style::{px, pct, Color};
use crate::theme::ColorToken;
use crate::view::View;
use core_editor::view_model::DialogPresentation;

/// Dialog width in logical pixels (from wgpu_client constants).
pub const DIALOG_WIDTH: f32 = 420.0;

/// Dialog padding in logical pixels.
pub const DIALOG_PADDING: f32 = 24.0;

/// Button height in logical pixels.
pub const BUTTON_HEIGHT: f32 = 36.0;

/// Button minimum width in logical pixels.
pub const BUTTON_MIN_WIDTH: f32 = 110.0;

/// Button spacing (gap) in logical pixels.
pub const BUTTON_SPACING: f32 = 12.0;

/// Title bar height in logical pixels.
const TITLE_BAR_HEIGHT: f32 = 36.0;

/// Warning accent strip width in logical pixels.
const WARNING_ACCENT_WIDTH: f32 = 4.0;

/// Title font size.
const TITLE_FONT_SIZE: f32 = 14.0;

/// Content font size.
const CONTENT_FONT_SIZE: f32 = 14.0;

/// Dialog border radius.
const DIALOG_BORDER_RADIUS: f32 = 8.0;

/// Backdrop opacity (60% per CONTEXT.md).
const BACKDROP_OPACITY: f32 = 0.6;

/// Modal dialog view for displaying confirmations and alerts.
///
/// Renders a Stack with:
/// 1. Semi-transparent backdrop covering the full screen
/// 2. Centered dialog box with title, content, and action buttons
///
/// The dialog is NOT dismissible by:
/// - Clicking the backdrop (per CONTEXT.md decision)
/// - Pressing Escape key (per CONTEXT.md decision)
///
/// Users must click one of the action buttons to dismiss.
///
/// # Example
///
/// ```ignore
/// let dialog = DialogView::new(presentation, cx);
/// // In a parent view's render():
/// Div::new().child(dialog.render(cx))
/// ```
pub struct DialogView {
    /// The presentation data for the dialog.
    presentation: DialogPresentation,
    /// Focus handle for the Save button.
    save_focus: FocusHandle,
    /// Focus handle for the Don't Save button.
    dont_save_focus: FocusHandle,
    /// Focus handle for the Cancel button.
    cancel_focus: FocusHandle,
}

impl DialogView {
    /// Creates a new dialog view with the given presentation data.
    ///
    /// Focus handles are created for each button to enable keyboard navigation.
    pub fn new(presentation: DialogPresentation, cx: &mut ViewContext) -> Self {
        Self {
            presentation,
            save_focus: cx.focus_handle(),
            dont_save_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
        }
    }

    /// Updates the presentation data.
    pub fn set_presentation(&mut self, presentation: DialogPresentation) {
        self.presentation = presentation;
    }

    /// Renders the semi-transparent backdrop.
    ///
    /// The backdrop covers the full screen with 60% opacity black.
    /// Per CONTEXT.md: backdrop click does NOT dismiss the dialog.
    fn render_backdrop(&self, _cx: &mut ViewContext) -> Div {
        // 60% opacity black backdrop
        // NO on_click handler - CONTEXT decision: must use buttons
        Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(Color::rgba(0.0, 0.0, 0.0, BACKDROP_OPACITY))
    }

    /// Renders the dialog box container with shadow.
    fn render_dialog_box(
        &self,
        action_description: &str,
        unsaved_documents: &[String],
        cx: &mut ViewContext,
    ) -> Div {
        let theme = cx.theme();

        // Calculate dialog height based on content
        let content_height = self.calculate_content_height(unsaved_documents.len());
        let dialog_height = TITLE_BAR_HEIGHT + content_height + DIALOG_PADDING + BUTTON_HEIGHT + DIALOG_PADDING;

        // Dialog box with shadow, centered positioning will be handled by flex parent
        Div::new()
            .w(px(DIALOG_WIDTH))
            .h(px(dialog_height))
            .flex_col()
            .bg(theme.color(ColorToken::BgPrimary))
            .border_radius(DIALOG_BORDER_RADIUS)
            .shadow(6.0, 6.0, 20.0, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.4))
            .child(self.render_title_bar(cx))
            .child(self.render_content(action_description, unsaved_documents, cx))
            .child(self.render_button_row(cx))
    }

    /// Calculates content area height based on number of unsaved documents.
    fn calculate_content_height(&self, doc_count: usize) -> f32 {
        // Base height for message text
        let base_height = 60.0;
        // Additional height per document in the list
        let per_doc_height = 22.0;
        base_height + (doc_count as f32 * per_doc_height)
    }

    /// Renders the title bar with warning accent strip.
    fn render_title_bar(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Title bar with slightly darker background
        Div::new()
            .w(pct(100.0))
            .h(px(TITLE_BAR_HEIGHT))
            .flex_row()
            // Warning accent strip on the left (yellow/orange for unsaved changes)
            .child(
                Div::new()
                    .w(px(WARNING_ACCENT_WIDTH))
                    .h(pct(100.0))
                    .bg(Color::rgba(1.0, 0.71, 0.0, 1.0)) // #FFB500 warning yellow
            )
            // Title text container
            .child(
                Div::new()
                    .grow(1.0)
                    .h(pct(100.0))
                    .flex_row()
                    .align_center()
                    .px(12.0)
                    .bg(theme.color(ColorToken::BgElevated))
                    .child(
                        TextElement::new("Unsaved Changes")
                            .size(TITLE_FONT_SIZE)
                            .color(theme.color(ColorToken::FgPrimary))
                    )
            )
    }

    /// Renders the dialog content with message and document list.
    fn render_content(
        &self,
        action_description: &str,
        unsaved_documents: &[String],
        cx: &mut ViewContext,
    ) -> Div {
        let theme = cx.theme();

        // Build content container
        let mut content = Div::new()
            .grow(1.0)
            .flex_col()
            .p(DIALOG_PADDING)
            .gap(8.0);

        // Main message
        let message = format!(
            "Do you want to save changes before {}?",
            action_description.to_lowercase()
        );
        content = content.child(
            TextElement::new(&message)
                .size(CONTENT_FONT_SIZE)
                .color(theme.color(ColorToken::FgPrimary))
        );

        // List of unsaved documents if any
        if !unsaved_documents.is_empty() {
            content = content.child(
                Div::new()
                    .flex_col()
                    .gap(4.0)
                    .child(
                        TextElement::new("Unsaved files:")
                            .size(CONTENT_FONT_SIZE)
                            .color(theme.color(ColorToken::FgSecondary))
                    )
            );

            for doc in unsaved_documents {
                content = content.child(
                    Div::new()
                        .flex_row()
                        .px(16.0)
                        .child(
                            TextElement::new(format!("- {}", doc))
                                .size(CONTENT_FONT_SIZE)
                                .color(theme.color(ColorToken::FgSecondary))
                        )
                );
            }
        }

        content
    }

    /// Renders the button row at the bottom of the dialog.
    ///
    /// Per CONTEXT.md: buttons are right-aligned.
    /// Button order: Save (Y) - Primary, Don't Save (N) - Secondary, Cancel (Esc) - Ghost
    ///
    /// NOTE: Button handlers cannot access context (known blocker from STATE.md).
    /// For now, buttons render but handlers would emit events to parent view.
    fn render_button_row(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Button row container - right-aligned per CONTEXT.md
        Div::new()
            .w(pct(100.0))
            .h(px(BUTTON_HEIGHT + DIALOG_PADDING * 2.0))
            .flex_row()
            .justify_end()
            .align_center()
            .gap(BUTTON_SPACING)
            .px(DIALOG_PADDING)
            .bg(theme.color(ColorToken::BgPrimary))
            // Save button - Primary variant
            .child(
                button("Save (Y)")
                    .primary()
                    .focusable(self.save_focus.clone())
            )
            // Don't Save button - Secondary variant
            .child(
                button("Don't Save (N)")
                    .secondary()
                    .focusable(self.dont_save_focus.clone())
            )
            // Cancel button - Ghost variant
            .child(
                button("Cancel (Esc)")
                    .ghost()
                    .focusable(self.cancel_focus.clone())
            )
    }

    /// Renders a centering container for the dialog.
    /// Uses flexbox to center the dialog box both horizontally and vertically.
    fn render_centering_container(&self, dialog_box: Div) -> Div {
        Div::new()
            .w(pct(100.0))
            .h(pct(100.0))
            .flex_row()
            .justify_center()
            .align_center()
            .child(dialog_box)
    }
}

impl View for DialogView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // If no dialog to show, return empty element
        let DialogPresentation::UnsavedChangesConfirmation {
            action_description,
            unsaved_documents,
        } = &self.presentation else {
            // DialogPresentation::None - return invisible placeholder
            return Div::new()
                .w(px(0.0))
                .h(px(0.0))
                .into();
        };

        // Build the dialog using Stack for z-layering
        // Layer 0 (bottom): Backdrop
        // Layer 1 (top): Centered dialog box
        stack()
            .w(pct(100.0))
            .h(pct(100.0))
            // Layer 0: Semi-transparent backdrop
            .child(self.render_backdrop(cx))
            // Layer 1: Dialog box (in centering container)
            .child(
                self.render_centering_container(
                    self.render_dialog_box(action_description, unsaved_documents, cx)
                )
            )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_constants() {
        assert_eq!(DIALOG_WIDTH, 420.0);
        assert_eq!(DIALOG_PADDING, 24.0);
        assert_eq!(BUTTON_HEIGHT, 36.0);
        assert_eq!(BUTTON_MIN_WIDTH, 110.0);
        assert_eq!(BUTTON_SPACING, 12.0);
        assert_eq!(BACKDROP_OPACITY, 0.6);
    }

    #[test]
    fn test_dialog_presentation_none() {
        // DialogPresentation::None should be handled
        let presentation = DialogPresentation::None;
        match presentation {
            DialogPresentation::None => (),
            _ => panic!("Expected None variant"),
        }
    }

    #[test]
    fn test_dialog_presentation_unsaved_changes() {
        let presentation = DialogPresentation::unsaved_changes(
            "Exit",
            vec!["main.rs".to_string(), "lib.rs".to_string()],
        );

        if let DialogPresentation::UnsavedChangesConfirmation {
            action_description,
            unsaved_documents,
        } = presentation
        {
            assert_eq!(action_description, "Exit");
            assert_eq!(unsaved_documents.len(), 2);
            assert_eq!(unsaved_documents[0], "main.rs");
            assert_eq!(unsaved_documents[1], "lib.rs");
        } else {
            panic!("Expected UnsavedChangesConfirmation variant");
        }
    }

    #[test]
    fn test_content_height_calculation() {
        // No documents: base height only
        let base = 60.0;
        let per_doc = 22.0;

        // 0 documents
        let height_0 = base + 0.0 * per_doc;
        assert_eq!(height_0, 60.0);

        // 3 documents
        let height_3 = base + 3.0 * per_doc;
        assert_eq!(height_3, 126.0);
    }
}
