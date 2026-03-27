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

/// Loads the last-used directory from persistent state.
///
/// Returns the stored path if it exists on disk, otherwise falls back to
/// the user's home directory.
fn load_last_dir() -> std::path::PathBuf {
    let state_path = config_state_path();
    if let Ok(text) = std::fs::read_to_string(&state_path) {
        // Minimal JSON parse: find "last_dir": "..." without pulling in serde.
        if let Some(start) = text.find("\"last_dir\"") {
            let after_key = &text[start + 10..];
            if let Some(colon) = after_key.find(':') {
                let after_colon = after_key[colon + 1..].trim();
                if after_colon.starts_with('"') {
                    let inner = &after_colon[1..];
                    if let Some(end) = inner.find('"') {
                        let raw = &inner[..end];
                        // Unescape backslashes and quotes stored as \\ and \"
                        let unescaped = raw.replace("\\\\", "\x00").replace("\\\"", "\"").replace("\x00", "\\");
                        let p = std::path::PathBuf::from(&unescaped);
                        if p.exists() {
                            return p;
                        }
                    }
                }
            }
        }
    }
    dirs::home_dir().unwrap_or_default()
}

/// Writes the last-used directory to persistent state.
///
/// Creates parent directories as needed. Write errors are silently ignored
/// (not worth crashing the editor over a preference write failure).
fn save_last_dir(dir: &std::path::Path) {
    let state_path = config_state_path();
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Manually escape the path string for JSON.
    let raw = dir.to_string_lossy();
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    let json = format!("{{\"last_dir\": \"{}\"}}", escaped);
    let _ = std::fs::write(&state_path, json);
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
}

impl CoreEditorAdapter {
    /// Create an adapter wrapping a new empty document.
    pub fn new() -> Self {
        Self {
            app: core_editor::app::App::new(),
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: load_last_dir(),
            status_message_expiry: None,
        }
    }

    /// Create an adapter that opens the given file path.
    pub fn open_file(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            app: core_editor::app::App::open_file(path)?,
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: load_last_dir(),
            status_message_expiry: None,
        })
    }

    /// Create an adapter that opens a directory (shows sidebar).
    pub fn open_directory(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            app: core_editor::app::App::open_directory(path)?,
            pending_status_message: None,
            last_viewport: (200, 40),
            pending_file_op: None,
            dialog_open: false,
            last_dir: load_last_dir(),
            status_message_expiry: None,
        })
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
            save_last_dir(parent);
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
                    save_last_dir(parent);
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
}

fn convert_tab_presentation(t: core_editor::view_model::TabPresentation) -> TabPresentation {
    TabPresentation {
        view_id: t.view_id,
        title: t.title,
        is_active: t.is_active,
        is_dirty: t.is_dirty,
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
        SaveAs | OpenFile | New | CloseTab | SwitchTab(_) | SwitchTabPrev
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
