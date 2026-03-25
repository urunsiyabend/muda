/// Convert spacing scale value to pixels.
/// Scale: 0=0, 1=4, 2=8, 3=12, 4=16, 5=20, 6=24, 7=28, 8=32, etc.
/// Supports negative values: sp(-2) = -8.0
pub fn sp(scale: i32) -> f32 {
    (scale as f32) * 4.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacingToken {
    None,   // 0
    Xs,     // 4 (sp(1))
    Sm,     // 8 (sp(2))
    Md,     // 12 (sp(3))
    Lg,     // 16 (sp(4))
    Xl,     // 24 (sp(6))
    Xxl,    // 32 (sp(8))
}

impl SpacingToken {
    pub fn px(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Xs => 4.0,
            Self::Sm => 8.0,
            Self::Md => 12.0,
            Self::Lg => 16.0,
            Self::Xl => 24.0,
            Self::Xxl => 32.0,
        }
    }
}

impl From<SpacingToken> for f32 {
    fn from(token: SpacingToken) -> f32 {
        token.px()
    }
}
