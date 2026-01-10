mod app;
mod command;
mod draw;
mod syntax;

use app::App;
use draw::ui;

use std::env;
use std::io;
use std::fs::File;
use crossterm::event::{self, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::EnterAlternateScreen;
use simplelog::{WriteLogger, LevelFilter, Config};
use log::debug;

fn main() -> io::Result<()> {
    let log_file = File::create("editor.log").unwrap();
    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file).unwrap();

    debug!("Editor başlatılıyor...");

    let args: Vec<String> = env::args().collect();
    let mut app = if args.len() > 1 {
        match App::open_file(&args[1]) {
            Ok(app) => {
                debug!("Dosya açıldı: {}", &args[1]);
                app
            }
            Err(e) => {
                debug!("Dosya açılamadı: {}, yeni dosya oluşturuluyor", e);
                let mut app = App::new();
                app.file_path = Some(std::path::PathBuf::from(&args[1]));
                app
            }
        }
    } else {
        App::new()
    };

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let size = terminal.size()?;
    let line_num_width = app.line_number_width();
    app.check_scrolling(
        (size.width as usize).saturating_sub(2).saturating_sub(line_num_width),
        (size.height as usize).saturating_sub(2),
    );

    while !app.should_quit {
        terminal.draw(|f| ui(f, &app))?;

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    debug!("Key: {:?}, Modifiers: {:?}", key.code, key.modifiers);

                    let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                    match key.code {
                        KeyCode::Esc if app.show_exit_dialog => {
                            app.show_exit_dialog = false;
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') if app.show_exit_dialog => {
                            let _ = app.save();
                            app.should_quit = true;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') if app.show_exit_dialog => {
                            app.should_quit = true;
                        }
                        _ if app.show_exit_dialog => {}

                        KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                            if is_shift && app.selection_anchor.is_none() {
                                app.selection_anchor = Some(app.content.line_to_char(app.cursor_y) + app.cursor_x);
                            } else if !is_shift {
                                app.selection_anchor = None;
                            }

                            match key.code {
                                KeyCode::Left => {
                                    if is_ctrl {
                                        app.move_cursor_word_left();
                                    } else {
                                        app.move_cursor_left();
                                    }
                                }
                                KeyCode::Right => {
                                    if is_ctrl {
                                        app.move_cursor_word_right();
                                    } else {
                                        app.move_cursor_right();
                                    }
                                }
                                KeyCode::Up => {
                                    if app.cursor_y > 0 {
                                        app.cursor_y -= 1;
                                        app.clamp_cursor();
                                    }
                                }
                                KeyCode::Down => {
                                    if app.cursor_y + 1 < app.content.len_lines() {
                                        app.cursor_y += 1;
                                        app.clamp_cursor();
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                            if is_shift && app.selection_anchor.is_none() {
                                app.selection_anchor = Some(app.content.line_to_char(app.cursor_y) + app.cursor_x);
                            } else if !is_shift {
                                app.selection_anchor = None;
                            }

                            let size = terminal.size()?;
                            let page_height = (size.height as usize).saturating_sub(4);

                            match key.code {
                                KeyCode::Home => {
                                    if is_ctrl {
                                        app.move_cursor_file_start();
                                    } else {
                                        app.move_cursor_home();
                                    }
                                }
                                KeyCode::End => {
                                    if is_ctrl {
                                        app.move_cursor_file_end();
                                    } else {
                                        app.move_cursor_end();
                                    }
                                }
                                KeyCode::PageUp => {
                                    app.page_up(page_height);
                                }
                                KeyCode::PageDown => {
                                    app.page_down(page_height);
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char('s') if is_ctrl => {
                            match app.save() {
                                Ok(true) => debug!("Dosya kaydedildi"),
                                Ok(false) => debug!("Dosya yolu yok, save as gerekli"),
                                Err(e) => debug!("Kaydetme hatası: {}", e),
                            }
                        }
                        KeyCode::Char('z') if is_ctrl && !is_shift => {
                            app.undo();
                        }
                        KeyCode::Char('Z') if is_ctrl && is_shift => {
                            app.redo();
                        }
                        KeyCode::Char('y') if is_ctrl => {
                            app.redo();
                        }
                        KeyCode::Char('a') if is_ctrl => {
                            app.select_all();
                        }
                        KeyCode::Char('l') if is_ctrl => {
                            app.toggle_line_numbers();
                        }
                        KeyCode::Char('k') if is_ctrl => {
                            app.cut_selection();
                        }
                        KeyCode::Char('u') if is_ctrl => {
                            app.paste();
                        }
                        KeyCode::Char('c') if is_ctrl => {
                            app.copy_selection();
                        }
                        KeyCode::Tab => {
                            if app.selection_anchor.is_some() {
                                app.delete_selection();
                            }
                            for _ in 0..4 {
                                app.insert_char(' ');
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.selection_anchor.is_some() {
                                app.delete_selection();
                            }
                            app.insert_char(c);
                        }
                        KeyCode::Enter => {
                            if app.selection_anchor.is_some() {
                                app.delete_selection();
                            }
                            app.insert_newline();
                        }
                        KeyCode::Backspace => {
                            if app.selection_anchor.is_some() {
                                app.delete_selection();
                            } else {
                                app.backspace();
                            }
                        }
                        KeyCode::Delete => {
                            if app.selection_anchor.is_some() {
                                app.delete_selection();
                            } else {
                                app.delete_at_cursor();
                            }
                        }
                        KeyCode::Esc => {
                            if app.show_exit_dialog {
                                app.show_exit_dialog = false;
                            } else if app.dirty {
                                app.show_exit_dialog = true;
                            } else {
                                app.should_quit = true;
                            }
                        }
                        _ => {}
                    }

                    let size = terminal.size()?;
                    let line_num_width = app.line_number_width();
                    app.check_scrolling(
                        (size.width as usize).saturating_sub(2).saturating_sub(line_num_width),
                        (size.height as usize).saturating_sub(2),
                    );
                }
            }
            Event::Paste(text) => {
                debug!("Paste event: {:?}", text);
                if app.selection_anchor.is_some() {
                    app.delete_selection();
                }
                app.insert_string_at_cursor(&text);

                let size = terminal.size()?;
                let line_num_width = app.line_number_width();
                app.check_scrolling(
                    (size.width as usize).saturating_sub(2).saturating_sub(line_num_width),
                    (size.height as usize).saturating_sub(2),
                );
            }
            Event::Resize(_, _) => {
                let size = terminal.size()?;
                let line_num_width = app.line_number_width();
                app.check_scrolling(
                    (size.width as usize).saturating_sub(2).saturating_sub(line_num_width),
                    (size.height as usize).saturating_sub(2),
                );
            }
            _ => {}
        }
    }

    debug!("Editor kapatılıyor...");
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}
