use crate::layout::constraints::{AvailableSpace, LayoutInput, LayoutOutput};
use crate::style::units::{Length, Point, Rect, Size};
use crate::style::{AlignItems, AlignSelf, FlexDirection, JustifyContent, Position, Style};

/// Compute flexbox layout for a tree of styled nodes.
///
/// Uses a three-pass algorithm:
/// 1. Top-down: Resolve fixed sizes from parent constraints
/// 2. Bottom-up: Measure auto sizes from content
/// 3. Top-down: Distribute space with flex-grow/shrink, apply alignment
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

    // Pass 1: Resolve fixed sizes (top-down)
    let mut resolved_sizes = vec![Size::new(0.0, 0.0); styles.len()];
    resolve_fixed_sizes(
        styles,
        children_of,
        root,
        available,
        &mut resolved_sizes,
    );

    // Pass 2: Measure auto sizes (bottom-up)
    measure_auto_sizes(
        styles,
        children_of,
        intrinsic_sizes,
        root,
        &mut resolved_sizes,
    );

    // Pass 3: Distribute space and position (top-down)
    distribute_and_position(
        styles,
        children_of,
        root,
        Point::new(0.0, 0.0),
        &resolved_sizes,
        &mut outputs,
    );

    outputs
}

/// Pass 1: Resolve definite sizes from Length values using parent constraints
fn resolve_fixed_sizes(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    node: usize,
    available: LayoutInput,
    resolved: &mut [Size<f32>],
) {
    let style = styles[node];

    // Resolve width
    let width = match style.width {
        Length::Px(v) => Some(v),
        Length::Percent(p) => {
            if let AvailableSpace::Definite(parent_width) = available.available_width {
                Some(parent_width * p / 100.0)
            } else {
                None
            }
        }
        Length::Auto => None,
    };

    // Resolve height
    let height = match style.height {
        Length::Px(v) => Some(v),
        Length::Percent(p) => {
            if let AvailableSpace::Definite(parent_height) = available.available_height {
                Some(parent_height * p / 100.0)
            } else {
                None
            }
        }
        Length::Auto => None,
    };

    // Apply min/max constraints
    let width = width.map(|w| {
        let mut w = w;
        if let Length::Px(min) = style.min_width {
            w = w.max(min);
        }
        if let Length::Px(max) = style.max_width {
            w = w.min(max);
        }
        w
    });

    let height = height.map(|h| {
        let mut h = h;
        if let Length::Px(min) = style.min_height {
            h = h.max(min);
        }
        if let Length::Px(max) = style.max_height {
            h = h.min(max);
        }
        h
    });

    resolved[node] = Size::new(width.unwrap_or(0.0), height.unwrap_or(0.0));

    // Recurse to children with available space reduced by padding
    let inner_available = LayoutInput::new(
        if let Some(w) = width {
            AvailableSpace::Definite(w - style.padding.left - style.padding.right)
        } else {
            available.available_width
        },
        if let Some(h) = height {
            AvailableSpace::Definite(h - style.padding.top - style.padding.bottom)
        } else {
            available.available_height
        },
    );

    for &child_id in &children_of[node] {
        resolve_fixed_sizes(styles, children_of, child_id, inner_available, resolved);
    }
}

/// Pass 2: Measure auto sizes from content (bottom-up)
fn measure_auto_sizes(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    intrinsic_sizes: &[Option<Size<f32>>],
    node: usize,
    resolved: &mut [Size<f32>],
) {
    let style = styles[node];
    let children = &children_of[node];

    // Recurse to children first (bottom-up)
    for &child_id in children {
        measure_auto_sizes(styles, children_of, intrinsic_sizes, child_id, resolved);
    }

    // If this node has an intrinsic size (e.g., text), use it
    if let Some(intrinsic) = intrinsic_sizes[node] {
        if resolved[node].width == 0.0 {
            resolved[node].width = intrinsic.width;
        }
        if resolved[node].height == 0.0 {
            resolved[node].height = intrinsic.height;
        }
        return;
    }

    // Compute auto dimensions from children
    if children.is_empty() {
        // Leaf with no intrinsic size: keep as 0 (will be stretched by flex)
        return;
    }

    // Filter out absolute positioned children (don't contribute to size)
    let flow_children: Vec<usize> = children
        .iter()
        .copied()
        .filter(|&c| styles[c].position != Position::Absolute)
        .collect();

    if flow_children.is_empty() {
        return;
    }

    let gap_count = flow_children.len().saturating_sub(1);
    let gap_total = style.gap * gap_count as f32;

    match style.flex_direction {
        FlexDirection::Row => {
            // Width = sum of child widths + gaps + padding
            if resolved[node].width == 0.0 {
                let children_width: f32 = flow_children.iter().map(|&c| resolved[c].width).sum();
                resolved[node].width =
                    children_width + gap_total + style.padding.left + style.padding.right;
            }
            // Height = max child height + padding
            if resolved[node].height == 0.0 {
                let max_child_height = flow_children
                    .iter()
                    .map(|&c| resolved[c].height)
                    .fold(0.0, f32::max);
                resolved[node].height =
                    max_child_height + style.padding.top + style.padding.bottom;
            }
        }
        FlexDirection::Column => {
            // Height = sum of child heights + gaps + padding
            if resolved[node].height == 0.0 {
                let children_height: f32 =
                    flow_children.iter().map(|&c| resolved[c].height).sum();
                resolved[node].height =
                    children_height + gap_total + style.padding.top + style.padding.bottom;
            }
            // Width = max child width + padding
            if resolved[node].width == 0.0 {
                let max_child_width = flow_children
                    .iter()
                    .map(|&c| resolved[c].width)
                    .fold(0.0, f32::max);
                resolved[node].width = max_child_width + style.padding.left + style.padding.right;
            }
        }
    }
}

/// Pass 3: Distribute space using flex properties and position children (top-down)
fn distribute_and_position(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    node: usize,
    offset: Point<f32>,
    resolved: &[Size<f32>],
    outputs: &mut [LayoutOutput],
) {
    let _style = styles[node]; // Reserved for future use
    let size = resolved[node];

    // Set this node's bounds
    outputs[node] = LayoutOutput::new(Rect::new(offset.x, offset.y, size.width, size.height));

    let children = &children_of[node];
    if children.is_empty() {
        return;
    }

    // Separate absolute and flow children
    let (abs_children, flow_children): (Vec<usize>, Vec<usize>) =
        children.iter().copied().partition(|&c| styles[c].position == Position::Absolute);

    // Layout flow children
    if !flow_children.is_empty() {
        layout_flex_children(
            styles,
            children_of,
            node,
            &flow_children,
            offset,
            size,
            resolved,
            outputs,
        );
    }

    // Layout absolute children
    for child_id in abs_children {
        layout_absolute_child(styles, children_of, child_id, offset, size, resolved, outputs);
    }
}

/// Layout children in flexbox flow
fn layout_flex_children(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    parent: usize,
    children: &[usize],
    parent_offset: Point<f32>,
    parent_size: Size<f32>,
    resolved: &[Size<f32>],
    outputs: &mut [LayoutOutput],
) {
    let style = styles[parent];

    // Inner container size (subtract padding)
    let inner_width = parent_size.width - style.padding.left - style.padding.right;
    let inner_height = parent_size.height - style.padding.top - style.padding.bottom;

    // Calculate flex-basis for each child
    let mut child_main_sizes: Vec<f32> = children
        .iter()
        .map(|&c| {
            let child_style = styles[c];
            let child_size = resolved[c];
            match child_style.flex_basis {
                Length::Px(v) => v,
                Length::Percent(p) => match style.flex_direction {
                    FlexDirection::Row => inner_width * p / 100.0,
                    FlexDirection::Column => inner_height * p / 100.0,
                },
                Length::Auto => match style.flex_direction {
                    FlexDirection::Row => child_size.width,
                    FlexDirection::Column => child_size.height,
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
        // Extra space: distribute with flex-grow
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
        // Deficit: shrink with flex-shrink
        let total_shrink: f32 = children.iter().map(|&c| styles[c].flex_shrink).sum();
        if total_shrink > 0.0 {
            for (i, &child_id) in children.iter().enumerate() {
                let shrink = styles[child_id].flex_shrink;
                if shrink > 0.0 {
                    let shrink_amount = (shrink / total_shrink) * remaining.abs();
                    child_main_sizes[i] = (child_main_sizes[i] - shrink_amount).max(0.0);
                }
            }
        }
    }

    // Determine main-axis start position based on justify-content
    let total_main_after_flex: f32 = child_main_sizes.iter().sum();
    let free_space = container_main - total_main_after_flex - gap_total;

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
        let child_size = resolved[child_id];
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
                // Stretch cross-axis (height) when child has no explicit height set.
                // CSS flexbox stretches auto-sized children, not just zero-sized ones.
                let height = if align == AlignItems::Stretch && matches!(child_style.height, Length::Auto) {
                    inner_height
                } else {
                    child_size.height
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
                // Stretch cross-axis (width) when child has no explicit width set.
                // CSS flexbox stretches auto-sized children, not just zero-sized ones.
                let width = if align == AlignItems::Stretch && matches!(child_style.width, Length::Auto) {
                    inner_width
                } else {
                    child_size.width
                };
                (width, child_main, align)
            }
        };

        // Cross-axis positioning
        let cross_pos = match style.flex_direction {
            FlexDirection::Row => match cross_align {
                AlignItems::Start => 0.0,
                AlignItems::Center => (inner_height - child_height) / 2.0,
                AlignItems::End => inner_height - child_height,
                AlignItems::Stretch => 0.0,
            },
            FlexDirection::Column => match cross_align {
                AlignItems::Start => 0.0,
                AlignItems::Center => (inner_width - child_width) / 2.0,
                AlignItems::End => inner_width - child_width,
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

        // Update resolved size with flex-adjusted dimensions
        let mut adjusted_resolved = resolved.to_vec();
        match style.flex_direction {
            FlexDirection::Row => {
                adjusted_resolved[child_id].width = child_width;
                adjusted_resolved[child_id].height = child_height;
            }
            FlexDirection::Column => {
                adjusted_resolved[child_id].width = child_width;
                adjusted_resolved[child_id].height = child_height;
            }
        }

        // Recurse to child
        distribute_and_position(
            styles,
            children_of,
            child_id,
            Point::new(x, y),
            &adjusted_resolved,
            outputs,
        );

        // Advance main-axis position
        main_pos += child_main + spacing;
    }
}

/// Layout an absolutely positioned child
fn layout_absolute_child(
    styles: &[&Style],
    children_of: &[Vec<usize>],
    child_id: usize,
    parent_offset: Point<f32>,
    parent_size: Size<f32>,
    resolved: &[Size<f32>],
    outputs: &mut [LayoutOutput],
) {
    let child_style = styles[child_id];
    let _child_size = resolved[child_id]; // Reserved for future use

    // Resolve offset from top/left/right/bottom
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

    // Recurse to child
    distribute_and_position(
        styles,
        children_of,
        child_id,
        Point::new(x, y),
        resolved,
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
}
