//! Enhanced CodeMirror integration with comprehensive WASM bindings
//! 
//! This module now provides a robust, type-safe interface to CodeMirror 6
//! with proper error handling and cross-platform support.

pub mod bindings;
pub mod wrapper;

#[cfg(target_arch = "wasm32")]
pub use wrapper::{CodeMirrorEditor as Editor, LineInfo, SelectionInfo, EditorCommand};

#[cfg(not(target_arch = "wasm32"))]
pub use crate::desktop::DesktopCodeMirrorEditor as Editor;

use latex_ide_ui::*;
use serde::{Serialize, Deserialize};

/// Common trait for CodeMirror editor operations across all platforms
pub trait CodeMirrorOps {
    /// Get the complete document content
    fn get_content(&self) -> String;
    
    /// Replace the entire document content
    fn set_content(&self, content: &str);
    
    /// Format the document using LaTeX formatting rules
    fn format(&self);
    
    /// Add a decoration (highlight, underline, etc.) to a text range
    /// Returns a unique decoration ID for later removal
    fn add_decoration(&self, from: usize, to: usize, decoration_type: DecorationType) -> String;
    
    /// Remove a decoration by its ID
    fn remove_decoration(&self, decoration_id: &str);
    
    /// Scroll to make the specified line visible
    fn scroll_to_line(&self, line: usize);
    
    /// Temporarily highlight a line (for PDF sync)
    fn highlight_line(&self, line: usize);
}

/// Types of decorations that can be applied to editor content
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DecorationType {
    /// Green background for AI-suggested additions
    Addition,
    /// Red strikethrough for AI-suggested deletions  
    Deletion,
    /// Yellow background for AI-suggested modifications
    Modification,
    /// Purple highlight for PDF sync
    SyncHighlight,
    /// Custom decoration with CSS class name
    Custom(String),
}

impl DecorationType {
    /// Convert decoration type to CSS class name
    pub fn to_class(&self) -> &str {
        match self {
            DecorationType::Addition => "cm-addition",
            DecorationType::Deletion => "cm-deletion", 
            DecorationType::Modification => "cm-modification",
            DecorationType::SyncHighlight => "cm-sync-highlight",
            DecorationType::Custom(class) => class,
        }
    }
    
    /// Get a human-readable description of the decoration type
    pub fn description(&self) -> &str {
        match self {
            DecorationType::Addition => "Addition (AI suggestion)",
            DecorationType::Deletion => "Deletion (AI suggestion)",
            DecorationType::Modification => "Modification (AI suggestion)", 
            DecorationType::SyncHighlight => "PDF sync highlight",
            DecorationType::Custom(_) => "Custom decoration",
        }
    }
}

/// Format LaTeX content with proper indentation and structure
/// 
/// This function handles basic LaTeX formatting including:
/// - Proper indentation for environments
/// - Alignment of math blocks
/// - Consistent spacing
pub fn format_latex(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut formatted = Vec::new();
    let mut indent_level: usize = 0;
    let mut in_math_environment = false;
    
    for line in lines {
        let trimmed = line.trim();
        
        // Skip empty lines but preserve them
        if trimmed.is_empty() {
            formatted.push(String::new());
            continue;
        }
        
        // Handle math environments
        if trimmed.starts_with("\\[") || trimmed.starts_with("\\begin{equation")
            || trimmed.starts_with("\\begin{align") || trimmed.starts_with("\\begin{gather") {
            in_math_environment = true;
        } else if trimmed.starts_with("\\]") || trimmed.starts_with("\\end{equation")
            || trimmed.starts_with("\\end{align") || trimmed.starts_with("\\end{gather") {
            in_math_environment = false;
        }
        
        // Decrease indent for end tags
        if trimmed.starts_with("\\end{") {
            indent_level = indent_level.saturating_sub(1);
        }
        
        // Create indentation
        let indent = if in_math_environment && !trimmed.starts_with("\\") {
            // Extra indent for math content
            "\t".repeat(indent_level + 1)
        } else {
            "\t".repeat(indent_level)
        };
        
        // Add indented line
        formatted.push(format!("{}{}", indent, trimmed));
        
        // Increase indent for begin tags
        if trimmed.starts_with("\\begin{") {
            indent_level += 1;
        }
        
        // Special handling for document structure
        if trimmed.starts_with("\\documentclass") || trimmed.starts_with("\\usepackage") {
            // These don't change indent but add spacing
            if !formatted.last().map_or(false, |l| l.is_empty()) {
                formatted.push(String::new());
            }
        }
    }
    
    formatted.join("\n")
}

/// Simplified props for backward compatibility
/// 
/// This maintains the existing API while leveraging the enhanced
/// internal implementation.
#[derive(Props, Clone, PartialEq)]
pub struct CodeMirrorProps {
    /// Document content signal
    pub content: Signal<String>,
    
    /// Callback for content changes
    #[props(default)]
    pub on_change: Option<EventHandler<String>>,
    
    /// Enable AI suggestion features
    #[props(default = false)]
    pub enable_ai_suggestions: bool,
    
    /// Enable PDF synchronization features  
    #[props(default = false)]
    pub enable_pdf_sync: bool,
    
    /// Additional CSS classes
    #[props(default)]
    pub class: Option<String>,
    
    /// Unique editor identifier
    #[props(default)]
    pub editor_id: Option<String>,
}

/// Configuration for creating a new editor instance
#[derive(Clone, Debug)]
pub struct EditorConfig {
    /// Initial document content
    pub initial_content: Option<String>,
    
    /// Enable AI suggestion features
    pub enable_ai_suggestions: bool,
    
    /// Enable PDF synchronization features
    pub enable_pdf_sync: bool,
    
    /// Use dark theme
    pub dark_theme: bool,
    
    /// Enable line wrapping
    pub line_wrapping: bool,
    
    /// Enable line numbers
    pub line_numbers: bool,
    
    /// Language mode (defaults to LaTeX)
    pub language_mode: LanguageMode,
    
    /// Custom extensions to add
    pub custom_extensions: Vec<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            initial_content: None,
            enable_ai_suggestions: false,
            enable_pdf_sync: false,
            dark_theme: false,
            line_wrapping: true,
            line_numbers: true,  // Enable line numbers by default
            language_mode: LanguageMode::LaTeX,
            custom_extensions: Vec::new(),
        }
    }
}

/// Supported language modes
#[derive(Clone, Debug, PartialEq)]
pub enum LanguageMode {
    LaTeX,
    Markdown,
    PlainText,
    Custom(String),
}

impl Default for LanguageMode {
    fn default() -> Self {
        LanguageMode::LaTeX
    }
}

impl CodeMirrorProps {
    /// Convert to the enhanced EditorConfig
    pub fn to_editor_config(&self) -> EditorConfig {
        EditorConfig {
            initial_content: Some(self.content.read().clone()),
            enable_ai_suggestions: self.enable_ai_suggestions,
            enable_pdf_sync: self.enable_pdf_sync,
            dark_theme: false, // Could be derived from theme context
            line_wrapping: true,
            line_numbers: true,
            language_mode: LanguageMode::LaTeX,
            custom_extensions: Vec::new(),
        }
    }
}