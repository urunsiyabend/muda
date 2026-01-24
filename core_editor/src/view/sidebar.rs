//! Sidebar state for file explorer.
//!
//! Manages the file explorer sidebar state including visibility,
//! file list, selection, and base directory.

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

/// Sidebar state for file explorer.
#[derive(Clone, Debug)]
pub struct Sidebar {
    /// Whether the sidebar is visible.
    pub visible: bool,
    /// The base directory being explored.
    pub base_directory: Option<PathBuf>,
    /// List of file entries in the current directory.
    pub entries: Vec<FileEntry>,
    /// Currently selected index in the file list.
    pub selected_index: usize,
    /// Scroll offset for the file list.
    pub scroll_offset: usize,
    /// Cached viewport height for scroll calculations.
    /// Updated by adjust_scroll_for_height().
    viewport_height: usize,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            visible: false,
            base_directory: None,
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20, // Reasonable default, will be updated by adjust_scroll_for_height
        }
    }
}

impl Sidebar {
    /// Creates a new sidebar with the given base directory.
    pub fn new(base_directory: Option<PathBuf>) -> Self {
        let mut sidebar = Self {
            visible: base_directory.is_some(),
            base_directory,
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20, // Will be updated by adjust_scroll_for_height
        };
        sidebar.refresh_entries();
        sidebar
    }

    /// Refreshes the file list from the base directory.
    pub fn refresh_entries(&mut self) {
        self.entries.clear();

        if let Some(ref base_dir) = self.base_directory {
            if let Ok(read_dir) = fs::read_dir(base_dir) {
                let mut entries: Vec<FileEntry> = read_dir
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let path = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();

                        // Skip hidden files (starting with .)
                        if name.starts_with('.') {
                            return None;
                        }

                        let is_dir = path.is_dir();
                        Some(FileEntry::new(name, path, is_dir))
                    })
                    .collect();

                // Sort: directories first, then alphabetically
                entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });

                self.entries = entries;
            }
        }

        // Reset selection if out of bounds
        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
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
    ///
    /// Uses the cached viewport height from the last `adjust_scroll_for_height` call.
    pub fn ensure_visible(&mut self) {
        self.update_scroll_for_selection(self.viewport_height);
    }

    /// Updates the scroll offset for a given viewport height.
    ///
    /// This should be called whenever the terminal/window is resized to update
    /// the cached viewport height and ensure proper scrolling behavior.
    pub fn adjust_scroll_for_height(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        // Cache the viewport height for use in ensure_visible
        self.viewport_height = viewport_height;
        self.update_scroll_for_selection(viewport_height);
    }

    /// Internal helper to update scroll offset based on selection and viewport.
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

    /// Returns visible entries for rendering.
    pub fn visible_entries(&self, viewport_height: usize) -> &[FileEntry] {
        let start = self.scroll_offset;
        let end = (start + viewport_height).min(self.entries.len());
        &self.entries[start..end]
    }

    /// Returns the width of the sidebar in characters.
    pub fn width(&self) -> usize {
        if self.visible { 25 } else { 0 }
    }

    /// Sets a new base directory and refreshes the file list.
    pub fn set_base_directory(&mut self, path: PathBuf) {
        self.base_directory = Some(path);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.refresh_entries();
    }

    /// Navigates to the parent directory.
    pub fn go_to_parent_directory(&mut self) {
        if let Some(ref base) = self.base_directory {
            // Capture the name of the directory we are currently in (to select it in parent)
            let previous_dir_name = base.file_name().map(|n| n.to_string_lossy().to_string());

            if let Some(parent) = base.parent() {
                let parent_path = parent.to_path_buf();
                self.set_base_directory(parent_path);

                // If we know which directory we came from, select it
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
}
