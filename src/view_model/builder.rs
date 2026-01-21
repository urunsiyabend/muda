//! ViewModelBuilder: Constructs RenderModel from domain state.
//!
//! This module contains the projection logic that transforms
//! Document + EditorView state into a render-ready RenderModel.

use ratatui::style::{Color, Style};

use crate::domain::{Document, TextPosition};
use crate::syntax::{HighlightSpan, SyntaxHighlighter};
use crate::view::EditorView;
use crate::view_model::{
    CaretPresentation, DialogPresentation, GutterModel, LinePresentation, RenderModel,
    StatusPresentation, StyledSpan, VisualPosition,
};

/// Builder for constructing RenderModel from domain state.
pub struct ViewModelBuilder;

impl ViewModelBuilder {
    /// Builds a complete RenderModel from document and view state.
    ///
    /// # Arguments
    /// * `document` - The document containing text and syntax state
    /// * `view` - The editor view with viewport, caret, and selection
    /// * `show_exit_dialog` - Whether to show the exit confirmation dialog
    pub fn build(
        document: &Document,
        view: &EditorView,
        show_exit_dialog: bool,
    ) -> RenderModel {
        let viewport = &view.viewport;
        let selection = view.selection_range();

        // Calculate cursor position in document coordinates
        let caret_offset = view.caret_offset();
        let caret_pos = document.offset_to_position(caret_offset);

        // Build gutter model
        let gutter = GutterModel::new(view.show_line_numbers(), document.len_lines());

        // Build visible lines
        let visible_lines = Self::build_visible_lines(
            document,
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
        let status = Self::build_status_presentation(document, caret_pos);

        // Build dialog state
        let dialog = if show_exit_dialog {
            DialogPresentation::ExitConfirmation
        } else {
            DialogPresentation::None
        };

        RenderModel {
            visible_lines,
            gutter,
            caret,
            status,
            dialog,
            scroll_x: viewport.scroll_x,
            scroll_y: viewport.scroll_y,
        }
    }

    /// Builds the visible lines with syntax highlighting and selection.
    fn build_visible_lines(
        document: &Document,
        scroll_x: usize,
        scroll_y: usize,
        width: usize,
        height: usize,
        cursor_line: usize,
        selection: Option<(usize, usize)>,
    ) -> Vec<LinePresentation> {
        let mut visible_lines = Vec::with_capacity(height);
        let content = document.content();
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
                &content,
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
            let selection_style = Style::default().bg(Color::Blue).fg(Color::White);
            spans.push(StyledSpan::new(selected_text, selection_style));
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
    fn build_status_presentation(document: &Document, caret_pos: TextPosition) -> StatusPresentation {
        let title = document.file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Yeni Dosya]")
            .to_string();

        let language = format!("{:?}", document.highlighter().language());

        StatusPresentation {
            title,
            dirty: document.is_dirty(),
            cursor_line: caret_pos.line + 1,    // 1-indexed
            cursor_column: caret_pos.column + 1, // 1-indexed
            language,
            total_lines: document.len_lines(),
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

        let model = ViewModelBuilder::build(&doc, &view, false);

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

        let model = ViewModelBuilder::build(&doc, &view, false);

        assert_eq!(model.caret.position.row, 1); // Second line
        assert_eq!(model.caret.position.column, 1); // Second char
        assert!(model.caret.visible);
    }

    #[test]
    fn test_dialog_state() {
        let doc = Document::from_str("test", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);

        let model_no_dialog = ViewModelBuilder::build(&doc, &view, false);
        assert!(matches!(model_no_dialog.dialog, DialogPresentation::None));

        let model_with_dialog = ViewModelBuilder::build(&doc, &view, true);
        assert!(matches!(model_with_dialog.dialog, DialogPresentation::ExitConfirmation));
    }

    #[test]
    fn test_status_presentation() {
        let doc = Document::from_str("Line 1\nLine 2\nLine 3", None);
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(80, 24);

        let model = ViewModelBuilder::build(&doc, &view, false);

        assert_eq!(model.status.cursor_line, 1);
        assert_eq!(model.status.cursor_column, 1);
        assert_eq!(model.status.total_lines, 3);
        assert!(!model.status.dirty);
    }
}
