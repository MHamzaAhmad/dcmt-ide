use ropey::Rope;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TextBuffer {
    rope: Rope,
    dirty: bool,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            dirty: false,
        }
    }
    
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            dirty: false,
        }
    }
    
    pub fn insert(&mut self, pos: usize, text: &str) {
        if pos <= self.rope.len_chars() {
            self.rope.insert(pos, text);
            self.dirty = true;
        }
    }
    
    pub fn delete(&mut self, start: usize, end: usize) {
        if start < end && end <= self.rope.len_chars() {
            self.rope.remove(start..end);
            self.dirty = true;
        }
    }
    
    pub fn get_line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }
    
    pub fn get_text(&self) -> String {
        self.rope.to_string()
    }
    
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }
    
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }
    
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }
    
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}