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
use std::path::Path;

fn main() -> io::Result<()> {
    let log_file = File::create("editor.log").unwrap();
    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file).unwrap();

    debug!("Editor starting...");

    let args: Vec<String> = env::args().collect();
    let mut app = if args.len() > 1 {
        let path = Path::new(&args[1]);

        if path.is_dir() {
            // Launched with a directory - show sidebar
            match App::open_directory(&args[1]) {
                Ok(app) => {
                    debug!("Opened directory: {}", &args[1]);
                    app
                }
                Err(e) => {
                    debug!(
                        "Could not open directory: {}, starting with new document",
                        e
                    );
                    App::new()
                }
            }
        } else {
            // Launched with a file - hide sidebar by default
            match App::open_file(&args[1]) {
                Ok(app) => {
                    debug!("File opened: {}", &args[1]);
                    app
                }
                Err(e) => {
                    debug!("Could not open file: {}, creating new document", e);
                    let mut app = App::new();
                    app.set_file_path(std::path::PathBuf::from(&args[1]));
                    app
                }
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
    let (editor_width, editor_height) = calculate_editor_dimensions(&app, size.width, size.height);
    app.check_scrolling(editor_width, editor_height);

    while !app.should_quit {
        let size = terminal.size()?;
        let viewport_height = (size.height as usize).saturating_sub(2);
        app.adjust_sidebar_scroll(viewport_height);

        let render_model = app.build_render_model(viewport_height);
        terminal.draw(|f| ui(f, &render_model))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                // Handle dialog state first (pending action confirmation)
                if app.has_pending_action() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            // Save first, then confirm
                            if let Err(e) = app.save() {
                                log::error!("Save on confirmation failed: {}", e);
                            }
                            app.confirm_pending_action();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            // Discard changes and confirm
                            app.confirm_pending_action();
                        }
                        KeyCode::Esc => {
                            // Cancel the pending action
                            app.cancel_pending_action();
                        }
                        _ => {}
                    }
                    continue;
                }

                // Global keybindings (work in both editor and sidebar focus)
                match key.code {
                    KeyCode::Char('s') if is_ctrl => {
                        // Save now handles its own status message (success/failure)
                        if let Err(e) = app.save() {
                            log::error!("Save failed: {}", e);
                        }
                        continue;
                    }
                    KeyCode::Char('b') if is_ctrl => {
                        // Toggle sidebar visibility
                        app.toggle_sidebar();
                        // Recalculate scrolling for new layout
                        let (editor_width, editor_height) =
                            calculate_editor_dimensions(&app, size.width, size.height);
                        app.check_scrolling(editor_width, editor_height);
                        continue;
                    }
                    KeyCode::Char('h') if is_ctrl => {
                        // Focus sidebar (only if visible)
                        app.focus_sidebar();
                        continue;
                    }
                    KeyCode::Char('l') if is_ctrl && !is_shift => {
                        // Focus editor
                        app.focus_editor();
                        continue;
                    }
                    KeyCode::Esc => {
                        // Request exit - will show confirmation dialog if unsaved changes
                        let _ = app.request_exit();
                        continue;
                    }
                    _ => {}
                }

                // Handle sidebar-specific keybindings
                if app.is_sidebar_focused() {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.sidebar_move_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.sidebar_move_down();
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                            app.sidebar_go_back();
                        }
                        KeyCode::Enter => {
                            if let Some(entry) = app.sidebar.selected_entry() {
                                if entry.is_dir {
                                    // Navigate into directory
                                    let new_path = entry.path.clone();
                                    app.sidebar.set_base_directory(new_path);
                                    app.needs_render = true;
                                } else {
                                    // Request to open the file (may trigger confirmation dialog)
                                    let path = entry.path.clone();
                                    let _ = app.request_open_file(path);
                                }
                            }
                            // Recalculate scrolling after opening a file
                            let (editor_width, editor_height) =
                                calculate_editor_dimensions(&app, size.width, size.height);
                            app.check_scrolling(editor_width, editor_height);
                        }
                        _ => {}
                    }
                    continue;
                }

                // Editor-specific keybindings
                match key.code {
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
                    _ => {}
                }

                let (editor_width, editor_height) =
                    calculate_editor_dimensions(&app, size.width, size.height);
                app.check_scrolling(editor_width, editor_height);
            }
            Event::Paste(text) => {
                debug!("Paste event: {:?}", text);
                if app.has_selection() {
                    app.delete_selection();
                }
                app.insert_string_at_cursor(&text);

                let (editor_width, editor_height) =
                    calculate_editor_dimensions(&app, size.width, size.height);
                app.check_scrolling(editor_width, editor_height);
            }
            Event::Resize(_, _) => {
                let size = terminal.size()?;
                let (editor_width, editor_height) =
                    calculate_editor_dimensions(&app, size.width, size.height);
                app.check_scrolling(editor_width, editor_height);
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

/// Calculates the editor dimensions accounting for sidebar width.
fn calculate_editor_dimensions(
    app: &App,
    terminal_width: u16,
    terminal_height: u16,
) -> (usize, usize) {
    let sidebar_width = if app.sidebar.visible {
        app.sidebar.width()
    } else {
        0
    };

    let line_num_width = app.line_number_width();

    // Width: terminal - sidebar - borders (2) - line numbers
    let editor_width = (terminal_width as usize)
        .saturating_sub(sidebar_width)
        .saturating_sub(2)
        .saturating_sub(line_num_width);

    // Height: terminal - borders (2)
    let editor_height = (terminal_height as usize).saturating_sub(2);

    (editor_width, editor_height)
}
