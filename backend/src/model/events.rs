use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub id: String,
    pub event_type: FileEventType,
    pub path: String,
    pub timestamp: u64,
    pub metadata: Option<FileEventMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEventMetadata {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub size: Option<u64>,
    pub is_dir: bool,
}

impl FileEvent {
    pub fn new(event_type: FileEventType, path: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            path,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: FileEventMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl FileEventMetadata {
    pub fn new(is_dir: bool) -> Self {
        Self {
            old_path: None,
            new_path: None,
            size: None,
            is_dir,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_rename(mut self, old_path: String, new_path: String) -> Self {
        self.old_path = Some(old_path);
        self.new_path = Some(new_path);
        self
    }
}