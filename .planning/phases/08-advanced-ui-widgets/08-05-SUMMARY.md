---
phase: 08
plan: 05
name: file-tree
subsystem: views
tags: [rust, file-tree, sidebar, tree-view, file-navigation]

dependency-graph:
  requires:
    - "08-02 (TreeItem element for row rendering)"
    - "07-02 (SidebarView with placeholder content)"
    - "core_editor view_model (SidebarPresentation)"
  provides:
    - "FileTreeView — hierarchical file tree with colored icons"
    - "FileTreeNode — tree data structure in core_editor"
    - "FileTreePresentation — presentation data for file tree"
    - "icon_color_for_extension() — Seti/Material-style icon colors"
    - "SidebarView wired with FileTree instead of placeholder"
  affects:
    - "08-08 (AppLayout uses SidebarView with FileTree)"

tech-stack:
  added: []
  patterns:
    - "Flatten-then-render: tree nodes flattened to (depth, node) pairs for linear rendering"
    - "TreeItem element delegation: FileTreeView creates tree_item() for each row"
    - "Extension-based icon coloring: 25+ extensions mapped to Seti/Material-style colors"
    - "SidebarView composition: FileTreeView stored as field, rendered in content area"

key-files:
  created:
    - "ora/src/views/file_tree.rs — FileTreeView, icon_color_for_extension()"
  modified:
    - "core_editor/src/view_model/mod.rs — added FileTreeNode, FileTreePresentation"
    - "ora/src/views/sidebar.rs — wired FileTree into content area, updated constructors"
    - "ora/src/views/mod.rs — registered file_tree module"
    - "ora/src/lib.rs — added FileTreeView re-export"
    - "ora/examples/editor_chrome_demo.rs — updated SidebarView::new() call"
---

## What was built

**FileTreeView** — Hierarchical file tree view using TreeItem elements for rendering.

### Components

1. **FileTreeNode** (in core_editor) — Tree data structure with `name`, `extension`, `is_dir`, `is_expanded`, and `children`. Builder methods `file()` and `dir()`.

2. **FileTreePresentation** (in core_editor) — Presentation data with `roots` (Vec<FileTreeNode>) and `selected_index`.

3. **FileTreeView** — View that flattens the tree into visible rows and renders each using `tree_item()` element:
   - Directories show amber (Warning) colored icon
   - Files show extension-based icon colors (25+ mappings)
   - Selected row highlighted with Accent color
   - Empty state returns blank panel (sidebar shows "No folder open")

4. **SidebarView integration** — Updated to accept `FileTreePresentation`, stores `FileTreeView` internally, renders tree in content area.

### Commits

| Hash | Message |
|------|---------|
| 0b713f5 | feat(08-05): add FileTreeView and wire into SidebarView |

### Test Results

- 5 flatten tests (empty, single file, expanded dir, collapsed dir, nested)
- 4 icon color tests (rust, typescript, unknown, python)
- 1 is_empty test
- Updated sidebar tests pass with new constructor signature
- All 106 lib tests pass

### Decisions

- icon_color_for_extension is a standalone pub fn (not a method) for reuse
- SidebarView::new() now takes (SidebarPresentation, FileTreePresentation) — breaking change from Phase 7
- Amber (ColorToken::Warning) for directory icons, extension-based for files
- flatten_nodes is a static method — no &self needed for recursion
