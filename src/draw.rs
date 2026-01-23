//! Rendering logic for the TUI editor.
//!
//! This module renders the editor UI using a RenderModel,
//! which provides a clean separation from domain internals.
//!
//! # Style Mapping
//!
//! This module contains the ratatui-specific style mapping logic.
//! The [`map_semantic_style`] function converts domain-level [`TextStyle`]
//! tokens to concrete ratatui styles.
//!
//! When adding a new rendering backend (e.g., GUI, web), create a similar
//! mapper for that backend's style system.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use mudatexteditor::view_model::{DialogPresentation, RenderModel, SidebarPresentation, TextStyle};

// =============================================================================
// Style Mapping: TextStyle → ratatui::Style
// =============================================================================

/// Maps a domain-level semantic style to a ratatui terminal style.
///
/// This is the single source of truth for TUI styling. To change how
/// a semantic token looks in the terminal, modify this function.
///
/// # Future Backends
///
/// Other rendering backends would implement their own mappers:
/// - Web: `fn map_semantic_style(style: TextStyle) -> CssClass`
/// - GUI: `fn map_semantic_style(style: TextStyle) -> NativeStyle`
#[allow(dead_code)]
fn map_semantic_style(style: TextStyle) -> Style {
    match style {
        TextStyle::Normal => Style::default(),

        // Selection
        TextStyle::Selection => Style::default().bg(Color::Blue).fg(Color::White),

        // Syntax highlighting
        TextStyle::Keyword => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        TextStyle::String => Style::default().fg(Color::Green),
        TextStyle::Number => Style::default().fg(Color::Cyan),
        TextStyle::Comment => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        TextStyle::Type => Style::default().fg(Color::Yellow),
        TextStyle::Function => Style::default().fg(Color::Blue),
        TextStyle::Variable => Style::default().fg(Color::White),
        TextStyle::Operator => Style::default().fg(Color::White),
        TextStyle::Punctuation => Style::default().fg(Color::DarkGray),
        TextStyle::Constant => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        TextStyle::Module => Style::default().fg(Color::Yellow),
        TextStyle::Attribute => Style::default().fg(Color::Cyan),
        TextStyle::Macro => Style::default().fg(Color::Magenta),

        // UI elements
        TextStyle::LineNumber => Style::default().fg(Color::DarkGray),
        TextStyle::CurrentLineNumber => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        TextStyle::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        TextStyle::Warning => Style::default().fg(Color::Yellow),
    }
}

/// Renders the editor UI from a RenderModel.
pub fn ui(f: &mut Frame, model: &RenderModel) {
    let size = f.area();

    // Calculate layout based on sidebar visibility
    let chunks = if model.sidebar.visible {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(model.sidebar.width as u16),
                Constraint::Min(1),
            ])
            .split(size)
    } else {
        // Single chunk for editor only
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(size)
    };

    // Render sidebar if visible
    if model.sidebar.visible {
        render_sidebar(f, chunks[0], &model.sidebar);
        render_editor(f, chunks[1], model);
    } else {
        render_editor(f, chunks[0], model);
    }

    // Handle dialogs (rendered on top)
    if let DialogPresentation::UnsavedChangesConfirmation { action_description, unsaved_documents } = &model.dialog {
        render_unsaved_changes_dialog(f, size, action_description, unsaved_documents);
    }
}

/// Renders the sidebar file explorer.
fn render_sidebar(f: &mut Frame, area: Rect, sidebar: &SidebarPresentation) {
    let items: Vec<ListItem> = sidebar
        .entries
        .iter()
        .map(|entry| {
            let prefix = if entry.is_dir { "📁 " } else { "📄 " };
            let text = format!("{}{}", prefix, entry.name);

            let style = if entry.is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(text, style)))
        })
        .collect();

    let border_style = if sidebar.focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" {} ", sidebar.directory_name);
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    f.render_widget(list, area);
}

/// Renders the main editor area.
fn render_editor(f: &mut Frame, area: Rect, model: &RenderModel) {
    // Split the area into editor content and status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),     // Editor content (minimum 3 lines)
            Constraint::Length(1),  // Status bar (1 line)
        ])
        .split(area);

    let editor_area = chunks[0];
    let status_area = chunks[1];

    // Build visible lines from the render model
    let visible_lines: Vec<Line> = model
        .visible_lines
        .iter()
        .map(|line_pres| {
            let mut spans = Vec::new();

            // Add line number if gutter is visible
            if model.gutter.visible {
                let line_num = format!(
                    "{:>width$} │",
                    line_pres.line_number,
                    width = model.gutter.line_number_width()
                );
                let num_style = if line_pres.is_current_line {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(line_num, num_style));
            }

            // Add content spans
            for styled_span in &line_pres.spans {
                spans.push(Span::styled(styled_span.text.clone(), styled_span.style));
            }

            Line::from(spans)
        })
        .collect();

    // Build title
    let title = format!(" {} ", model.display_title());

    // Render the editor widget
    let editor_widget = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(editor_widget, editor_area);

    // Render status bar
    render_status_bar(f, status_area, model);

    // Render cursor (only if no dialog and caret is visible)
    if matches!(model.dialog, DialogPresentation::None) && model.caret.visible {
        let cursor_screen_x = editor_area.x
            + 1
            + model.gutter.width as u16
            + model.caret.position.column as u16;
        let cursor_screen_y = editor_area.y + 1 + model.caret.position.row as u16;
        f.set_cursor_position((cursor_screen_x, cursor_screen_y));
    }
}

/// Renders the status bar at the bottom of the editor.
fn render_status_bar(f: &mut Frame, area: Rect, model: &RenderModel) {
    let status = &model.status;

    // Build status bar content
    let status_text = if let Some(ref message) = status.message {
        // Show status message (errors, confirmations, etc.)
        Line::from(vec![
            Span::styled(
                format!(" {} ", message),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        // Show normal status: position, language, etc.
        let position = format!(" Ln {}, Col {} ", status.cursor_line, status.cursor_column);
        let language = format!(" {} ", status.language);
        let lines_info = format!(" {} lines ", status.total_lines);

        Line::from(vec![
            Span::styled(position, Style::default().fg(Color::Cyan)),
            Span::raw("│"),
            Span::styled(language, Style::default().fg(Color::Green)),
            Span::raw("│"),
            Span::styled(lines_info, Style::default().fg(Color::DarkGray)),
        ])
    };

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_widget(status_bar, area);
}

/// Renders the unsaved changes confirmation dialog.
fn render_unsaved_changes_dialog(
    f: &mut Frame,
    size: Rect,
    action_description: &str,
    unsaved_documents: &[String],
) {
    // Build the document list string
    let doc_list = if unsaved_documents.len() == 1 {
        format!("  File: {}", unsaved_documents[0])
    } else {
        let names: Vec<&str> = unsaved_documents.iter().map(|s| s.as_str()).collect();
        format!("  Files: {}", names.join(", "))
    };

    // Calculate dialog dimensions based on content
    let content_width = doc_list.len().max(60).min(70) as u16;
    let dialog_width = content_width + 4; // padding
    let dialog_height = 6;
    let dialog_x = (size.width.saturating_sub(dialog_width)) / 2;
    let dialog_y_pos = (size.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(dialog_x, dialog_y_pos, dialog_width, dialog_height);

    f.render_widget(Clear, dialog_area);

    let title = format!(" {} ", action_description);
    let dialog = Paragraph::new(vec![
        Line::from(""),
        Line::from("  Unsaved changes will be lost!"),
        Line::from(doc_list),
        Line::from("  [Y] Save & Continue  [N] Discard  [Esc] Cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title),
    )
    .style(Style::default().fg(Color::White));

    f.render_widget(dialog, dialog_area);
}
