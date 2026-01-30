//! Status bar view - placeholder for Task 2.

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::Div;
use crate::view::View;

/// Placeholder for StatusBarView - will be implemented in Task 2.
pub struct StatusBarView;

impl View for StatusBarView {
    fn render(&self, _cx: &mut ViewContext) -> AnyElement {
        Div::new().into()
    }
}
