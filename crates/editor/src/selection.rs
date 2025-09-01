use crate::cursor::Position;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    
    pub fn normalize(&self) -> Self {
        if self.start.line < self.end.line || 
           (self.start.line == self.end.line && self.start.column <= self.end.column) {
            self.clone()
        } else {
            Self::new(self.end, self.start)
        }
    }
    
    pub fn contains(&self, position: Position) -> bool {
        let normalized = self.normalize();
        position.line >= normalized.start.line && 
        position.line <= normalized.end.line &&
        (position.line != normalized.start.line || position.column >= normalized.start.column) &&
        (position.line != normalized.end.line || position.column <= normalized.end.column)
    }
}