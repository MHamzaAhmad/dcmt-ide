use yrs::{Doc, Transact, Update, StateVector, GetString, ReadTxn, Array};
use yrs::types::{TextRef, MapRef, Text, Map};
use yrs::types::array::ArrayRef;
use yrs::updates::decoder::Decode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::Result;

// All types are defined in this file for simplicity

/// LaTeX document structure using Yrs CRDT
#[derive(Clone, Debug)]
pub struct LaTeXDocument {
    pub id: Uuid,
    pub doc: Doc,
    pub content: TextRef,
    pub metadata: MapRef,
    pub packages: ArrayRef,
    pub figures: MapRef,
    pub bibliography: MapRef,
}

impl LaTeXDocument {
    pub fn new(id: Option<Uuid>) -> Self {
        let doc = Doc::new();
        let content = doc.get_or_insert_text("content");
        let metadata = doc.get_or_insert_map("metadata");
        let packages = doc.get_or_insert_array("packages");
        let figures = doc.get_or_insert_map("figures");
        let bibliography = doc.get_or_insert_map("bibliography");
        
        Self {
            id: id.unwrap_or_else(Uuid::new_v4),
            doc,
            content,
            metadata,
            packages,
            figures,
            bibliography,
        }
    }
    
    pub fn from_latex_content(latex: &str) -> Self {
        let mut doc = Self::new(None);
        
        // Parse LaTeX content and populate CRDT structures
        {
            let mut txn = doc.doc.transact_mut();
            doc.content.insert(&mut txn, 0, latex);
        }
        
        // Extract metadata, packages, etc.
        doc.parse_latex_structure(latex);
        
        doc
    }
    
    pub fn get_content(&self) -> String {
        let txn = self.doc.transact();
        self.content.get_string(&txn)
    }
    
    pub fn insert_text(&self, index: u32, text: &str) {
        let mut txn = self.doc.transact_mut();
        self.content.insert(&mut txn, index, text);
    }
    
    pub fn delete_text(&self, index: u32, length: u32) {
        let mut txn = self.doc.transact_mut();
        self.content.remove_range(&mut txn, index, length);
    }
    
    pub fn add_package(&self, package: &str) {
        let mut txn = self.doc.transact_mut();
        self.packages.push_back(&mut txn, package);
    }
    
    pub fn set_metadata(&self, key: &str, value: &str) {
        let mut txn = self.doc.transact_mut();
        self.metadata.insert(&mut txn, key, value);
    }
    
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        let txn = self.doc.transact();
        self.metadata.get(&txn, key).map(|v| v.to_string(&txn))
    }
    
    fn parse_latex_structure(&mut self, latex: &str) {
        // Extract document class
        if let Some(doc_class) = extract_document_class(latex) {
            self.set_metadata("documentclass", &doc_class);
        }
        
        // Extract packages
        for package in extract_packages(latex) {
            self.add_package(&package);
        }
        
        // Extract title, author, etc.
        if let Some(title) = extract_title(latex) {
            self.set_metadata("title", &title);
        }
        
        if let Some(author) = extract_author(latex) {
            self.set_metadata("author", &author);
        }
    }
    
    pub fn to_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&StateVector::default())
    }
    
    pub fn apply_update(&self, update: &[u8]) -> Result<()> {
        let mut txn = self.doc.transact_mut();
        let update = Update::decode_v1(update)?;
        let _ = txn.apply_update(update);
        Ok(())
    }
    
    pub fn state_vector(&self) -> StateVector {
        let txn = self.doc.transact();
        txn.state_vector()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub cursor_position: Option<u32>,
    pub selection_start: Option<u32>,
    pub selection_end: Option<u32>,
}

pub struct CollaborationEngine {
    documents: Arc<Mutex<HashMap<Uuid, LaTeXDocument>>>,
    update_sender: broadcast::Sender<CollaborationEvent>,
    user_info: UserInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CollaborationEvent {
    DocumentUpdate {
        document_id: Uuid,
        update: Vec<u8>,
    },
    AwarenessUpdate {
        document_id: Uuid,
        awareness: Vec<u8>,
    },
    UserJoined {
        document_id: Uuid,
        user: UserInfo,
    },
    UserLeft {
        document_id: Uuid,
        user_id: Uuid,
    },
}

impl CollaborationEngine {
    pub fn new(user_info: UserInfo) -> (Self, broadcast::Receiver<CollaborationEvent>) {
        let (update_sender, update_receiver) = broadcast::channel(1000);
        
        let engine = Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            update_sender,
            user_info,
        };
        
        (engine, update_receiver)
    }
    
    pub fn create_document(&self, latex_content: Option<&str>) -> Uuid {
        let doc = if let Some(content) = latex_content {
            LaTeXDocument::from_latex_content(content)
        } else {
            LaTeXDocument::new(None)
        };
        
        let doc_id = doc.id;
        
        {
            let mut documents = self.documents.lock().unwrap();
            documents.insert(doc_id, doc);
        }
        
        // TODO: Set up awareness for this document when needed
        
        doc_id
    }
    
    pub fn get_document(&self, doc_id: &Uuid) -> Option<LaTeXDocument> {
        let documents = self.documents.lock().unwrap();
        documents.get(doc_id).cloned()
    }
    
    pub fn apply_update(&self, doc_id: &Uuid, update: &[u8]) -> Result<()> {
        let documents = self.documents.lock().unwrap();
        
        if let Some(doc) = documents.get(doc_id) {
            doc.apply_update(update)?;
            
            // Broadcast the update to other clients
            let _ = self.update_sender.send(CollaborationEvent::DocumentUpdate {
                document_id: *doc_id,
                update: update.to_vec(),
            });
        }
        
        Ok(())
    }
    
    pub fn update_cursor(&self, _doc_id: &Uuid, _position: u32) {
        // TODO: Implement cursor position tracking with awareness
    }
    
    pub fn update_selection(&self, _doc_id: &Uuid, _start: u32, _end: u32) {
        // TODO: Implement selection tracking with awareness
    }
    
    pub fn get_all_users(&self) -> Vec<UserInfo> {
        // TODO: Return users from awareness
        vec![self.user_info.clone()]
    }
}

// Helper functions for parsing LaTeX structure (simplified for now)
fn extract_document_class(latex: &str) -> Option<String> {
    // Simple string matching for now
    latex.lines()
        .find(|line| line.contains("\\documentclass"))
        .and_then(|line| {
            let start = line.find('{')?;
            let end = line.find('}')?;
            if end > start {
                Some(line[start + 1..end].to_string())
            } else {
                None
            }
        })
}

fn extract_packages(latex: &str) -> Vec<String> {
    // Simple string matching for now  
    latex.lines()
        .filter(|line| line.contains("\\usepackage"))
        .filter_map(|line| {
            let start = line.find('{')?;
            let end = line.find('}')?;
            if end > start {
                Some(line[start + 1..end].to_string())
            } else {
                None
            }
        })
        .collect()
}

fn extract_title(latex: &str) -> Option<String> {
    // Simple string matching for now
    latex.lines()
        .find(|line| line.contains("\\title"))
        .and_then(|line| {
            let start = line.find('{')?;
            let end = line.find('}')?;
            if end > start {
                Some(line[start + 1..end].to_string())
            } else {
                None
            }
        })
}

fn extract_author(latex: &str) -> Option<String> {
    // Simple string matching for now
    latex.lines()
        .find(|line| line.contains("\\author"))
        .and_then(|line| {
            let start = line.find('{')?;
            let end = line.find('}')?;
            if end > start {
                Some(line[start + 1..end].to_string())
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_latex_document_creation() {
        let doc = LaTeXDocument::new(None);
        assert!(!doc.id.is_nil());
    }
    
    #[test]
    fn test_latex_parsing() {
        let latex = r#"
\documentclass{article}
\usepackage{amsmath}
\usepackage{graphicx}
\title{Test Document}
\author{John Doe}

\begin{document}
Hello world!
\end{document}
        "#;
        
        let doc = LaTeXDocument::from_latex_content(latex);
        assert_eq!(doc.get_metadata("documentclass"), Some("article".to_string()));
        assert_eq!(doc.get_metadata("title"), Some("Test Document".to_string()));
        assert_eq!(doc.get_metadata("author"), Some("John Doe".to_string()));
    }
}