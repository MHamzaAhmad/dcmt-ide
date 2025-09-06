use crate::models::{FileInfo, FileContent, FileType, CreateFileRequest, FileResult, FileError};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub struct FileService {
    workspace_path: PathBuf,
}

impl FileService {
    pub fn new(workspace_path: PathBuf) -> Self {
        info!("Initializing FileService with workspace: {:?}", workspace_path);
        Self { workspace_path }
    }

    pub fn get_directory_tree(&self, relative_path: &str) -> FileResult<FileInfo> {
        let path = if relative_path.is_empty() || relative_path == "/" {
            self.workspace_path.clone()
        } else {
            self.workspace_path.join(relative_path)
        };

        debug!("Getting directory tree for: {:?}", path);

        if !path.exists() {
            return Err(FileError::NotFound {
                path: relative_path.to_string(),
            });
        }

        self.build_file_info(&path, relative_path)
    }

    fn build_file_info(&self, path: &Path, relative_path: &str) -> FileResult<FileInfo> {
        let metadata = fs::metadata(path)?;
        let name = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut file_info = FileInfo {
            name: if relative_path.is_empty() { "workspace".to_string() } else { name },
            path: relative_path.to_string(),
            file_type: if metadata.is_dir() { FileType::Directory } else { FileType::File },
            size: Some(metadata.len()),
            modified: metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            children: None,
        };

        if metadata.is_dir() {
            let mut children = Vec::new();
            
            match fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let child_path = entry.path();
                        let child_relative = if relative_path.is_empty() {
                            child_path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        } else {
                            format!("{}/{}", relative_path, child_path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy())
                        };

                        match self.build_file_info(&child_path, &child_relative) {
                            Ok(child_info) => children.push(child_info),
                            Err(e) => {
                                warn!("Failed to build info for child {:?}: {}", child_path, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read directory {:?}: {}", path, e);
                }
            }

            // Sort children: directories first, then files, both alphabetically
            children.sort_by(|a, b| {
                use std::cmp::Ordering;
                match (&a.file_type, &b.file_type) {
                    (FileType::Directory, FileType::File) => Ordering::Less,
                    (FileType::File, FileType::Directory) => Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });

            file_info.children = Some(children);
        }

        Ok(file_info)
    }

    pub fn read_file_content(&self, relative_path: &str) -> FileResult<FileContent> {
        let path = self.workspace_path.join(relative_path);
        
        debug!("Reading file content: {:?}", path);

        if !path.exists() {
            return Err(FileError::NotFound {
                path: relative_path.to_string(),
            });
        }

        if path.is_dir() {
            return Err(FileError::InvalidPath {
                path: relative_path.to_string(),
            });
        }

        let content = fs::read_to_string(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                path: relative_path.to_string(),
            },
            _ => FileError::from(e),
        })?;

        let metadata = fs::metadata(&path)?;

        Ok(FileContent {
            path: relative_path.to_string(),
            content,
            size: metadata.len(),
            modified: metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        })
    }

    pub fn read_file_raw(&self, relative_path: &str) -> FileResult<Vec<u8>> {
        let path = self.workspace_path.join(relative_path);
        
        debug!("Reading raw file content: {:?}", path);

        if !path.exists() {
            return Err(FileError::NotFound {
                path: relative_path.to_string(),
            });
        }

        if path.is_dir() {
            return Err(FileError::InvalidPath {
                path: relative_path.to_string(),
            });
        }

        let content = fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                path: relative_path.to_string(),
            },
            _ => FileError::from(e),
        })?;

        Ok(content)
    }

    pub fn write_file_content(&self, relative_path: &str, content: &str) -> FileResult<()> {
        let path = self.workspace_path.join(relative_path);
        
        debug!("Writing file content: {:?}", path);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                path: relative_path.to_string(),
            },
            _ => FileError::from(e),
        })?;

        info!("File written successfully: {}", relative_path);
        Ok(())
    }

    pub fn create_file_or_directory(&self, request: CreateFileRequest) -> FileResult<()> {
        let path = self.workspace_path.join(&request.path);
        
        debug!("Creating {}: {:?}", if request.is_directory { "directory" } else { "file" }, path);

        if path.exists() {
            return Err(FileError::AlreadyExists {
                path: request.path,
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        if request.is_directory {
            fs::create_dir(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                    path: request.path.clone(),
                },
                _ => FileError::from(e),
            })?;
        } else {
            let content = request.content.as_deref().unwrap_or("");
            fs::write(&path, content).map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                    path: request.path.clone(),
                },
                _ => FileError::from(e),
            })?;
        }

        info!("Created successfully: {}", request.path);
        Ok(())
    }

    pub fn delete_file_or_directory(&self, relative_path: &str) -> FileResult<()> {
        let path = self.workspace_path.join(relative_path);
        
        debug!("Deleting: {:?}", path);

        if !path.exists() {
            return Err(FileError::NotFound {
                path: relative_path.to_string(),
            });
        }

        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                    path: relative_path.to_string(),
                },
                _ => FileError::from(e),
            })?;
        } else {
            fs::remove_file(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                    path: relative_path.to_string(),
                },
                _ => FileError::from(e),
            })?;
        }

        info!("Deleted successfully: {}", relative_path);
        Ok(())
    }

    pub fn rename_file(&self, old_relative_path: &str, new_relative_path: &str) -> FileResult<()> {
        let old_path = self.workspace_path.join(old_relative_path);
        let new_path = self.workspace_path.join(new_relative_path);
        
        debug!("Renaming {:?} to {:?}", old_path, new_path);

        if !old_path.exists() {
            return Err(FileError::NotFound {
                path: old_relative_path.to_string(),
            });
        }

        if new_path.exists() {
            return Err(FileError::AlreadyExists {
                path: new_relative_path.to_string(),
            });
        }

        // Ensure parent directory exists for new path
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&old_path, &new_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                path: old_relative_path.to_string(),
            },
            _ => FileError::from(e),
        })?;

        info!("Renamed successfully: {} -> {}", old_relative_path, new_relative_path);
        Ok(())
    }

    pub fn file_exists(&self, relative_path: &str) -> bool {
        let path = self.workspace_path.join(relative_path);
        path.exists()
    }

    pub fn get_workspace_path(&self) -> &PathBuf {
        &self.workspace_path
    }
}