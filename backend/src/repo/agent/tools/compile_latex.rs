use async_trait::async_trait;
use std::path::Path;
use serde_json::{Value, json};
use crate::model::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use crate::model::{LaTeXCompileRequest, LaTeXProvider};
use crate::repo::agent::{AgentTool, AgentRepo};

/// Tool for compiling LaTeX documents with detailed error reporting
pub struct CompileLatexTool;

#[async_trait]
impl AgentTool for CompileLatexTool {
    fn name(&self) -> &str {
        "compile"
    }
    
    fn display_name(&self) -> &str {
        "Compile LaTeX"
    }
    
    fn progressive_form(&self) -> &str {
        "Compiling LaTeX"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "compile".to_string(),
                description: "Compile LaTeX documents in the workspace. This tool should be run at the end of any LaTeX editing job to ensure the document compiles correctly and to identify any errors that need to be fixed.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "engine": {
                            "type": "string",
                            "enum": ["auto", "pdflatex", "xelatex", "lualatex"],
                            "description": "LaTeX engine to use. 'auto' will try available engines in order.",
                            "default": "auto"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force compilation even if another compilation is in progress.",
                            "default": false
                        }
                    },
                    "additionalProperties": false
                }),
                display_name: Some(self.display_name().to_string()),
                progressive_form: Some(self.progressive_form().to_string()),
            },
        }
    }

    async fn execute(&self, _workspace: &Path, args: Value, repo: Option<&AgentRepo>) -> AgentResult<String> {
        let Some(repo) = repo else {
            return Err(AgentError::Generic(anyhow::anyhow!("AgentRepo is required for LaTeX compilation")));
        };

        // Parse arguments
        let engine = args.get("engine")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");
        
        let _force = args.get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Convert engine string to LaTeXProvider
        let provider = match engine {
            "auto" => LaTeXProvider::Auto,
            "pdflatex" => LaTeXProvider::Pdflatex,
            "xelatex" => LaTeXProvider::Xelatex,
            "lualatex" => LaTeXProvider::Lualatex,
            _ => return Err(AgentError::InvalidToolArguments {
                tool: "compile".to_string(),
                error: format!("Unsupported engine: {}", engine),
            }),
        };

        // Get the LaTeX service directly from the repo
        let latex_service = repo.get_latex_service();

        // Create compilation request
        let request = LaTeXCompileRequest {
            provider,
        };

        // Execute compilation
        match latex_service.compile_workspace(request).await {
            Ok(response) => {
                if response.success {
                    // Successful compilation
                    let pdf_path = response.output_file.unwrap_or_else(|| "document.pdf".to_string());
                    Ok(json!({
                        "success": true,
                        "message": response.message,
                        "pdf_path": pdf_path,
                        "engine": engine
                    }).to_string())
                } else {
                    // Compilation failed - return detailed error information
                    let error_details = response.errors.unwrap_or_else(|| vec!["Unknown compilation error occurred".to_string()]);

                    Ok(json!({
                        "success": false,
                        "message": response.message,
                        "errors": error_details,
                        "engine": engine,
                        "error_count": error_details.len()
                    }).to_string())
                }
            }
            Err(e) => {
                // Service-level error
                Ok(json!({
                    "success": false,
                    "message": "LaTeX compilation service error",
                    "errors": [e.to_string()],
                    "engine": engine,
                    "error_count": 1
                }).to_string())
            }
        }
    }
}