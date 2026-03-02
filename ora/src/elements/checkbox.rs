use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::elements::text::{TextElement, TextState};
use crate::events::focus::{FocusHandle, FocusId};
use crate::events::mouse::HitboxId;
use crate::style::*;
use crate::theme::ColorToken;

// ---------------------------------------------------------------------------
// Sizing constants
// ---------------------------------------------------------------------------

/// Box size for Sm checkbox (14x14)
const CHECKBOX_SIZE_SM: f32 = 14.0;
/// Box size for Md checkbox (18x18)
const CHECKBOX_SIZE_MD: f32 = 18.0;
/// Box size for Lg checkbox (22x22)
const CHECKBOX_SIZE_LG: f32 = 22.0;

/// Label font size for Sm
const CHECKBOX_FONT_SM: f32 = 12.0;
/// Label font size for Md
const CHECKBOX_FONT_MD: f32 = 13.0;
/// Label font size for Lg
const CHECKBOX_FONT_LG: f32 = 14.0;

/// Gap between checkbox box and label text
const CHECKBOX_GAP: f32 = 8.0;

/// Border radius for checkbox box corners
const CHECKBOX_BORDER_RADIUS: f32 = 3.0;

// ---------------------------------------------------------------------------
// Toggle sizing constants
// ---------------------------------------------------------------------------

/// Track width for Sm toggle
const TOGGLE_WIDTH_SM: f32 = 28.0;
/// Track width for Md toggle
const TOGGLE_WIDTH_MD: f32 = 36.0;
/// Track height for Sm toggle
const TOGGLE_HEIGHT_SM: f32 = 14.0;
/// Track height for Md toggle
const TOGGLE_HEIGHT_MD: f32 = 18.0;
/// Inset from track edge to knob edge
const TOGGLE_KNOB_INSET: f32 = 2.0;

// ---------------------------------------------------------------------------
// CheckboxSize enum
// ---------------------------------------------------------------------------

/// Size tier for the Checkbox widget.
///
/// Note: After Plan 08-01 (WidgetSize) and Plan 08-03 both complete, Plan 08-08
/// will unify this with `WidgetSize`. For now it's a local enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckboxSize {
    /// Compact: 14px box, 12px label
    Sm,
    /// Standard: 18px box, 13px label
    #[default]
    Md,
    /// Prominent: 22px box, 14px label
    Lg,
}

impl CheckboxSize {
    /// Returns the box dimension (width = height) in pixels.
    fn box_size(&self) -> f32 {
        match self {
            Self::Sm => CHECKBOX_SIZE_SM,
            Self::Md => CHECKBOX_SIZE_MD,
            Self::Lg => CHECKBOX_SIZE_LG,
        }
    }

    /// Returns the label font size in pixels.
    fn font_size(&self) -> f32 {
        match self {
            Self::Sm => CHECKBOX_FONT_SM,
            Self::Md => CHECKBOX_FONT_MD,
            Self::Lg => CHECKBOX_FONT_LG,
        }
    }
}

// ---------------------------------------------------------------------------
// Checkbox element
// ---------------------------------------------------------------------------

/// Interactive checkbox element with checked/unchecked states and a text label.
///
/// Renders as a flex row: [box] [label text]
///
/// Visual states:
/// - Unchecked: 1px bordered square (Border color), transparent fill
/// - Checked:   filled square (Accent color)
/// - Disabled:  50% alpha on all colors
/// - Hover:     BgElevated background on container
/// - Focused:   2px focus ring around the checkbox box (Accent color)
pub struct Checkbox {
    label: String,
    checked: bool,
    disabled: bool,
    size: CheckboxSize,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
    focus_handle: Option<FocusHandle>,
    /// TextElement for the label — created in request_layout
    label_element: Option<TextElement>,
    /// TextElement for the checkmark character — created in request_layout
    check_element: Option<TextElement>,
}

/// Constructor function — creates an unchecked checkbox with a label.
pub fn checkbox(label: impl Into<String>) -> Checkbox {
    Checkbox::new(label.into())
}

impl Checkbox {
    /// Create a new checkbox with the given label (defaults to unchecked, Md size).
    pub fn new(label: String) -> Self {
        Self {
            label,
            checked: false,
            disabled: false,
            size: CheckboxSize::Md,
            on_change: None,
            focus_handle: None,
            label_element: None,
            check_element: None,
        }
    }

    /// Set the checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Enable or disable the checkbox.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the size tier.
    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    /// Attach a change handler called with the new checked state on toggle.
    pub fn on_change(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Make the checkbox keyboard-focusable.
    pub fn focusable(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }
}

/// State persisted through the rendering lifecycle for a single frame.
pub struct CheckboxState {
    /// Layout ID for the outer flex row container
    row_layout_id: LayoutId,
    /// Layout ID for the checkbox box (fixed square)
    box_layout_id: LayoutId,
    /// Layout ID + text state for the label
    label_layout_id: LayoutId,
    label_state: TextState,
    /// Layout ID + text state for the checkmark glyph (only relevant when checked)
    check_layout_id: LayoutId,
    check_state: TextState,
    /// Hitbox for the entire row (mouse interaction)
    hitbox_id: Option<HitboxId>,
    /// Focus registration
    focus_id: Option<FocusId>,
}

impl Element for Checkbox {
    type RequestLayoutState = CheckboxState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        let box_size = self.size.box_size();
        let font_size = self.size.font_size();

        // --- Outer flex row container ---
        let mut row_style = Style::default();
        row_style.display = Display::Flex;
        row_style.flex_direction = FlexDirection::Row;
        row_style.align_items = AlignItems::Center;
        row_style.gap = CHECKBOX_GAP;
        let row_layout_id = cx.request_layout(&row_style);

        // --- Fixed square for the checkbox box ---
        let mut box_style = Style::default();
        box_style.width = Length::Px(box_size);
        box_style.height = Length::Px(box_size);
        box_style.flex_shrink = 0.0; // don't shrink the box
        let box_layout_id = cx.request_layout(&box_style);
        cx.add_child(row_layout_id, box_layout_id);

        // --- Label text element ---
        let mut label_element = TextElement::new(self.label.clone())
            .size(font_size)
            .color(Color::white()); // colour updated in paint()
        let (label_layout_id, label_state) = label_element.request_layout(cx);
        cx.add_child(row_layout_id, label_layout_id);
        self.label_element = Some(label_element);

        // --- Checkmark text element (Unicode check) ---
        // We always measure & layout even when unchecked so IDs are stable.
        // It is only painted when checked == true.
        let mut check_element = TextElement::new("✓")
            .size(font_size)
            .color(Color::white()); // colour set during paint()
        let (check_layout_id, check_state) = check_element.request_layout(cx);
        // The checkmark is positioned absolutely inside the box at paint time;
        // it is NOT added as a child of row_layout_id here.
        self.check_element = Some(check_element);

        (
            row_layout_id,
            CheckboxState {
                row_layout_id,
                box_layout_id,
                label_layout_id,
                label_state,
                check_layout_id,
                check_state,
                hitbox_id: None,
                focus_id: None,
            },
        )
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        // Register hitbox for the whole row (clicking anywhere toggles)
        let row_bounds = cx.bounds(state.row_layout_id);
        let hitbox_id = cx.register_hitbox(row_bounds, true);
        state.hitbox_id = Some(hitbox_id);

        // Optionally register focus
        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
            state.focus_id = Some(focus_handle.id);
        }

        // Prepaint label text
        if let Some(label_el) = &mut self.label_element {
            label_el.prepaint(&mut state.label_state, cx);
        }

        // Prepaint checkmark text (needed so the buffer is consumed during paint)
        if let Some(check_el) = &mut self.check_element {
            check_el.prepaint(&mut state.check_state, cx);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let is_hovered = state.hitbox_id.map_or(false, |id| cx.is_hovered(id));
        let is_focused = state.focus_id.map_or(false, |id| cx.is_focused(id));

        // Determine alpha multiplier for disabled state
        let alpha = if self.disabled { 0.5 } else { 1.0 };

        // Extract all needed colors from theme up front (before mutable borrows)
        let hover_bg_color = {
            let mut c = cx.theme().color(ColorToken::BgElevated);
            c.a *= alpha;
            c
        };
        let box_fill_color = if self.checked {
            let mut c = cx.theme().color(ColorToken::Accent);
            c.a *= alpha;
            c
        } else {
            Color::transparent()
        };
        let box_border_color = {
            let mut c = cx.theme().color(ColorToken::Border);
            c.a *= alpha;
            c
        };
        let ring_color = {
            let mut c = cx.theme().color(ColorToken::Accent);
            c.a *= alpha;
            c
        };
        let label_color = if self.disabled {
            let mut c = cx.theme().color(ColorToken::FgMuted);
            c.a *= 0.5;
            c
        } else {
            cx.theme().color(ColorToken::FgPrimary)
        };

        // --- Paint hover background on row container ---
        if is_hovered && !self.disabled {
            let row_bounds = cx.bounds(state.row_layout_id);
            let mut hover_style = Style::default();
            hover_style.background = Background::Solid(hover_bg_color);
            hover_style.border_radius = Corners::all(CHECKBOX_BORDER_RADIUS);
            cx.paint_styled_rect(&hover_style, &row_bounds);
        }

        // --- Paint checkbox box ---
        let box_bounds = cx.bounds(state.box_layout_id);

        let mut box_style = Style::default();
        box_style.border_radius = Corners::all(CHECKBOX_BORDER_RADIUS);

        if self.checked {
            box_style.background = Background::Solid(box_fill_color);
        } else {
            box_style.background = Background::None;
            box_style.border.widths = Edges::all(1.0);
            box_style.border.color = box_border_color;
        }

        cx.paint_styled_rect(&box_style, &box_bounds);

        // --- Paint focus ring (2px Accent ring around box) ---
        if is_focused {
            let inset = -2.0; // expand 2px beyond the box
            let focus_rect = Rect {
                origin: Point::new(box_bounds.origin.x + inset, box_bounds.origin.y + inset),
                size: Size::new(
                    box_bounds.size.width - 2.0 * inset,
                    box_bounds.size.height - 2.0 * inset,
                ),
            };
            let mut ring_style = Style::default();
            ring_style.background = Background::None;
            ring_style.border.widths = Edges::all(2.0);
            ring_style.border.color = ring_color;
            ring_style.border_radius = Corners::all(CHECKBOX_BORDER_RADIUS + 2.0);
            cx.paint_styled_rect(&ring_style, &focus_rect);
        }

        // --- Paint checkmark glyph (always call paint to consume the buffer) ---
        if self.checked {
            if let Some(check_el) = &mut self.check_element {
                let mut check_color = Color::white();
                check_color.a *= alpha;
                check_el.set_color(check_color);
                check_el.paint(&mut state.check_state, cx);
            }
        } else {
            // Discard the buffer without painting (consume it so it gets dropped)
            if let Some(check_el) = &mut self.check_element {
                check_el.paint(&mut state.check_state, cx);
            }
        }

        // --- Paint label ---
        if let Some(label_el) = &mut self.label_element {
            label_el.set_color(label_color);
            label_el.paint(&mut state.label_state, cx);
        }
    }
}

// ---------------------------------------------------------------------------
// Toggle element
// ---------------------------------------------------------------------------

/// Interactive pill-shaped toggle switch element.
///
/// Visual states:
/// - Off: BgSecondary pill track, white knob at left
/// - On:  Accent pill track, white knob at right
/// - Disabled: 50% alpha on all colors
/// - Hover (off): BgElevated track
/// - Hover (on):  AccentHover track
pub struct Toggle {
    checked: bool,
    disabled: bool,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
    focus_handle: Option<FocusHandle>,
}

/// Constructor function — creates an unchecked toggle switch.
pub fn toggle() -> Toggle {
    Toggle::new()
}

impl Toggle {
    /// Create a new toggle (defaults to unchecked, not disabled).
    pub fn new() -> Self {
        Self {
            checked: false,
            disabled: false,
            on_change: None,
            focus_handle: None,
        }
    }

    /// Set the on/off state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Enable or disable the toggle.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Attach a change handler called with the new state on toggle.
    pub fn on_change(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Make the toggle keyboard-focusable.
    pub fn focusable(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }
}

impl Default for Toggle {
    fn default() -> Self {
        Self::new()
    }
}

/// State persisted through the rendering lifecycle for a single Toggle frame.
pub struct ToggleState {
    /// Layout ID for the pill track (fixed size)
    layout_id: LayoutId,
    /// Hitbox for mouse interaction
    hitbox_id: Option<HitboxId>,
    /// Focus registration
    focus_id: Option<FocusId>,
}

impl Element for Toggle {
    type RequestLayoutState = ToggleState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // Fixed size for the track pill
        let mut style = Style::default();
        style.width = Length::Px(TOGGLE_WIDTH_MD);
        style.height = Length::Px(TOGGLE_HEIGHT_MD);
        style.flex_shrink = 0.0;
        let layout_id = cx.request_layout(&style);

        (layout_id, ToggleState {
            layout_id,
            hitbox_id: None,
            focus_id: None,
        })
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);

        if let Some(focus_handle) = &self.focus_handle {
            cx.register_focusable(focus_handle.id);
            state.focus_id = Some(focus_handle.id);
        }
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        let is_hovered = state.hitbox_id.map_or(false, |id| cx.is_hovered(id));

        // Alpha for disabled state
        let alpha = if self.disabled { 0.5 } else { 1.0 };

        // Extract track color from theme before mutable borrows
        let track_color = if self.checked {
            if is_hovered && !self.disabled {
                let mut c = cx.theme().color(ColorToken::AccentHover);
                c.a *= alpha;
                c
            } else {
                let mut c = cx.theme().color(ColorToken::Accent);
                c.a *= alpha;
                c
            }
        } else if is_hovered && !self.disabled {
            let mut c = cx.theme().color(ColorToken::BgElevated);
            c.a *= alpha;
            c
        } else {
            let mut c = cx.theme().color(ColorToken::BgSecondary);
            c.a *= alpha;
            c
        };

        let track_radius = bounds.size.height / 2.0; // pill shape

        let mut track_style = Style::default();
        track_style.background = Background::Solid(track_color);
        track_style.border_radius = Corners::all(track_radius);
        cx.paint_styled_rect(&track_style, &bounds);

        // --- Paint knob ---
        let knob_size = TOGGLE_HEIGHT_MD - 2.0 * TOGGLE_KNOB_INSET;
        let knob_y = bounds.origin.y + TOGGLE_KNOB_INSET;
        let knob_x = if self.checked {
            // Knob on the right
            bounds.origin.x + TOGGLE_WIDTH_MD - knob_size - TOGGLE_KNOB_INSET
        } else {
            // Knob on the left
            bounds.origin.x + TOGGLE_KNOB_INSET
        };

        let knob_bounds = Rect {
            origin: Point::new(knob_x, knob_y),
            size: Size::new(knob_size, knob_size),
        };

        let mut knob_color = Color::white();
        knob_color.a *= alpha;

        let mut knob_style = Style::default();
        knob_style.background = Background::Solid(knob_color);
        knob_style.border_radius = Corners::all(knob_size / 2.0); // circular
        cx.paint_styled_rect(&knob_style, &knob_bounds);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_default() {
        let cb = checkbox("Accept terms");
        assert!(!cb.checked, "Checkbox should be unchecked by default");
    }

    #[test]
    fn test_checkbox_checked() {
        let cb = checkbox("Accept terms").checked(true);
        assert!(cb.checked, "Builder .checked(true) should set checked state");
    }

    #[test]
    fn test_toggle_default() {
        let t = toggle();
        assert!(!t.checked, "Toggle should be unchecked by default");
    }
}
