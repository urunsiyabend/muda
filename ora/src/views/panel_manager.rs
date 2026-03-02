//! Panel manager view for the bottom panel system.
//!
//! Renders a resizable bottom panel with tabs for Output, Problems, Terminal, and Debug.
//! Provides drag handle for height adjustment and preset cycling via double-click.
//!
//! # CONTEXT.md Locked Decisions
//!
//! - Drag handle + presets (double-click toggles collapsed/half/full)
//! - Instant toggle (no animation)
//! - Panel tabs drag to reorder (reorder_tab API ready; drag wiring deferred to AppLayout)
//! - Panel sizes persist (persistence deferred to AppLayout or later)

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::events::FocusHandle;
use crate::style::{px, pct, Color};
use crate::theme::ColorToken;
use crate::view::View;

/// Panel types available in the IDE bottom panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    Output,
    Problems,
    Terminal,
    Debug,
}

impl PanelKind {
    /// Returns the human-readable label for this panel kind.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Problems => "Problems",
            Self::Terminal => "Terminal",
            Self::Debug => "Debug",
        }
    }
}

/// Preset height positions for double-click cycling.
///
/// Cycling order: Collapsed -> Half -> Full -> Collapsed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelPreset {
    /// Panel is hidden (height = 0).
    Collapsed,
    /// Panel at default half height (200px).
    Half,
    /// Panel at full expanded height (400px).
    Full,
}

/// Default panel height when expanded (Half preset).
pub const DEFAULT_PANEL_HEIGHT: f32 = 200.0;

/// Minimum allowed panel height when visible.
pub const MIN_PANEL_HEIGHT: f32 = 100.0;

/// Maximum allowed panel height.
pub const MAX_PANEL_HEIGHT: f32 = 500.0;

/// Full preset height.
const FULL_PANEL_HEIGHT: f32 = 400.0;

/// Resize handle height (visual hitbox area; contains 1px centered line).
pub const RESIZE_HANDLE_HEIGHT: f32 = 4.0;

/// Tab bar row height in the panel.
pub const PANEL_TAB_HEIGHT: f32 = 32.0;

/// Font size for panel tab labels.
const PANEL_TAB_FONT_SIZE: f32 = 12.0;

/// Horizontal padding within each panel tab.
const PANEL_TAB_PADDING_H: f32 = 12.0;

/// Padding for the content area.
const PANEL_CONTENT_PADDING: f32 = 8.0;

/// Panel layout state.
///
/// Tracks height, collapsed state, active tab, and tab order.
/// Persistence (saving state across sessions) is deferred to AppLayout or later.
pub struct PanelState {
    /// Current panel height in pixels.
    pub height: f32,
    /// Whether panel is collapsed (not rendered).
    pub is_collapsed: bool,
    /// Preferred height to restore when expanding from collapsed.
    pub preferred_height: f32,
    /// Currently active panel tab.
    pub active_tab: PanelKind,
    /// Tab display order (reorderable by drag).
    pub tab_order: Vec<PanelKind>,
}

/// The panel manager view.
///
/// Renders a bottom panel with:
/// - A 4px resize handle at the top (with hover highlight)
/// - A tab bar row with active/inactive tab styling
/// - A content area growing to fill remaining space
///
/// When collapsed, renders a 0-height placeholder div.
///
/// # Example
///
/// ```ignore
/// let panel = PanelManagerView::new();
/// // In a parent view's render():
/// Div::new().child(panel.render(cx))
/// ```
pub struct PanelManagerView {
    state: PanelState,
    #[allow(dead_code)]
    focus_handle: Option<FocusHandle>,
}

impl PanelManagerView {
    /// Creates a new panel manager with default state.
    ///
    /// Default state: Output tab active, 200px height, all four tabs in order.
    pub fn new() -> Self {
        Self {
            state: PanelState {
                height: DEFAULT_PANEL_HEIGHT,
                is_collapsed: false,
                preferred_height: DEFAULT_PANEL_HEIGHT,
                active_tab: PanelKind::Output,
                tab_order: vec![
                    PanelKind::Output,
                    PanelKind::Problems,
                    PanelKind::Terminal,
                    PanelKind::Debug,
                ],
            },
            focus_handle: None,
        }
    }

    /// Returns the current visible height (0 if collapsed).
    pub fn height(&self) -> f32 {
        if self.state.is_collapsed {
            0.0
        } else {
            self.state.height
        }
    }

    /// Returns whether the panel is currently collapsed.
    pub fn is_collapsed(&self) -> bool {
        self.state.is_collapsed
    }

    /// Sets the panel height, clamped to [MIN_PANEL_HEIGHT, MAX_PANEL_HEIGHT].
    ///
    /// Also updates `preferred_height` so toggle_collapsed can restore to this height.
    pub fn set_height(&mut self, h: f32) {
        let clamped = h.max(MIN_PANEL_HEIGHT).min(MAX_PANEL_HEIGHT);
        self.state.height = clamped;
        self.state.preferred_height = clamped;
    }

    /// Toggles the panel between collapsed and its preferred height.
    pub fn toggle_collapsed(&mut self) {
        if self.state.is_collapsed {
            self.state.is_collapsed = false;
            self.state.height = self.state.preferred_height;
        } else {
            self.state.is_collapsed = true;
        }
    }

    /// Cycles through panel presets: Collapsed -> Half -> Full -> Collapsed.
    pub fn cycle_preset(&mut self) {
        let current = self.current_preset();
        let next = match current {
            PanelPreset::Collapsed => PanelPreset::Half,
            PanelPreset::Half => PanelPreset::Full,
            PanelPreset::Full => PanelPreset::Collapsed,
        };
        self.apply_preset(next);
    }

    /// Returns the current preset based on state.
    fn current_preset(&self) -> PanelPreset {
        if self.state.is_collapsed {
            PanelPreset::Collapsed
        } else if self.state.height <= DEFAULT_PANEL_HEIGHT {
            PanelPreset::Half
        } else {
            PanelPreset::Full
        }
    }

    /// Applies a preset to the panel state.
    fn apply_preset(&mut self, preset: PanelPreset) {
        match preset {
            PanelPreset::Collapsed => {
                self.state.is_collapsed = true;
            }
            PanelPreset::Half => {
                self.state.is_collapsed = false;
                self.state.height = DEFAULT_PANEL_HEIGHT;
                self.state.preferred_height = DEFAULT_PANEL_HEIGHT;
            }
            PanelPreset::Full => {
                self.state.is_collapsed = false;
                self.state.height = FULL_PANEL_HEIGHT;
                self.state.preferred_height = FULL_PANEL_HEIGHT;
            }
        }
    }

    /// Sets the currently active tab.
    pub fn set_active_tab(&mut self, kind: PanelKind) {
        self.state.active_tab = kind;
    }

    /// Reorders tabs by swapping positions at `from` and `to` indices.
    ///
    /// No-op if either index is out of bounds.
    pub fn reorder_tab(&mut self, from: usize, to: usize) {
        if from < self.state.tab_order.len() && to < self.state.tab_order.len() {
            self.state.tab_order.swap(from, to);
        }
    }

    /// Renders the resize handle at the top of the panel.
    ///
    /// Visual: 4px tall div with a centered 1px horizontal line.
    /// On hover, the entire area turns accent color.
    fn render_resize_handle(&self, cx: &mut ViewContext) -> Div {
        // Extract colors before mutable cx operations (borrow-safe pattern)
        let border_color = cx.theme().color(ColorToken::Border);
        let accent_color = cx.theme().color(ColorToken::Accent);

        Div::new()
            .w(pct(100.0))
            .h(px(RESIZE_HANDLE_HEIGHT))
            .flex_col()
            .justify_center()
            .hover_bg(accent_color)
            .child(
                Div::new()
                    .w(pct(100.0))
                    .h(px(1.0))
                    .bg(border_color),
            )
    }

    /// Renders the tab bar row with active/inactive tab styling.
    fn render_tab_bar(&self, cx: &mut ViewContext) -> Div {
        // Extract all colors before building children (borrow-safe pattern)
        let bg_secondary = cx.theme().color(ColorToken::BgSecondary);
        let border_color = cx.theme().color(ColorToken::Border);
        let bg_primary = cx.theme().color(ColorToken::BgPrimary);
        let bg_elevated = cx.theme().color(ColorToken::BgElevated);
        let fg_primary = cx.theme().color(ColorToken::FgPrimary);
        let fg_secondary = cx.theme().color(ColorToken::FgSecondary);
        let accent_color = cx.theme().color(ColorToken::Accent);

        let mut row = Div::new()
            .flex_row()
            .h(px(PANEL_TAB_HEIGHT))
            .w(pct(100.0))
            .shrink(0.0)
            .bg(bg_secondary)
            .border(1.0, border_color);

        for kind in &self.state.tab_order {
            let is_active = *kind == self.state.active_tab;

            let tab_bg = if is_active {
                bg_primary
            } else {
                Color::transparent()
            };
            let tab_text_color = if is_active { fg_primary } else { fg_secondary };
            let _bottom_border_color = if is_active {
                accent_color
            } else {
                Color::transparent()
            };

            let tab = Div::new()
                .flex_row()
                .align_center()
                .h(px(PANEL_TAB_HEIGHT))
                .px(PANEL_TAB_PADDING_H)
                .bg(tab_bg)
                .hover_bg(bg_elevated)
                .child(
                    TextElement::new(kind.label())
                        .size(PANEL_TAB_FONT_SIZE)
                        .color(tab_text_color),
                );

            row = row.child(tab);
        }

        row
    }

    /// Renders the content area for the currently active panel.
    fn render_content(&self, cx: &mut ViewContext) -> Div {
        // Extract colors before mutable cx operations (borrow-safe pattern)
        let bg_primary = cx.theme().color(ColorToken::BgPrimary);
        let fg_muted = cx.theme().color(ColorToken::FgMuted);

        let content_text = match self.state.active_tab {
            PanelKind::Output => "Output panel content",
            PanelKind::Problems => "No problems detected",
            PanelKind::Terminal => "Terminal ready",
            PanelKind::Debug => "Debug console",
        };

        Div::new()
            .flex_col()
            .grow(1.0)
            .p(PANEL_CONTENT_PADDING)
            .bg(bg_primary)
            .child(
                TextElement::new(content_text)
                    .size(13.0)
                    .color(fg_muted),
            )
    }
}

impl Default for PanelManagerView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for PanelManagerView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Collapsed: render a zero-height placeholder
        if self.state.is_collapsed {
            return Div::new().w(pct(100.0)).h(px(0.0)).into();
        }

        // Extract container background before mutable cx operations (borrow-safe pattern)
        let bg_secondary = cx.theme().color(ColorToken::BgSecondary);

        // Build main container
        let resize_handle = self.render_resize_handle(cx);
        let tab_bar = self.render_tab_bar(cx);
        let content = self.render_content(cx);

        Div::new()
            .flex_col()
            .w(pct(100.0))
            .h(px(self.state.height))
            .bg(bg_secondary)
            .child(resize_handle)
            .child(tab_bar)
            .child(content)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_manager_creation() {
        let panel = PanelManagerView::new();
        assert_eq!(panel.state.height, DEFAULT_PANEL_HEIGHT);
        assert!(!panel.state.is_collapsed);
        assert_eq!(panel.state.preferred_height, DEFAULT_PANEL_HEIGHT);
        assert_eq!(panel.state.active_tab, PanelKind::Output);
        assert_eq!(panel.state.tab_order.len(), 4);
        assert_eq!(panel.state.tab_order[0], PanelKind::Output);
        assert_eq!(panel.state.tab_order[1], PanelKind::Problems);
        assert_eq!(panel.state.tab_order[2], PanelKind::Terminal);
        assert_eq!(panel.state.tab_order[3], PanelKind::Debug);
        assert_eq!(panel.height(), DEFAULT_PANEL_HEIGHT);
        assert!(!panel.is_collapsed());
    }

    #[test]
    fn test_panel_preset_cycling() {
        let mut panel = PanelManagerView::new();

        // Start at Half (200px, not collapsed)
        assert_eq!(panel.current_preset(), PanelPreset::Half);

        // Half -> Full
        panel.cycle_preset();
        assert_eq!(panel.current_preset(), PanelPreset::Full);
        assert!(!panel.state.is_collapsed);
        assert_eq!(panel.state.height, FULL_PANEL_HEIGHT);

        // Full -> Collapsed
        panel.cycle_preset();
        assert_eq!(panel.current_preset(), PanelPreset::Collapsed);
        assert!(panel.state.is_collapsed);
        assert_eq!(panel.height(), 0.0);

        // Collapsed -> Half
        panel.cycle_preset();
        assert_eq!(panel.current_preset(), PanelPreset::Half);
        assert!(!panel.state.is_collapsed);
        assert_eq!(panel.state.height, DEFAULT_PANEL_HEIGHT);
    }

    #[test]
    fn test_panel_height_clamping() {
        let mut panel = PanelManagerView::new();

        // Below minimum: clamped to MIN
        panel.set_height(50.0);
        assert_eq!(panel.state.height, MIN_PANEL_HEIGHT);
        assert_eq!(panel.state.preferred_height, MIN_PANEL_HEIGHT);

        // Above maximum: clamped to MAX
        panel.set_height(999.0);
        assert_eq!(panel.state.height, MAX_PANEL_HEIGHT);
        assert_eq!(panel.state.preferred_height, MAX_PANEL_HEIGHT);

        // Within range: exact value preserved
        panel.set_height(300.0);
        assert_eq!(panel.state.height, 300.0);
        assert_eq!(panel.state.preferred_height, 300.0);

        // Exact min boundary
        panel.set_height(MIN_PANEL_HEIGHT);
        assert_eq!(panel.state.height, MIN_PANEL_HEIGHT);

        // Exact max boundary
        panel.set_height(MAX_PANEL_HEIGHT);
        assert_eq!(panel.state.height, MAX_PANEL_HEIGHT);
    }

    #[test]
    fn test_panel_tab_reorder() {
        let mut panel = PanelManagerView::new();

        // Initial order: [Output, Problems, Terminal, Debug]
        assert_eq!(panel.state.tab_order[0], PanelKind::Output);
        assert_eq!(panel.state.tab_order[1], PanelKind::Problems);

        // Swap first two tabs
        panel.reorder_tab(0, 1);
        assert_eq!(panel.state.tab_order[0], PanelKind::Problems);
        assert_eq!(panel.state.tab_order[1], PanelKind::Output);

        // Swap last two tabs
        panel.reorder_tab(2, 3);
        assert_eq!(panel.state.tab_order[2], PanelKind::Debug);
        assert_eq!(panel.state.tab_order[3], PanelKind::Terminal);

        // Out-of-bounds: no-op
        panel.reorder_tab(0, 99);
        // Order unchanged for valid indices
        assert_eq!(panel.state.tab_order[0], PanelKind::Problems);

        // Self-swap: no change
        panel.reorder_tab(0, 0);
        assert_eq!(panel.state.tab_order[0], PanelKind::Problems);
    }

    #[test]
    fn test_panel_toggle_collapsed() {
        let mut panel = PanelManagerView::new();
        panel.set_height(250.0);

        // Collapse
        panel.toggle_collapsed();
        assert!(panel.is_collapsed());
        assert_eq!(panel.height(), 0.0);

        // Expand restores preferred height
        panel.toggle_collapsed();
        assert!(!panel.is_collapsed());
        assert_eq!(panel.height(), 250.0);
    }

    #[test]
    fn test_panel_kind_labels() {
        assert_eq!(PanelKind::Output.label(), "Output");
        assert_eq!(PanelKind::Problems.label(), "Problems");
        assert_eq!(PanelKind::Terminal.label(), "Terminal");
        assert_eq!(PanelKind::Debug.label(), "Debug");
    }
}
