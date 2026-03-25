use crate::context::ViewContext;
use crate::elements::{Div, TextElement};
use crate::style::{px, Color};
use crate::theme::ColorToken;
use std::time::{Duration, Instant};

const TOAST_WIDTH: f32 = 320.0;
const TOAST_HEIGHT: f32 = 48.0;
const TOAST_PADDING_H: f32 = 16.0;
const TOAST_PADDING_V: f32 = 12.0;
const TOAST_BORDER_RADIUS: f32 = 6.0;
const TOAST_FONT_SIZE: f32 = 13.0;
const TOAST_MARGIN: f32 = 16.0;
/// Auto-dismiss duration for Info and Success toasts (4 seconds).
pub const AUTO_DISMISS_DURATION: Duration = Duration::from_secs(4);

/// Severity level for a toast notification.
///
/// Info and Success auto-dismiss after `AUTO_DISMISS_DURATION`.
/// Warning and Error persist until manually dismissed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastSeverity {
    /// Returns true if this severity should auto-dismiss.
    pub fn auto_dismiss(&self) -> bool {
        matches!(self, Self::Info | Self::Success)
    }

    /// Returns the `ColorToken` used for the left accent strip.
    pub fn color_token(&self) -> ColorToken {
        match self {
            Self::Info => ColorToken::Accent,
            Self::Success => ColorToken::Success,
            Self::Warning => ColorToken::Warning,
            Self::Error => ColorToken::Error,
        }
    }
}

/// A single toast notification with lifecycle tracking.
#[derive(Clone, Debug)]
pub struct ToastNotification {
    /// The notification message text.
    pub message: String,
    /// Severity level controls color and auto-dismiss behavior.
    pub severity: ToastSeverity,
    /// When the notification was created.
    pub created_at: Instant,
    /// When the notification should automatically dismiss (None = persist forever).
    pub dismiss_at: Option<Instant>,
    /// Whether the user manually dismissed this notification.
    pub dismissed: bool,
}

impl ToastNotification {
    /// Create a new notification. Auto-dismiss is set based on severity.
    pub fn new(message: impl Into<String>, severity: ToastSeverity) -> Self {
        let dismiss_at = if severity.auto_dismiss() {
            Some(Instant::now() + AUTO_DISMISS_DURATION)
        } else {
            None
        };
        Self {
            message: message.into(),
            severity,
            created_at: Instant::now(),
            dismiss_at,
            dismissed: false,
        }
    }

    /// Returns true if this notification should be removed from the stack.
    pub fn should_dismiss(&self) -> bool {
        if self.dismissed {
            return true;
        }
        if let Some(dismiss_at) = self.dismiss_at {
            Instant::now() >= dismiss_at
        } else {
            false
        }
    }

    /// Mark this notification as manually dismissed.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }
}

/// Toast container that manages multiple stacked notifications.
///
/// Renders in the bottom-right corner of the viewport (the parent Stack handles
/// absolute positioning). Multiple toasts stack vertically from bottom to top.
///
/// # Auto-dismiss
///
/// Call `tick()` from the event loop's `about_to_wait` hook to remove expired
/// notifications. No async timers needed.
///
/// # Example
///
/// ```rust,ignore
/// let mut toasts = Toast::new();
/// toasts.show("File saved", ToastSeverity::Success);
/// toasts.show("Connection error", ToastSeverity::Error);
/// // In event loop: toasts.tick();
/// ```
pub struct Toast {
    notifications: Vec<ToastNotification>,
}

impl Toast {
    /// Create an empty toast container.
    pub fn new() -> Self {
        Self { notifications: Vec::new() }
    }

    /// Add a new notification.
    pub fn show(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.notifications.push(ToastNotification::new(message, severity));
    }

    /// Remove all expired or manually dismissed notifications.
    /// Call from the event loop's `about_to_wait` hook.
    pub fn tick(&mut self) {
        self.notifications.retain(|t| !t.should_dismiss());
    }

    /// Dismiss the most recent persistent (non-auto-dismiss) notification.
    /// Used when the user clicks the dismiss button.
    pub fn dismiss_latest(&mut self) {
        if let Some(toast) = self.notifications.iter_mut().rev()
            .find(|t| !t.dismissed && t.dismiss_at.is_none())
        {
            toast.dismiss();
        }
    }

    /// Dismiss all currently active notifications.
    pub fn dismiss_all(&mut self) {
        for t in self.notifications.iter_mut() {
            t.dismissed = true;
        }
    }

    /// Returns true if there are any active (non-dismissed) notifications.
    pub fn has_notifications(&self) -> bool {
        self.notifications.iter().any(|t| !t.dismissed)
    }

    /// Render all active notifications in a bottom-right aligned column.
    ///
    /// Returns a `Div` that the parent should position at the bottom-right of
    /// the viewport using a Stack with appropriate padding/offset.
    pub fn render(&self, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();

        // Extract all colors before any mutable calls (borrow checker pattern)
        let bg_elevated = theme.color(ColorToken::BgElevated);
        let border_color = theme.color(ColorToken::Border);
        let fg_primary = theme.color(ColorToken::FgPrimary);
        let fg_muted = theme.color(ColorToken::FgMuted);
        let bg_secondary = theme.color(ColorToken::BgSecondary);

        // Pre-compute severity colors for each notification
        let severity_colors: Vec<Color> = self.notifications.iter()
            .map(|n| theme.color(n.severity.color_token()))
            .collect();

        // Outer container: flex column, aligned to end (right side)
        let mut container = Div::new()
            .flex_col()
            .align_end()
            .p(TOAST_MARGIN);

        for (i, notification) in self.notifications.iter().enumerate() {
            if notification.dismissed {
                continue;
            }

            let severity_color = severity_colors[i];

            // Each toast: flex row with left accent strip + message + optional dismiss button
            let toast_div = Div::new()
                .flex_row()
                .align_center()
                .w(px(TOAST_WIDTH))
                .py(TOAST_PADDING_V)
                .px(TOAST_PADDING_H)
                .mb(8.0)
                .bg(bg_elevated)
                .border(1.0, border_color)
                .border_radius(TOAST_BORDER_RADIUS)
                .shadow(0.0, 4.0, 16.0, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.3))
                // Left accent strip colored by severity
                .child(
                    Div::new()
                        .w(px(3.0))
                        .h(px(TOAST_HEIGHT - TOAST_PADDING_V * 2.0))
                        .bg(severity_color)
                        .border_radius(2.0)
                        .mr(12.0),
                )
                // Message text (grows to fill available space)
                .child(
                    TextElement::new(&notification.message)
                        .size(TOAST_FONT_SIZE)
                        .color(fg_primary)
                        .grow(1.0),
                )
                // Dismiss button (only for persistent toasts with no auto-dismiss)
                .child(if notification.dismiss_at.is_none() {
                    Div::new()
                        .flex_row()
                        .align_center()
                        .justify_center()
                        .w(px(20.0))
                        .h(px(20.0))
                        .ml(8.0)
                        .hover_bg(bg_secondary)
                        .border_radius(3.0)
                        .child(
                            TextElement::new("x")
                                .size(12.0)
                                .color(fg_muted),
                        )
                } else {
                    Div::new().w(px(0.0))
                });

            container = container.child(toast_div);
        }

        container
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_toast_auto_dismiss() {
        // Info and Success should have dismiss_at set
        let info = ToastNotification::new("hello", ToastSeverity::Info);
        assert!(info.dismiss_at.is_some(), "Info toast should have dismiss_at");
        assert!(!info.dismissed);
        assert!(!info.should_dismiss()); // Not expired yet

        let success = ToastNotification::new("saved", ToastSeverity::Success);
        assert!(success.dismiss_at.is_some(), "Success toast should have dismiss_at");
    }

    #[test]
    fn test_toast_persist() {
        // Warning and Error should NOT have dismiss_at set
        let warning = ToastNotification::new("caution", ToastSeverity::Warning);
        assert!(warning.dismiss_at.is_none(), "Warning toast should not auto-dismiss");
        assert!(!warning.should_dismiss());

        let error = ToastNotification::new("failed", ToastSeverity::Error);
        assert!(error.dismiss_at.is_none(), "Error toast should not auto-dismiss");
        assert!(!error.should_dismiss());
    }

    #[test]
    fn test_toast_tick() {
        let mut toast = Toast::new();
        toast.show("info message", ToastSeverity::Info);
        toast.show("error message", ToastSeverity::Error);

        // Both visible initially
        assert_eq!(toast.notifications.len(), 2);
        assert!(toast.has_notifications());

        // Manually mark the info toast as expired by adjusting dismiss_at
        if let Some(n) = toast.notifications.get_mut(0) {
            // Set dismiss_at in the past
            n.dismiss_at = Some(Instant::now() - Duration::from_secs(1));
        }

        // Tick removes expired toasts
        toast.tick();

        // Only the error (persistent) toast should remain
        assert_eq!(toast.notifications.len(), 1);
        assert_eq!(toast.notifications[0].severity, ToastSeverity::Error);
    }

    #[test]
    fn test_toast_severity_colors() {
        // Verify severity maps to expected ColorToken
        assert_eq!(ToastSeverity::Info.color_token(), ColorToken::Accent);
        assert_eq!(ToastSeverity::Success.color_token(), ColorToken::Success);
        assert_eq!(ToastSeverity::Warning.color_token(), ColorToken::Warning);
        assert_eq!(ToastSeverity::Error.color_token(), ColorToken::Error);
    }

    #[test]
    fn test_toast_dismiss_latest() {
        let mut toast = Toast::new();
        toast.show("first error", ToastSeverity::Error);
        toast.show("second error", ToastSeverity::Error);

        // dismiss_latest targets the most recent persistent toast
        toast.dismiss_latest();

        // Second (index 1) should be dismissed, first (index 0) still active
        assert!(!toast.notifications[0].dismissed);
        assert!(toast.notifications[1].dismissed);
    }

    #[test]
    fn test_toast_auto_dismiss_flag() {
        assert!(ToastSeverity::Info.auto_dismiss());
        assert!(ToastSeverity::Success.auto_dismiss());
        assert!(!ToastSeverity::Warning.auto_dismiss());
        assert!(!ToastSeverity::Error.auto_dismiss());
    }
}
