//! ViewModelBuilder: Constructs RenderModel from domain state.
//!
//! This module contains the projection logic that transforms
//! Document + EditorView state into a render-ready RenderModel.

use std::path::PathBuf;

use crate::domain::{Document, DocumentId, ProtectionError, TextPosition};
use crate::syntax::{HighlightSpan, SyntaxHighlighter};
use crate::view::{EditorView, FocusState, Sidebar};
use crate::view_model::{
    CaretPresentation, DialogPresentation, FileEntryPresentation, GutterModel, LinePresentation,
    RenderModel, SidebarPresentation, StatusPresentation, StyledSpan, TabBarPresentation,
    TabPresentation, VisualPosition,
};

/// Represents a pending action that requires user confirmation.
#[derive(Clone, Debug)]
pub enum PendingAction {
    /// Exit the application.
    Exit,
    /// Close a specific document.
    CloseDocument(DocumentId),
    /// Open a file (potentially switching away from unsaved document).
    OpenFile(PathBuf),
}

impl PendingAction {
    /// Returns a human-readable description of the action.
    pub fn description(&self) -> &'static str {
        match self {
            PendingAction::Exit => "Exit",
            PendingAction::CloseDocument(_) => "Close file",
            PendingAction::OpenFile(_) => "Open file",
        }
    }
}

/// Builder for constructing RenderModel from domain state.
pub struct ViewModelBuilder;

impl ViewModelBuilder {
    /// Builds a complete RenderModel from document and view state.
    ///
    /// # Arguments
    /// * `document` - The document containing text and syntax state
    /// * `view` - The editor view with viewport, caret, and selection
    /// * `pending_action` - Optional pending action that needs confirmation
    /// * `protection_error` - Optional protection error for the pending action
    /// * `sidebar` - The sidebar state
    /// * `focus` - The current focus state
    /// * `viewport_height` - The available viewport height for sidebar
    /// * `status_message` - Optional status message to display
    /// * `open_views` - List of (view_id, title, is_active, is_dirty) for all open views
    pub fn build(
        document: &Document,
        view: &EditorView,
        pending_action: Option<&PendingAction>,
        protection_error: Option<&ProtectionError>,
        sidebar: &Sidebar,
        focus: FocusState,
        viewport_height: usize,
        status_message: Option<&str>,
        open_views: &[(u64, String, bool, bool)],
    ) -> RenderModel {
        let viewport = &view.viewport;
        let selection = view.selection_range();

        // Calculate cursor position in document coordinates
        let caret_offset = view.caret_offset();
        let caret_pos = document.offset_to_position(caret_offset);

        // Build gutter model
        let gutter = GutterModel::new(view.show_line_numbers(), document.len_lines());

        // Get content for highlighting - use cache if available, otherwise fetch once
        // Note: After an edit, the cache may be stale. We refresh it lazily here.
        let content_for_highlighting = match document.cached_content() {
            Some(s) => std::borrow::Cow::Borrowed(s),
            None => {
                // Cache is stale, we need to get fresh content.
                // This is O(n) but happens once per edit, not per navigation.
                std::borrow::Cow::Owned(document.content())
            }
        };

        // Build visible lines
        let visible_lines = Self::build_visible_lines(
            document,
            &content_for_highlighting,
            viewport.scroll_x,
            viewport.scroll_y,
            viewport.width,
            viewport.height,
            caret_pos.line,
            selection,
        );

        // Calculate caret screen position
        let caret = Self::build_caret_presentation(
            caret_pos,
            viewport.scroll_x,
            viewport.scroll_y,
            viewport.width,
            viewport.height,
        );

        // Build status line
        let status = Self::build_status_presentation(document, caret_pos, status_message);

        // Build tab bar
        let tab_bar = Self::build_tab_bar_presentation(open_views);

        // Build dialog state from pending action and protection error
        let dialog = Self::build_dialog_presentation(pending_action, protection_error);

        // Build sidebar presentation
        let sidebar_pres = Self::build_sidebar_presentation(sidebar, focus, viewport_height);

        RenderModel {
            visible_lines,
            gutter,
            caret,
            status,
            tab_bar,
            dialog,
            sidebar: sidebar_pres,
            scroll_x: viewport.scroll_x,
            scroll_y: viewport.scroll_y,
        }
    }

    /// Builds the tab bar presentation from open views info.
    /// Each tuple contains (view_id, title, is_active, is_dirty).
    fn build_tab_bar_presentation(open_views: &[(u64, String, bool, bool)]) -> TabBarPresentation {
        let tabs: Vec<TabPresentation> = open_views
            .iter()
            .map(|(view_id, title, is_active, is_dirty)| {
                TabPresentation::new(*view_id, title.clone(), *is_active, *is_dirty)
            })
            .collect();

        TabBarPresentation {
            visible: true, // Always show tab bar
            tabs,
        }
    }

    /// Builds the dialog presentation from pending action and protection error.
    fn build_dialog_presentation(
        pending_action: Option<&PendingAction>,
        protection_error: Option<&ProtectionError>,
    ) -> DialogPresentation {
        match (pending_action, protection_error) {
            (Some(action), Some(error)) => {
                let action_desc = action.description().to_string();
                let unsaved_docs = error.document_titles()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                DialogPresentation::UnsavedChangesConfirmation {
                    action_description: action_desc,
                    unsaved_documents: unsaved_docs,
                }
            }
            _ => DialogPresentation::None,
        }
    }

    /// Builds a RenderModel when only the sidebar should be shown (no document).
    pub fn build_sidebar_only(
        sidebar: &Sidebar,
        focus: FocusState,
        viewport_height: usize,
    ) -> RenderModel {
        let sidebar_pres = Self::build_sidebar_presentation(sidebar, focus, viewport_height);
        RenderModel {
            sidebar: sidebar_pres,
            ..Default::default()
        }
    }

    /// Builds the sidebar presentation from sidebar state.
    fn build_sidebar_presentation(
        sidebar: &Sidebar,
        focus: FocusState,
        viewport_height: usize,
    ) -> SidebarPresentation {
        if !sidebar.visible {
            return SidebarPresentation::default();
        }

        let directory_name = sidebar
            .base_directory
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Files")
            .to_string();

        let visible_entries = sidebar.visible_entries(viewport_height);
        let entries: Vec<FileEntryPresentation> = visible_entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let actual_index = sidebar.scroll_offset + i;
                FileEntryPresentation::new(
                    entry.name.clone(),
                    entry.is_dir,
                    actual_index == sidebar.selected_index,
                )
            })
            .collect();

        SidebarPresentation {
            visible: true,
            focused: focus == FocusState::Sidebar,
            directory_name,
            entries,
            width: sidebar.width(),
        }
    }

    /// Builds the visible lines with syntax highlighting and selection.
    ///
    /// # Arguments
    /// * `content_for_highlighting` - Pre-fetched content string for tree-sitter highlighting.
    ///   Caller should use Document::cached_content() to avoid repeated cloning.
    fn build_visible_lines(
        document: &Document,
        content_for_highlighting: &str,
        scroll_x: usize,
        scroll_y: usize,
        width: usize,
        height: usize,
        cursor_line: usize,
        selection: Option<(usize, usize)>,
    ) -> Vec<LinePresentation> {
        let mut visible_lines = Vec::with_capacity(height);
        let buffer = document.buffer();
        let highlighter = document.highlighter();

        for i in 0..height {
            let line_idx = scroll_y + i;
            if line_idx >= document.len_lines() {
                break;
            }

            let is_current = line_idx == cursor_line;
            let line_text = document.line(line_idx);

            // Calculate character offsets for this line
            let line_start_char = buffer.line_to_offset(line_idx);
            let line_start_byte = buffer.char_to_byte(line_start_char);
            let line_len_chars = buffer.line_len(line_idx);

            // Get the portion of the line visible in the viewport
            let display_text: String = line_text
                .chars()
                .skip(scroll_x)
                .take(width)
                .collect();

            // Build styled spans for this line
            let spans = Self::build_line_spans(
                &display_text,
                content_for_highlighting,
                &line_text,
                line_idx,
                line_start_char,
                line_start_byte,
                line_len_chars,
                scroll_x,
                highlighter,
                selection,
            );

            visible_lines.push(LinePresentation::with_spans(
                line_idx + 1, // 1-indexed for display
                is_current,
                spans,
            ));
        }

        visible_lines
    }

    /// Builds styled spans for a single line, merging syntax and selection highlighting.
    fn build_line_spans(
        display_text: &str,
        content: &str,
        full_line: &str,
        line_idx: usize,
        line_start_char: usize,
        line_start_byte: usize,
        line_len_chars: usize,
        scroll_x: usize,
        highlighter: &SyntaxHighlighter,
        selection: Option<(usize, usize)>,
    ) -> Vec<StyledSpan> {
        let mut spans = Vec::new();

        if display_text.is_empty() {
            return spans;
        }

        // Check if this line has any selection
        let line_end_char = line_start_char + line_len_chars;
        let selection_on_line = selection.and_then(|(sel_start, sel_end)| {
            if sel_start < line_end_char && sel_end > line_start_char {
                // Calculate relative positions within display_text
                let rel_start = sel_start
                    .saturating_sub(line_start_char)
                    .saturating_sub(scroll_x);
                let rel_end = sel_end
                    .saturating_sub(line_start_char)
                    .saturating_sub(scroll_x);

                let display_len = display_text.chars().count();
                let clamped_start = rel_start.min(display_len);
                let clamped_end = rel_end.min(display_len);

                if clamped_end > clamped_start {
                    Some((clamped_start, clamped_end))
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Get syntax highlights for this line
        let highlights = highlighter.highlight_line(content, line_idx, line_start_byte, full_line);

        match selection_on_line {
            Some((sel_start, sel_end)) => {
                // Split into pre-selection, selection, and post-selection
                Self::build_spans_with_selection(
                    &mut spans,
                    display_text,
                    &highlights,
                    sel_start,
                    sel_end,
                    scroll_x,
                );
            }
            None => {
                // No selection - just apply syntax highlighting
                Self::build_highlighted_spans(&mut spans, display_text, &highlights, 0, scroll_x);
            }
        }

        spans
    }

    /// Builds spans with selection highlighting.
    fn build_spans_with_selection(
        spans: &mut Vec<StyledSpan>,
        text: &str,
        highlights: &[HighlightSpan],
        sel_start: usize,
        sel_end: usize,
        scroll_x: usize,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        // Pre-selection part (with syntax highlighting)
        if sel_start > 0 {
            let pre_text: String = chars[..sel_start].iter().collect();
            Self::build_highlighted_spans(spans, &pre_text, highlights, 0, scroll_x);
        }

        // Selected part (override with selection style)
        if sel_end > sel_start {
            let selected_text: String = chars[sel_start..sel_end].iter().collect();
            spans.push(StyledSpan::selection(selected_text));
        }

        // Post-selection part (with syntax highlighting)
        if sel_end < text_len {
            let post_text: String = chars[sel_end..].iter().collect();
            Self::build_highlighted_spans(spans, &post_text, highlights, sel_end, scroll_x);
        }
    }

    /// Builds spans with syntax highlighting applied.
    fn build_highlighted_spans(
        spans: &mut Vec<StyledSpan>,
        text: &str,
        highlights: &[HighlightSpan],
        offset: usize,
        scroll_x: usize,
    ) {
        if highlights.is_empty() {
            spans.push(StyledSpan::raw(text.to_string()));
            return;
        }

        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();
        let mut current_pos = 0;

        for hl in highlights {
            // Adjust highlight positions for the text segment we're processing
            let hl_start = hl.start_col.saturating_sub(scroll_x + offset);
            let hl_end = hl.end_col.saturating_sub(scroll_x + offset);

            // Skip highlights outside our range
            if hl_end <= current_pos || hl_start >= text_len {
                continue;
            }

            let start = hl_start.max(current_pos);
            let end = hl_end.min(text_len);

            // Add unhighlighted text before this highlight
            if start > current_pos {
                let normal: String = chars[current_pos..start].iter().collect();
                spans.push(StyledSpan::raw(normal));
            }

            // Add highlighted text
            if end > start {
                let highlighted: String = chars[start..end].iter().collect();
                spans.push(StyledSpan::new(highlighted, hl.style));
            }

            current_pos = end;
        }

        // Add remaining unhighlighted text
        if current_pos < text_len {
            let remaining: String = chars[current_pos..].iter().collect();
            spans.push(StyledSpan::raw(remaining));
        }
    }

    /// Builds the caret presentation with screen position.
    fn build_caret_presentation(
        caret_pos: TextPosition,
        scroll_x: usize,
        scroll_y: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) -> CaretPresentation {
        // Calculate screen position
        let screen_row = caret_pos.line.saturating_sub(scroll_y);
        let screen_col = caret_pos.column.saturating_sub(scroll_x);

        // Check if caret is visible
        let visible = caret_pos.line >= scroll_y
            && caret_pos.line < scroll_y + viewport_height
            && caret_pos.column >= scroll_x
            && caret_pos.column < scroll_x + viewport_width;

        CaretPresentation {
            position: VisualPosition::new(screen_row, screen_col),
            visible,
        }
    }

    /// Builds the status line presentation.
    fn build_status_presentation(
        document: &Document,
        caret_pos: TextPosition,
        status_message: Option<&str>,
    ) -> StatusPresentation {
        let title = document.file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[New File]")
            .to_string();

        let language = format!("{:?}", document.highlighter().language());

        StatusPresentation {
            title,
            dirty: document.is_dirty(),
            cursor_line: caret_pos.line + 1,    // 1-indexed
            cursor_column: caret_pos.column + 1, // 1-indexed
            language,
            total_lines: document.len_lines(),
            message: status_message.map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Document;
    use crate::view::EditorView;

    #[test]
    fn test_build_basic_render_model() {
        let doc = Document::from_str("Hello\nWorld", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24); // Set viewport dimensions
        let sidebar = Sidebar::default();
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, None, &open_views);

        assert_eq!(model.visible_lines.len(), 2);
        assert_eq!(model.visible_lines[0].line_number, 1);
        assert_eq!(model.visible_lines[1].line_number, 2);
        assert!(model.visible_lines[0].is_current_line);
        assert!(!model.visible_lines[1].is_current_line);
    }

    #[test]
    fn test_caret_position() {
        let doc = Document::from_str("Hello\nWorld", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24); // Set viewport dimensions
        view.move_caret_to(7, false); // "W" in "World"
        let sidebar = Sidebar::default();
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, None, &open_views);

        assert_eq!(model.caret.position.row, 1); // Second line
        assert_eq!(model.caret.position.column, 1); // Second char
        assert!(model.caret.visible);
    }

    #[test]
    fn test_dialog_state() {
        use crate::domain::{DocumentId, ProtectionError, UnsavedDocument};

        let doc = Document::from_str("test", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);
        let sidebar = Sidebar::default();
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        // No dialog when no pending action
        let model_no_dialog = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, None, &open_views);
        assert!(matches!(model_no_dialog.dialog, DialogPresentation::None));

        // Dialog shown when pending action + protection error
        let pending = PendingAction::Exit;
        let unsaved = UnsavedDocument::new(DocumentId::new(), "test.txt");
        let error = ProtectionError::UnsavedChanges(unsaved);
        let model_with_dialog = ViewModelBuilder::build(&doc, &view, Some(&pending), Some(&error), &sidebar, FocusState::Editor, 24, None, &open_views);

        if let DialogPresentation::UnsavedChangesConfirmation { action_description, unsaved_documents } = model_with_dialog.dialog {
            assert_eq!(action_description, "Exit");
            assert_eq!(unsaved_documents, vec!["test.txt"]);
        } else {
            panic!("Expected UnsavedChangesConfirmation dialog");
        }
    }

    #[test]
    fn test_status_presentation() {
        let doc = Document::from_str("Line 1\nLine 2\nLine 3", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);
        let sidebar = Sidebar::default();
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, None, &open_views);

        assert_eq!(model.status.cursor_line, 1);
        assert_eq!(model.status.cursor_column, 1);
        assert_eq!(model.status.total_lines, 3);
        assert!(!model.status.dirty);
        assert!(model.status.message.is_none());
    }

    #[test]
    fn test_status_message() {
        let doc = Document::from_str("test", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);
        let sidebar = Sidebar::default();
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, Some("File saved"), &open_views);

        assert_eq!(model.status.message, Some("File saved".to_string()));
    }

    #[test]
    fn test_sidebar_presentation() {
        let doc = Document::from_str("test", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);
        let open_views: Vec<(u64, String, bool, bool)> = vec![];

        // Test hidden sidebar
        let sidebar = Sidebar::default();
        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Editor, 24, None, &open_views);
        assert!(!model.sidebar.visible);

        // Test visible sidebar
        let mut sidebar = Sidebar::default();
        sidebar.visible = true;
        let model = ViewModelBuilder::build(&doc, &view, None, None, &sidebar, FocusState::Sidebar, 24, None, &open_views);
        assert!(model.sidebar.visible);
        assert!(model.sidebar.focused);
    }
}
