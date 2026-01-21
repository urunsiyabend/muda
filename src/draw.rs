//! Rendering logic for the TUI editor.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::app::App;

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();
    let line_num_width = app.line_number_width();
    let width = (size.width as usize)
        .saturating_sub(2)
        .saturating_sub(line_num_width);
    let height = (size.height as usize).saturating_sub(2);

    // Get view state via methods
    let scroll_x = app.scroll_x();
    let scroll_y = app.scroll_y();
    let cursor_x = app.cursor_x();
    let cursor_y = app.cursor_y();
    let show_line_numbers = app.show_line_numbers();

    let selection = app.get_selection_range();
    let mut visible_lines = Vec::new();
    let content = app.content();
    let total_lines = content.len_lines();
    let gutter_width = if show_line_numbers {
        total_lines.to_string().len()
    } else {
        0
    };

    let content_str = content.to_string();

    for (i, line) in content.lines().skip(scroll_y).take(height).enumerate() {
        let current_line_idx = scroll_y + i;
        let line_start_char = content.line_to_char(current_line_idx);
        let line_start_byte = content.char_to_byte(line_start_char);
        let line_len = line.len_chars();

        let line_str = line.to_string().replace('\n', "").replace('\r', "");
        let display_str: String = line_str.chars().skip(scroll_x).take(width).collect();

        let mut spans = Vec::new();

        if show_line_numbers {
            let line_num = format!(
                "{:>gutter_width$} │",
                current_line_idx + 1,
                gutter_width = gutter_width
            );
            let num_style = if current_line_idx == cursor_y {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(line_num, num_style));
        }

        if let Some((sel_start, sel_end)) = selection {
            let line_end_char = line_start_char + line_len;

            if sel_start < line_end_char && sel_end > line_start_char {
                let relative_sel_start = sel_start
                    .saturating_sub(line_start_char)
                    .saturating_sub(scroll_x);
                let relative_sel_end = sel_end
                    .saturating_sub(line_start_char)
                    .saturating_sub(scroll_x);

                let split_start = relative_sel_start.clamp(0, display_str.chars().count());
                let split_end = relative_sel_end.clamp(0, display_str.chars().count());

                if split_start > 0 {
                    let pre_text: String = display_str.chars().take(split_start).collect();
                    add_highlighted_spans(
                        &mut spans,
                        &pre_text,
                        app,
                        &content_str,
                        current_line_idx,
                        line_start_byte,
                        0,
                        &line_str,
                        scroll_x,
                    );
                }
                if split_end > split_start {
                    let selected_text: String = display_str
                        .chars()
                        .skip(split_start)
                        .take(split_end - split_start)
                        .collect();
                    spans.push(Span::styled(
                        selected_text,
                        Style::default().bg(Color::Blue).fg(Color::White),
                    ));
                }
                if split_end < display_str.chars().count() {
                    let post_text: String = display_str.chars().skip(split_end).collect();
                    add_highlighted_spans(
                        &mut spans,
                        &post_text,
                        app,
                        &content_str,
                        current_line_idx,
                        line_start_byte,
                        split_end,
                        &line_str,
                        scroll_x,
                    );
                }
            } else {
                add_highlighted_spans(
                    &mut spans,
                    &display_str,
                    app,
                    &content_str,
                    current_line_idx,
                    line_start_byte,
                    0,
                    &line_str,
                    scroll_x,
                );
            }
        } else {
            add_highlighted_spans(
                &mut spans,
                &display_str,
                app,
                &content_str,
                current_line_idx,
                line_start_byte,
                0,
                &line_str,
                scroll_x,
            );
        }

        visible_lines.push(Line::from(spans));
    }

    let title = format!(" {} ", app.get_title());

    let editor_widget =
        Paragraph::new(visible_lines).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(editor_widget, size);

    if app.show_exit_dialog {
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
    } else {
        let cursor_screen_x =
            size.x + 1 + line_num_width as u16 + (cursor_x as u16).saturating_sub(scroll_x as u16);
        let cursor_screen_y = size.y + 1 + (cursor_y as u16).saturating_sub(scroll_y as u16);
        f.set_cursor_position((cursor_screen_x, cursor_screen_y));
    }
}

fn add_highlighted_spans(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    app: &App,
    content_str: &str,
    line_idx: usize,
    line_start_byte: usize,
    offset: usize,
    full_line: &str,
    scroll_x: usize,
) {
    let highlights =
        app.highlighter()
            .highlight_line(content_str, line_idx, line_start_byte, full_line);

    if highlights.is_empty() {
        spans.push(Span::raw(text.to_string()));
        return;
    }

    let mut current_pos = 0;
    let text_chars: Vec<char> = text.chars().collect();

    for hl in highlights {
        let hl_start = hl.start_col.saturating_sub(scroll_x + offset);
        let hl_end = hl.end_col.saturating_sub(scroll_x + offset);

        if hl_end <= current_pos || hl_start >= text_chars.len() {
            continue;
        }

        let start = hl_start.max(current_pos);
        let end = hl_end.min(text_chars.len());

        if start > current_pos {
            let normal: String = text_chars[current_pos..start].iter().collect();
            spans.push(Span::raw(normal));
        }

        if end > start {
            let highlighted: String = text_chars[start..end].iter().collect();
            spans.push(Span::styled(highlighted, hl.style));
        }

        current_pos = end;
    }

    if current_pos < text_chars.len() {
        let remaining: String = text_chars[current_pos..].iter().collect();
        spans.push(Span::raw(remaining));
    }
}
