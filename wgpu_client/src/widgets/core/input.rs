//! Text input widget with cursor and selection support.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock,
    LayoutConstraints, CornerRadii,
    ComponentSize, Radius, Space,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent, WidgetValue,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Input field configuration.
#[derive(Clone, Debug)]
pub struct InputProps {
    /// Current text value
    pub value: String,
    /// Placeholder text when empty
    pub placeholder: String,
    /// Size preset
    pub size: ComponentSize,
    /// Whether input is disabled
    pub disabled: bool,
    /// Whether input has an error
    pub error: bool,
    /// Maximum character length (0 = unlimited)
    pub max_length: usize,
}

impl Default for InputProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            size: ComponentSize::Md,
            disabled: false,
            error: false,
            max_length: 0,
        }
    }
}

impl InputProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

/// A text input widget.
pub struct Input {
    id: WidgetId,
    props: InputProps,
    bounds: Bounds,
    interaction: InteractionTracker,
    palette: ColorPalette,

    // Cursor state
    cursor_pos: usize,
    selection_start: Option<usize>,

    // Cursor blink
    cursor_visible: bool,
    cursor_blink_timer: f32,
}

impl Input {
    const CURSOR_BLINK_RATE: f32 = 0.5; // seconds

    pub fn new(props: InputProps) -> Self {
        let mut input = Self {
            id: WidgetId::new(),
            props: props.clone(),
            bounds: Bounds::default(),
            interaction: InteractionTracker::new(),
            palette: ColorPalette::dark(),
            cursor_pos: 0,
            selection_start: None,
            cursor_visible: true,
            cursor_blink_timer: 0.0,
        };

        input.interaction.set_disabled(props.disabled);
        input.interaction.set_error(props.error);
        input
    }

    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        Self::new(InputProps::new().with_placeholder(placeholder))
    }

    /// Get the current value.
    pub fn value(&self) -> &str {
        &self.props.value
    }

    /// Set the value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.props.value = value.into();
        self.cursor_pos = self.cursor_pos.min(self.props.value.len());
    }

    /// Set error state.
    pub fn set_error(&mut self, error: bool) {
        self.props.error = error;
        self.interaction.set_error(error);
    }

    /// Reset cursor blink timer (called on user activity).
    fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }

    /// Insert text at cursor position.
    fn insert_text(&mut self, text: &str) {
        // Delete selection first if exists
        self.delete_selection();

        // Check max length
        if self.props.max_length > 0 &&
           self.props.value.len() + text.len() > self.props.max_length {
            return;
        }

        // Insert at cursor
        self.props.value.insert_str(self.cursor_pos, text);
        self.cursor_pos += text.len();
        self.reset_cursor_blink();
    }

    /// Delete character before cursor (backspace).
    fn delete_backward(&mut self) {
        if self.delete_selection() {
            return;
        }

        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.props.value.remove(self.cursor_pos);
            self.reset_cursor_blink();
        }
    }

    /// Delete character after cursor (delete).
    fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }

        if self.cursor_pos < self.props.value.len() {
            self.props.value.remove(self.cursor_pos);
            self.reset_cursor_blink();
        }
    }

    /// Delete selected text if there is a selection.
    /// Returns true if there was a selection to delete.
    fn delete_selection(&mut self) -> bool {
        if let Some(start) = self.selection_start {
            let (from, to) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };

            self.props.value.drain(from..to);
            self.cursor_pos = from;
            self.selection_start = None;
            self.reset_cursor_blink();
            true
        } else {
            false
        }
    }

    /// Move cursor left.
    fn move_left(&mut self, select: bool) {
        if self.cursor_pos > 0 {
            if select && self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            } else if !select {
                self.selection_start = None;
            }
            self.cursor_pos -= 1;
            self.reset_cursor_blink();
        }
    }

    /// Move cursor right.
    fn move_right(&mut self, select: bool) {
        if self.cursor_pos < self.props.value.len() {
            if select && self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            } else if !select {
                self.selection_start = None;
            }
            self.cursor_pos += 1;
            self.reset_cursor_blink();
        }
    }

    /// Move cursor to start.
    fn move_home(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_pos);
        } else if !select {
            self.selection_start = None;
        }
        self.cursor_pos = 0;
        self.reset_cursor_blink();
    }

    /// Move cursor to end.
    fn move_end(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_pos);
        } else if !select {
            self.selection_start = None;
        }
        self.cursor_pos = self.props.value.len();
        self.reset_cursor_blink();
    }

    /// Select all text.
    fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_pos = self.props.value.len();
    }
}

impl Widget for Input {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, dt: f32) -> bool {
        if !self.interaction.is_focused() {
            return false;
        }

        // Update cursor blink
        self.cursor_blink_timer += dt;
        if self.cursor_blink_timer >= Self::CURSOR_BLINK_RATE {
            self.cursor_blink_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
            return true;
        }
        false
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let height = self.props.size.height();
        let width = constraints.max_width.min(300.0).max(constraints.min_width);
        (width, height)
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn build(&mut self) -> WidgetOutput {
        let mut output = WidgetOutput::new();
        let state = self.interaction.state();

        // Background
        let bg_color = if state.is_disabled() {
            self.palette.bg_tertiary
        } else {
            self.palette.bg_secondary
        };

        let border_color = if state.has_error() {
            self.palette.error
        } else if state.is_focused() {
            self.palette.focus
        } else {
            self.palette.border_default
        };

        let radius = Radius::Sm.px();
        let rect = StyledRect::new(self.bounds)
            .with_fill(bg_color)
            .with_border(1.0, border_color)
            .with_corner_radii(CornerRadii::all(radius));

        output.add_rect(rect);

        // Text content area
        let padding = Space::Sm.px();
        let text_bounds = Bounds {
            x: self.bounds.x + padding,
            y: self.bounds.y,
            width: self.bounds.width - padding * 2.0,
            height: self.bounds.height,
        };

        // Show placeholder or value
        let (display_text, text_color) = if self.props.value.is_empty() && !state.is_focused() {
            (&self.props.placeholder, self.palette.fg_muted)
        } else {
            (&self.props.value, self.palette.fg_primary)
        };

        let text = TextBlock::new(display_text, text_bounds)
            .with_color(if state.is_disabled() { self.palette.fg_muted } else { text_color })
            .with_font_size(self.props.size.font_size());

        output.add_text(text);

        // Cursor (only when focused and visible)
        if state.is_focused() && self.cursor_visible {
            let char_width = self.props.size.font_size() * 0.6;
            let cursor_x = self.bounds.x + padding + self.cursor_pos as f32 * char_width;
            let cursor_height = self.props.size.font_size() * 1.2;
            let cursor_y = self.bounds.y + (self.bounds.height - cursor_height) / 2.0;

            let cursor_rect = StyledRect::new(Bounds {
                x: cursor_x,
                y: cursor_y,
                width: 2.0,
                height: cursor_height,
            }).with_fill(self.palette.fg_primary);

            output.add_rect(cursor_rect);
        }

        // Selection highlight
        if let Some(start) = self.selection_start {
            if start != self.cursor_pos {
                let char_width = self.props.size.font_size() * 0.6;
                let (from, to) = if start < self.cursor_pos {
                    (start, self.cursor_pos)
                } else {
                    (self.cursor_pos, start)
                };

                let sel_x = self.bounds.x + padding + from as f32 * char_width;
                let sel_width = (to - from) as f32 * char_width;
                let sel_height = self.props.size.font_size() * 1.2;
                let sel_y = self.bounds.y + (self.bounds.height - sel_height) / 2.0;

                let sel_rect = StyledRect::new(Bounds {
                    x: sel_x,
                    y: sel_y,
                    width: sel_width,
                    height: sel_height,
                }).with_fill(self.palette.selection);

                // Insert selection before cursor
                output.rects.insert(output.rects.len() - 1, sel_rect);
            }
        }

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        if self.props.disabled {
            return false;
        }

        match event {
            PointerEvent::Enter => {
                self.interaction.on_pointer_enter();
                true
            }
            PointerEvent::Leave => {
                self.interaction.on_pointer_leave();
                true
            }
            PointerEvent::Click { x, .. } => {
                // Position cursor based on click position
                let padding = Space::Sm.px();
                let char_width = self.props.size.font_size() * 0.6;
                let relative_x = x - self.bounds.x - padding;
                let char_pos = (relative_x / char_width).round() as usize;
                self.cursor_pos = char_pos.min(self.props.value.len());
                self.selection_start = None;
                self.reset_cursor_blink();
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, event: KeyEvent) -> bool {
        if self.props.disabled || !event.pressed {
            return false;
        }

        let shift = event.modifiers.shift;
        let ctrl = event.modifiers.ctrl;

        match event.key {
            KeyCode::ArrowLeft => {
                self.move_left(shift);
                true
            }
            KeyCode::ArrowRight => {
                self.move_right(shift);
                true
            }
            KeyCode::Home => {
                self.move_home(shift);
                true
            }
            KeyCode::End => {
                self.move_end(shift);
                true
            }
            KeyCode::Backspace => {
                self.delete_backward();
                true
            }
            KeyCode::Delete => {
                self.delete_forward();
                true
            }
            KeyCode::A if ctrl => {
                self.select_all();
                true
            }
            _ => {
                // Handle text input
                if let Some(text) = &event.text {
                    if !text.is_empty() && text.chars().all(|c| !c.is_control()) {
                        self.insert_text(text);
                        return true;
                    }
                }
                false
            }
        }
    }

    fn on_focus(&mut self, gained: bool) {
        if gained {
            self.interaction.on_focus();
            self.reset_cursor_blink();
        } else {
            self.interaction.on_blur();
            self.selection_start = None;
        }
    }

    fn can_focus(&self) -> bool {
        !self.props.disabled
    }
}
