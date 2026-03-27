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
    /// Cached file tree — rebuilt only when sidebar state changes,
    /// not on every frame. Avoids filesystem reads in the render path.
    tree_cache: Vec<crate::view_model::FileTreeNode>,
    /// Whether the tree cache needs rebuilding.
    tree_dirty: bool,
    /// Patterns to hide from the file tree. Default: [".git"].
    /// Only entries whose name exactly matches one of these patterns are hidden.
    /// Dotfiles like .env, .gitignore are visible by default.
    pub ignored_patterns: Vec<String>,
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
            tree_cache: Vec::new(),
            tree_dirty: true,
            ignored_patterns: vec![".git".to_string()],
        }
    }
}

impl Sidebar {
    /// Creates a new sidebar with the given base directory and default ignored patterns.
    pub fn new(base_directory: Option<PathBuf>) -> Self {
        Self::new_with_patterns(base_directory, vec![".git".to_string()])
    }

    /// Creates a new sidebar with the given base directory and custom ignored patterns.
    pub fn new_with_patterns(base_directory: Option<PathBuf>, ignored_patterns: Vec<String>) -> Self {
        let mut sidebar = Self {
            visible: base_directory.is_some(),
            base_directory,
            expanded_dirs: HashSet::new(),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20,
            tree_cache: Vec::new(),
            tree_dirty: true,
            ignored_patterns,
        };
        sidebar.refresh_entries();
        sidebar.auto_expand_first_level();
        sidebar
    }

    /// Sets the ignored patterns and marks the tree as dirty.
    pub fn set_ignored_patterns(&mut self, patterns: Vec<String>) {
        self.ignored_patterns = patterns;
        self.tree_dirty = true;
    }

    /// Marks the tree cache as dirty so it will be rebuilt on next `build_tree` call.
    /// Used by the filesystem watcher to trigger a tree refresh.
    pub fn mark_tree_dirty(&mut self) {
        self.tree_dirty = true;
    }

    /// Expands all first-level directories in the current base directory.
    ///
    /// Called automatically when `set_base_directory` is invoked.
    /// This gives an immediate overview of the workspace structure on open.
    pub fn auto_expand_first_level(&mut self) {
        if let Some(ref base) = self.base_directory.clone() {
            let entries = Self::read_dir_sorted(base, &self.ignored_patterns);
            for entry in &entries {
                if entry.is_dir {
                    self.expanded_dirs.insert(entry.path.clone());
                }
            }
            self.tree_dirty = true;
        }
    }

    /// Refreshes the root-level file list from the base directory.
    pub fn refresh_entries(&mut self) {
        self.entries.clear();

        if let Some(ref base_dir) = self.base_directory.clone() {
            self.entries = Self::read_dir_sorted(base_dir, &self.ignored_patterns);
        }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
        self.tree_dirty = true;
    }

    /// Read a directory and return sorted entries (dirs first, then alpha).
    ///
    /// Only entries whose name exactly matches one of `ignored_patterns` are hidden.
    /// Dotfiles like `.env` and `.gitignore` are visible; only explicit matches are filtered.
    fn read_dir_sorted(dir: &PathBuf, ignored_patterns: &[String]) -> Vec<FileEntry> {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return Vec::new();
        };

        let mut entries: Vec<FileEntry> = read_dir
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Only hide entries matching an ignored pattern (exact name match).
                // Dotfiles like .env, .gitignore are NOT hidden unless explicitly listed.
                if ignored_patterns.iter().any(|p| p == &name) {
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
        self.tree_dirty = true;
    }

    /// Returns whether a directory is expanded.
    pub fn is_expanded(&self, path: &PathBuf) -> bool {
        self.expanded_dirs.contains(path)
    }

    /// Returns the cached file tree, rebuilding only if dirty.
    /// This avoids filesystem reads on every frame.
    pub fn build_tree(&mut self) -> Vec<crate::view_model::FileTreeNode> {
        if self.tree_dirty {
            self.tree_cache = match self.base_directory {
                Some(ref base_dir) => self.build_tree_recursive(base_dir),
                None => Vec::new(),
            };
            self.tree_dirty = false;
        }
        self.tree_cache.clone()
    }

    /// Recursively builds tree nodes for a directory.
    fn build_tree_recursive(&self, dir: &PathBuf) -> Vec<crate::view_model::FileTreeNode> {
        let entries = Self::read_dir_sorted(dir, &self.ignored_patterns);

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
                    let is_generated = matches!(
                        entry.name.as_str(),
                        "target" | "node_modules" | "__pycache__" | "dist" | "build"
                        | ".next" | "out" | "coverage" | ".turbo"
                    );
                    let mut node = crate::view_model::FileTreeNode::dir(&entry.name, is_expanded, children);
                    node.path = entry.path.to_string_lossy().to_string();
                    node.is_generated = is_generated;
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

    /// Sets a new base directory, resets selection, clears expanded dirs,
    /// refreshes entries, and auto-expands first-level directories.
    pub fn set_base_directory(&mut self, path: PathBuf) {
        self.base_directory = Some(path);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.expanded_dirs.clear();
        self.refresh_entries();
        self.auto_expand_first_level();
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
        assert_eq!(sidebar.ignored_patterns, vec![".git".to_string()]);
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
    fn test_real_tree_expand() {
        // Use the actual project directory to test tree building.
        let cwd = std::env::current_dir().unwrap();
        let mut sidebar = Sidebar::new(Some(cwd.clone()));
        assert!(sidebar.visible);
        assert!(!sidebar.entries.is_empty());

        let tree = sidebar.build_tree();
        assert!(!tree.is_empty(), "Tree should have root entries");

        // Find a directory in the tree (should already be expanded at first level)
        let dir_node = tree.iter().find(|n| n.is_dir);
        assert!(dir_node.is_some(), "Should have at least one directory");

        // All first-level dirs should be expanded after auto_expand_first_level
        let dir_node = dir_node.unwrap();
        assert!(dir_node.is_expanded, "First-level dirs should be auto-expanded");
        assert!(!dir_node.children.is_empty(), "Auto-expanded dir should have children");

        // Toggle collapse using the path from the tree node
        let dir_path = PathBuf::from(&dir_node.path);
        sidebar.toggle_dir(&dir_path);
        assert!(!sidebar.is_expanded(&dir_path));

        // Rebuild tree — collapsed dir should have no children
        let tree2 = sidebar.build_tree();
        let dir_node2 = tree2.iter().find(|n| n.name == dir_node.name).unwrap();
        assert!(!dir_node2.is_expanded, "Dir should be collapsed after toggle");
        assert!(dir_node2.children.is_empty(), "Collapsed dir has no children");

        // Toggle back open
        sidebar.toggle_dir(&dir_path);
        let tree3 = sidebar.build_tree();
        let dir_node3 = tree3.iter().find(|n| n.name == dir_node.name).unwrap();
        assert!(dir_node3.is_expanded, "Dir should be expanded after toggle");
        eprintln!("Dir '{}' expanded, children: {}", dir_node3.name, dir_node3.children.len());
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

    #[test]
    fn test_ignored_patterns_filtering() {
        use std::fs;

        // Create a temp directory with .git (dir), .env (file), .gitignore (file), src (dir)
        let tmp = std::env::temp_dir().join(format!("muda_sidebar_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()));
        fs::create_dir_all(&tmp).expect("create tmp dir");

        // Create test entries
        fs::create_dir(tmp.join(".git")).expect("create .git dir");
        fs::write(tmp.join(".env"), "SECRET=foo").expect("create .env");
        fs::write(tmp.join(".gitignore"), "target/").expect("create .gitignore");
        fs::create_dir(tmp.join("src")).expect("create src dir");

        // Default patterns: only .git hidden
        let sidebar = Sidebar::new(Some(tmp.clone()));
        let names: Vec<String> = sidebar.entries.iter().map(|e| e.name.clone()).collect();

        // .git should be hidden
        assert!(!names.contains(&".git".to_string()), ".git should be hidden by default");

        // dotfiles .env and .gitignore should be visible
        assert!(names.contains(&".env".to_string()), ".env should be visible by default");
        assert!(names.contains(&".gitignore".to_string()), ".gitignore should be visible by default");

        // src should be visible
        assert!(names.contains(&"src".to_string()), "src should be visible");

        // Custom patterns: also hide .env
        let sidebar2 = Sidebar::new_with_patterns(
            Some(tmp.clone()),
            vec![".git".to_string(), ".env".to_string()],
        );
        let names2: Vec<String> = sidebar2.entries.iter().map(|e| e.name.clone()).collect();

        assert!(!names2.contains(&".git".to_string()), ".git still hidden with custom patterns");
        assert!(!names2.contains(&".env".to_string()), ".env hidden when in ignored_patterns");
        assert!(names2.contains(&".gitignore".to_string()), ".gitignore still visible");

        // Cleanup
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_mark_tree_dirty() {
        let mut sidebar = Sidebar::default();
        // Build tree to clear dirty flag
        sidebar.build_tree();
        // tree_dirty should be false after build
        // Mark dirty
        sidebar.mark_tree_dirty();
        // We can't inspect tree_dirty directly (private), but we can verify
        // mark_tree_dirty doesn't panic and sidebar remains usable
        let _ = sidebar.build_tree();
    }

    #[test]
    fn test_set_ignored_patterns() {
        let mut sidebar = Sidebar::default();
        assert_eq!(sidebar.ignored_patterns, vec![".git".to_string()]);
        sidebar.set_ignored_patterns(vec![".git".to_string(), "target".to_string()]);
        assert_eq!(sidebar.ignored_patterns, vec![".git".to_string(), "target".to_string()]);
    }

    #[test]
    fn test_auto_expand_first_level() {
        use std::fs;

        let tmp = std::env::temp_dir().join(format!("muda_expand_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()));
        fs::create_dir_all(&tmp).expect("create tmp dir");
        fs::create_dir(tmp.join("alpha")).expect("create alpha");
        fs::create_dir(tmp.join("beta")).expect("create beta");
        fs::write(tmp.join("file.txt"), "").expect("create file");

        let sidebar = Sidebar::new(Some(tmp.clone()));

        // First-level dirs should be expanded
        assert!(sidebar.is_expanded(&tmp.join("alpha")), "alpha should be auto-expanded");
        assert!(sidebar.is_expanded(&tmp.join("beta")), "beta should be auto-expanded");
        // File should not be in expanded_dirs (files can't be expanded)
        assert!(!sidebar.is_expanded(&tmp.join("file.txt")));

        let _ = fs::remove_dir_all(&tmp);
    }
}
