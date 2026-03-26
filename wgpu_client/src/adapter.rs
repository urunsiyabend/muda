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
use ora::editor_adapter::{
    BufferDataSource, CaretPresentation, CommandDispatcher, CursorDirection, DialogPresentation,
    EditorCommand, FileEntryPresentation, GutterModel, LinePresentation,
    MoveScope as OraMoveScope, RenderModel, SidebarPresentation, StyledSpan, StatusPresentation,
    TabBarPresentation, TabPresentation, TextStyle, VisualPosition, WindowDataSource,
};

// =============================================================================
// CoreEditorAdapter struct
// =============================================================================

/// Adapter that wraps `core_editor::app::App` and implements `EditorDataSource`.
///
/// This is wgpu_client's only connection point to core_editor. All ora views
/// receive `&dyn EditorDataSource` and never see core_editor types directly.
pub struct CoreEditorAdapter {
    pub app: core_editor::app::App,
    /// Pending status message set by stub command handlers.
    /// Consumed and injected into `StatusPresentation` during `build_render_model`.
    pending_status_message: Option<String>,
}

impl CoreEditorAdapter {
    /// Create an adapter wrapping a new empty document.
    pub fn new() -> Self {
        Self { app: core_editor::app::App::new(), pending_status_message: None }
    }

    /// Create an adapter that opens the given file path.
    pub fn open_file(path: &str) -> std::io::Result<Self> {
        Ok(Self { app: core_editor::app::App::open_file(path)?, pending_status_message: None })
    }

    /// Create an adapter that opens a directory (shows sidebar).
    pub fn open_directory(path: &str) -> std::io::Result<Self> {
        Ok(Self { app: core_editor::app::App::open_directory(path)?, pending_status_message: None })
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

/// Convert core_editor's flat sidebar to ora's SidebarPresentation (flat entry list).
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
        let pending = unsafe {
            &mut (*(self as *const CoreEditorAdapter as *mut CoreEditorAdapter)).pending_status_message
        };
        if let Some(msg) = pending.take() {
            render_model.status.message = Some(msg);
        }
        render_model
    }

    fn resize_viewport(&mut self, width_chars: usize, height_lines: usize) {
        self.app.check_scrolling(width_chars, height_lines);
    }

    fn viewport_lines(&self) -> usize {
        self.app.workspace
            .active_view()
            .map(|v| v.viewport.height)
            .filter(|&h| h > 0)
            .unwrap_or(40)
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
}

impl CommandDispatcher for CoreEditorAdapter {
    fn dispatch_command(&mut self, cmd: EditorCommand) {
        // Save is an app-level operation that doesn't go through the dispatcher.
        if matches!(cmd, EditorCommand::Save) {
            let _ = self.app.save();
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
            EditorCommand::OpenSidebarFile(path) => {
                // Double-click on file in sidebar: open in editor.
                let path = std::path::PathBuf::from(path);
                let _ = self.app.request_open_file(path);
                return;
            }
            EditorCommand::ToggleSidebarDir(path) => {
                // Click on directory in sidebar: navigate into it.
                let path = std::path::PathBuf::from(path);
                self.app.sidebar.set_base_directory(path);
                self.app.needs_render = true;
                return;
            }
            _ => {}
        }

        // Stub handlers for v2 commands not yet implemented.
        let stub_msg = match &cmd {
            EditorCommand::SaveAs => Some("Save As: not yet available"),
            EditorCommand::OpenFile => Some("Open File: not yet available"),
            EditorCommand::New => Some("New File: not yet available"),
            EditorCommand::Find => Some("Find: not yet available"),
            EditorCommand::Replace => Some("Replace: not yet available"),
            EditorCommand::ReplaceAll => Some("Replace All: not yet available"),
            EditorCommand::GoToLine => Some("Go to Line: not yet available"),
            _ => None,
        };
        if let Some(msg) = stub_msg {
            self.pending_status_message = Some(msg.to_string());
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
