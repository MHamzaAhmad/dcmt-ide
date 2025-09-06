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