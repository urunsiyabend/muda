//! Sidebar state for file explorer.
//!
//! Manages the file explorer sidebar state including visibility,
//! base directory, and expand/collapse state for directories.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Represents an entry in the file explorer.
#[derive(Clone, Debug)]
pub struct FileEntry {
    /// The file/directory name.
    pub name: String,
    /// Full path to the file/directory.
    pub path: PathBuf,
    /// Whether this is a directory.
    pub is_dir: bool,
}

impl FileEntry {
    pub fn new(name: String, path: PathBuf, is_dir: bool) -> Self {
        Self { name, path, is_dir }
    }
}

/// Focus state for the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusState {
    /// Editor pane is focused.
    Editor,
    /// Sidebar (file explorer) is focused.
    Sidebar,
}

impl Default for FocusState {
    fn default() -> Self {
        Self::Editor
    }
}

/// Sidebar state for file explorer with tree expand/collapse.
#[derive(Clone, Debug)]
pub struct Sidebar {
    /// Whether the sidebar is visible.
    pub visible: bool,
    /// The root directory being explored.
    pub base_directory: Option<PathBuf>,
    /// Set of expanded directory paths (for tree expand/collapse).
    pub expanded_dirs: HashSet<PathBuf>,
    /// Flat list of entries in root directory (kept for keyboard navigation).
    pub entries: Vec<FileEntry>,
    /// Currently selected index in the flat list.
    pub selected_index: usize,
    /// Scroll offset for the file list.
    pub scroll_offset: usize,
    /// Cached viewport height for scroll calculations.
    viewport_height: usize,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            visible: false,
            base_directory: None,
            expanded_dirs: HashSet::new(),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20,
        }
    }
}

impl Sidebar {
    /// Creates a new sidebar with the given base directory.
    pub fn new(base_directory: Option<PathBuf>) -> Self {
        let mut sidebar = Self {
            visible: base_directory.is_some(),
            base_directory,
            expanded_dirs: HashSet::new(),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20,
        };
        sidebar.refresh_entries();
        sidebar
    }

    /// Refreshes the root-level file list from the base directory.
    pub fn refresh_entries(&mut self) {
        self.entries.clear();

        if let Some(ref base_dir) = self.base_directory {
            self.entries = Self::read_dir_sorted(base_dir);
        }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
    }

    /// Read a directory and return sorted entries (dirs first, then alpha).
    fn read_dir_sorted(dir: &PathBuf) -> Vec<FileEntry> {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return Vec::new();
        };

        let mut entries: Vec<FileEntry> = read_dir
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and common noise directories
                if name.starts_with('.') {
                    return None;
                }
                if matches!(name.as_str(), "target" | "node_modules" | "__pycache__") {
                    return None;
                }

                let is_dir = path.is_dir();
                Some(FileEntry::new(name, path, is_dir))
            })
            .collect();

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        entries
    }

    /// Toggles a directory's expanded state.
    pub fn toggle_dir(&mut self, path: &PathBuf) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.clone());
        }
    }

    /// Returns whether a directory is expanded.
    pub fn is_expanded(&self, path: &PathBuf) -> bool {
        self.expanded_dirs.contains(path)
    }

    /// Builds a recursive file tree from the base directory,
    /// expanding directories that are in `expanded_dirs`.
    pub fn build_tree(&self) -> Vec<crate::view_model::FileTreeNode> {
        let Some(ref base_dir) = self.base_directory else {
            return Vec::new();
        };
        self.build_tree_recursive(base_dir)
    }

    /// Recursively builds tree nodes for a directory.
    fn build_tree_recursive(&self, dir: &PathBuf) -> Vec<crate::view_model::FileTreeNode> {
        let entries = Self::read_dir_sorted(dir);

        entries
            .into_iter()
            .map(|entry| {
                if entry.is_dir {
                    let is_expanded = self.expanded_dirs.contains(&entry.path);
                    let children = if is_expanded {
                        self.build_tree_recursive(&entry.path)
                    } else {
                        Vec::new()
                    };
                    let mut node = crate::view_model::FileTreeNode::dir(&entry.name, is_expanded, children);
                    node.path = entry.path.to_string_lossy().to_string();
                    node
                } else {
                    let ext = entry.path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let mut node = crate::view_model::FileTreeNode::file(&entry.name, &ext);
                    node.path = entry.path.to_string_lossy().to_string();
                    node
                }
            })
            .collect()
    }

    /// Toggles sidebar visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Shows the sidebar.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hides the sidebar.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Moves selection up.
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    /// Moves selection down.
    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    /// Ensures the selected item is visible within the viewport.
    pub fn ensure_visible(&mut self) {
        self.update_scroll_for_selection(self.viewport_height);
    }

    /// Updates the scroll offset for a given viewport height.
    pub fn adjust_scroll_for_height(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        self.viewport_height = viewport_height;
        self.update_scroll_for_selection(viewport_height);
    }

    fn update_scroll_for_selection(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected_index - viewport_height + 1;
        }
    }

    /// Returns the currently selected entry.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    /// Returns the path of the currently selected entry.
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_entry().map(|e| &e.path)
    }

    /// Returns visible entries for rendering (flat, for keyboard nav compatibility).
    pub fn visible_entries(&self, viewport_height: usize) -> &[FileEntry] {
        let start = self.scroll_offset;
        let end = (start + viewport_height).min(self.entries.len());
        &self.entries[start..end]
    }

    /// Returns the width of the sidebar in pixels.
    pub fn width(&self) -> usize {
        if self.visible { 220 } else { 0 }
    }

    /// Selects entry at the given index.
    pub fn select_index(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected_index = index;
            self.ensure_visible();
        }
    }

    /// Sets a new base directory and refreshes the file list.
    pub fn set_base_directory(&mut self, path: PathBuf) {
        self.base_directory = Some(path);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.expanded_dirs.clear();
        self.refresh_entries();
    }

    /// Navigates to the parent directory.
    pub fn go_to_parent_directory(&mut self) {
        if let Some(ref base) = self.base_directory {
            let previous_dir_name = base.file_name().map(|n| n.to_string_lossy().to_string());

            if let Some(parent) = base.parent() {
                let parent_path = parent.to_path_buf();
                self.set_base_directory(parent_path);

                if let Some(name) = previous_dir_name {
                    if let Some(index) = self.entries.iter().position(|e| e.name == name) {
                        self.selected_index = index;
                        self.ensure_visible();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_default() {
        let sidebar = Sidebar::default();
        assert!(!sidebar.visible);
        assert!(sidebar.base_directory.is_none());
        assert!(sidebar.entries.is_empty());
    }

    #[test]
    fn test_sidebar_toggle() {
        let mut sidebar = Sidebar::default();
        assert!(!sidebar.visible);
        sidebar.toggle();
        assert!(sidebar.visible);
        sidebar.toggle();
        assert!(!sidebar.visible);
    }

    #[test]
    fn test_sidebar_navigation() {
        let mut sidebar = Sidebar::default();
        sidebar.entries = vec![
            FileEntry::new("a.txt".to_string(), PathBuf::from("a.txt"), false),
            FileEntry::new("b.txt".to_string(), PathBuf::from("b.txt"), false),
            FileEntry::new("c.txt".to_string(), PathBuf::from("c.txt"), false),
        ];

        assert_eq!(sidebar.selected_index, 0);
        sidebar.move_down();
        assert_eq!(sidebar.selected_index, 1);
        sidebar.move_down();
        assert_eq!(sidebar.selected_index, 2);
        sidebar.move_down(); // Should not go beyond last item
        assert_eq!(sidebar.selected_index, 2);
        sidebar.move_up();
        assert_eq!(sidebar.selected_index, 1);
    }

    #[test]
    fn test_focus_state_default() {
        let focus = FocusState::default();
        assert_eq!(focus, FocusState::Editor);
    }

    #[test]
    fn test_toggle_dir() {
        let mut sidebar = Sidebar::default();
        let dir = PathBuf::from("/some/dir");
        assert!(!sidebar.is_expanded(&dir));
        sidebar.toggle_dir(&dir);
        assert!(sidebar.is_expanded(&dir));
        sidebar.toggle_dir(&dir);
        assert!(!sidebar.is_expanded(&dir));
    }
}
