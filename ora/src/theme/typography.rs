#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontFamily {
    #[default]
    Ui,     // System sans-serif for UI text
    Code,   // Monospace for code/technical content
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextSize {
    // Semantic names (preferred)
    Small,      // 12px
    #[default]
    Body,       // 14px (base)
    Code,       // 14px (monospace rhythm)
    Heading,    // 18px
    Title,      // 24px

    // Scale aliases (Tailwind-style)
    Xs,         // = Small (12px)
    Sm,         // = Body (14px)
    Md,         // = Heading (18px)
    Lg,         // = Title (24px)
}

impl TextSize {
    /// Font size in pixels
    pub fn font_size(&self) -> f32 {
        match self {
            Self::Small | Self::Xs => 12.0,
            Self::Body | Self::Sm | Self::Code => 14.0,
            Self::Heading | Self::Md => 18.0,
            Self::Title | Self::Lg => 24.0,
        }
    }

    /// Line height in pixels (bundled with font size)
    pub fn line_height(&self) -> f32 {
        match self {
            Self::Small | Self::Xs => 18.0,         // 1.5 ratio
            Self::Body | Self::Sm => 21.0,          // 1.5 ratio
            Self::Code => 19.6,                     // 1.4 ratio (tighter for monospace)
            Self::Heading | Self::Md => 22.5,       // 1.25 ratio
            Self::Title | Self::Lg => 30.0,         // 1.25 ratio
        }
    }

    /// Suggested font family for this text size
    pub fn suggested_font(&self) -> FontFamily {
        match self {
            Self::Code => FontFamily::Code,
            _ => FontFamily::Ui,
        }
    }
}
