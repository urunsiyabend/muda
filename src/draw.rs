//! Rendering logic for the TUI editor.
//!
//! This module renders the editor UI using a RenderModel,
//! which provides a clean separation from domain internals.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use mudatexteditor::view_model::{DialogPresentation, RenderModel};

/// Renders the editor UI from a RenderModel.
pub fn ui(f: &mut Frame, model: &RenderModel) {
    let size = f.area();

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

    f.render_widget(editor_widget, size);

    // Handle dialogs or cursor
    match &model.dialog {
        DialogPresentation::ExitConfirmation => {
            render_exit_dialog(f, size);
        }
        DialogPresentation::None => {
            // Render cursor
            if model.caret.visible {
                let cursor_screen_x = size.x
                    + 1
                    + model.gutter.width as u16
                    + model.caret.position.column as u16;
                let cursor_screen_y = size.y + 1 + model.caret.position.row as u16;
                f.set_cursor_position((cursor_screen_x, cursor_screen_y));
            }
        }
    }
}

/// Renders the exit confirmation dialog.
fn render_exit_dialog(f: &mut Frame, size: Rect) {
    let dialog_width = 50;
    let dialog_height = 5;
    let dialog_x = (size.width.saturating_sub(dialog_width)) / 2;
    let dialog_y_pos = (size.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(dialog_x, dialog_y_pos, dialog_width, dialog_height);

    f.render_widget(Clear, dialog_area);

    let dialog = Paragraph::new(vec![
        Line::from(""),
        Line::from("  Kaydedilmemiş değişiklikler var!"),
        Line::from("  [Y] Kaydet ve Çık  [N] Kaydetmeden Çık  [Esc] İptal"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Uyarı "),
    )
    .style(Style::default().fg(Color::White));

    f.render_widget(dialog, dialog_area);
}
