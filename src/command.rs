use ropey::Rope;

#[derive(Clone, Debug)]
pub enum Command {
    InsertChar {
        pos: usize,
        ch: char,
    },
    InsertString {
        pos: usize,
        text: String,
    },
    Delete {
        pos: usize,
        deleted_text: String,
    },
}

impl Command {
    pub fn execute(&self, content: &mut Rope) {
        match self {
            Command::InsertChar { pos, ch } => {
                content.insert_char(*pos, *ch);
            }
            Command::InsertString { pos, text } => {
                content.insert(*pos, text);
            }
            Command::Delete { pos, deleted_text } => {
                content.remove(*pos..*pos + deleted_text.chars().count());
            }
        }
    }

    pub fn undo(&self, content: &mut Rope) {
        match self {
            Command::InsertChar { pos, .. } => {
                content.remove(*pos..*pos + 1);
            }
            Command::InsertString { pos, text } => {
                content.remove(*pos..*pos + text.chars().count());
            }
            Command::Delete { pos, deleted_text } => {
                content.insert(*pos, deleted_text);
            }
        }
    }
}

#[derive(Default)]
pub struct CommandHistory {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, cmd: Command, content: &mut Rope) {
        cmd.execute(content);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, content: &mut Rope) -> Option<usize> {
        if let Some(cmd) = self.undo_stack.pop() {
            let pos = match &cmd {
                Command::InsertChar { pos, .. } => *pos,
                Command::InsertString { pos, .. } => *pos,
                Command::Delete { pos, deleted_text } => *pos + deleted_text.chars().count(),
            };
            cmd.undo(content);
            self.redo_stack.push(cmd);
            Some(pos)
        } else {
            None
        }
    }

    pub fn redo(&mut self, content: &mut Rope) -> Option<usize> {
        if let Some(cmd) = self.redo_stack.pop() {
            let pos = match &cmd {
                Command::InsertChar { pos, .. } => *pos + 1,
                Command::InsertString { pos, text } => *pos + text.chars().count(),
                Command::Delete { pos, .. } => *pos,
            };
            cmd.execute(content);
            self.undo_stack.push(cmd);
            Some(pos)
        } else {
            None
        }
    }
}
