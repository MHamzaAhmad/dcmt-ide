use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug)]
pub struct Cursor {
    pub position: Position,
    pub desired_column: Option<usize>,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            position: Position::new(0, 0),
            desired_column: None,
        }
    }
    
    pub fn move_to(&mut self, position: Position) {
        self.position = position;
        self.desired_column = None;
    }
    
    pub fn move_left(&mut self) {
        if self.position.column > 0 {
            self.position.column -= 1;
            self.desired_column = None;
        }
    }
    
    pub fn move_right(&mut self, max_column: usize) {
        if self.position.column < max_column {
            self.position.column += 1;
            self.desired_column = None;
        }
    }
    
    pub fn move_up(&mut self) {
        if self.position.line > 0 {
            self.position.line -= 1;
            if let Some(col) = self.desired_column {
                self.position.column = col;
            } else {
                self.desired_column = Some(self.position.column);
            }
        }
    }
    
    pub fn move_down(&mut self, max_line: usize) {
        if self.position.line < max_line {
            self.position.line += 1;
            if let Some(col) = self.desired_column {
                self.position.column = col;
            } else {
                self.desired_column = Some(self.position.column);
            }
        }
    }
    
    pub fn move_to_line_start(&mut self) {
        self.position.column = 0;
        self.desired_column = None;
    }
    
    pub fn move_to_line_end(&mut self, line_length: usize) {
        self.position.column = line_length;
        self.desired_column = None;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}