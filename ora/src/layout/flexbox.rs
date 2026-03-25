use crate::layout::constraints::{AvailableSpace, LayoutInput, LayoutOutput};
use crate::style::units::{Length, Point, Rect, Size};
use crate::style::{AlignItems, AlignSelf, FlexDirection, JustifyContent, Position, Style};

/// Compute flexbox layout for a tree of styled nodes.
///
/// Uses a classic UI two-pass algorithm:
/// 1. **Measure pass** (bottom-up): Compute desired sizes from content and constraints
/// 2. **Arrange pass** (top-down): Distribute space, apply flex-grow/shrink, position children
///
/// The arrange pass knows final parent sizes, so percentage-based children resolve correctly
/// even when their parent uses flex-grow.
///
/// # Arguments
/// * `styles` - Styles indexed by node ID (0 = root, 1..N = children)
/// * `children_of` - Child node IDs for each node (children_of[i] = children of node i)
/// * `intrinsic_sizes` - Pre-measured sizes (e.g., text dimensions), None if not applicable
/// * `root` - Root node index to compute from
/// * `available` - Available space from parent
///
/// # Returns
/// Vec of LayoutOutput with computed bounds for each node index
pub fn compute_flexbox(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    root: usize,
    available: LayoutInput,
) -> Vec<LayoutOutput> {
    let mut outputs = vec![LayoutOutput::zero(); styles.len()];

    // Pass 1: Measure - compute desired sizes (bottom-up with top-down constraint propagation)
    let mut measured_sizes = vec![Size::new(0.0, 0.0); styles.len()];
    measure_pass(
        styles,
        children_of,
        intrinsic_sizes,
        root,
        available,
        &mut measured_sizes,
    );

    // Pass 2: Arrange - distribute space and position (top-down with final sizes)
    // Pass measured_sizes so arrange can use them for children not yet processed
    arrange_pass(
        styles,
        children_of,
        intrinsic_sizes,
        &measured_sizes,
        root,
        Point::new(0.0, 0.0),
        measured_sizes[root],
        &mut outputs,
    );

    outputs
}

/// Resolve a Length value to pixels given available space.
fn resolve_length(length: Length, available: AvailableSpace) -> Option<f32> {
    match length {
        Length::Px(v) => Some(v),
        Length::Percent(p) => {
            if let AvailableSpace::Definite(parent_size) = available {
                Some(parent_size * p / 100.0)
            } else {
                None
            }
        }
        Length::Auto => None,
    }
}

/// Apply min/max constraints to a dimension.
/// Works for both definite values and auto (where min provides a floor).
fn apply_min_max(value: Option<f32>, min: Length, max: Length) -> Option<f32> {
    let min_val = match min {
        Length::Px(v) => Some(v),
        _ => None,
    };
    let max_val = match max {
        Length::Px(v) => Some(v),
        _ => None,
    };

    match (value, min_val, max_val) {
        // Has definite value: apply both min and max
        (Some(v), Some(min), Some(max)) => Some(v.max(min).min(max)),
        (Some(v), Some(min), None) => Some(v.max(min)),
        (Some(v), None, Some(max)) => Some(v.min(max)),
        (Some(v), None, None) => Some(v),
        // Auto value: min_height/min_width provides a floor for auto-sized elements
        (None, Some(min), _) => Some(min),
        (None, None, _) => None,
    }
}

/// Pass 1: Measure - compute desired sizes bottom-up with constraint propagation.
///
/// This pass determines how much space each element wants, respecting:
/// - Explicit dimensions (px, %)
/// - min/max constraints (even for auto-sized elements)
/// - Intrinsic content sizes (text, images)
/// - Child content for containers
fn measure_pass(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    node: usize,
    available: LayoutInput,
    measured: &mut [Size<f32>],
) {
    let style = styles[node];

    // First, resolve explicit dimensions from style
    let explicit_width = resolve_length(style.width, available.available_width);
    let explicit_height = resolve_length(style.height, available.available_height);

    // Calculate available space for children (subtract padding)
    // IMPORTANT: When parent dimension is auto, we pass MinContent to prevent
    // percentage children from resolving against grandparent's size.
    // Percentage children will be properly resolved in the arrange pass.
    let child_available = LayoutInput::new(
        match explicit_width {
            Some(w) => AvailableSpace::Definite((w - style.padding.left - style.padding.right).max(0.0)),
            None => AvailableSpace::MinContent,
        },
        match explicit_height {
            Some(h) => AvailableSpace::Definite((h - style.padding.top - style.padding.bottom).max(0.0)),
            None => AvailableSpace::MinContent,
        },
    );

    // Recursively measure children first (bottom-up)
    let children = &children_of[node];
    for &child_id in children {
        measure_pass(styles, children_of, intrinsic_sizes, child_id, child_available, measured);
    }

    // Determine this node's size
    let mut width = explicit_width;
    let mut height = explicit_height;

    // If no explicit size, compute from content
    if width.is_none() || height.is_none() {
        // Check for intrinsic size (text, images)
        if let Some(intrinsic) = intrinsic_sizes[node] {
            if width.is_none() {
                width = Some(intrinsic.width);
            }
            if height.is_none() {
                height = Some(intrinsic.height);
            }
        } else {
            // Compute from children
            let flow_children: Vec<usize> = children
                .iter()
                .copied()
                .filter(|&c| styles[c].position != Position::Absolute)
                .collect();

            if !flow_children.is_empty() {
                let gap_count = flow_children.len().saturating_sub(1);
                let gap_total = style.gap * gap_count as f32;

                match style.flex_direction {
                    FlexDirection::Row => {
                        if width.is_none() {
                            let children_width: f32 = flow_children.iter().map(|&c| measured[c].width).sum();
                            width = Some(children_width + gap_total + style.padding.left + style.padding.right);
                        }
                        if height.is_none() {
                            let max_child_height = flow_children
                                .iter()
                                .map(|&c| measured[c].height)
                                .fold(0.0, f32::max);
                            height = Some(max_child_height + style.padding.top + style.padding.bottom);
                        }
                    }
                    FlexDirection::Column => {
                        if height.is_none() {
                            let children_height: f32 = flow_children.iter().map(|&c| measured[c].height).sum();
                            height = Some(children_height + gap_total + style.padding.top + style.padding.bottom);
                        }
                        if width.is_none() {
                            let max_child_width = flow_children
                                .iter()
                                .map(|&c| measured[c].width)
                                .fold(0.0, f32::max);
                            width = Some(max_child_width + style.padding.left + style.padding.right);
                        }
                    }
                }
            }
        }
    }

    // Apply min/max constraints (this is the key fix: min_height works even with auto)
    let width = apply_min_max(width, style.min_width, style.max_width);
    let height = apply_min_max(height, style.min_height, style.max_height);

    measured[node] = Size::new(width.unwrap_or(0.0), height.unwrap_or(0.0));
}

/// Pass 2: Arrange - distribute space and position elements (top-down).
///
/// This pass knows the final size of each element and:
/// - Distributes remaining space via flex-grow/shrink
/// - Re-resolves percentage-based children with actual parent size
/// - Positions children according to justify/align
/// - Sets final bounds in outputs
fn arrange_pass(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    measured: &[Size<f32>],
    node: usize,
    offset: Point<f32>,
    final_size: Size<f32>,
    outputs: &mut [LayoutOutput],
) {
    // Set this node's final bounds
    outputs[node] = LayoutOutput::new(Rect::new(offset.x, offset.y, final_size.width, final_size.height));

    let children = &children_of[node];
    if children.is_empty() {
        return;
    }

    // Separate absolute and flow children
    let (abs_children, flow_children): (Vec<usize>, Vec<usize>) =
        children.iter().copied().partition(|&c| styles[c].position == Position::Absolute);

    // Layout flow children with flex distribution
    if !flow_children.is_empty() {
        arrange_flex_children(
            styles,
            children_of,
            intrinsic_sizes,
            measured,
            node,
            &flow_children,
            offset,
            final_size,
            outputs,
        );
    }

    // Layout absolute children
    for child_id in abs_children {
        arrange_absolute_child(styles, children_of, intrinsic_sizes, measured, child_id, offset, final_size, outputs);
    }
}

/// Compute the size of a child, re-resolving percentages with actual parent size.
///
/// This is the key fix for percentage + flex-grow: we re-resolve percentage dimensions
/// in the arrange pass when we know the actual parent size after flex distribution.
fn resolve_child_size(
    child_style: &Style,
    intrinsic_size: Option<Size<f32>>,
    measured_size: Size<f32>,
    inner_width: f32,
    inner_height: f32,
) -> Size<f32> {
    // Re-resolve width: percentage now uses actual parent inner width
    let width = match child_style.width {
        Length::Px(v) => v,
        Length::Percent(p) => inner_width * p / 100.0,
        Length::Auto => {
            // Use intrinsic or measured size for auto
            intrinsic_size.map(|s| s.width).unwrap_or(measured_size.width)
        }
    };

    // Re-resolve height: percentage now uses actual parent inner height
    let height = match child_style.height {
        Length::Px(v) => v,
        Length::Percent(p) => inner_height * p / 100.0,
        Length::Auto => {
            intrinsic_size.map(|s| s.height).unwrap_or(measured_size.height)
        }
    };

    // Apply min/max constraints
    let width = apply_min_max(Some(width), child_style.min_width, child_style.max_width).unwrap_or(0.0);
    let height = apply_min_max(Some(height), child_style.min_height, child_style.max_height).unwrap_or(0.0);

    Size::new(width, height)
}

/// Arrange children in flexbox flow with flex distribution.
///
/// This function:
/// 1. Re-resolves percentage children with actual parent size
/// 2. Calculates flex-basis for each child
/// 3. Distributes remaining space via flex-grow/shrink
/// 4. Positions children with justify/align
/// 5. Recursively arranges grandchildren with final sizes
fn arrange_flex_children(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    measured: &[Size<f32>],
    parent: usize,
    children: &[usize],
    parent_offset: Point<f32>,
    parent_size: Size<f32>,
    outputs: &mut [LayoutOutput],
) {
    let style = styles[parent];

    // Inner container size (subtract padding)
    let inner_width = (parent_size.width - style.padding.left - style.padding.right).max(0.0);
    let inner_height = (parent_size.height - style.padding.top - style.padding.bottom).max(0.0);

    // Re-resolve each child's size with actual parent dimensions
    // This fixes percentage children of flex-grown parents
    // Use measured sizes from the measure pass (not outputs which may not be populated)
    let child_sizes: Vec<Size<f32>> = children
        .iter()
        .map(|&c| {
            resolve_child_size(styles[c], intrinsic_sizes[c], measured[c], inner_width, inner_height)
        })
        .collect();

    // Calculate flex-basis for each child
    let mut child_main_sizes: Vec<f32> = children
        .iter()
        .zip(child_sizes.iter())
        .map(|(&c, size)| {
            let child_style = styles[c];
            match child_style.flex_basis {
                Length::Px(v) => v,
                Length::Percent(p) => match style.flex_direction {
                    FlexDirection::Row => inner_width * p / 100.0,
                    FlexDirection::Column => inner_height * p / 100.0,
                },
                Length::Auto => match style.flex_direction {
                    FlexDirection::Row => size.width,
                    FlexDirection::Column => size.height,
                },
            }
        })
        .collect();

    let gap_count = children.len().saturating_sub(1);
    let gap_total = style.gap * gap_count as f32;

    // Main axis calculations
    let container_main = match style.flex_direction {
        FlexDirection::Row => inner_width,
        FlexDirection::Column => inner_height,
    };

    let total_main: f32 = child_main_sizes.iter().sum();
    let remaining = container_main - total_main - gap_total;

    // Apply flex-grow or flex-shrink
    if remaining > 0.0 {
        let total_grow: f32 = children.iter().map(|&c| styles[c].flex_grow).sum();
        if total_grow > 0.0 {
            for (i, &child_id) in children.iter().enumerate() {
                let grow = styles[child_id].flex_grow;
                if grow > 0.0 {
                    child_main_sizes[i] += (grow / total_grow) * remaining;
                }
            }
        }
    } else if remaining < 0.0 {
        let total_shrink: f32 = children.iter().map(|&c| styles[c].flex_shrink).sum();
        if total_shrink > 0.0 {
            for (i, &child_id) in children.iter().enumerate() {
                let shrink = styles[child_id].flex_shrink;
                if shrink > 0.0 {
                    // Respect min constraints when shrinking
                    let min_main = match style.flex_direction {
                        FlexDirection::Row => match styles[child_id].min_width {
                            Length::Px(v) => v,
                            _ => 0.0,
                        },
                        FlexDirection::Column => match styles[child_id].min_height {
                            Length::Px(v) => v,
                            _ => 0.0,
                        },
                    };
                    let shrink_amount = (shrink / total_shrink) * remaining.abs();
                    child_main_sizes[i] = (child_main_sizes[i] - shrink_amount).max(min_main);
                }
            }
        }
    }

    // Determine main-axis start position based on justify-content
    let total_main_after_flex: f32 = child_main_sizes.iter().sum();
    let free_space = (container_main - total_main_after_flex - gap_total).max(0.0);

    let (main_start, spacing) = match style.justify_content {
        JustifyContent::Start => (0.0, style.gap),
        JustifyContent::Center => (free_space / 2.0, style.gap),
        JustifyContent::End => (free_space, style.gap),
        JustifyContent::SpaceBetween => {
            if children.len() > 1 {
                (0.0, free_space / (children.len() - 1) as f32 + style.gap)
            } else {
                (0.0, style.gap)
            }
        }
    };

    // Position children
    let mut main_pos = main_start;

    for (i, &child_id) in children.iter().enumerate() {
        let child_style = styles[child_id];
        let child_resolved = child_sizes[i];
        let child_main = child_main_sizes[i];

        // Determine cross-axis size and alignment
        let (child_width, child_height, cross_align) = match style.flex_direction {
            FlexDirection::Row => {
                let align = match child_style.align_self {
                    AlignSelf::Auto => style.align_items,
                    AlignSelf::Start => AlignItems::Start,
                    AlignSelf::Center => AlignItems::Center,
                    AlignSelf::End => AlignItems::End,
                    AlignSelf::Stretch => AlignItems::Stretch,
                };
                // Stretch cross-axis when child has auto height
                let height = if align == AlignItems::Stretch && matches!(child_style.height, Length::Auto) {
                    inner_height
                } else {
                    child_resolved.height
                };
                (child_main, height, align)
            }
            FlexDirection::Column => {
                let align = match child_style.align_self {
                    AlignSelf::Auto => style.align_items,
                    AlignSelf::Start => AlignItems::Start,
                    AlignSelf::Center => AlignItems::Center,
                    AlignSelf::End => AlignItems::End,
                    AlignSelf::Stretch => AlignItems::Stretch,
                };
                // Stretch cross-axis when child has auto width
                let width = if align == AlignItems::Stretch && matches!(child_style.width, Length::Auto) {
                    inner_width
                } else {
                    child_resolved.width
                };
                (width, child_main, align)
            }
        };

        // Apply min/max to final dimensions
        let child_width = apply_min_max(Some(child_width), child_style.min_width, child_style.max_width).unwrap_or(0.0);
        let child_height = apply_min_max(Some(child_height), child_style.min_height, child_style.max_height).unwrap_or(0.0);

        // Cross-axis positioning
        let cross_pos = match style.flex_direction {
            FlexDirection::Row => match cross_align {
                AlignItems::Start => 0.0,
                AlignItems::Center => ((inner_height - child_height) / 2.0).max(0.0),
                AlignItems::End => (inner_height - child_height).max(0.0),
                AlignItems::Stretch => 0.0,
            },
            FlexDirection::Column => match cross_align {
                AlignItems::Start => 0.0,
                AlignItems::Center => ((inner_width - child_width) / 2.0).max(0.0),
                AlignItems::End => (inner_width - child_width).max(0.0),
                AlignItems::Stretch => 0.0,
            },
        };

        // Apply margin offsets
        let margin = &child_style.margin;
        let (x, y) = match style.flex_direction {
            FlexDirection::Row => (
                parent_offset.x + style.padding.left + main_pos + margin.left,
                parent_offset.y + style.padding.top + cross_pos + margin.top,
            ),
            FlexDirection::Column => (
                parent_offset.x + style.padding.left + cross_pos + margin.left,
                parent_offset.y + style.padding.top + main_pos + margin.top,
            ),
        };

        // Final child size
        let final_child_size = Size::new(child_width, child_height);

        // Recurse to child with its final size
        arrange_pass(
            styles,
            children_of,
            intrinsic_sizes,
            measured,
            child_id,
            Point::new(x, y),
            final_child_size,
            outputs,
        );

        // Advance main-axis position
        main_pos += child_main + spacing;
    }
}

/// Arrange an absolutely positioned child.
fn arrange_absolute_child(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    measured: &[Size<f32>],
    child_id: usize,
    parent_offset: Point<f32>,
    parent_size: Size<f32>,
    outputs: &mut [LayoutOutput],
) {
    let child_style = styles[child_id];

    // Resolve position from top/left/right/bottom
    let x = match child_style.left {
        Length::Px(v) => parent_offset.x + v,
        Length::Percent(p) => parent_offset.x + parent_size.width * p / 100.0,
        Length::Auto => parent_offset.x,
    };

    let y = match child_style.top {
        Length::Px(v) => parent_offset.y + v,
        Length::Percent(p) => parent_offset.y + parent_size.height * p / 100.0,
        Length::Auto => parent_offset.y,
    };

    // Resolve child size (percentages relative to parent)
    let child_size = resolve_child_size(
        child_style,
        intrinsic_sizes[child_id],
        measured[child_id],
        parent_size.width,
        parent_size.height,
    );

    // Recurse to child
    arrange_pass(
        styles,
        children_of,
        intrinsic_sizes,
        measured,
        child_id,
        Point::new(x, y),
        child_size,
        outputs,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_style() -> Style {
        Style::default()
    }

    #[test]
    fn test_row_basic() {
        // 3 children of 100px width in a 400px row
        let mut style_root = default_style();
        style_root.width = Length::Px(400.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child, &style_child, &style_child];
        let children_of = vec![vec![1, 2, 3], vec![], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(400.0, 100.0),
        );

        // Children should be positioned at 0, 100, 200
        assert_eq!(outputs[1].bounds.origin.x, 0.0);
        assert_eq!(outputs[2].bounds.origin.x, 100.0);
        assert_eq!(outputs[3].bounds.origin.x, 200.0);
    }

    #[test]
    fn test_column_basic() {
        // 3 children of 50px height in a 200px column
        let mut style_root = default_style();
        style_root.width = Length::Px(200.0);
        style_root.height = Length::Px(200.0);
        style_root.flex_direction = FlexDirection::Column;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child, &style_child, &style_child];
        let children_of = vec![vec![1, 2, 3], vec![], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(200.0, 200.0),
        );

        // Children should be positioned at 0, 50, 100
        assert_eq!(outputs[1].bounds.origin.y, 0.0);
        assert_eq!(outputs[2].bounds.origin.y, 50.0);
        assert_eq!(outputs[3].bounds.origin.y, 100.0);
    }

    #[test]
    fn test_gap() {
        // Row with gap=10, 3 children of 100px
        let mut style_root = default_style();
        style_root.width = Length::Px(400.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;
        style_root.gap = 10.0;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child, &style_child, &style_child];
        let children_of = vec![vec![1, 2, 3], vec![], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(400.0, 100.0),
        );

        // Children should be at 0, 110, 220 (100 + 10 gap)
        assert_eq!(outputs[1].bounds.origin.x, 0.0);
        assert_eq!(outputs[2].bounds.origin.x, 110.0);
        assert_eq!(outputs[3].bounds.origin.x, 220.0);
    }

    #[test]
    fn test_flex_grow() {
        // Row 300px, 2 children flex_grow=1 each
        let mut style_root = default_style();
        style_root.width = Length::Px(300.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_child = default_style();
        style_child.flex_grow = 1.0;
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child, &style_child];
        let children_of = vec![vec![1, 2], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(300.0, 100.0),
        );

        // Each child should get 150px (300 / 2)
        assert_eq!(outputs[1].bounds.size.width, 150.0);
        assert_eq!(outputs[2].bounds.size.width, 150.0);
    }

    #[test]
    fn test_flex_shrink() {
        // Row 200px, 3 children 100px each, flex_shrink=1
        let mut style_root = default_style();
        style_root.width = Length::Px(200.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(50.0);
        style_child.flex_shrink = 1.0;

        let styles = vec![&style_root, &style_child, &style_child, &style_child];
        let children_of = vec![vec![1, 2, 3], vec![], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(200.0, 100.0),
        );

        // Each child should shrink to ~66.67px (200 / 3)
        assert!((outputs[1].bounds.size.width - 66.67).abs() < 0.1);
        assert!((outputs[2].bounds.size.width - 66.67).abs() < 0.1);
        assert!((outputs[3].bounds.size.width - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_padding() {
        // Container with padding=10, child 100px
        let mut style_root = default_style();
        style_root.width = Length::Px(120.0);
        style_root.height = Length::Px(120.0);
        style_root.flex_direction = FlexDirection::Row;
        style_root.padding = crate::style::units::Edges::all(10.0);

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(100.0);

        let styles = vec![&style_root, &style_child];
        let children_of = vec![vec![1], vec![]];
        let intrinsic_sizes = vec![None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(120.0, 120.0),
        );

        // Child should be positioned at (10, 10)
        assert_eq!(outputs[1].bounds.origin.x, 10.0);
        assert_eq!(outputs[1].bounds.origin.y, 10.0);
    }

    #[test]
    fn test_justify_center() {
        // Row 400px, 2 children 100px each, centered
        let mut style_root = default_style();
        style_root.width = Length::Px(400.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;
        style_root.justify_content = JustifyContent::Center;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child, &style_child];
        let children_of = vec![vec![1, 2], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(400.0, 100.0),
        );

        // First child should start at 100px (centered)
        assert_eq!(outputs[1].bounds.origin.x, 100.0);
        assert_eq!(outputs[2].bounds.origin.x, 200.0);
    }

    #[test]
    fn test_align_stretch() {
        // Column 200px wide, child auto width -> stretches to 200px
        let mut style_root = default_style();
        style_root.width = Length::Px(200.0);
        style_root.height = Length::Px(200.0);
        style_root.flex_direction = FlexDirection::Column;
        style_root.align_items = AlignItems::Stretch;

        let mut style_child = default_style();
        style_child.height = Length::Px(50.0);
        // width is Auto, should stretch

        let styles = vec![&style_root, &style_child];
        let children_of = vec![vec![1], vec![]];
        let intrinsic_sizes = vec![None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(200.0, 200.0),
        );

        // Child should stretch to 200px width
        assert_eq!(outputs[1].bounds.size.width, 200.0);
    }

    #[test]
    fn test_percent_sizing() {
        // Parent 400px, child width=50% -> child gets 200px
        let mut style_root = default_style();
        style_root.width = Length::Px(400.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_child = default_style();
        style_child.width = Length::Percent(50.0);
        style_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child];
        let children_of = vec![vec![1], vec![]];
        let intrinsic_sizes = vec![None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(400.0, 100.0),
        );

        // Child should be 200px wide (50% of 400px)
        assert_eq!(outputs[1].bounds.size.width, 200.0);
    }

    #[test]
    fn test_min_height_with_auto() {
        // Child with auto height but min_height=50px should get 50px
        // This is the key fix: min_height works even when height is auto
        let mut style_root = default_style();
        style_root.width = Length::Px(200.0);
        style_root.height = Length::Px(200.0);
        style_root.flex_direction = FlexDirection::Column;

        let mut style_child = default_style();
        style_child.width = Length::Px(100.0);
        // height is Auto (default), but min_height is 50px
        style_child.min_height = Length::Px(50.0);

        let styles = vec![&style_root, &style_child];
        let children_of = vec![vec![1], vec![]];
        let intrinsic_sizes = vec![None, None]; // No intrinsic size

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(200.0, 200.0),
        );

        // Child should have min_height of 50px even with auto height
        assert_eq!(outputs[1].bounds.size.height, 50.0);
    }

    #[test]
    fn test_min_width_with_auto() {
        // Child with auto width but min_width=80px should get 80px
        let mut style_root = default_style();
        style_root.width = Length::Px(200.0);
        style_root.height = Length::Px(200.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_child = default_style();
        style_child.height = Length::Px(100.0);
        // width is Auto (default), but min_width is 80px
        style_child.min_width = Length::Px(80.0);

        let styles = vec![&style_root, &style_child];
        let children_of = vec![vec![1], vec![]];
        let intrinsic_sizes = vec![None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(200.0, 200.0),
        );

        // Child should have min_width of 80px even with auto width
        assert_eq!(outputs[1].bounds.size.width, 80.0);
    }

    #[test]
    fn test_percent_child_of_flex_grow_parent() {
        // This tests the key fix: percentage children of flex-grown parents
        // Root row with two children:
        // - Fixed child: 100px
        // - Flex child (grow=1): gets remaining space (200px in 300px container)
        //   - Grandchild: 100% width should get 200px (parent's final size)
        let mut style_root = default_style();
        style_root.width = Length::Px(300.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_fixed = default_style();
        style_fixed.width = Length::Px(100.0);
        style_fixed.height = Length::Px(100.0);

        let mut style_flex_parent = default_style();
        style_flex_parent.flex_grow = 1.0; // Will get 200px
        style_flex_parent.height = Length::Px(100.0);
        style_flex_parent.flex_direction = FlexDirection::Row;

        let mut style_percent_child = default_style();
        style_percent_child.width = Length::Percent(100.0); // Should be 200px
        style_percent_child.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_fixed, &style_flex_parent, &style_percent_child];
        let children_of = vec![vec![1, 2], vec![], vec![3], vec![]];
        let intrinsic_sizes = vec![None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(300.0, 100.0),
        );

        // Flex parent gets 200px (300 - 100)
        assert_eq!(outputs[2].bounds.size.width, 200.0);
        // Percent child gets 100% of 200px = 200px
        assert_eq!(outputs[3].bounds.size.width, 200.0);
    }

    #[test]
    fn test_space_between_with_percent_children() {
        // Row with space-between justification and percent-width children
        // Tests that percentage children resolve correctly for space-between layout
        let mut style_root = default_style();
        style_root.width = Length::Px(400.0);
        style_root.height = Length::Px(100.0);
        style_root.flex_direction = FlexDirection::Row;
        style_root.justify_content = JustifyContent::SpaceBetween;

        let mut style_left = default_style();
        style_left.width = Length::Px(50.0);
        style_left.height = Length::Px(50.0);

        let mut style_right = default_style();
        style_right.width = Length::Px(50.0);
        style_right.height = Length::Px(50.0);

        let styles = vec![&style_root, &style_left, &style_right];
        let children_of = vec![vec![1, 2], vec![], vec![]];
        let intrinsic_sizes = vec![None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(400.0, 100.0),
        );

        // With space-between: left at 0, right at 350 (400 - 50)
        assert_eq!(outputs[1].bounds.origin.x, 0.0);
        assert_eq!(outputs[2].bounds.origin.x, 350.0);
    }

    #[test]
    fn test_nested_flex_grow_with_percent() {
        // Simulates the status bar scenario:
        // - Root row (full width)
        // - Sidebar (fixed width)
        // - Main area (flex-grow)
        //   - Status bar (100% width of main area)
        //     - Content (space-between with left/right sections)
        let mut style_root = default_style();
        style_root.width = Length::Px(1000.0);
        style_root.height = Length::Px(600.0);
        style_root.flex_direction = FlexDirection::Row;

        let mut style_sidebar = default_style();
        style_sidebar.width = Length::Px(200.0);
        style_sidebar.height = Length::Percent(100.0);

        let mut style_main = default_style();
        style_main.flex_grow = 1.0; // Gets 800px
        style_main.height = Length::Percent(100.0);
        style_main.flex_direction = FlexDirection::Column;

        let mut style_status = default_style();
        style_status.width = Length::Percent(100.0); // Should get 800px
        style_status.height = Length::Px(28.0);
        style_status.flex_direction = FlexDirection::Row;
        style_status.justify_content = JustifyContent::SpaceBetween;

        let mut style_left = default_style();
        style_left.width = Length::Px(100.0);
        style_left.height = Length::Px(20.0);

        let mut style_right = default_style();
        style_right.width = Length::Px(150.0);
        style_right.height = Length::Px(20.0);

        let styles = vec![
            &style_root, &style_sidebar, &style_main, &style_status,
            &style_left, &style_right
        ];
        let children_of = vec![
            vec![1, 2],      // root -> sidebar, main
            vec![],          // sidebar
            vec![3],         // main -> status
            vec![4, 5],      // status -> left, right
            vec![],          // left
            vec![],          // right
        ];
        let intrinsic_sizes = vec![None, None, None, None, None, None];

        let outputs = compute_flexbox(
            &styles,
            &children_of,
            &intrinsic_sizes,
            0,
            LayoutInput::definite(1000.0, 600.0),
        );

        // Main area gets 800px (1000 - 200)
        assert_eq!(outputs[2].bounds.size.width, 800.0);
        // Status bar gets 100% of 800px = 800px
        assert_eq!(outputs[3].bounds.size.width, 800.0);
        // Left section at x=200 (after sidebar)
        assert_eq!(outputs[4].bounds.origin.x, 200.0);
        // Right section at x=200+800-150=850
        assert_eq!(outputs[5].bounds.origin.x, 850.0);
    }
}
