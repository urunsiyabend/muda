//! Flexbox-like layout system.
//!
//! Provides a simplified flexbox implementation for arranging UI components
//! in rows or columns with alignment and spacing control.

use crate::components::Bounds;
use crate::design_system::tokens::Spacing;

/// Layout direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    /// Lay out children horizontally (left to right)
    #[default]
    Row,
    /// Lay out children vertically (top to bottom)
    Column,
    /// Lay out children horizontally (right to left)
    RowReverse,
    /// Lay out children vertically (bottom to top)
    ColumnReverse,
}

impl FlexDirection {
    /// Check if this is a row direction.
    pub fn is_row(self) -> bool {
        matches!(self, FlexDirection::Row | FlexDirection::RowReverse)
    }

    /// Check if this is a column direction.
    pub fn is_column(self) -> bool {
        matches!(self, FlexDirection::Column | FlexDirection::ColumnReverse)
    }

    /// Check if this is a reverse direction.
    pub fn is_reverse(self) -> bool {
        matches!(self, FlexDirection::RowReverse | FlexDirection::ColumnReverse)
    }
}

/// Cross-axis alignment (perpendicular to the main axis).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    /// Align to the start of the cross axis
    Start,
    /// Center on the cross axis
    #[default]
    Center,
    /// Align to the end of the cross axis
    End,
    /// Stretch to fill the cross axis
    Stretch,
}

/// Main-axis content justification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    /// Pack items at the start
    #[default]
    Start,
    /// Center items on the main axis
    Center,
    /// Pack items at the end
    End,
    /// Distribute with space between items
    SpaceBetween,
    /// Distribute with space around items
    SpaceAround,
    /// Distribute with equal space between and around items
    SpaceEvenly,
}

/// Layout constraints passed to children.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutConstraints {
    /// Minimum width
    pub min_width: f32,
    /// Maximum width
    pub max_width: f32,
    /// Minimum height
    pub min_height: f32,
    /// Maximum height
    pub max_height: f32,
}

impl LayoutConstraints {
    /// Create tight constraints (exact size).
    pub fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Create loose constraints (0 to max).
    pub fn loose(max_width: f32, max_height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width,
            min_height: 0.0,
            max_height,
        }
    }

    /// Create unbounded constraints.
    pub fn unbounded() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }

    /// Create constraints from bounds.
    pub fn from_bounds(bounds: Bounds) -> Self {
        Self::tight(bounds.width, bounds.height)
    }

    /// Constrain a size to fit within these constraints.
    pub fn constrain(&self, width: f32, height: f32) -> (f32, f32) {
        (
            width.clamp(self.min_width, self.max_width),
            height.clamp(self.min_height, self.max_height),
        )
    }

    /// Check if the constraints are tight (exact size).
    pub fn is_tight(&self) -> bool {
        self.min_width == self.max_width && self.min_height == self.max_height
    }

    /// Get the tight width if constraints are tight, otherwise None.
    pub fn tight_width(&self) -> Option<f32> {
        if self.min_width == self.max_width {
            Some(self.min_width)
        } else {
            None
        }
    }

    /// Get the tight height if constraints are tight, otherwise None.
    pub fn tight_height(&self) -> Option<f32> {
        if self.min_height == self.max_height {
            Some(self.min_height)
        } else {
            None
        }
    }
}

/// Result of a layout operation.
#[derive(Clone, Debug, Default)]
pub struct LayoutResult {
    /// The final size of the laid out element
    pub size: (f32, f32),
    /// Bounds for each child
    pub children: Vec<Bounds>,
}

/// Flexbox-like layout configuration.
#[derive(Clone, Debug)]
pub struct FlexLayout {
    /// Layout direction
    pub direction: FlexDirection,
    /// Cross-axis alignment
    pub align_items: AlignItems,
    /// Main-axis justification
    pub justify_content: JustifyContent,
    /// Gap between items in pixels
    pub gap: f32,
    /// Padding inside the container
    pub padding: Spacing,
}

impl FlexLayout {
    /// Create a new flex layout with row direction.
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            ..Default::default()
        }
    }

    /// Create a new flex layout with column direction.
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// Set the direction.
    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the cross-axis alignment.
    pub fn with_align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    /// Set the main-axis justification.
    pub fn with_justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    /// Set the gap between items.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the padding.
    pub fn with_padding(mut self, padding: Spacing) -> Self {
        self.padding = padding;
        self
    }

    /// Lay out children within the given constraints.
    ///
    /// `child_sizes` should contain the natural (preferred) size of each child as (width, height).
    /// Returns the layout result with positions for each child.
    pub fn layout(&self, constraints: LayoutConstraints, child_sizes: &[(f32, f32)]) -> LayoutResult {
        if child_sizes.is_empty() {
            return LayoutResult {
                size: (self.padding.horizontal(), self.padding.vertical()),
                children: Vec::new(),
            };
        }

        let is_row = self.direction.is_row();
        let is_reverse = self.direction.is_reverse();

        // Available space after padding
        let available_main = if is_row {
            constraints.max_width - self.padding.horizontal()
        } else {
            constraints.max_height - self.padding.vertical()
        };

        let available_cross = if is_row {
            constraints.max_height - self.padding.vertical()
        } else {
            constraints.max_width - self.padding.horizontal()
        };

        // Calculate total main-axis size of children + gaps
        let total_gap = if child_sizes.len() > 1 {
            self.gap * (child_sizes.len() - 1) as f32
        } else {
            0.0
        };

        let total_main: f32 = child_sizes.iter()
            .map(|&(w, h)| if is_row { w } else { h })
            .sum();

        let total_main_with_gap = total_main + total_gap;

        // Calculate starting position based on justification
        let (main_start, extra_gap) = self.calculate_justification(
            available_main,
            total_main_with_gap,
            child_sizes.len(),
        );

        // Find max cross-axis size for stretch
        let max_cross: f32 = child_sizes.iter()
            .map(|&(w, h)| if is_row { h } else { w })
            .fold(0.0_f32, |a, b| a.max(b));

        let cross_size = max_cross.min(available_cross);

        // Position each child
        let mut children = Vec::with_capacity(child_sizes.len());
        let mut main_pos = self.padding.left + main_start;
        if !is_row {
            main_pos = self.padding.top + main_start;
        }

        let order: Vec<usize> = if is_reverse {
            (0..child_sizes.len()).rev().collect()
        } else {
            (0..child_sizes.len()).collect()
        };

        for &idx in &order {
            let (child_w, child_h) = child_sizes[idx];
            let child_main = if is_row { child_w } else { child_h };
            let child_cross = if is_row { child_h } else { child_w };

            // Calculate cross-axis position based on alignment
            let (final_cross, final_cross_size) = self.calculate_cross_alignment(
                child_cross,
                cross_size,
                available_cross,
            );

            let cross_pos = if is_row {
                self.padding.top + final_cross
            } else {
                self.padding.left + final_cross
            };

            let bounds = if is_row {
                Bounds {
                    x: main_pos,
                    y: cross_pos,
                    width: child_w,
                    height: if self.align_items == AlignItems::Stretch { final_cross_size } else { child_h },
                }
            } else {
                Bounds {
                    x: cross_pos,
                    y: main_pos,
                    width: if self.align_items == AlignItems::Stretch { final_cross_size } else { child_w },
                    height: child_h,
                }
            };

            children.push(bounds);
            main_pos += child_main + self.gap + extra_gap;
        }

        // Calculate final container size
        let final_size = if is_row {
            (
                (total_main_with_gap + self.padding.horizontal()).min(constraints.max_width),
                (cross_size + self.padding.vertical()).min(constraints.max_height),
            )
        } else {
            (
                (cross_size + self.padding.horizontal()).min(constraints.max_width),
                (total_main_with_gap + self.padding.vertical()).min(constraints.max_height),
            )
        };

        // If reverse, we built the array in reverse order, so restore original order
        if is_reverse {
            children.reverse();
        }

        LayoutResult {
            size: final_size,
            children,
        }
    }

    /// Calculate main-axis starting position and extra gap based on justification.
    fn calculate_justification(&self, available: f32, total: f32, count: usize) -> (f32, f32) {
        let free_space = (available - total).max(0.0);

        match self.justify_content {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::End => (free_space, 0.0),
            JustifyContent::Center => (free_space / 2.0, 0.0),
            JustifyContent::SpaceBetween => {
                if count <= 1 {
                    (0.0, 0.0)
                } else {
                    (0.0, free_space / (count - 1) as f32)
                }
            }
            JustifyContent::SpaceAround => {
                let space = free_space / count as f32;
                (space / 2.0, space)
            }
            JustifyContent::SpaceEvenly => {
                let space = free_space / (count + 1) as f32;
                (space, space)
            }
        }
    }

    /// Calculate cross-axis position and size based on alignment.
    fn calculate_cross_alignment(&self, child_size: f32, max_child: f32, available: f32) -> (f32, f32) {
        match self.align_items {
            AlignItems::Start => (0.0, child_size),
            AlignItems::End => (max_child - child_size, child_size),
            AlignItems::Center => ((max_child - child_size) / 2.0, child_size),
            AlignItems::Stretch => (0.0, available),
        }
    }
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            gap: 0.0,
            padding: Spacing::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_layout() {
        let layout = FlexLayout::row().with_gap(8.0);
        let constraints = LayoutConstraints::loose(400.0, 100.0);
        let child_sizes = vec![(100.0, 32.0), (100.0, 32.0), (100.0, 32.0)];

        let result = layout.layout(constraints, &child_sizes);

        assert_eq!(result.children.len(), 3);
        assert_eq!(result.children[0].x, 0.0);
        assert_eq!(result.children[1].x, 108.0); // 100 + 8 gap
        assert_eq!(result.children[2].x, 216.0); // 100 + 8 + 100 + 8
    }

    #[test]
    fn test_column_layout() {
        let layout = FlexLayout::column().with_gap(8.0);
        let constraints = LayoutConstraints::loose(200.0, 400.0);
        let child_sizes = vec![(100.0, 32.0), (100.0, 32.0)];

        let result = layout.layout(constraints, &child_sizes);

        assert_eq!(result.children[0].y, 0.0);
        assert_eq!(result.children[1].y, 40.0); // 32 + 8 gap
    }

    #[test]
    fn test_center_justify() {
        let layout = FlexLayout::row()
            .with_justify_content(JustifyContent::Center);
        let constraints = LayoutConstraints::loose(300.0, 100.0);
        let child_sizes = vec![(100.0, 32.0)];

        let result = layout.layout(constraints, &child_sizes);

        assert_eq!(result.children[0].x, 100.0); // Centered in 300px
    }

    #[test]
    fn test_space_between() {
        let layout = FlexLayout::row()
            .with_justify_content(JustifyContent::SpaceBetween);
        let constraints = LayoutConstraints::loose(300.0, 100.0);
        let child_sizes = vec![(50.0, 32.0), (50.0, 32.0), (50.0, 32.0)];

        let result = layout.layout(constraints, &child_sizes);

        assert_eq!(result.children[0].x, 0.0);
        assert_eq!(result.children[2].x, 250.0); // 300 - 50
    }
}
