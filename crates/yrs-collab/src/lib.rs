use yrs::{Doc, Text, Map, Array, Transact, Update, StateVector, ReadTxn, WriteTxn};
use yrs::awareness::{Awareness, AwarenessUpdate, Event};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::Result;

pub mod document;
pub mod collaboration;
pub mod awareness;
pub mod sync;

pub use document::LaTeXDocument;
pub use collaboration::CollaborationEngine;
pub use awareness::UserAwareness;
pub use sync::YrsSync;

/// LaTeX document structure using Yrs CRDT
#[derive(Clone, Debug)]
pub struct LaTeXDocument {
    pub id: Uuid,
    pub doc: Doc,
    pub content: Text,
    pub metadata: Map,
    pub packages: Array,
    pub figures: Map,
    pub bibliography: Map,
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
        let txn = doc.doc.transact_mut();
        doc.content.insert(&txn, 0, latex);
        
        // Extract metadata, packages, etc.
        doc.parse_latex_structure(latex);
        
        doc
    }
    
    pub fn get_content(&self) -> String {
        let txn = self.doc.transact();
        self.content.get_string(&txn)
    }
    
    pub fn insert_text(&self, index: u32, text: &str) {
        let txn = self.doc.transact_mut();
        self.content.insert(&txn, index, text);
    }
    
    pub fn delete_text(&self, index: u32, length: u32) {
        let txn = self.doc.transact_mut();
        self.content.remove_range(&txn, index, length);
    }
    
    pub fn add_package(&self, package: &str) {
        let txn = self.doc.transact_mut();
        self.packages.insert(&txn, self.packages.len(&txn), package.into());
    }
    
    pub fn set_metadata(&self, key: &str, value: &str) {
        let txn = self.doc.transact_mut();
        self.metadata.insert(&txn, key.to_string(), value.into());
    }
    
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        let txn = self.doc.transact();
        self.metadata.get(&txn, key).and_then(|v| v.to_string())
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
    
    pub fn to_update(&self) -> Update {
        let txn = self.doc.transact();
        txn.encode_update_v1()
    }
    
    pub fn apply_update(&self, update: &Update) -> Result<()> {
        let txn = self.doc.transact_mut();
        txn.apply_update(update.clone())?;
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
    awareness: Arc<Mutex<Awareness>>,
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
            awareness: Arc::new(Mutex::new(Awareness::new())),
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
        
        // Set up awareness for this document
        {
            let mut awareness = self.awareness.lock().unwrap();
            awareness.set_local_state(serde_json::to_value(&self.user_info).unwrap());
        }
        
        doc_id
    }
    
    pub fn get_document(&self, doc_id: &Uuid) -> Option<LaTeXDocument> {
        let documents = self.documents.lock().unwrap();
        documents.get(doc_id).cloned()
    }
    
    pub fn apply_update(&self, doc_id: &Uuid, update: &[u8]) -> Result<()> {
        let documents = self.documents.lock().unwrap();
        
        if let Some(doc) = documents.get(doc_id) {
            let update = Update::decode_v1(update)?;
            doc.apply_update(&update)?;
            
            // Broadcast the update to other clients
            let _ = self.update_sender.send(CollaborationEvent::DocumentUpdate {
                document_id: *doc_id,
                update: update.encode_v1(),
            });
        }
        
        Ok(())
    }
    
    pub fn update_cursor(&self, doc_id: &Uuid, position: u32) {
        // Update local user cursor position
        let mut user_info = self.user_info.clone();
        user_info.cursor_position = Some(position);
        
        // Update awareness
        {
            let mut awareness = self.awareness.lock().unwrap();
            awareness.set_local_state(serde_json::to_value(&user_info).unwrap());
        }
        
        // Broadcast awareness update
        if let Ok(awareness_update) = self.get_awareness_update() {
            let _ = self.update_sender.send(CollaborationEvent::AwarenessUpdate {
                document_id: *doc_id,
                awareness: awareness_update,
            });
        }
    }
    
    pub fn update_selection(&self, doc_id: &Uuid, start: u32, end: u32) {
        let mut user_info = self.user_info.clone();
        user_info.selection_start = Some(start);
        user_info.selection_end = Some(end);
        
        {
            let mut awareness = self.awareness.lock().unwrap();
            awareness.set_local_state(serde_json::to_value(&user_info).unwrap());
        }
        
        if let Ok(awareness_update) = self.get_awareness_update() {
            let _ = self.update_sender.send(CollaborationEvent::AwarenessUpdate {
                document_id: *doc_id,
                awareness: awareness_update,
            });
        }
    }
    
    fn get_awareness_update(&self) -> Result<Vec<u8>> {
        let awareness = self.awareness.lock().unwrap();
        let update = awareness.update()?;
        Ok(update.encode_v1())
    }
    
    pub fn get_all_users(&self) -> Vec<UserInfo> {
        let awareness = self.awareness.lock().unwrap();
        let mut users = Vec::new();
        
        for (_, state) in awareness.clients() {
            if let Ok(user_info) = serde_json::from_value::<UserInfo>(state.clone()) {
                users.push(user_info);
            }
        }
        
        users
    }
}

// Helper functions for parsing LaTeX structure
fn extract_document_class(latex: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"\\documentclass(?:\[[^\]]*\])?\{([^}]+)\}").ok()?;
    re.captures(latex)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_packages(latex: &str) -> Vec<String> {
    use regex::Regex;
    let re = Regex::new(r"\\usepackage(?:\[[^\]]*\])?\{([^}]+)\}").unwrap();
    re.captures_iter(latex)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

fn extract_title(latex: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"\\title\{([^}]+)\}").ok()?;
    re.captures(latex)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_author(latex: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"\\author\{([^}]+)\}").ok()?;
    re.captures(latex)?.get(1).map(|m| m.as_str().to_string())
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