pub mod units;

pub use units::{Corners, Edges, Length, Point, Rect, Size, px, pct};

/// RGBA color with floating-point components (0.0..1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn transparent() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }

    pub fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::transparent()
    }
}

/// Linear gradient specification
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gradient {
    pub start: Color,
    pub end: Color,
    pub angle_radians: f32,
}

/// Background fill options
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Background {
    #[default]
    None,
    Solid(Color),
    Linear(Gradient),
}

/// Border specification with per-side widths and color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub widths: Edges<f32>,
    pub color: Color,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            widths: Edges::zero(),
            color: Color::transparent(),
        }
    }
}

/// Box shadow specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: Color::transparent(),
        }
    }
}

/// Overflow behavior
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

/// Flex container direction
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Flex container main-axis alignment
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Flex container cross-axis alignment
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlignItems {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

/// Flex item cross-axis alignment override
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    Start,
    Center,
    End,
    Stretch,
}

/// Positioning mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

/// Display mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Display {
    #[default]
    Flex,
    None,
}

/// Complete style specification for an element
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // Display and positioning
    pub display: Display,
    pub position: Position,

    // Flexbox layout properties
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,

    // Sizing
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub max_width: Length,
    pub min_height: Length,
    pub max_height: Length,

    // Spacing
    pub padding: Edges<f32>,
    pub margin: Edges<f32>,
    pub gap: f32,

    // Absolute positioning offsets
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,

    // Visual properties
    pub background: Background,
    pub border: Border,
    pub border_radius: Corners<f32>,
    pub box_shadow: Option<BoxShadow>,

    // Overflow and scrolling
    pub overflow: Overflow,
    pub overflow_y: Overflow,
    pub scroll_offset: Point<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::default(),
            position: Position::default(),
            flex_direction: FlexDirection::default(),
            justify_content: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_self: AlignSelf::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0, // CSS default is 1.0, not 0.0
            flex_basis: Length::Auto,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            max_width: Length::Auto,
            min_height: Length::Auto,
            max_height: Length::Auto,
            padding: Edges::zero(),
            margin: Edges::zero(),
            gap: 0.0,
            top: Length::Auto,
            right: Length::Auto,
            bottom: Length::Auto,
            left: Length::Auto,
            background: Background::default(),
            border: Border::default(),
            border_radius: Corners::zero(),
            box_shadow: None,
            overflow: Overflow::default(),
            overflow_y: Overflow::default(),
            scroll_offset: Point::new(0.0, 0.0),
        }
    }
}
