//! Toast notification widget.
//!
//! Non-blocking notifications that appear temporarily to provide
//! feedback about an operation.

use std::time::{Duration, Instant};

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    InteractionState, InteractionTracker,
    StyledRect, TextBlock, TextAlign,
    LayoutConstraints, CornerRadii, Radius, Elevation,
    Tween, Easing,
    tokens::ColorPalette,
};
use crate::widgets::{
    Widget, WidgetId, WidgetOutput, WidgetEvent,
    PointerEvent, PointerButton, KeyEvent, KeyCode,
};

/// Toast notification types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastKind {
    /// Informational message
    #[default]
    Info,
    /// Success message
    Success,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

/// A single toast notification.
pub struct Toast {
    id: WidgetId,
    kind: ToastKind,
    message: String,
    bounds: Bounds,
    palette: ColorPalette,
    interaction: InteractionTracker,

    // Lifecycle
    created_at: Instant,
    duration: Duration,
    dismissed: bool,

    // Animation
    opacity_tween: Tween<f32>,
    slide_tween: Tween<f32>,
}

impl Toast {
    const WIDTH: f32 = 320.0;
    const MIN_HEIGHT: f32 = 48.0;
    const PADDING: f32 = 12.0;
    const DEFAULT_DURATION: Duration = Duration::from_secs(4);
    const FADE_DURATION: Duration = Duration::from_millis(200);

    pub fn new(kind: ToastKind, message: impl Into<String>) -> Self {
        let mut toast = Self {
            id: WidgetId::new(),
            kind,
            message: message.into(),
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            interaction: InteractionTracker::new(),
            created_at: Instant::now(),
            duration: Self::DEFAULT_DURATION,
            dismissed: false,
            opacity_tween: Tween::new(0.0, 1.0, Self::FADE_DURATION, Easing::EaseOut),
            slide_tween: Tween::new(20.0, 0.0, Self::FADE_DURATION, Easing::EaseOut),
        };

        toast.opacity_tween.start();
        toast.slide_tween.start();
        toast
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(ToastKind::Info, message)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(ToastKind::Success, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(ToastKind::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(ToastKind::Error, message)
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Dismiss the toast.
    pub fn dismiss(&mut self) {
        if !self.dismissed {
            self.dismissed = true;
            // Start fade out
            self.opacity_tween.retarget(0.0);
            self.opacity_tween.start();
            self.slide_tween.retarget(-20.0);
            self.slide_tween.start();
        }
    }

    /// Check if the toast should be removed.
    pub fn should_remove(&self) -> bool {
        self.dismissed && self.opacity_tween.is_complete()
    }

    /// Check if the toast has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Get the accent color for this toast kind.
    fn accent_color(&self) -> Color {
        match self.kind {
            ToastKind::Info => self.palette.info,
            ToastKind::Success => self.palette.success,
            ToastKind::Warning => self.palette.warning,
            ToastKind::Error => self.palette.error,
        }
    }
}

impl Widget for Toast {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn interaction_state(&self) -> InteractionState {
        self.interaction.state()
    }

    fn update(&mut self, _dt: f32) -> bool {
        // Check for auto-dismiss
        if !self.dismissed && self.is_expired() {
            self.dismiss();
        }

        let opacity_changed = self.opacity_tween.update();
        let slide_changed = self.slide_tween.update();
        opacity_changed || slide_changed
    }

    fn layout(&mut self, constraints: LayoutConstraints) -> (f32, f32) {
        let width = Self::WIDTH.min(constraints.max_width);

        // Calculate height based on message length
        let char_width = 8.0;
        let chars_per_line = ((width - Self::PADDING * 2.0) / char_width) as usize;
        let lines = (self.message.len() / chars_per_line).max(1);
        let text_height = lines as f32 * 18.0;
        let height = (Self::MIN_HEIGHT).max(text_height + Self::PADDING * 2.0);

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

        let opacity = *self.opacity_tween.value();
        let slide_offset = *self.slide_tween.value();

        if opacity <= 0.0 {
            return output;
        }

        // Apply slide offset
        let adjusted_bounds = Bounds {
            x: self.bounds.x,
            y: self.bounds.y + slide_offset,
            width: self.bounds.width,
            height: self.bounds.height,
        };

        // Background with accent stripe
        let bg_color = self.palette.bg_elevated.with_alpha(opacity);
        let bg = StyledRect::new(adjusted_bounds)
            .with_fill(bg_color)
            .with_border(1.0, self.palette.border_default.with_alpha(opacity))
            .with_corner_radii(CornerRadii::all(Radius::Md.px()))
            .with_elevation(Elevation::High);
        output.add_rect(bg);

        // Accent stripe on left
        let accent = StyledRect::new(Bounds {
            x: adjusted_bounds.x,
            y: adjusted_bounds.y,
            width: 4.0,
            height: adjusted_bounds.height,
        })
        .with_fill(self.accent_color().with_alpha(opacity))
        .with_corner_radii(CornerRadii::left(Radius::Md.px()));
        output.add_rect(accent);

        // Icon area (simplified as colored dot)
        let icon_color = self.accent_color().with_alpha(opacity);
        let icon = StyledRect::new(Bounds {
            x: adjusted_bounds.x + Self::PADDING,
            y: adjusted_bounds.y + (adjusted_bounds.height - 8.0) / 2.0,
            width: 8.0,
            height: 8.0,
        })
        .with_fill(icon_color)
        .with_corner_radii(CornerRadii::all(4.0));
        output.add_rect(icon);

        // Message
        let message_bounds = Bounds {
            x: adjusted_bounds.x + Self::PADDING + 16.0,
            y: adjusted_bounds.y,
            width: adjusted_bounds.width - Self::PADDING * 2.0 - 32.0,
            height: adjusted_bounds.height,
        };

        let text = TextBlock::new(&self.message, message_bounds)
            .with_color(self.palette.fg_primary.with_alpha(opacity))
            .with_font_size(13.0);
        output.add_text(text);

        // Close button (X)
        let close_x = adjusted_bounds.x + adjusted_bounds.width - Self::PADDING - 16.0;
        let close_y = adjusted_bounds.y + (adjusted_bounds.height - 16.0) / 2.0;

        if self.interaction.is_hovered() {
            let close_bg = StyledRect::new(Bounds {
                x: close_x,
                y: close_y,
                width: 16.0,
                height: 16.0,
            })
            .with_fill(self.palette.interactive_hover.with_alpha(opacity))
            .with_corner_radii(CornerRadii::all(8.0));
            output.add_rect(close_bg);
        }

        let close_mark = StyledRect::new(Bounds {
            x: close_x + 5.0,
            y: close_y + 7.0,
            width: 6.0,
            height: 2.0,
        }).with_fill(self.palette.fg_muted.with_alpha(opacity));
        output.add_rect(close_mark);

        output
    }

    fn on_pointer(&mut self, event: PointerEvent) -> bool {
        match event {
            PointerEvent::Enter => {
                self.interaction.on_pointer_enter();
                true
            }
            PointerEvent::Leave => {
                self.interaction.on_pointer_leave();
                true
            }
            PointerEvent::Click { button: PointerButton::Left, x, .. } => {
                // Check if click is on close button
                let close_x = self.bounds.x + self.bounds.width - Self::PADDING - 16.0;
                if x >= close_x {
                    self.dismiss();
                }
                true
            }
            _ => false,
        }
    }

    fn on_key(&mut self, _event: KeyEvent) -> bool {
        false
    }

    fn on_focus(&mut self, _gained: bool) {}

    fn can_focus(&self) -> bool {
        false
    }
}

/// Manages multiple toast notifications.
pub struct ToastManager {
    toasts: Vec<Toast>,
    max_visible: usize,
    spacing: f32,
    margin: f32,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            max_visible: 5,
            spacing: 8.0,
            margin: 16.0,
        }
    }

    /// Add a new toast notification.
    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);

        // Dismiss oldest if we have too many
        while self.toasts.len() > self.max_visible {
            if let Some(toast) = self.toasts.first_mut() {
                toast.dismiss();
            }
            break;
        }
    }

    /// Add an info toast.
    pub fn info(&mut self, message: impl Into<String>) {
        self.push(Toast::info(message));
    }

    /// Add a success toast.
    pub fn success(&mut self, message: impl Into<String>) {
        self.push(Toast::success(message));
    }

    /// Add a warning toast.
    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Toast::warning(message));
    }

    /// Add an error toast.
    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Toast::error(message));
    }

    /// Update all toasts.
    pub fn update(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;

        for toast in &mut self.toasts {
            if toast.update(dt) {
                needs_redraw = true;
            }
        }

        // Remove dismissed toasts
        self.toasts.retain(|t| !t.should_remove());

        needs_redraw
    }

    /// Layout toasts from bottom-right of the screen.
    pub fn layout(&mut self, screen_width: f32, screen_height: f32) {
        let x = screen_width - Toast::WIDTH - self.margin;
        let mut y = screen_height - self.margin;

        for toast in self.toasts.iter_mut().rev() {
            let constraints = LayoutConstraints::loose(Toast::WIDTH, 200.0);
            let (_, height) = toast.layout(constraints);

            y -= height;
            toast.set_bounds(Bounds {
                x,
                y,
                width: Toast::WIDTH,
                height,
            });

            y -= self.spacing;
        }
    }

    /// Build render output for all toasts.
    pub fn build(&mut self) -> WidgetOutput {
        let mut output = WidgetOutput::new();

        for toast in &mut self.toasts {
            output.merge(toast.build());
        }

        output
    }

    /// Handle pointer event. Returns true if any toast handled it.
    pub fn on_pointer(&mut self, x: f32, y: f32, event: PointerEvent) -> bool {
        for toast in &mut self.toasts {
            if toast.hit_test(x, y) {
                if toast.on_pointer(event.clone()) {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}
