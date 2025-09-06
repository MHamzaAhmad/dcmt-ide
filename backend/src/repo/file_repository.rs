use crate::model::{FileInfo, FileContent, sanitize_path};
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

pub struct FileRepository {
    workspace_path: PathBuf,
}

impl FileRepository {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    fn get_full_path(&self, relative_path: &str) -> Result<PathBuf> {
        let sanitized = sanitize_path(relative_path)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        
        let full_path = self.workspace_path.join(sanitized);
        
        // Ensure the path is within workspace bounds
        if !full_path.starts_with(&self.workspace_path) {
            return Err(anyhow::anyhow!("Path outside workspace bounds"));
        }
        
        Ok(full_path)
    }

    pub async fn get_directory_tree(&self, path: &str) -> Result<FileInfo> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Getting directory tree for: {:?}", full_path);
        
        if !full_path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path));
        }

        self.build_file_info(&full_path, true).await
    }

    fn build_file_info<'a>(&'a self, path: &'a Path, include_children: bool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FileInfo>> + Send + 'a>> {
        Box::pin(async move {
        let metadata = fs::metadata(path).await
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;
        
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let relative_path = path.strip_prefix(&self.workspace_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let is_dir = metadata.is_dir();
        let size = if is_dir { None } else { Some(metadata.len()) };
        let modified = metadata.modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs());

        let mut file_info = FileInfo::new(relative_path, name, is_dir, size, modified);

        if is_dir && include_children {
            if let Ok(mut entries) = fs::read_dir(path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    match self.build_file_info(&entry.path(), true).await {
                        Ok(child_info) => file_info.add_child(child_info),
                        Err(e) => {
                            warn!("Failed to read entry {:?}: {}", entry.path(), e);
                        }
                    }
                }
            }
        }

        Ok(file_info)
        })
    }

    pub async fn read_file_content(&self, path: &str) -> Result<FileContent> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Reading file content: {:?}", full_path);
        
        if !full_path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path));
        }

        if full_path.is_dir() {
            return Err(anyhow::anyhow!("Cannot read content of directory: {}", path));
        }

        let content = fs::read_to_string(&full_path).await
            .with_context(|| format!("Failed to read file: {}", path))?;

        Ok(FileContent {
            path: path.to_string(),
            content,
            encoding: "utf-8".to_string(),
        })
    }

    pub async fn read_file_raw(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Reading raw file content: {:?}", full_path);
        
        if !full_path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path));
        }

        if full_path.is_dir() {
            return Err(anyhow::anyhow!("Cannot read content of directory: {}", path));
        }

        let content = fs::read(&full_path).await
            .with_context(|| format!("Failed to read raw file: {}", path))?;

        Ok(content)
    }

    pub async fn create_file(&self, path: &str, content: Option<&str>, is_dir: bool) -> Result<()> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Creating {}: {:?}", if is_dir { "directory" } else { "file" }, full_path);
        
        if full_path.exists() {
            return Err(anyhow::anyhow!("Path already exists: {}", path));
        }

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create parent directories for: {}", path))?;
        }

        if is_dir {
            fs::create_dir(&full_path).await
                .with_context(|| format!("Failed to create directory: {}", path))?;
        } else {
            let content = content.unwrap_or("");
            fs::write(&full_path, content).await
                .with_context(|| format!("Failed to create file: {}", path))?;
        }

        Ok(())
    }

    pub async fn update_file(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Updating file: {:?}", full_path);
        
        if !full_path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path));
        }

        if full_path.is_dir() {
            return Err(anyhow::anyhow!("Cannot update directory content: {}", path));
        }

        fs::write(&full_path, content).await
            .with_context(|| format!("Failed to update file: {}", path))?;

        Ok(())
    }

    pub async fn delete_file(&self, path: &str) -> Result<()> {
        let full_path = self.get_full_path(path)?;
        
        debug!("Deleting: {:?}", full_path);
        
        if !full_path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path));
        }

        if full_path.is_dir() {
            fs::remove_dir_all(&full_path).await
                .with_context(|| format!("Failed to delete directory: {}", path))?;
        } else {
            fs::remove_file(&full_path).await
                .with_context(|| format!("Failed to delete file: {}", path))?;
        }

        Ok(())
    }

    pub async fn rename_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old_full_path = self.get_full_path(old_path)?;
        let new_full_path = self.get_full_path(new_path)?;
        
        debug!("Renaming: {:?} to {:?}", old_full_path, new_full_path);
        
        if !old_full_path.exists() {
            return Err(anyhow::anyhow!("Source path does not exist: {}", old_path));
        }

        if new_full_path.exists() {
            return Err(anyhow::anyhow!("Destination path already exists: {}", new_path));
        }

        // Ensure parent directory exists for new path
        if let Some(parent) = new_full_path.parent() {
            fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create parent directories for: {}", new_path))?;
        }

        fs::rename(&old_full_path, &new_full_path).await
            .with_context(|| format!("Failed to rename {} to {}", old_path, new_path))?;

        Ok(())
    }
}