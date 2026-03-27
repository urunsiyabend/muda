//! CoreEditorAdapter: bridges core_editor::app::App to ora's EditorDataSource trait.
//!
//! This module owns all type conversions from core_editor view_model types
//! to ora::editor_adapter mirror types. Because both type sets are defined in
//! external crates, the Rust orphan rule prevents direct `From` impls — so
//! conversion is done through private free functions in this module instead.
//!
//! The public surface is only `CoreEditorAdapter` and its `EditorDataSource` impl.

use core_editor::commands::editor_command::{Direction, MoveScope as CoreMoveScope};
use core_editor::commands::EditorCommand as CoreEditorCommand;
use std::time::Instant;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use notify_debouncer_full::notify::RecursiveMode;
use std::sync::mpsc;
use ora::editor_adapter::{
    BufferDataSource, CaretPresentation, CommandDispatcher, CursorDirection, DialogPresentation,
    EditorCommand, FileEntryPresentation, FileOpDataSource, GutterModel, LinePresentation,
    MoveScope as OraMoveScope, PendingFileOp, RenderModel, SidebarPresentation, StyledSpan,
    StatusPresentation, TabBarPresentation, TabPresentation, TextStyle, VisualPosition,
    WindowDataSource,
};

// =============================================================================
// CoreEditorAdapter struct
// =============================================================================

/// Returns the platform configuration state file path.
///
/// On Windows: `%APPDATA%\muda\state.json`
/// On macOS/Linux: `$XDG_CONFIG_HOME/muda/state.json` or `~/.config/muda/state.json`
fn config_state_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("muda")
        .join("state.json")
}

/// Persistent editor state stored in state.json.
struct AppState {
    /// Last directory visited in a file open/save dialog.
    last_dir: std::path::PathBuf,
    /// Last opened workspace folder, restored on startup.
    workspace_path: Option<std::path::PathBuf>,
    /// Directory/file names to hide from the sidebar tree.
    ignored_patterns: Vec<String>,
}

/// Escape a string for inclusion in a JSON string value.
fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Extracts a JSON string field value from a flat JSON text.
///
/// Finds `"key": "value"` and returns the unescaped value string.
fn extract_json_string_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let start = text.find(&needle)?;
    let after_key = &text[start + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim();
    if after_colon.starts_with('"') {
        let inner = &after_colon[1..];
        // Find closing quote that is not escaped
        let mut end = None;
        let mut prev_backslash = false;
        for (i, c) in inner.char_indices() {
            if c == '\\' {
                prev_backslash = !prev_backslash;
            } else {
                if c == '"' && !prev_backslash {
                    end = Some(i);
                    break;
                }
                prev_backslash = false;
            }
        }
        let raw = &inner[..end?];
        // Unescape \\ → \ and \" → "
        let unescaped = raw.replace("\\\\", "\x00").replace("\\\"", "\"").replace("\x00", "\\");
        Some(unescaped)
    } else {
        None
    }
}

/// Extracts a JSON array of strings field value from a flat JSON text.
///
/// Finds `"key": ["val1", "val2"]` and returns the values.
fn extract_json_string_array(text: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{}\"", key);
    let start = text.find(&needle)?;
    let after_key = &text[start + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim();
    if !after_colon.starts_with('[') {
        return None;
    }
    let bracket_content = &after_colon[1..];
    let close = bracket_content.find(']')?;
    let inner = &bracket_content[..close];

    let mut results = Vec::new();
    let mut remaining = inner;
    loop {
        remaining = remaining.trim();
        if remaining.is_empty() {
            break;
        }
        if !remaining.starts_with('"') {
            // Skip non-string tokens (e.g., commas if we're off)
            if let Some(next_quote) = remaining.find('"') {
                remaining = &remaining[next_quote..];
            } else {
                break;
            }
        }
        let inner_str = &remaining[1..];
        // Find closing quote
        let mut end = None;
        let mut prev_backslash = false;
        for (i, c) in inner_str.char_indices() {
            if c == '\\' {
                prev_backslash = !prev_backslash;
            } else {
                if c == '"' && !prev_backslash {
                    end = Some(i);
                    break;
                }
                prev_backslash = false;
            }
        }
        let end_idx = match end {
            Some(e) => e,
            None => break,
        };
        let raw = &inner_str[..end_idx];
        let unescaped = raw.replace("\\\\", "\x00").replace("\\\"", "\"").replace("\x00", "\\");
        results.push(unescaped);
        remaining = &inner_str[end_idx + 1..];
        // Skip comma
        remaining = remaining.trim_start_matches(',');
    }

    Some(results)
}

/// Loads full editor state from the platform config file.
///
/// Falls back gracefully: missing fields use sensible defaults.
fn load_state() -> AppState {
    let text = std::fs::read_to_string(config_state_path()).unwrap_or_default();

    let last_dir = extract_json_string_field(&text, "last_dir")
        .and_then(|p| {
            let pb = std::path::PathBuf::from(&p);
            if pb.exists() { Some(pb) } else { None }
        })
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());

    let workspace_path = extract_json_string_field(&text, "workspace_path")
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists());

    let ignored_patterns = extract_json_string_array(&text, "ignored_patterns")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![".git".to_string()]);

    AppState { last_dir, workspace_path, ignored_patterns }
}

/// Writes full editor state to the platform config file atomically.
///
/// Creates parent directories as needed. Write errors are silently ignored
/// (not worth crashing the editor over a preference write failure).
fn save_state(state: &AppState) {
    let state_path = config_state_path();
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let escaped_last = escape_json_string(&state.last_dir.to_string_lossy());

    let workspace_json = state.workspace_path.as_ref()
        .map(|p| format!("\"{}\"", escape_json_string(&p.to_string_lossy())))
        .unwrap_or_else(|| "null".to_string());

    let patterns_json = state.ignored_patterns.iter()
        .map(|p| format!("\"{}\"", escape_json_string(p)))
        .collect::<Vec<_>>()
        .join(", ");

    let json = format!(
        "{{\"last_dir\": \"{}\", \"workspace_path\": {}, \"ignored_patterns\": [{}]}}",
        escaped_last, workspace_json, patterns_json
    );
    let _ = std::fs::write(&state_path, json);
}

/// Returns the last opened workspace path if it exists on disk.
///
/// Used by `main.rs` to restore the workspace at startup before creating the adapter.
pub fn load_workspace_path() -> Option<std::path::PathBuf> {
    load_state().workspace_path
}

/// Adapter that wraps `core_editor::app::App` and implements `EditorDataSource`.
///
/// This is wgpu_client's only connection point to core_editor. All ora views
/// receive `&dyn EditorDataSource` and never see core_editor types directly.
pub struct CoreEditorAdapter {
    pub app: core_editor::app::App,
    /// Pending status message set by stub command handlers.
    /// Consumed and injected into `StatusPresentation` during `build_render_model`.
    pending_status_message: Option<String>,
    /// Last known viewport size from resize events, applied to new views.
    last_viewport: (usize, usize),
    /// Queued file operation to be processed by the event loop on the next frame.
    ///
    /// The event loop calls `take_pending_file_op()` each frame and opens the
    /// appropriate native dialog if `Some`. This is `None` most frames.
    pub pending_file_op: Option<PendingFileOp>,
    /// Guard flag that prevents concurrent file dialogs.
    ///
    /// Set to `true` by the event loop before spawning a dialog task and reset
    /// to `false` when the dialog completes. Commands that would open a dialog
    /// check this before setting `pending_file_op`.
    pub dialog_open: bool,
    /// Last directory visited in a file dialog, persisted across sessions.
    ///
    /// Loaded from the platform config directory at startup and updated whenever
    /// a file is opened or saved. Pre-populates future open/save dialogs.
    last_dir: std::path::PathBuf,
    /// Expiry instant for `pending_status_message`.
    ///
    /// Set to `Instant::now() + 3s` whenever `pending_status_message` is set.
    /// The event loop wakes at this instant to redraw and clear the message.
    status_message_expiry: Option<Instant>,
    /// Last opened workspace folder, persisted in state.json.
    ///
    /// Set when `open_directory()` is called. Restored at startup by main.rs
    /// via `load_workspace_path()`.
    workspace_path: Option<std::path::PathBuf>,
    /// Sidebar ignore patterns loaded from state.json.
    ///
    /// Passed to `Sidebar` so the file tree filters only the configured entries.
    /// Default: `[".git"]`.
    ignored_patterns: Vec<String>,
    /// Filesystem watcher debouncer — dropping stops watching.
    ///
    /// `None` when no workspace is open or the watcher failed to start.
    watcher: Option<Debouncer<notify_debouncer_full::notify::RecommendedWatcher, FileIdMap>>,
    /// Receiver for debounced filesystem events.
    ///
    /// Polled non-blocking via `try_recv()` in `poll_watcher_events()`.
    watcher_rx: Option<mpsc::Receiver<DebounceEventResult>>,
    /// Paths of files that have been deleted externally while open in the editor.
    ///
    /// Used in `build_render_model` to set `is_deleted = true` on the corresponding tab.
    /// Entries remain until the tab is closed.
    deleted_paths: std::collections::HashSet<std::path::PathBuf>,
    /// Paths of open files that were modified externally while the buffer was dirty.
    ///
    /// On each render cycle, the first entry is shown as `ExternalModificationPrompt`.
    /// Cleared when the user responds (reload or keep) via `respond_external_modification_prompt`.
    pending_reload_prompts: Vec<std::path::PathBuf>,
    /// Paths from watcher events collected by `poll_watcher_events` for processing.
    ///
    /// Drained by `handle_external_file_changes` after the watcher poll pass.
    pending_external_changes: Vec<std::path::PathBuf>,
}

impl CoreEditorAdapter {
    /// Create an adapter wrapping a new empty document.
    pub fn new() -> Self {
        let state = load_state();
        Self {
            app: core_editor::app::App::new(),
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: state.last_dir,
            status_message_expiry: None,
            workspace_path: state.workspace_path,
            ignored_patterns: state.ignored_patterns,
            watcher: None,
            watcher_rx: None,
            deleted_paths: std::collections::HashSet::new(),
            pending_reload_prompts: Vec::new(),
            pending_external_changes: Vec::new(),
        }
    }

    /// Create an adapter that opens the given file path.
    pub fn open_file(path: &str) -> std::io::Result<Self> {
        let state = load_state();
        Ok(Self {
            app: core_editor::app::App::open_file(path)?,
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: state.last_dir,
            status_message_expiry: None,
            workspace_path: state.workspace_path,
            ignored_patterns: state.ignored_patterns,
            watcher: None,
            watcher_rx: None,
            deleted_paths: std::collections::HashSet::new(),
            pending_reload_prompts: Vec::new(),
            pending_external_changes: Vec::new(),
        })
    }

    /// Create an adapter that opens a directory (shows sidebar).
    ///
    /// Persists the opened workspace path to state.json and configures the
    /// sidebar with `ignored_patterns` from state. First-level directories
    /// are auto-expanded via `Sidebar::new_with_patterns`.
    pub fn open_directory(path: &str) -> std::io::Result<Self> {
        let state = load_state();
        let canonical = std::fs::canonicalize(std::path::Path::new(path))?;

        // Build the App with ignored_patterns applied to the sidebar
        let mut app = core_editor::app::App::new();
        app.sidebar = core_editor::view::Sidebar::new_with_patterns(
            Some(canonical.clone()),
            state.ignored_patterns.clone(),
        );
        app.sidebar.visible = true;
        app.focus = core_editor::view::FocusState::Sidebar;

        let new_state = AppState {
            last_dir: state.last_dir.clone(),
            workspace_path: Some(canonical.clone()),
            ignored_patterns: state.ignored_patterns.clone(),
        };
        save_state(&new_state);

        let mut adapter = Self {
            app,
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: state.last_dir,
            status_message_expiry: None,
            workspace_path: Some(canonical.clone()),
            ignored_patterns: state.ignored_patterns,
            watcher: None,
            watcher_rx: None,
            deleted_paths: std::collections::HashSet::new(),
            pending_reload_prompts: Vec::new(),
            pending_external_changes: Vec::new(),
        };
        adapter.create_watcher(&canonical);
        Ok(adapter)
    }

    /// Creates and starts a debounced filesystem watcher on the given path.
    ///
    /// Drops any existing watcher first. A 300ms debounce window is used so
    /// that rapid events (e.g. during `cargo build`) are coalesced into a
    /// single sidebar refresh instead of causing continuous redraws.
    fn create_watcher(&mut self, path: &std::path::Path) {
        // Drop existing watcher first
        self.watcher = None;
        self.watcher_rx = None;

        let (tx, rx) = mpsc::channel::<DebounceEventResult>();
        match new_debouncer(
            std::time::Duration::from_millis(300),
            None,
            move |res: DebounceEventResult| { let _ = tx.send(res); },
        ) {
            Ok(mut debouncer) => {
                if let Err(e) = debouncer.watch(path, RecursiveMode::Recursive) {
                    log::warn!("Failed to watch {:?}: {:?}", path, e);
                    return;
                }
                log::info!("Filesystem watcher started for {:?}", path);
                self.watcher = Some(debouncer);
                self.watcher_rx = Some(rx);
            }
            Err(e) => {
                log::warn!("Failed to create filesystem watcher: {:?}", e);
            }
        }
    }

}

impl FileOpDataSource for CoreEditorAdapter {
    fn take_pending_file_op(&mut self) -> Option<PendingFileOp> {
        self.pending_file_op.take()
    }

    fn handle_file_loaded(&mut self, path: std::path::PathBuf, content: String) {
        // Update last_dir to this file's parent directory.
        if let Some(parent) = path.parent() {
            self.last_dir = parent.to_path_buf();
            save_state(&AppState {
                last_dir: self.last_dir.clone(),
                workspace_path: self.workspace_path.clone(),
                ignored_patterns: self.ignored_patterns.clone(),
            });
        }

        let (doc_id, was_existing) = self.app.workspace.open_document_with_content(path, content);

        if was_existing {
            // File already open — find its view and activate it.
            let view_id = self.app.workspace
                .views()
                .find(|(_, v)| v.document_id() == doc_id)
                .map(|(id, _)| *id);
            if let Some(view_id) = view_id {
                self.app.workspace.set_active_view(view_id);
            }
        } else {
            // New document — create a view and activate it.
            self.app.workspace.create_view(doc_id);
            // Apply stored viewport size to the new view.
            let (w, h) = self.last_viewport;
            self.app.check_scrolling(w, h);
        }

        self.dialog_open = false;
        self.app.needs_render = true;
    }

    fn handle_file_error(&mut self, message: String) {
        self.pending_status_message = Some(message);
        self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
        self.dialog_open = false;
        self.app.needs_render = true;
    }

    fn is_dialog_open(&self) -> bool {
        self.dialog_open
    }

    fn set_dialog_open(&mut self, open: bool) {
        self.dialog_open = open;
    }

    fn handle_file_saved(&mut self, path: std::path::PathBuf) {
        // Write the file to disk via save_as (sets file_path, clears dirty).
        match self.app.save_as(path.to_str().unwrap_or_default()) {
            Ok(()) => {
                self.pending_status_message = Some("File saved".to_string());
                self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
                // Register the new path in the buffer registry.
                self.app.workspace.register_path_for_active_doc(&path);
                // Update last_dir.
                if let Some(parent) = path.parent() {
                    self.last_dir = parent.to_path_buf();
                    save_state(&AppState {
                        last_dir: self.last_dir.clone(),
                        workspace_path: self.workspace_path.clone(),
                        ignored_patterns: self.ignored_patterns.clone(),
                    });
                }
            }
            Err(e) => {
                self.pending_status_message = Some(format!("Save failed: {}", e));
                self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
            }
        }
        self.app.needs_render = true;
    }

    fn active_doc_has_path(&self) -> bool {
        self.app.workspace
            .active_document()
            .and_then(|d| d.file_path())
            .is_some()
    }

    fn save_active_doc(&mut self) -> Result<bool, String> {
        match self.app.save() {
            Ok(true) => {
                self.app.needs_render = true;
                Ok(true)
            }
            Ok(false) => Ok(false),
            Err(e) => {
                let msg = format!("Save failed: {}", e);
                self.pending_status_message = Some(msg.clone());
                self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
                self.app.needs_render = true;
                Err(msg)
            }
        }
    }

    fn last_opened_directory(&self) -> std::path::PathBuf {
        self.last_dir.clone()
    }

    fn status_message_expiry(&self) -> Option<Instant> {
        self.status_message_expiry
    }

    fn handle_folder_opened(&mut self, path: std::path::PathBuf) {
        let canonical = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(e) => {
                self.pending_status_message = Some(format!("Cannot open folder: {}", e));
                self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
                self.app.needs_render = true;
                return;
            }
        };

        // Close all open tabs before switching workspace.
        // This avoids having tabs from the old workspace visible in the new one.
        // Close all views by repeatedly closing the active tab.
        loop {
            let view_count = self.app.workspace.view_count();
            if view_count == 0 {
                break;
            }
            // close_active_tab may not close if protection dialog fires, but since
            // we're switching workspaces we don't prompt here. Limit loop iterations
            // to avoid infinite loop if close fails.
            let views_before = view_count;
            self.app.close_active_tab();
            if self.app.workspace.view_count() == views_before {
                // Tab count didn't decrease (e.g., unsaved changes protection).
                // Force-close: just clear all views and documents.
                break;
            }
        }
        // Clear all external-change tracking since we're switching workspace.
        self.deleted_paths.clear();
        self.pending_reload_prompts.clear();
        self.pending_external_changes.clear();

        // Update sidebar with the new base directory (also calls auto_expand_first_level).
        self.app.sidebar.set_base_directory(canonical.clone());
        self.app.sidebar.show();

        // Persist new workspace path.
        self.workspace_path = Some(canonical.clone());
        save_state(&AppState {
            last_dir: self.last_dir.clone(),
            workspace_path: self.workspace_path.clone(),
            ignored_patterns: self.ignored_patterns.clone(),
        });

        // Restart the filesystem watcher on the new workspace root.
        self.create_watcher(&canonical);

        self.app.needs_render = true;
    }

    fn poll_watcher_events(&mut self) -> bool {
        let rx = match &self.watcher_rx {
            Some(rx) => rx,
            None => return false,
        };
        let mut had_events = false;
        loop {
            match rx.try_recv() {
                Ok(Ok(events)) => {
                    had_events = true;
                    // Collect unique changed paths for external-change processing.
                    for event in &events {
                        for path in &event.paths {
                            let canonical = std::fs::canonicalize(path)
                                .unwrap_or_else(|_| path.clone());
                            if !self.pending_external_changes.contains(&canonical) {
                                self.pending_external_changes.push(canonical);
                            }
                        }
                    }
                }
                Ok(Err(errors)) => {
                    for e in errors {
                        log::warn!("Filesystem watcher error: {:?}", e);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    log::warn!("Filesystem watcher channel disconnected");
                    break;
                }
            }
        }
        if had_events {
            self.app.sidebar.mark_tree_dirty();
            self.app.needs_render = true;
        }
        had_events
    }

    fn start_watcher(&mut self, path: std::path::PathBuf) {
        self.create_watcher(&path);
    }

    fn stop_watcher(&mut self) {
        self.watcher = None;
        self.watcher_rx = None;
        self.pending_external_changes.clear();
    }

    fn handle_external_file_changes(&mut self) {
        if self.pending_external_changes.is_empty() {
            return;
        }

        let changed_paths = std::mem::take(&mut self.pending_external_changes);

        for path in changed_paths {
            // Check if this path corresponds to an open document.
            let doc_id = self.app.workspace
                .documents()
                .find(|(_, doc)| {
                    doc.file_path()
                        .map(|p| {
                            let canonical = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                            canonical == path
                        })
                        .unwrap_or(false)
                })
                .map(|(id, _)| *id);

            let doc_id = match doc_id {
                Some(id) => id,
                None => continue, // Not an open file — sidebar-only change
            };

            if !path.exists() {
                // File was deleted externally.
                log::info!("External deletion detected: {:?}", path);
                self.deleted_paths.insert(path);
                self.app.needs_render = true;
            } else {
                // File was modified externally.
                let is_dirty = self.app.workspace
                    .document(doc_id)
                    .map(|d| d.is_dirty())
                    .unwrap_or(false);

                if !is_dirty {
                    // Clean buffer: auto-reload silently.
                    log::info!("Auto-reloading clean buffer from disk: {:?}", path);
                    match std::fs::read_to_string(&path) {
                        Ok(mut content) => {
                            // Strip UTF-8 BOM if present (same pattern as file loading).
                            if content.starts_with('\u{FEFF}') {
                                content = content[3..].to_string();
                            }
                            if let Some(doc) = self.app.workspace.document_mut(doc_id) {
                                doc.reload_content(&content);
                            }
                            self.app.needs_render = true;
                        }
                        Err(e) => {
                            log::warn!("Failed to reload {:?}: {:?}", path, e);
                        }
                    }
                } else {
                    // Dirty buffer: queue a prompt if not already queued.
                    log::info!("Queueing external modification prompt for dirty buffer: {:?}", path);
                    if !self.pending_reload_prompts.contains(&path) {
                        self.pending_reload_prompts.push(path);
                        self.app.needs_render = true;
                    }
                }
            }
        }
    }

    fn respond_external_modification_prompt(&mut self, reload: bool) {
        // Take the first pending prompt.
        if self.pending_reload_prompts.is_empty() {
            return;
        }
        let path = self.pending_reload_prompts.remove(0);

        if reload {
            // Reload from disk, discarding local edits.
            let doc_id = self.app.workspace
                .documents()
                .find(|(_, doc)| {
                    doc.file_path()
                        .map(|p| {
                            let canonical = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                            canonical == path
                        })
                        .unwrap_or(false)
                })
                .map(|(id, _)| *id);

            if let Some(doc_id) = doc_id {
                match std::fs::read_to_string(&path) {
                    Ok(mut content) => {
                        if content.starts_with('\u{FEFF}') {
                            content = content[3..].to_string();
                        }
                        if let Some(doc) = self.app.workspace.document_mut(doc_id) {
                            doc.reload_content(&content);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to reload {:?} after user confirmed: {:?}", path, e);
                    }
                }
            }
        }
        // "Keep" case: just remove from queue (local edits preserved).

        self.app.needs_render = true;
    }
}

impl Default for CoreEditorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Type conversion functions: core_editor view_model → ora::editor_adapter
//
// Both crates are external, so `From` impls violate the orphan rule.
// These free functions serve the same purpose.
// =============================================================================

fn convert_text_style(s: core_editor::view_model::TextStyle) -> TextStyle {
    use core_editor::view_model::TextStyle as S;
    match s {
        S::Normal => TextStyle::Normal,
        S::Selection => TextStyle::Selection,
        S::Keyword => TextStyle::Keyword,
        S::String => TextStyle::String,
        S::Number => TextStyle::Number,
        S::Comment => TextStyle::Comment,
        S::Type => TextStyle::Type,
        S::Function => TextStyle::Function,
        S::Variable => TextStyle::Variable,
        S::Operator => TextStyle::Operator,
        S::Punctuation => TextStyle::Punctuation,
        S::Constant => TextStyle::Constant,
        S::Module => TextStyle::Module,
        S::Attribute => TextStyle::Attribute,
        S::Macro => TextStyle::Macro,
        S::LineNumber => TextStyle::LineNumber,
        S::CurrentLineNumber => TextStyle::CurrentLineNumber,
        S::Error => TextStyle::Error,
        S::Warning => TextStyle::Warning,
    }
}

fn convert_visual_position(p: core_editor::view_model::VisualPosition) -> VisualPosition {
    VisualPosition { row: p.row, column: p.column }
}

fn convert_styled_span(s: core_editor::view_model::StyledSpan) -> StyledSpan {
    StyledSpan { text: s.text, style: convert_text_style(s.style) }
}

fn convert_line_presentation(l: core_editor::view_model::LinePresentation) -> LinePresentation {
    let spans: Vec<StyledSpan> = l.spans.into_iter().map(convert_styled_span).collect();

    // Extract selection ranges from Selection-styled spans before they
    // reach the text layer. Column offsets are character-based (0-indexed).
    let mut selection_ranges: Vec<(usize, usize)> = Vec::new();
    let mut col: usize = 0;
    for span in &spans {
        let span_len = span.text.chars().count();
        if span.style == TextStyle::Selection {
            selection_ranges.push((col, col + span_len));
        }
        col += span_len;
    }

    LinePresentation {
        line_number: l.line_number,
        is_current_line: l.is_current_line,
        spans,
        selection_ranges,
    }
}

fn convert_gutter_model(g: core_editor::view_model::GutterModel) -> GutterModel {
    GutterModel { width: g.width, visible: g.visible, total_lines: g.total_lines }
}

fn convert_status_presentation(s: core_editor::view_model::StatusPresentation) -> StatusPresentation {
    StatusPresentation {
        title: s.title,
        dirty: s.dirty,
        cursor_line: s.cursor_line,
        cursor_column: s.cursor_column,
        language: s.language,
        total_lines: s.total_lines,
        message: s.message,
    }
}

fn convert_caret_presentation(c: core_editor::view_model::CaretPresentation) -> CaretPresentation {
    CaretPresentation {
        position: convert_visual_position(c.position),
        visible: c.visible,
    }
}

fn convert_dialog_presentation(d: core_editor::view_model::DialogPresentation) -> DialogPresentation {
    use core_editor::view_model::DialogPresentation as D;
    match d {
        D::None => DialogPresentation::None,
        D::UnsavedChangesConfirmation { action_description, unsaved_documents } => {
            DialogPresentation::UnsavedChangesConfirmation {
                action_description,
                unsaved_documents,
            }
        }
    }
    // Note: ExternalModificationPrompt is injected directly in build_render_model
    // after conversion since it lives purely in the adapter layer.
}

fn convert_tab_presentation(t: core_editor::view_model::TabPresentation) -> TabPresentation {
    TabPresentation {
        view_id: t.view_id,
        title: t.title,
        is_active: t.is_active,
        is_dirty: t.is_dirty,
        is_deleted: t.is_deleted,
    }
}

fn convert_tab_bar_presentation(t: core_editor::view_model::TabBarPresentation) -> TabBarPresentation {
    TabBarPresentation {
        tabs: t.tabs.into_iter().map(convert_tab_presentation).collect(),
        visible: t.visible,
    }
}

fn convert_file_tree_node(n: core_editor::view_model::FileTreeNode) -> ora::editor_adapter::FileTreeNode {
    ora::editor_adapter::FileTreeNode {
        name: n.name,
        path: n.path,
        extension: n.extension,
        is_dir: n.is_dir,
        is_expanded: n.is_expanded,
        is_generated: n.is_generated,
        children: n.children.into_iter().map(convert_file_tree_node).collect(),
    }
}

fn convert_sidebar_presentation(s: core_editor::view_model::SidebarPresentation) -> SidebarPresentation {
    SidebarPresentation {
        visible: s.visible,
        focused: s.focused,
        directory_name: s.directory_name,
        entries: s.entries.into_iter().map(|e| FileEntryPresentation {
            name: e.name,
            path: e.path,
            is_dir: e.is_dir,
            is_selected: e.is_selected,
        }).collect(),
        tree: s.tree.into_iter().map(convert_file_tree_node).collect(),
        width: s.width,
    }
}

fn convert_render_model(m: core_editor::view_model::RenderModel) -> RenderModel {
    RenderModel {
        visible_lines: m.visible_lines.into_iter().map(convert_line_presentation).collect(),
        gutter: convert_gutter_model(m.gutter),
        caret: convert_caret_presentation(m.caret),
        status: convert_status_presentation(m.status),
        tab_bar: convert_tab_bar_presentation(m.tab_bar),
        dialog: convert_dialog_presentation(m.dialog),
        sidebar: convert_sidebar_presentation(m.sidebar),
        scroll_x: m.scroll_x,
        scroll_y: m.scroll_y,
        // Sub-line offset is managed by ora's event loop, not core_editor.
        // The adapter always sets this to 0.0; EditorRootView overwrites it
        // from the shared scroll offset cell.
        scroll_y_offset_px: 0.0,
        // Focus state is managed by ora's event loop, not core_editor.
        // EditorRootView overwrites this from the shared focus state cell.
        editor_focused: true,
    }
}

// =============================================================================
// EditorCommand conversion: ora EditorCommand → core_editor EditorCommand
// =============================================================================

fn to_core_direction(d: CursorDirection) -> Direction {
    match d {
        CursorDirection::Left => Direction::Left,
        CursorDirection::Right => Direction::Right,
        CursorDirection::Up => Direction::Up,
        CursorDirection::Down => Direction::Down,
    }
}

fn to_core_scope(s: OraMoveScope) -> CoreMoveScope {
    match s {
        OraMoveScope::Char => CoreMoveScope::Char,
        OraMoveScope::Word => CoreMoveScope::Word,
        OraMoveScope::Line => CoreMoveScope::Line,
        OraMoveScope::Page => CoreMoveScope::Page,
        OraMoveScope::Document => CoreMoveScope::Document,
    }
}

fn to_core_command(cmd: EditorCommand) -> Option<CoreEditorCommand> {
    use EditorCommand::*;
    Some(match cmd {
        MoveCursor { direction, scope, extend_selection } => CoreEditorCommand::MoveCursor {
            direction: to_core_direction(direction),
            scope: to_core_scope(scope),
            extend_selection,
        },
        InsertChar(c) => CoreEditorCommand::InsertChar(c),
        InsertText(s) => CoreEditorCommand::InsertText(s),
        InsertNewline => CoreEditorCommand::InsertNewline,
        Backspace => CoreEditorCommand::Backspace,
        Delete => CoreEditorCommand::Delete,
        Copy => CoreEditorCommand::Copy,
        Cut => CoreEditorCommand::Cut,
        Paste => CoreEditorCommand::Paste,
        SelectAll => CoreEditorCommand::SelectAll,
        Undo => CoreEditorCommand::Undo,
        Redo => CoreEditorCommand::Redo,
        Save => {
            // Save is handled at app level via app.save(); return None here.
            return None;
        }
        ToggleLineNumbers => CoreEditorCommand::ToggleLineNumbers,
        Scroll(lines) => CoreEditorCommand::Scroll { lines },
        ClickAt { line, col, extend_selection, click_count } =>
            CoreEditorCommand::ClickAt { line, col, extend_selection, click_count },
        DragTo { line, col, snap_mode } =>
            CoreEditorCommand::DragTo { line, col, snap_mode },
        GutterClickAt { line } =>
            CoreEditorCommand::GutterClickAt { line },
        // v2 commands — handled before to_core_command is called,
        // but listed here for exhaustiveness.
        SaveAs | OpenFile | OpenFolder | New | CloseTab | SwitchTab(_) | SwitchTabPrev
        | Find | Replace | ReplaceAll | GoToLine | OpenSidebarFile(_)
        | ToggleSidebarDir(_) => return None,
    })
}

// =============================================================================
// Sub-trait implementations (FIX-04)
// EditorDataSource is satisfied automatically by the blanket impl in ora.
// =============================================================================

impl BufferDataSource for CoreEditorAdapter {
    fn build_render_model(&self, viewport_lines: usize) -> RenderModel {
        // build_render_model requires &mut self on core_editor::App (it clears
        // status_message after building). We cast away const temporarily.
        //
        // SAFETY: Single-threaded GUI application. No other reference to self
        // exists during this call. The mutations are limited to clearing
        // Option<String> fields — no structural aliasing issues.
        let app = unsafe {
            &mut (*(self as *const CoreEditorAdapter as *mut CoreEditorAdapter)).app
        };
        let core_model = app.build_render_model(viewport_lines);
        let mut render_model = convert_render_model(core_model);

        // Inject pending status message from stub handlers, overriding core's message.
        // Message persists until its expiry instant (3 seconds after being set).
        //
        // SAFETY: Single-threaded GUI application. No other reference exists during
        // this call. Mutations are limited to clearing Option<String/Instant> fields.
        let pending = unsafe {
            &mut (*(self as *const CoreEditorAdapter as *mut CoreEditorAdapter)).pending_status_message
        };
        let expiry = unsafe {
            &mut (*(self as *const CoreEditorAdapter as *mut CoreEditorAdapter)).status_message_expiry
        };
        if let Some(ref msg) = *pending {
            if expiry.map_or(false, |exp| Instant::now() < exp) {
                render_model.status.message = Some(msg.clone());
            } else {
                *pending = None;
                *expiry = None;
            }
        }

        // Mark tabs whose file was deleted externally.
        if !self.deleted_paths.is_empty() {
            for tab in &mut render_model.tab_bar.tabs {
                // Find the document for this view_id, check if its path is deleted.
                let view_id = tab.view_id;
                let is_del = app.workspace
                    .views()
                    .find(|(id, _)| id.as_u64() == view_id)
                    .and_then(|(_, view)| app.workspace.document(view.document_id()))
                    .and_then(|doc| doc.file_path())
                    .map(|p| {
                        let canonical = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                        self.deleted_paths.contains(&canonical)
                    })
                    .unwrap_or(false);
                tab.is_deleted = is_del;
            }
        }

        // Show ExternalModificationPrompt for the first pending dirty-buffer reload.
        // This overrides the core dialog only when there is no core dialog active.
        if matches!(render_model.dialog, ora::editor_adapter::DialogPresentation::None) {
            if let Some(path) = self.pending_reload_prompts.first() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("(unknown)")
                    .to_string();
                render_model.dialog = ora::editor_adapter::DialogPresentation::ExternalModificationPrompt {
                    file_name,
                };
            }
        }

        render_model
    }

    fn resize_viewport(&mut self, width_chars: usize, height_lines: usize) {
        self.last_viewport = (width_chars, height_lines);
        self.app.check_scrolling(width_chars, height_lines);
    }

    fn viewport_lines(&self) -> usize {
        self.app.workspace
            .active_view()
            .map(|v| v.viewport.height)
            .filter(|&h| h > 0)
            .unwrap_or(self.last_viewport.1.max(1))
    }

    fn scroll_y(&self) -> usize {
        self.app.workspace
            .active_view()
            .map(|v| v.viewport.scroll_y)
            .unwrap_or(0)
    }

    fn total_lines(&self) -> usize {
        self.app.workspace
            .active_document()
            .map(|d| d.len_lines())
            .unwrap_or(1)
    }

    fn sidebar_width_px(&self) -> f32 {
        if self.app.sidebar.visible {
            // Must match SIDEBAR_DEFAULT_WIDTH from ora::views::sidebar.
            480.0
        } else {
            0.0
        }
    }

    fn gutter_width_chars(&self) -> usize {
        let total_lines = self.app.workspace
            .active_document()
            .map(|d| d.len_lines())
            .unwrap_or(1);
        let digits = total_lines.to_string().len();
        digits + 2 // digits + space + separator
    }

    fn scroll_x(&self) -> usize {
        self.app.workspace
            .active_view()
            .map(|v| v.viewport.scroll_x)
            .unwrap_or(0)
    }
}

impl CommandDispatcher for CoreEditorAdapter {
    fn dispatch_command(&mut self, cmd: EditorCommand) {
        // Save is handled by the event loop via PendingFileOp::Save so it can
        // decide between silent save (named file) and Save As dialog (untitled).
        if matches!(cmd, EditorCommand::Save) {
            if !self.dialog_open {
                self.pending_file_op = Some(PendingFileOp::Save);
            }
            return;
        }

        // Tab management commands — wired to real App handlers.
        match &cmd {
            EditorCommand::SwitchTab(0) => {
                // Ctrl+Tab: cycle to next tab in visual order.
                self.app.switch_tab_relative(1);
                return;
            }
            EditorCommand::SwitchTab(view_id) => {
                // Click on a specific tab or Ctrl+1..9 direct switch.
                self.app.switch_tab(*view_id);
                return;
            }
            EditorCommand::SwitchTabPrev => {
                // Ctrl+Shift+Tab: cycle to previous tab in visual order.
                self.app.switch_tab_relative(-1);
                return;
            }
            EditorCommand::CloseTab => {
                // Ctrl+W: close active tab with dirty-buffer protection.
                self.app.close_active_tab();
                return;
            }
            EditorCommand::New => {
                // Ctrl+N: create a new untitled buffer and switch to it.
                self.app.new_untitled();
                // Apply stored viewport size to the newly created view.
                let (w, h) = self.last_viewport;
                self.app.check_scrolling(w, h);
                return;
            }
            EditorCommand::OpenFile => {
                // Ctrl+O: queue an Open dialog for the event loop to handle.
                if !self.dialog_open {
                    self.pending_file_op = Some(PendingFileOp::Open);
                }
                return;
            }
            EditorCommand::SaveAs => {
                // Ctrl+Shift+S: queue a Save As dialog for the event loop to handle.
                if !self.dialog_open {
                    self.pending_file_op = Some(PendingFileOp::SaveAs);
                }
                return;
            }
            EditorCommand::OpenFolder => {
                // Ctrl+Shift+O: queue an Open Folder dialog for the event loop to handle.
                if !self.dialog_open {
                    self.pending_file_op = Some(PendingFileOp::OpenFolder);
                }
                return;
            }
            EditorCommand::OpenSidebarFile(path) => {
                // Double-click on file in sidebar: open in editor.
                let path = std::path::PathBuf::from(path);
                let _ = self.app.request_open_file(path);
                // Apply stored viewport size to the newly created view
                let (w, h) = self.last_viewport;
                self.app.check_scrolling(w, h);
                return;
            }
            EditorCommand::ToggleSidebarDir(path) => {
                // Click on directory in sidebar: expand/collapse in tree.
                let path_buf = std::path::PathBuf::from(path);
                log::info!("ToggleSidebarDir: {:?}, was_expanded={}", path_buf, self.app.sidebar.is_expanded(&path_buf));
                self.app.sidebar.toggle_dir(&path_buf);
                log::info!("ToggleSidebarDir: now_expanded={}", self.app.sidebar.is_expanded(&path_buf));
                self.app.needs_render = true;
                return;
            }
            _ => {}
        }

        // Stub handlers for v2 commands not yet implemented.
        let stub_msg = match &cmd {
            EditorCommand::Find => Some("Find: not yet available"),
            EditorCommand::Replace => Some("Replace: not yet available"),
            EditorCommand::ReplaceAll => Some("Replace All: not yet available"),
            EditorCommand::GoToLine => Some("Go to Line: not yet available"),
            _ => None,
        };
        if let Some(msg) = stub_msg {
            self.pending_status_message = Some(msg.to_string());
            self.status_message_expiry = Some(Instant::now() + std::time::Duration::from_secs(3));
            return;
        }

        if let Some(core_cmd) = to_core_command(cmd) {
            self.app.dispatch(core_cmd);
        }
    }
}

impl WindowDataSource for CoreEditorAdapter {
    fn window_title(&self) -> String {
        let doc_title = self.app.workspace
            .active_document()
            .and_then(|d| d.file_path())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Muda")
            .to_string();

        if self.app.dirty() {
            format!("*{} - Muda", doc_title)
        } else {
            format!("{} - Muda", doc_title)
        }
    }
}
// EditorDataSource is satisfied automatically by the blanket impl in ora.
