mod app;
mod draw;

use app::App;
use draw::ui;

use crossterm::event::{self, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::EnterAlternateScreen;
use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};
use std::env;
use std::fs::File;
use std::io;

fn main() -> io::Result<()> {
    let log_file = File::create("editor.log").unwrap();
    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file).unwrap();

    debug!("Editor starting...");

    let args: Vec<String> = env::args().collect();
    let mut app = if args.len() > 1 {
        match App::open_file(&args[1]) {
            Ok(app) => {
                debug!("File opened: {}", &args[1]);
                app
            }
            Err(e) => {
                debug!("Could not open file: {}, creating new document", e);
                let mut app = App::new();
                if let Some(doc) = app.document_mut() {
                    doc.set_file_path(std::path::PathBuf::from(&args[1]));
                }
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
        (size.width as usize)
            .saturating_sub(2)
            .saturating_sub(line_num_width),
        (size.height as usize).saturating_sub(2),
    );

    while !app.should_quit {
        let render_model = app.build_render_model();
        terminal.draw(|f| ui(f, &render_model))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Char('s') if is_ctrl => {
                        let _ = app.save();
                    }
                    KeyCode::Char('c') if is_ctrl => {
                        app.copy_selection();
                    }
                    KeyCode::Char('x') if is_ctrl => {
                        app.cut_selection();
                    }
                    KeyCode::Char('v') if is_ctrl => {
                        if app.has_selection() {
                            app.delete_selection();
                        }
                        app.paste();
                    }
                    KeyCode::Char('z') if is_ctrl => {
                        app.undo();
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
                    KeyCode::Char('y') | KeyCode::Char('Y') if app.show_exit_dialog => {
                        let _ = app.save();
                        app.should_quit = true;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') if app.show_exit_dialog => {
                        app.should_quit = true;
                    }
                    _ if app.show_exit_dialog => {}

                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        // Handle selection with shift
                        if is_shift && !app.has_selection() {
                            app.begin_selection();
                        } else if !is_shift {
                            app.clear_selection();
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
                                app.move_cursor_up();
                            }
                            KeyCode::Down => {
                                app.move_cursor_down();
                            }
                            _ => {}
                        }

                        // Extend selection if shift is held
                        if is_shift {
                            let offset = app.cursor_to_char_idx();
                            app.extend_selection_to(offset);
                        }
                    }
                    KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                        if is_shift && !app.has_selection() {
                            app.begin_selection();
                        } else if !is_shift {
                            app.clear_selection();
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

                        // Extend selection if shift is held
                        if is_shift {
                            let offset = app.cursor_to_char_idx();
                            app.extend_selection_to(offset);
                        }
                    }
                    KeyCode::Tab => {
                        if app.has_selection() {
                            app.delete_selection();
                        }
                        for _ in 0..4 {
                            app.insert_char(' ');
                        }
                    }
                    KeyCode::Char(c) => {
                        if app.has_selection() {
                            app.delete_selection();
                        }
                        app.insert_char(c);
                    }
                    KeyCode::Enter => {
                        if app.has_selection() {
                            app.delete_selection();
                        }
                        app.insert_newline();
                    }
                    KeyCode::Backspace => {
                        if app.has_selection() {
                            app.delete_selection();
                        } else {
                            app.backspace();
                        }
                    }
                    KeyCode::Delete => {
                        if app.has_selection() {
                            app.delete_selection();
                        } else {
                            app.delete_at_cursor();
                        }
                    }
                    KeyCode::Esc => {
                        if app.show_exit_dialog {
                            app.show_exit_dialog = false;
                        } else if app.dirty() {
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
                    (size.width as usize)
                        .saturating_sub(2)
                        .saturating_sub(line_num_width),
                    (size.height as usize).saturating_sub(2),
                );
            }
            Event::Paste(text) => {
                debug!("Paste event: {:?}", text);
                if app.has_selection() {
                    app.delete_selection();
                }
                app.insert_string_at_cursor(&text);

                let size = terminal.size()?;
                let line_num_width = app.line_number_width();
                app.check_scrolling(
                    (size.width as usize)
                        .saturating_sub(2)
                        .saturating_sub(line_num_width),
                    (size.height as usize).saturating_sub(2),
                );
            }
            Event::Resize(_, _) => {
                let size = terminal.size()?;
                let line_num_width = app.line_number_width();
                app.check_scrolling(
                    (size.width as usize)
                        .saturating_sub(2)
                        .saturating_sub(line_num_width),
                    (size.height as usize).saturating_sub(2),
                );
            }
            _ => {}
        }
    }

    debug!("Editor shutting down...");
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    Ok(())
}
