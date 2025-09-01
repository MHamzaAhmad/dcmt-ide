use crate::buffer::TextBuffer;
use crate::cursor::Cursor;
use crate::selection::Selection;

pub enum EditorCommand {
    InsertChar(char),
    InsertText(String),
    DeleteChar,
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveToLineStart,
    MoveToLineEnd,
    SelectAll,
    Copy,
    Cut,
    Paste(String),
    Undo,
    Redo,
}

pub struct CommandExecutor {
    undo_stack: Vec<TextBuffer>,
    redo_stack: Vec<TextBuffer>,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
    
    pub fn execute(&mut self, command: EditorCommand, buffer: &mut TextBuffer, cursor: &mut Cursor) {
        match command {
            EditorCommand::InsertChar(ch) => {
                self.save_state(buffer);
                let pos = self.cursor_to_char_index(cursor, buffer);
                buffer.insert(pos, &ch.to_string());
                cursor.move_right(buffer.get_line(cursor.position.line).map(|l| l.len()).unwrap_or(0));
            }
            EditorCommand::InsertText(text) => {
                self.save_state(buffer);
                let pos = self.cursor_to_char_index(cursor, buffer);
                buffer.insert(pos, &text);
                // Update cursor position based on inserted text
            }
            EditorCommand::Backspace => {
                if cursor.position.column > 0 || cursor.position.line > 0 {
                    self.save_state(buffer);
                    let pos = self.cursor_to_char_index(cursor, buffer);
                    if pos > 0 {
                        buffer.delete(pos - 1, pos);
                        cursor.move_left();
                    }
                }
            }
            EditorCommand::DeleteChar => {
                self.save_state(buffer);
                let pos = self.cursor_to_char_index(cursor, buffer);
                if pos < buffer.len_chars() {
                    buffer.delete(pos, pos + 1);
                }
            }
            EditorCommand::MoveCursorLeft => {
                cursor.move_left();
            }
            EditorCommand::MoveCursorRight => {
                let line_len = buffer.get_line(cursor.position.line).map(|l| l.len()).unwrap_or(0);
                cursor.move_right(line_len);
            }
            EditorCommand::MoveCursorUp => {
                cursor.move_up();
            }
            EditorCommand::MoveCursorDown => {
                cursor.move_down(buffer.len_lines().saturating_sub(1));
            }
            EditorCommand::MoveToLineStart => {
                cursor.move_to_line_start();
            }
            EditorCommand::MoveToLineEnd => {
                let line_len = buffer.get_line(cursor.position.line).map(|l| l.len()).unwrap_or(0);
                cursor.move_to_line_end(line_len);
            }
            EditorCommand::Undo => {
                if let Some(previous) = self.undo_stack.pop() {
                    self.redo_stack.push(buffer.clone());
                    *buffer = previous;
                }
            }
            EditorCommand::Redo => {
                if let Some(next) = self.redo_stack.pop() {
                    self.undo_stack.push(buffer.clone());
                    *buffer = next;
                }
            }
            _ => {}
        }
    }
    
    fn save_state(&mut self, buffer: &TextBuffer) {
        self.undo_stack.push(buffer.clone());
        self.redo_stack.clear();
        
        // Keep undo stack size reasonable
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }
    
    fn cursor_to_char_index(&self, cursor: &Cursor, buffer: &TextBuffer) -> usize {
        let line_start = buffer.line_to_char(cursor.position.line);
        line_start + cursor.position.column.min(
            buffer.get_line(cursor.position.line).map(|l| l.len()).unwrap_or(0)
        )
    }
}