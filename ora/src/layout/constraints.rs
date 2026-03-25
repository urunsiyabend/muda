/// Available space offered by parent during layout negotiation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AvailableSpace {
    /// Definite size constraint
    Definite(f32),
    /// Size to minimum content
    #[default]
    MinContent,
    /// Size to maximum content
    MaxContent,
}

/// Input to layout computation - what the parent provides
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutInput {
    pub available_width: AvailableSpace,
    pub available_height: AvailableSpace,
}

impl LayoutInput {
    pub fn new(available_width: AvailableSpace, available_height: AvailableSpace) -> Self {
        Self {
            available_width,
            available_height,
        }
    }

    pub fn definite(width: f32, height: f32) -> Self {
        Self {
            available_width: AvailableSpace::Definite(width),
            available_height: AvailableSpace::Definite(height),
        }
    }
}

/// Output of layout computation - computed bounds for an element
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutOutput {
    pub bounds: crate::style::units::Rect,
}

impl LayoutOutput {
    pub fn new(bounds: crate::style::units::Rect) -> Self {
        Self { bounds }
    }

    pub fn zero() -> Self {
        Self {
            bounds: crate::style::units::Rect::zero(),
        }
    }
}
