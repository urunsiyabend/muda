//! Component sizing tokens.
//!
//! Defines standard sizes for interactive components like buttons, inputs, etc.
//! These ensure consistent touch targets and visual hierarchy.

/// Standard component sizes.
///
/// Components can support multiple sizes to fit different contexts:
/// - Sm: Compact interfaces, toolbars, nested controls
/// - Md: Default size for most contexts
/// - Lg: Primary actions, touch-friendly interfaces
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComponentSize {
    /// Small - 24px height, compact padding
    Sm,
    /// Medium (default) - 32px height, balanced padding
    #[default]
    Md,
    /// Large - 40px height, generous padding
    Lg,
}

impl ComponentSize {
    /// Get the standard height for this size.
    pub const fn height(self) -> f32 {
        match self {
            ComponentSize::Sm => 24.0,
            ComponentSize::Md => 32.0,
            ComponentSize::Lg => 40.0,
        }
    }

    /// Get horizontal padding for this size.
    pub const fn padding_x(self) -> f32 {
        match self {
            ComponentSize::Sm => 8.0,
            ComponentSize::Md => 12.0,
            ComponentSize::Lg => 16.0,
        }
    }

    /// Get vertical padding for this size.
    pub const fn padding_y(self) -> f32 {
        match self {
            ComponentSize::Sm => 4.0,
            ComponentSize::Md => 6.0,
            ComponentSize::Lg => 8.0,
        }
    }

    /// Get icon size for this component size.
    pub const fn icon_size(self) -> f32 {
        match self {
            ComponentSize::Sm => 14.0,
            ComponentSize::Md => 16.0,
            ComponentSize::Lg => 20.0,
        }
    }

    /// Get font size for this component size.
    pub const fn font_size(self) -> f32 {
        match self {
            ComponentSize::Sm => 12.0,
            ComponentSize::Md => 14.0,
            ComponentSize::Lg => 16.0,
        }
    }

    /// Get gap between icon and text.
    pub const fn icon_gap(self) -> f32 {
        match self {
            ComponentSize::Sm => 4.0,
            ComponentSize::Md => 6.0,
            ComponentSize::Lg => 8.0,
        }
    }

    /// Get minimum touch target size (for accessibility).
    pub const fn min_touch_target(self) -> f32 {
        // Minimum 44px for touch targets per WCAG guidelines
        match self {
            ComponentSize::Sm => 24.0, // Acceptable for mouse-only interfaces
            ComponentSize::Md => 32.0,
            ComponentSize::Lg => 44.0, // Full touch target
        }
    }
}
