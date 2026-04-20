//! FileTree view for hierarchical file navigation.
//!
//! Renders a flat list of tree rows derived from nested FileTreeNode data.
//! Only children of expanded directories are included.
//!
//! # Design
//!
//! The view flattens the tree into `(depth, node)` pairs, then renders each
//! as a row using the TreeItem element (chevron + colored icon + label).
//!
//! # CONTEXT Decisions Applied
//!
//! - Indent guides as 1px rects per depth level (GPU-efficient, no borders)
//! - Colored file-type icons using Seti/Material-style palette
//! - Chevron uses ASCII "v" / ">" (no icon system dependency)
//! - BgSecondary background for the whole panel

use std::rc::Rc;

use crate::context::ViewContext;
use crate::editor_adapter::EditorCommand;
use crate::element::AnyElement;
use crate::elements::Div;
use crate::elements::tree_item::{tree_item, TREE_ITEM_HEIGHT};
use crate::style::{Color, px};
use crate::theme::ColorToken;
use crate::editor_adapter::{FileTreeNode, FileTreePresentation};
use crate::view::View;

// NOTE: Row geometry is delegated to TreeItem element which owns
// TREE_ITEM_HEIGHT, INDENT_WIDTH, CHEVRON_WIDTH, ICON_WIDTH, and TREE_FONT_SIZE.

/// Return a Seti/Material-style icon color for the given file extension.
///
/// Covers common languages and falls back to a neutral gray for unknown types.
pub fn icon_color_for_extension(ext: &str) -> Color {
    match ext {
        "rs" => Color::rgb(0.87, 0.40, 0.20),              // Rust orange
        "toml" | "yaml" | "yml" => Color::rgb(0.60, 0.60, 0.60), // Config gray
        "json" => Color::rgb(0.95, 0.78, 0.30),            // JSON gold
        "js" => Color::rgb(0.95, 0.85, 0.30),              // JavaScript yellow
        "ts" | "tsx" => Color::rgb(0.18, 0.53, 0.90),      // TypeScript blue
        "jsx" => Color::rgb(0.36, 0.80, 0.95),             // JSX cyan
        "py" => Color::rgb(0.30, 0.60, 0.85),              // Python blue
        "html" | "htm" => Color::rgb(0.90, 0.35, 0.20),   // HTML red-orange
        "css" | "scss" | "sass" => Color::rgb(0.33, 0.55, 0.85), // CSS blue
        "md" | "txt" | "rst" => Color::rgb(0.65, 0.75, 0.85), // Docs light blue
        "sh" | "bash" | "zsh" => Color::rgb(0.55, 0.75, 0.45), // Shell green
        "go" => Color::rgb(0.30, 0.75, 0.85),              // Go cyan
        "java" | "kt" | "kts" => Color::rgb(0.85, 0.35, 0.30), // JVM red
        "c" | "h" => Color::rgb(0.40, 0.55, 0.80),         // C blue
        "cpp" | "hpp" | "cc" | "hh" => Color::rgb(0.45, 0.50, 0.85), // C++ purple-blue
        "rb" => Color::rgb(0.85, 0.25, 0.25),              // Ruby red
        "swift" => Color::rgb(0.90, 0.42, 0.25),           // Swift orange
        "lua" => Color::rgb(0.30, 0.45, 0.80),             // Lua blue
        "zig" => Color::rgb(0.90, 0.60, 0.20),             // Zig amber
        "wgsl" | "glsl" | "hlsl" => Color::rgb(0.55, 0.85, 0.65), // Shader green
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => {
            Color::rgb(0.75, 0.55, 0.85) // Image purple
        }
        "lock" => Color::rgb(0.50, 0.50, 0.50),            // Lockfiles gray
        "env" | "gitignore" | "gitattributes" => Color::rgb(0.70, 0.65, 0.60), // Config tan
        _ => Color::rgb(0.65, 0.65, 0.65),                 // Default gray
    }
}

/// Buffer zone (rows above and below viewport) for smooth virtualized scrolling.
///
/// 20 extra rows on each side means ~60 rows total rendered instead of 500+.
const VIRTUAL_BUFFER_ROWS: usize = 20;

/// View that renders a hierarchical file tree panel.
///
/// Consumes `FileTreePresentation` from core_editor and renders each entry as
/// a `TreeItem` row with proper indentation, chevrons, and colored icons.
///
/// Virtualization: only rows in the visible viewport plus a buffer zone are
/// emitted as elements. Top/bottom spacer divs maintain scroll geometry.
pub struct FileTreeView {
    /// The presentation data (tree structure + selection state).
    pub presentation: FileTreePresentation,
    /// Command dispatch callback for file open actions.
    dispatch: Option<Rc<dyn Fn(EditorCommand)>>,
    /// Current pixel scroll offset (passed from sidebar's SharedScrollState).
    scroll_offset: f32,
    /// Visible area height in pixels (sidebar content area height).
    viewport_height: f32,
}

impl FileTreeView {
    /// Creates a new `FileTreeView` from the given presentation data.
    pub fn new(presentation: FileTreePresentation) -> Self {
        Self {
            presentation,
            dispatch: None,
            scroll_offset: 0.0,
            viewport_height: 600.0,
        }
    }

    /// Set the current scroll offset for virtualization (read from SharedScrollState).
    pub fn with_scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the visible viewport height for virtualization math.
    pub fn with_viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Attach a command dispatch callback for handling file clicks.
    pub fn with_dispatch(mut self, dispatch: Rc<dyn Fn(EditorCommand)>) -> Self {
        self.dispatch = Some(dispatch);
        self
    }

    /// Updates the presentation data (called when tree state changes).
    pub fn set_presentation(&mut self, presentation: FileTreePresentation) {
        self.presentation = presentation;
    }

    /// Returns whether the tree has any root nodes.
    pub fn is_empty(&self) -> bool {
        self.presentation.roots.is_empty()
    }

    /// Returns the total number of visible (expanded) rows in the tree.
    ///
    /// Used by the sidebar to compute max scroll height for the file tree.
    pub fn visible_row_count(&self) -> usize {
        let mut rows = Vec::new();
        Self::flatten_nodes(&self.presentation.roots, 0, &mut rows);
        rows.len()
    }

    /// Flatten the tree into `(depth, node)` pairs for linear rendering.
    ///
    /// Only children of expanded directories are included in the output.
    pub fn flatten_nodes<'a>(
        nodes: &'a [FileTreeNode],
        depth: usize,
        out: &mut Vec<(usize, &'a FileTreeNode)>,
    ) {
        for node in nodes {
            out.push((depth, node));
            if node.is_dir && node.is_expanded {
                Self::flatten_nodes(&node.children, depth + 1, out);
            }
        }
    }

    /// Compute the virtualized visible slice indices [start, end) given scroll state.
    ///
    /// Returns `(start, end, top_spacer_h, bottom_spacer_h)`.
    ///
    /// - `start`/`end`: range into the flat rows array to render
    /// - `top_spacer_h`: pixels of blank space above the visible rows
    /// - `bottom_spacer_h`: pixels of blank space below the visible rows
    pub fn virtual_slice(total_rows: usize, scroll_offset: f32, viewport_height: f32) -> (usize, usize, f32, f32) {
        if total_rows == 0 {
            return (0, 0, 0.0, 0.0);
        }

        let row_height = TREE_ITEM_HEIGHT;

        // First row that is (partially) visible
        let first_visible = (scroll_offset / row_height).floor() as usize;
        // Number of rows fitting in viewport (round up so partial rows are included)
        let visible_count = (viewport_height / row_height).ceil() as usize + 1;

        // Extend by buffer zone on both sides
        let start = first_visible.saturating_sub(VIRTUAL_BUFFER_ROWS);
        let end = (first_visible + visible_count + VIRTUAL_BUFFER_ROWS).min(total_rows);

        let top_spacer_h = start as f32 * row_height;
        let bottom_spacer_h = (total_rows - end) as f32 * row_height;

        (start, end, top_spacer_h, bottom_spacer_h)
    }
}

impl View for FileTreeView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Flatten the entire tree into (depth, node) pairs (only visible rows)
        let mut rows: Vec<(usize, &FileTreeNode)> = Vec::new();
        Self::flatten_nodes(&self.presentation.roots, 0, &mut rows);

        let bg = cx.theme().color(ColorToken::BgSecondary);

        // Empty state: render blank panel; caller shows "No folder open" text
        if rows.is_empty() {
            return Div::new()
                .flex_col()
                .grow(1.0)
                .bg(bg)
                .into();
        }

        // Compute the virtualized slice: only render visible rows plus buffer zone
        let (start, end, top_spacer_h, bottom_spacer_h) =
            Self::virtual_slice(rows.len(), self.scroll_offset, self.viewport_height);

        let mut container = Div::new()
            .flex_col()
            .grow(1.0)
            .bg(bg);

        // Top spacer ALWAYS present (even at zero height) so the child count
        // stays constant across scroll frames. A missing spacer would shrink
        // the element tree by one LayoutId and invalidate the layout cache
        // indexing on paint-only frames, producing stale bounds on every
        // subsequent sibling / descendant — the mechanism behind the
        // text-area scroll visual glitch.
        container = container.child(
            Div::new()
                .h(px(top_spacer_h))
                .shrink(0.0)
        );

        // Render only the visible slice (start..end)
        for (slice_i, (depth, node)) in rows[start..end].iter().enumerate() {
            // Use GLOBAL flat index for selection highlighting (not slice-local)
            let flat_index = start + slice_i;
            let is_selected = self.presentation.selected_index == Some(flat_index);

            // Build icon color: amber for directories, extension-based for files
            let icon_color = if node.is_dir {
                cx.theme().color(ColorToken::Warning)
            } else {
                icon_color_for_extension(&node.extension)
            };

            let mut item = tree_item(&node.name)
                .depth(*depth)
                .selected(is_selected)
                .icon_color(icon_color)
                .generated(node.is_generated);

            item = if node.is_dir {
                item.directory(node.is_expanded)
            } else {
                item
            };

            // Wire click handlers via dispatch
            if let Some(ref dispatch) = self.dispatch {
                let dispatch = dispatch.clone();
                let path = node.path.clone();
                if node.is_dir {
                    // Single click on directory: toggle/navigate
                    item = item.on_toggle({
                        let dispatch = dispatch.clone();
                        let path = path.clone();
                        move || {
                            dispatch(EditorCommand::ToggleSidebarDir(path.clone()));
                        }
                    });
                } else {
                    // Double click on file: open (click_count check is in TreeItem)
                    item = item.on_click(move || {
                        dispatch(EditorCommand::OpenSidebarFile(path.clone()));
                    });
                }
            }

            container = container.child(item);
        }

        // Bottom spacer ALWAYS present (same rationale as top spacer above).
        container = container.child(
            Div::new()
                .h(px(bottom_spacer_h))
                .shrink(0.0)
        );

        container.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_adapter::{FileTreeNode, FileTreePresentation};

    // -------------------------------------------------------------------------
    // flatten_nodes tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_file_tree_flatten_empty() {
        let mut out = Vec::new();
        FileTreeView::flatten_nodes(&[], 0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_file_tree_flatten_single_file() {
        let nodes = vec![FileTreeNode::file("main.rs", "rs")];
        let mut out = Vec::new();
        FileTreeView::flatten_nodes(&nodes, 0, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 0); // depth 0
        assert_eq!(out[0].1.name, "main.rs");
    }

    #[test]
    fn test_file_tree_flatten_expanded_dir() {
        let nodes = vec![
            FileTreeNode::dir(
                "src",
                true, // expanded
                vec![
                    FileTreeNode::file("main.rs", "rs"),
                    FileTreeNode::file("lib.rs", "rs"),
                ],
            ),
        ];
        let mut out = Vec::new();
        FileTreeView::flatten_nodes(&nodes, 0, &mut out);

        // dir + 2 children = 3 rows
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].1.name, "src");
        assert_eq!(out[0].0, 0);
        assert_eq!(out[1].1.name, "main.rs");
        assert_eq!(out[1].0, 1); // depth 1
        assert_eq!(out[2].1.name, "lib.rs");
        assert_eq!(out[2].0, 1);
    }

    #[test]
    fn test_file_tree_flatten_collapsed_dir_hides_children() {
        let nodes = vec![
            FileTreeNode::dir(
                "src",
                false, // collapsed
                vec![
                    FileTreeNode::file("main.rs", "rs"),
                    FileTreeNode::file("lib.rs", "rs"),
                ],
            ),
        ];
        let mut out = Vec::new();
        FileTreeView::flatten_nodes(&nodes, 0, &mut out);

        // Only the dir itself, children hidden
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1.name, "src");
    }

    #[test]
    fn test_file_tree_flatten_nested_expanded() {
        let nodes = vec![
            FileTreeNode::dir(
                "root",
                true,
                vec![
                    FileTreeNode::dir(
                        "inner",
                        true,
                        vec![FileTreeNode::file("deep.rs", "rs")],
                    ),
                ],
            ),
        ];
        let mut out = Vec::new();
        FileTreeView::flatten_nodes(&nodes, 0, &mut out);

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].1.name, "root");
        assert_eq!(out[0].0, 0);
        assert_eq!(out[1].1.name, "inner");
        assert_eq!(out[1].0, 1);
        assert_eq!(out[2].1.name, "deep.rs");
        assert_eq!(out[2].0, 2);
    }

    // -------------------------------------------------------------------------
    // icon_color_for_extension tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_icon_color_lookup_rust() {
        let c = icon_color_for_extension("rs");
        // Rust orange: R=0.87, G=0.40, B=0.20
        assert!((c.r - 0.87).abs() < 0.01);
        assert!((c.g - 0.40).abs() < 0.01);
        assert!((c.b - 0.20).abs() < 0.01);
    }

    #[test]
    fn test_icon_color_lookup_typescript() {
        let c = icon_color_for_extension("ts");
        // TypeScript blue: R=0.18, G=0.53, B=0.90
        assert!((c.r - 0.18).abs() < 0.01);
        assert!((c.g - 0.53).abs() < 0.01);
        assert!((c.b - 0.90).abs() < 0.01);

        // tsx should match ts
        let c2 = icon_color_for_extension("tsx");
        assert!((c2.r - c.r).abs() < 0.01);
    }

    #[test]
    fn test_icon_color_lookup_unknown() {
        let c = icon_color_for_extension("xyz_unknown_ext");
        // Default gray: R=0.65, G=0.65, B=0.65
        assert!((c.r - 0.65).abs() < 0.01);
        assert!((c.g - 0.65).abs() < 0.01);
        assert!((c.b - 0.65).abs() < 0.01);
    }

    #[test]
    fn test_icon_color_lookup_python() {
        let c = icon_color_for_extension("py");
        // Python blue: 0.30, 0.60, 0.85
        assert!((c.r - 0.30).abs() < 0.01);
        assert!((c.g - 0.60).abs() < 0.01);
        assert!((c.b - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_file_tree_view_is_empty() {
        let view = FileTreeView::new(FileTreePresentation::default());
        assert!(view.is_empty());

        let non_empty = FileTreeView::new(FileTreePresentation {
            roots: vec![FileTreeNode::file("main.rs", "rs")],
            selected_index: None,
        });
        assert!(!non_empty.is_empty());
    }

    // -------------------------------------------------------------------------
    // virtual_slice tests
    // -------------------------------------------------------------------------

    /// With 100 rows and a 240px viewport (10 visible at 24px each), the rendered
    /// slice should be at most 10 visible + 20 buffer above + 20 buffer below = 50 rows.
    #[test]
    fn test_virtual_slice_limits_rows_to_visible_plus_buffer() {
        let total_rows = 100;
        let scroll_offset = 0.0; // at top
        let viewport_height = 240.0; // 10 visible rows at 24px each

        let (start, end, top_h, bottom_h) =
            FileTreeView::virtual_slice(total_rows, scroll_offset, viewport_height);

        // At top: start = 0 (saturating_sub of 0 with buffer 20)
        assert_eq!(start, 0);
        // end = min(0 + 11 + 20, 100) = 31
        // visible_count = ceil(240/24) + 1 = 10 + 1 = 11
        // end = (0 + 11 + 20).min(100) = 31
        assert_eq!(end, 31);

        // Rows rendered = end - start = 31 (well under 100)
        let rendered = end - start;
        assert!(rendered <= 50, "expected ≤50 rows rendered, got {}", rendered);

        // No top spacer (we're at the very top)
        assert_eq!(top_h, 0.0);
        // Bottom spacer covers rows 31..100 = 69 rows
        assert!((bottom_h - (total_rows - end) as f32 * 24.0).abs() < 0.01);
    }

    /// When scrolled to the middle, both top and bottom spacers should be non-zero.
    #[test]
    fn test_virtual_slice_mid_scroll_has_both_spacers() {
        let total_rows = 100;
        // Scroll to row 50 (pixel 50*24 = 1200)
        let scroll_offset = 50.0 * 24.0;
        let viewport_height = 240.0; // 10 visible rows

        let (start, end, top_h, bottom_h) =
            FileTreeView::virtual_slice(total_rows, scroll_offset, viewport_height);

        // first_visible = floor(1200 / 24) = 50
        // start = 50 - 20 = 30
        // end = min(50 + 11 + 20, 100) = min(81, 100) = 81
        assert_eq!(start, 30);
        assert_eq!(end, 81);

        // Top spacer covers rows 0..30
        assert!((top_h - 30.0 * 24.0).abs() < 0.01);
        // Bottom spacer covers rows 81..100 = 19 rows
        assert!((bottom_h - 19.0 * 24.0).abs() < 0.01);
    }

    /// At the bottom of the list, bottom spacer is 0 and top spacer is non-zero.
    #[test]
    fn test_virtual_slice_at_bottom_no_bottom_spacer() {
        let total_rows = 50;
        // Scroll past end (should be clamped by ScrollArea but test the math)
        let scroll_offset = 50.0 * 24.0; // row 50 (beyond end)
        let viewport_height = 240.0;

        let (_start, end, _top_h, bottom_h) =
            FileTreeView::virtual_slice(total_rows, scroll_offset, viewport_height);

        // end is clamped to total_rows
        assert_eq!(end, total_rows);
        // No bottom spacer when end == total_rows
        assert_eq!(bottom_h, 0.0);
    }

    /// Empty list returns zero-range and zero spacers.
    #[test]
    fn test_virtual_slice_empty_rows() {
        let (start, end, top_h, bottom_h) =
            FileTreeView::virtual_slice(0, 0.0, 600.0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
        assert_eq!(top_h, 0.0);
        assert_eq!(bottom_h, 0.0);
    }

    /// Selection index maps correctly: selected_index is a global flat index.
    /// When start=30, a node at global index 35 should be rendered at slice offset 5.
    #[test]
    fn test_virtual_slice_global_selection_index() {
        let total_rows = 100;
        let scroll_offset = 50.0 * 24.0; // first_visible = 50
        let viewport_height = 240.0;

        let (start, end, _, _) =
            FileTreeView::virtual_slice(total_rows, scroll_offset, viewport_height);

        // start=30, end=81
        // Global index 35 is within the rendered slice (start..end)
        let global_index = 35_usize;
        assert!(global_index >= start && global_index < end,
            "global index {} should be in [{}, {})", global_index, start, end);

        // The slice-local index is global_index - start
        let slice_local = global_index - start;
        assert_eq!(slice_local, 5);

        // For selection: flat_index = start + slice_i = 30 + 5 = 35 ✓
        let recovered_global = start + slice_local;
        assert_eq!(recovered_global, global_index);
    }

    /// visible_row_count returns the total expanded rows count.
    #[test]
    fn test_visible_row_count() {
        let view = FileTreeView::new(FileTreePresentation::default());
        assert_eq!(view.visible_row_count(), 0);

        let view_with_files = FileTreeView::new(FileTreePresentation {
            roots: vec![
                FileTreeNode::file("a.rs", "rs"),
                FileTreeNode::file("b.rs", "rs"),
                FileTreeNode::dir("src", false, vec![
                    FileTreeNode::file("main.rs", "rs"),
                ]),
            ],
            selected_index: None,
        });
        // 2 files + 1 collapsed dir = 3 visible rows (collapsed children not counted)
        assert_eq!(view_with_files.visible_row_count(), 3);
    }
}
