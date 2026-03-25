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

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::Div;
use crate::elements::tree_item::tree_item;
use crate::style::Color;
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

/// View that renders a hierarchical file tree panel.
///
/// Consumes `FileTreePresentation` from core_editor and renders each entry as
/// a `TreeItem` row with proper indentation, chevrons, and colored icons.
pub struct FileTreeView {
    /// The presentation data (tree structure + selection state).
    presentation: FileTreePresentation,
}

impl FileTreeView {
    /// Creates a new `FileTreeView` from the given presentation data.
    pub fn new(presentation: FileTreePresentation) -> Self {
        Self { presentation }
    }

    /// Updates the presentation data (called when tree state changes).
    pub fn set_presentation(&mut self, presentation: FileTreePresentation) {
        self.presentation = presentation;
    }

    /// Returns whether the tree has any root nodes.
    pub fn is_empty(&self) -> bool {
        self.presentation.roots.is_empty()
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
}

impl View for FileTreeView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Collect all visible (depth, node) pairs
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

        let mut container = Div::new()
            .flex_col()
            .grow(1.0)
            .bg(bg);

        for (flat_index, (depth, node)) in rows.iter().enumerate() {
            let is_selected = self.presentation.selected_index == Some(flat_index);

            // Build icon color: amber for directories, extension-based for files
            let icon_color = if node.is_dir {
                cx.theme().color(ColorToken::Warning)
            } else {
                icon_color_for_extension(&node.extension)
            };

            let item = tree_item(&node.name)
                .depth(*depth)
                .selected(is_selected)
                .icon_color(icon_color);

            let item = if node.is_dir {
                item.directory(node.is_expanded)
            } else {
                item
            };

            container = container.child(item);
        }

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
}
