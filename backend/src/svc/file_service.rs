use crate::model::{FileInfo, FileContent, CreateFileRequest, UpdateFileRequest, FileEvent, FileEventType, FileEventMetadata};
use crate::repo::FileRepository;
use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

pub type EventSender = broadcast::Sender<FileEvent>;
pub type EventReceiver = broadcast::Receiver<FileEvent>;

#[derive(Clone)]
pub struct FileService {
    repository: Arc<FileRepository>,
    event_sender: EventSender,
    workspace_path: PathBuf,
    _watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
}

impl FileService {
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let repository = Arc::new(FileRepository::new(workspace_path.clone()));
        let (event_sender, _) = broadcast::channel(1000);
        
        let service = Self {
            repository,
            event_sender,
            workspace_path: workspace_path.clone(),
            _watcher: Arc::new(RwLock::new(None)),
        };

        // Initialize file watcher
        if let Err(e) = service.init_watcher(workspace_path) {
            error!("Failed to initialize file watcher: {}", e);
        }

        Ok(service)
    }

    fn init_watcher(&self, workspace_path: PathBuf) -> Result<()> {
        let event_sender = self.event_sender.clone();
        let workspace_path_clone = workspace_path.clone();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = Self::handle_fs_event(event, &event_sender, &workspace_path_clone) {
                            error!("Error handling file system event: {}", e);
                        }
                    }
                    Err(e) => error!("File watcher error: {:?}", e),
                }
            },
            Config::default(),
        )?;

        watcher.watch(&workspace_path, RecursiveMode::Recursive)?;
        
        tokio::spawn(async move {
            let _watcher = watcher; // Keep watcher alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        info!("File watcher initialized for: {:?}", workspace_path);
        Ok(())
    }

    fn handle_fs_event(
        event: Event,
        sender: &EventSender,
        workspace_path: &PathBuf,
    ) -> Result<()> {
        debug!("File system event: {:?}", event);

        let file_events = match event.kind {
            EventKind::Create(_) => {
                event.paths.iter().map(|path| {
                    let relative_path = path.strip_prefix(workspace_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    
                    let metadata = FileEventMetadata::new(path.is_dir());
                    FileEvent::new(FileEventType::Created, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            EventKind::Modify(_) => {
                event.paths.iter().map(|path| {
                    let relative_path = path.strip_prefix(workspace_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    
                    let mut metadata = FileEventMetadata::new(path.is_dir());
                    if let Ok(file_metadata) = std::fs::metadata(path) {
                        metadata = metadata.with_size(file_metadata.len());
                    }
                    
                    FileEvent::new(FileEventType::Modified, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            EventKind::Remove(_) => {
                event.paths.iter().map(|path| {
                    let relative_path = path.strip_prefix(workspace_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    
                    let metadata = FileEventMetadata::new(false); // We can't determine if it was a dir
                    FileEvent::new(FileEventType::Deleted, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            _ => return Ok(()), // Ignore other event types
        };

        for file_event in file_events {
            if let Err(e) = sender.send(file_event) {
                warn!("Failed to send file event: {}", e);
            }
        }

        Ok(())
    }

    pub fn subscribe_to_events(&self) -> EventReceiver {
        self.event_sender.subscribe()
    }

    pub async fn get_directory_tree(&self, path: &str) -> Result<FileInfo> {
        let path = if path.is_empty() || path == "/" { "." } else { path };
        self.repository.get_directory_tree(path).await
    }

    pub async fn get_file_content(&self, path: &str) -> Result<FileContent> {
        self.repository.read_file_content(path).await
    }

    pub async fn create_file_or_directory(&self, request: CreateFileRequest) -> Result<()> {
        let content = request.content.as_deref();
        self.repository.create_file(&request.path, content, request.is_dir).await?;

        // Send event
        let metadata = FileEventMetadata::new(request.is_dir);
        let event = FileEvent::new(FileEventType::Created, request.path).with_metadata(metadata);
        let _ = self.event_sender.send(event);

        Ok(())
    }

    pub async fn update_file_content(&self, path: &str, request: UpdateFileRequest) -> Result<()> {
        self.repository.update_file(path, &request.content).await?;

        // Send event
        let mut metadata = FileEventMetadata::new(false);
        if let Ok(file_metadata) = std::fs::metadata(self.workspace_path.join(path)) {
            metadata = metadata.with_size(file_metadata.len());
        }
        let event = FileEvent::new(FileEventType::Modified, path.to_string()).with_metadata(metadata);
        let _ = self.event_sender.send(event);

        Ok(())
    }

    pub async fn delete_file_or_directory(&self, path: &str) -> Result<()> {
        // Get metadata before deletion
        let file_path = self.workspace_path.join(path);
        let is_dir = file_path.is_dir();
        
        self.repository.delete_file(path).await?;

        // Send event
        let metadata = FileEventMetadata::new(is_dir);
        let event = FileEvent::new(FileEventType::Deleted, path.to_string()).with_metadata(metadata);
        let _ = self.event_sender.send(event);

        Ok(())
    }

    pub async fn rename_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        // Get metadata before renaming
        let file_path = self.workspace_path.join(old_path);
        let is_dir = file_path.is_dir();
        
        self.repository.rename_file(old_path, new_path).await?;

        // Send event
        let metadata = FileEventMetadata::new(is_dir)
            .with_rename(old_path.to_string(), new_path.to_string());
        let event = FileEvent::new(FileEventType::Renamed, new_path.to_string()).with_metadata(metadata);
        let _ = self.event_sender.send(event);

        Ok(())
    }

    pub fn get_active_connections(&self) -> usize {
        self.event_sender.receiver_count()
    }
}