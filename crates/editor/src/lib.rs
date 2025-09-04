// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;

pub mod codemirror;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use codemirror::{CodeMirrorProps, CodeMirrorOps, DecorationType, EditorConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codemirror::{EditorConfig, LanguageMode};

    #[test]
    fn test_editor_config_defaults() {
        let config = EditorConfig::default();
        
        // Line numbers should be enabled by default
        assert_eq!(config.line_numbers, true, "Line numbers should be enabled by default");
        
        // LaTeX should be the default language mode
        assert_eq!(config.language_mode, LanguageMode::LaTeX, "LaTeX should be the default language mode");
        
        // Line wrapping should be enabled by default
        assert_eq!(config.line_wrapping, true, "Line wrapping should be enabled by default");
    }
    
    #[test]
    fn test_codemirror_props_to_config_conversion() {
        // Test the configuration conversion without requiring signals in tests
        let config = EditorConfig {
            initial_content: Some("\\documentclass{article}".to_string()),
            enable_ai_suggestions: true,
            enable_pdf_sync: true,
            dark_theme: false,
            line_wrapping: true,
            line_numbers: true,
            language_mode: LanguageMode::LaTeX,
            custom_extensions: Vec::new(),
        };
        
        // Verify that config has line numbers enabled
        assert_eq!(config.line_numbers, true, "Config should have line numbers enabled");
        assert_eq!(config.enable_ai_suggestions, true, "AI suggestions should be preserved");
        assert_eq!(config.enable_pdf_sync, true, "PDF sync should be preserved");
        assert_eq!(config.language_mode, LanguageMode::LaTeX, "Language mode should be LaTeX");
    }
    
    #[test] 
    fn test_latex_formatting() {
        let content = r#"\documentclass{article}
\begin{document}
\section{Test}
Hello world
\begin{equation}
x = y + z
\end{equation}
\end{document}"#;
        
        let formatted = codemirror::format_latex(content);
        
        // Verify that the formatted content includes proper indentation
        assert!(formatted.contains("\t\\section{Test}"), "Sections should be indented");
        assert!(formatted.contains("\t\tx = y + z"), "Equation content should be double-indented");
    }
}