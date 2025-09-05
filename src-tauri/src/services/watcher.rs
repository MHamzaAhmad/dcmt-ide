use crate::models::{FileEvent, FileEventType, FileEventMetadata, FileResult, FileError};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct FileWatcher {
    _watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
    app_handle: AppHandle,
    workspace_path: PathBuf,
}

impl FileWatcher {
    pub fn new(app_handle: AppHandle, workspace_path: PathBuf) -> FileResult<Self> {
        let watcher_service = Self {
            _watcher: Arc::new(RwLock::new(None)),
            app_handle,
            workspace_path,
        };

        watcher_service.init_watcher()?;
        Ok(watcher_service)
    }

    fn init_watcher(&self) -> FileResult<()> {
        let app_handle = self.app_handle.clone();
        let workspace_path = self.workspace_path.clone();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = Self::handle_fs_event(event, &app_handle, &workspace_path) {
                            error!("Error handling file system event: {}", e);
                        }
                    }
                    Err(e) => error!("File watcher error: {:?}", e),
                }
            },
            Config::default(),
        )?;

        watcher.watch(&self.workspace_path, RecursiveMode::Recursive)?;
        
        // Store the watcher to keep it alive
        let watcher_clone = Arc::clone(&self._watcher);
        tokio::spawn(async move {
            let mut watcher_guard = watcher_clone.write().await;
            *watcher_guard = Some(watcher);
            
            // Keep the watcher alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        info!("File watcher initialized for: {:?}", self.workspace_path);
        Ok(())
    }

    fn handle_fs_event(
        event: Event,
        app_handle: &AppHandle,
        workspace_path: &PathBuf,
    ) -> FileResult<()> {
        debug!("File system event: {:?}", event);

        // Filter out temporary files and system files
        let paths: Vec<_> = event.paths.iter()
            .filter(|path| !Self::should_ignore_path(path))
            .collect();

        if paths.is_empty() {
            return Ok(());
        }

        let file_events = match event.kind {
            EventKind::Create(_) => {
                paths.iter().map(|path| {
                    let relative_path = Self::get_relative_path(path, workspace_path);
                    let metadata = FileEventMetadata::new(path.is_dir());
                    FileEvent::new(FileEventType::Created, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            EventKind::Modify(_) => {
                paths.iter().map(|path| {
                    let relative_path = Self::get_relative_path(path, workspace_path);
                    let mut metadata = FileEventMetadata::new(path.is_dir());
                    
                    if let Ok(file_metadata) = std::fs::metadata(path) {
                        metadata = metadata.with_size(file_metadata.len());
                    }
                    
                    FileEvent::new(FileEventType::Modified, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            EventKind::Remove(_) => {
                paths.iter().map(|path| {
                    let relative_path = Self::get_relative_path(path, workspace_path);
                    let metadata = FileEventMetadata::new(false); // Can't determine if it was a dir after deletion
                    FileEvent::new(FileEventType::Deleted, relative_path).with_metadata(metadata)
                }).collect::<Vec<_>>()
            }
            _ => return Ok(()), // Ignore other event types
        };

        // Emit events to the frontend
        for file_event in file_events {
            let event_name = match file_event.event_type {
                FileEventType::Created => "file-created",
                FileEventType::Modified => "file-modified",
                FileEventType::Deleted => "file-deleted",
                FileEventType::Renamed => "file-renamed",
            };

            if let Err(e) = app_handle.emit(event_name, &file_event) {
                warn!("Failed to emit file event {}: {}", event_name, e);
            } else {
                debug!("Emitted event {}: {}", event_name, file_event.path);
            }
        }

        Ok(())
    }

    fn get_relative_path(path: &std::path::Path, workspace_path: &PathBuf) -> String {
        path.strip_prefix(workspace_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    fn should_ignore_path(path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Ignore temporary files, system files, and common build artifacts
        let ignore_patterns = [
            ".DS_Store",
            ".git/",
            "node_modules/",
            "target/",
            "dist/",
            "build/",
            ".tmp",
            ".temp",
            "~$",
            ".swp",
            ".lock",
        ];

        for pattern in &ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        // Ignore hidden files starting with dot (except .gitignore, .env, etc.)
        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy();
            if name.starts_with('.') && 
               !name.starts_with(".git") && 
               !name.starts_with(".env") &&
               name != ".gitignore" {
                return true;
            }
        }

        false
    }

    pub async fn stop(&self) -> FileResult<()> {
        let mut watcher_guard = self._watcher.write().await;
        if watcher_guard.take().is_some() {
            info!("File watcher stopped");
        }
        Ok(())
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        info!("FileWatcher dropped");
    }
}

// Helper function to emit rename events
pub fn emit_rename_event(
    app_handle: &AppHandle,
    old_path: &str,
    new_path: &str,
    is_directory: bool,
) -> FileResult<()> {
    let metadata = FileEventMetadata::new(is_directory)
        .with_rename(old_path.to_string(), new_path.to_string());
    
    let event = FileEvent::new(FileEventType::Renamed, new_path.to_string())
        .with_metadata(metadata);

    app_handle.emit("file-renamed", &event)
        .map_err(|e| FileError::WatchError {
            message: format!("Failed to emit rename event: {}", e),
        })?;

    debug!("Emitted rename event: {} -> {}", old_path, new_path);
    Ok(())
}