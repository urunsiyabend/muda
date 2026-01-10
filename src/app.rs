use ropey::Rope;
use arboard::Clipboard;
use log::debug;
use std::path::PathBuf;
use std::fs;
use crate::command::{Command, CommandHistory};
use crate::syntax::{SyntaxHighlighter, SyntaxLanguage};

pub struct App {
    pub content: Rope,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub should_quit: bool,
    pub selection_anchor: Option<usize>,
    pub history: CommandHistory,
    pub show_line_numbers: bool,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    pub show_exit_dialog: bool,
    pub highlighter: SyntaxHighlighter,
}

impl App {
    pub fn new() -> Self {
        Self {
            content: Rope::from_str(""),
            cursor_x: 0,
            cursor_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            should_quit: false,
            selection_anchor: None,
            history: CommandHistory::new(),
            show_line_numbers: true,
            file_path: None,
            dirty: false,
            show_exit_dialog: false,
            highlighter: SyntaxHighlighter::new(SyntaxLanguage::Plain),
        }
    }

    pub fn open_file(path: &str) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let path_buf = PathBuf::from(path);
        let language = SyntaxLanguage::from_extension(&path_buf);
        log::debug!("Opened file: {}", path);
        log::debug!("Detected language: {:?}", language);
        let mut highlighter = SyntaxHighlighter::new(language);
        highlighter.parse(&content);

        Ok(Self {
            content: Rope::from_str(&content),
            cursor_x: 0,
            cursor_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            should_quit: false,
            selection_anchor: None,
            history: CommandHistory::new(),
            show_line_numbers: true,
            file_path: Some(path_buf),
            dirty: false,
            show_exit_dialog: false,
            highlighter,
        })
    }

    pub fn save(&mut self) -> std::io::Result<bool> {
        if let Some(path) = &self.file_path {
            fs::write(path, self.content.to_string())?;
            self.dirty = false;
            debug!("Dosya kaydedildi: {:?}", path);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        fs::write(path, self.content.to_string())?;
        self.file_path = Some(PathBuf::from(path));
        self.dirty = false;
        debug!("Dosya kaydedildi: {}", path);
        Ok(())
    }

    pub fn get_title(&self) -> String {
        let name = self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Yeni Dosya]");

        if self.dirty {
            format!("*{}", name)
        } else {
            name.to_string()
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.update_syntax();
    }

    fn get_line_len(&self, line_idx: usize) -> usize {
        let line = self.content.line(line_idx);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    fn cursor_to_char_idx(&self) -> usize {
        self.content.line_to_char(self.cursor_y) + self.cursor_x
    }

    fn char_idx_to_cursor(&self, char_idx: usize) -> (usize, usize) {
        let y = self.content.char_to_line(char_idx);
        let x = char_idx - self.content.line_to_char(y);
        (x, y)
    }

    fn set_cursor_from_char_idx(&mut self, char_idx: usize) {
        let clamped = char_idx.min(self.content.len_chars());
        let (x, y) = self.char_idx_to_cursor(clamped);
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.get_line_len(self.cursor_y);
        }
    }

    pub fn move_cursor_right(&mut self) {
        let line_len = self.get_line_len(self.cursor_y);

        if self.cursor_x < line_len {
            self.cursor_x += 1;
        } else if self.cursor_y + 1 < self.content.len_lines() {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    pub fn move_cursor_word_left(&mut self) {
        while self.cursor_x > 0 || self.cursor_y > 0 {
            let char_idx = self.cursor_to_char_idx();
            if char_idx == 0 {
                break;
            }
            let ch = self.content.char(char_idx - 1);
            if !ch.is_whitespace() && ch != '\n' {
                break;
            }
            self.move_cursor_left();
        }

        while self.cursor_x > 0 || self.cursor_y > 0 {
            let char_idx = self.cursor_to_char_idx();
            if char_idx == 0 {
                break;
            }
            let ch = self.content.char(char_idx - 1);
            if ch.is_whitespace() || ch == '\n' {
                break;
            }
            self.move_cursor_left();
        }
    }

    pub fn move_cursor_word_right(&mut self) {
        let total_chars = self.content.len_chars();

        while self.cursor_to_char_idx() < total_chars {
            let char_idx = self.cursor_to_char_idx();
            let ch = self.content.char(char_idx);
            if ch.is_whitespace() || ch == '\n' {
                break;
            }
            self.move_cursor_right();
        }

        while self.cursor_to_char_idx() < total_chars {
            let char_idx = self.cursor_to_char_idx();
            let ch = self.content.char(char_idx);
            if !ch.is_whitespace() && ch != '\n' {
                break;
            }
            self.move_cursor_right();
        }
    }

    pub fn check_scrolling(&mut self, width: usize, height: usize) {
        let scrolloff_x = 5;
        let scrolloff_y = 2;

        if self.cursor_x < self.scroll_x + scrolloff_x {
            self.scroll_x = self.cursor_x.saturating_sub(scrolloff_x);
        } else if self.cursor_x >= self.scroll_x + width - scrolloff_x {
            self.scroll_x = (self.cursor_x + scrolloff_x + 1).saturating_sub(width);
        }

        if self.cursor_y < self.scroll_y + scrolloff_y {
            self.scroll_y = self.cursor_y.saturating_sub(scrolloff_y);
        } else if self.cursor_y >= self.scroll_y + height - scrolloff_y {
            self.scroll_y = (self.cursor_y + scrolloff_y + 1).saturating_sub(height);
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let char_idx = self.cursor_to_char_idx();
        let cmd = Command::InsertChar { pos: char_idx, ch };
        self.history.execute(cmd, &mut self.content);
        self.cursor_x += 1;
        self.mark_dirty();
    }

    pub fn insert_newline(&mut self) {
        let current_line = self.content.line(self.cursor_y).to_string();
        let indent: String = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        let char_idx = self.cursor_to_char_idx();

        let newline_cmd = Command::InsertChar { pos: char_idx, ch: '\n' };
        self.history.execute(newline_cmd, &mut self.content);

        self.cursor_y += 1;
        self.cursor_x = 0;

        if !indent.is_empty() {
            let indent_cmd = Command::InsertString {
                pos: char_idx + 1,
                text: indent.clone(),
            };
            self.history.execute(indent_cmd, &mut self.content);
            self.cursor_x = indent.len();
        }

        self.mark_dirty();
    }

    pub fn clamp_cursor(&mut self) {
        let line_len = self.get_line_len(self.cursor_y);
        if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }

    pub fn backspace(&mut self) {
        let char_idx = self.cursor_to_char_idx();
        if char_idx > 0 {
            let deleted_char = self.content.char(char_idx - 1);

            if self.cursor_x == 0 {
                self.cursor_y -= 1;
                self.cursor_x = self.get_line_len(self.cursor_y);
            } else {
                self.cursor_x -= 1;
            }

            let cmd = Command::Delete {
                pos: char_idx - 1,
                deleted_text: deleted_char.to_string(),
            };
            self.history.execute(cmd, &mut self.content);
            self.mark_dirty();
        }
    }

    pub fn delete_at_cursor(&mut self) {
        let char_idx = self.cursor_to_char_idx();
        if char_idx < self.content.len_chars() {
            let deleted_char = self.content.char(char_idx);
            let cmd = Command::Delete {
                pos: char_idx,
                deleted_text: deleted_char.to_string(),
            };
            self.history.execute(cmd, &mut self.content);
            self.mark_dirty();
        }
    }

    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let current_idx = self.cursor_to_char_idx();
            if anchor < current_idx { (anchor, current_idx) } else { (current_idx, anchor) }
        })
    }

    pub fn copy_selection(&self) {
        if let Some((start, end)) = self.get_selection_range() {
            let text = self.content.slice(start..end).to_string();
            if let Ok(mut cb) = Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }
    }

    pub fn cut_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            debug!("=== CUT SELECTION ===");
            debug!("Selection range: start={}, end={}", start, end);

            let text = self.content.slice(start..end).to_string();

            if let Ok(mut cb) = Clipboard::new() {
                let _ = cb.set_text(text.clone());
            }

            let cmd = Command::Delete {
                pos: start,
                deleted_text: text,
            };
            self.history.execute(cmd, &mut self.content);

            self.set_cursor_from_char_idx(start);
            self.selection_anchor = None;
            self.mark_dirty();
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let text = self.content.slice(start..end).to_string();

            let cmd = Command::Delete {
                pos: start,
                deleted_text: text,
            };
            self.history.execute(cmd, &mut self.content);

            self.set_cursor_from_char_idx(start);
            self.selection_anchor = None;
            self.mark_dirty();
        }
    }

    pub fn insert_string_at_cursor(&mut self, text: &str) {
        let char_idx = self.cursor_to_char_idx();
        let cmd = Command::InsertString {
            pos: char_idx,
            text: text.to_string(),
        };
        self.history.execute(cmd, &mut self.content);

        let new_char_idx = char_idx + text.chars().count();
        self.set_cursor_from_char_idx(new_char_idx);
        self.selection_anchor = None;
        self.mark_dirty();
    }

    pub fn paste(&mut self) {
        if let Ok(mut cb) = Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                self.insert_string_at_cursor(&text);
            }
        }
    }

    pub fn undo(&mut self) {
        debug!("=== UNDO ===");
        if let Some(pos) = self.history.undo(&mut self.content) {
            self.set_cursor_from_char_idx(pos);
            self.selection_anchor = None;
            self.mark_dirty();
            debug!("Undo complete, cursor at ({}, {})", self.cursor_x, self.cursor_y);
        }
    }

    pub fn redo(&mut self) {
        debug!("=== REDO ===");
        if let Some(pos) = self.history.redo(&mut self.content) {
            self.set_cursor_from_char_idx(pos);
            self.selection_anchor = None;
            self.mark_dirty();
            debug!("Redo complete, cursor at ({}, {})", self.cursor_x, self.cursor_y);
        }
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        let total_chars = self.content.len_chars();
        self.set_cursor_from_char_idx(total_chars);
    }

    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn line_number_width(&self) -> usize {
        if self.show_line_numbers {
            let line_count = self.content.len_lines();
            let digits = line_count.to_string().len();
            digits + 2
        } else {
            0
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_x = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_x = self.get_line_len(self.cursor_y);
    }

    pub fn move_cursor_file_start(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn move_cursor_file_end(&mut self) {
        self.cursor_y = self.content.len_lines().saturating_sub(1);
        self.cursor_x = self.get_line_len(self.cursor_y);
    }

    pub fn page_up(&mut self, page_height: usize) {
        self.cursor_y = self.cursor_y.saturating_sub(page_height);
        self.clamp_cursor();
    }

    pub fn page_down(&mut self, page_height: usize) {
        self.cursor_y = (self.cursor_y + page_height).min(self.content.len_lines().saturating_sub(1));
        self.clamp_cursor();
    }

    pub fn update_syntax(&mut self) {
        let content = self.content.to_string();
        self.highlighter.parse(&content);
    }
}
