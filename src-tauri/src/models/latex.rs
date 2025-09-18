use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaTeXCompileRequest {
    pub provider: LaTeXProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaTeXCompileResponse {
    pub success: bool,
    pub message: String,
    pub errors: Option<Vec<String>>,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LaTeXProvider {
    #[default]
    Auto,
    Pdflatex,
    Xelatex,
    Lualatex,
}

impl LaTeXProvider {
    pub fn engine_name(&self) -> Option<&'static str> {
        match self {
            LaTeXProvider::Auto => None,
            LaTeXProvider::Pdflatex => Some("pdflatex"),
            LaTeXProvider::Xelatex => Some("xelatex"),
            LaTeXProvider::Lualatex => Some("lualatex"),
        }
    }

    pub fn get_auto_priority_engines() -> Vec<&'static str> {
        vec!["pdflatex", "xelatex", "lualatex"]
    }
}

impl LaTeXCompileResponse {
    pub fn success(message: String, output_file: Option<String>) -> Self {
        Self {
            success: true,
            message,
            errors: None,
            output_file,
        }
    }

    pub fn error(message: String, errors: Vec<String>) -> Self {
        Self {
            success: false,
            message,
            errors: Some(errors),
            output_file: None,
        }
    }
}

// Canonical LaTeX build state (desktop) — mirrors backend for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatexBuildPhase {
    Idle,
    Queued,
    Started,
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatexBuildState {
    pub main_file: Option<String>,
    pub phase: LatexBuildPhase,
    pub pdf_path: Option<String>,
    pub pdf_version: u64,
    pub engine: Option<String>,
    pub errors: Option<Vec<String>>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub session_id: String,
}