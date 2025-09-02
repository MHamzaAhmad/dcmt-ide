//! Desktop native directory picker

use std::path::PathBuf;

/// Opens a native directory picker dialog
/// Returns Some(PathBuf) if a directory was selected, None if cancelled
#[cfg(feature = "desktop")]
pub fn pick_workspace_directory() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Open LaTeX Project Folder")
        .set_directory(get_default_directory())
        .pick_folder()
}

/// Gets the default directory to open the picker in
#[cfg(feature = "desktop")]
fn get_default_directory() -> PathBuf {
    // Try to get user's documents directory, fallback to home
    if let Some(docs_dir) = dirs::document_dir() {
        docs_dir
    } else if let Some(home_dir) = dirs::home_dir() {
        home_dir
    } else {
        PathBuf::from(".")
    }
}

/// For non-desktop builds, return None
#[cfg(not(feature = "desktop"))]
pub fn pick_workspace_directory() -> Option<PathBuf> {
    None
}

/// Open a file picker for selecting LaTeX files specifically
#[cfg(feature = "desktop")]  
pub fn pick_latex_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Select Main LaTeX File")
        .set_directory(get_default_directory())
        .add_filter("LaTeX files", &["tex"])
        .add_filter("All files", &["*"])
        .pick_file()
}

#[cfg(not(feature = "desktop"))]
pub fn pick_latex_file() -> Option<PathBuf> {
    None
}

/// Create a new project dialog with template selection
#[cfg(feature = "desktop")]
pub fn create_new_project_dialog() -> Option<(PathBuf, String)> {
    // First, pick where to create the project
    let parent_dir = rfd::FileDialog::new()
        .set_title("Choose Location for New Project")
        .set_directory(get_default_directory())
        .pick_folder()?;
    
    // For now, just return with a default name
    // TODO: Add a custom dialog for project name and template selection
    let project_name = "new-latex-project".to_string();
    let project_path = parent_dir.join(&project_name);
    
    Some((project_path, project_name))
}

#[cfg(not(feature = "desktop"))]  
pub fn create_new_project_dialog() -> Option<(PathBuf, String)> {
    None
}

/// Save file dialog for LaTeX documents
#[cfg(feature = "desktop")]
pub fn save_latex_file(default_name: Option<&str>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save LaTeX File")
        .set_directory(get_default_directory())
        .add_filter("LaTeX files", &["tex"])
        .add_filter("All files", &["*"]);
    
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(name);
    }
    
    dialog.save_file()
}

#[cfg(not(feature = "desktop"))]
pub fn save_latex_file(_default_name: Option<&str>) -> Option<PathBuf> {
    None
}