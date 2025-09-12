use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationEvent {
    pub id: String,
    pub event_type: CompilationEventType,
    pub main_file: String,
    pub timestamp: u64,
    pub metadata: Option<CompilationEventMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompilationEventType {
    Queued,
    Started,
    Success,
    Error,
    #[serde(rename = "main_file_detected")]
    MainFileDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationEventMetadata {
    pub reason: Option<String>,
    pub pdf_path: Option<String>,
    pub errors: Option<Vec<String>>,
    pub engine: Option<String>,
    pub duration_ms: Option<u64>,
}

impl CompilationEvent {
    pub fn new(event_type: CompilationEventType, main_file: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            main_file,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: CompilationEventMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn queued(main_file: String, reason: String) -> Self {
        Self::new(CompilationEventType::Queued, main_file)
            .with_metadata(CompilationEventMetadata::new().with_reason(reason))
    }

    pub fn started(main_file: String, engine: String) -> Self {
        Self::new(CompilationEventType::Started, main_file)
            .with_metadata(CompilationEventMetadata::new().with_engine(engine))
    }

    pub fn success(main_file: String, pdf_path: String, engine: String, duration_ms: u64) -> Self {
        Self::new(CompilationEventType::Success, main_file)
            .with_metadata(CompilationEventMetadata::new()
                .with_pdf_path(pdf_path)
                .with_engine(engine)
                .with_duration(duration_ms))
    }

    pub fn error(main_file: String, errors: Vec<String>, engine: Option<String>) -> Self {
        let mut metadata = CompilationEventMetadata::new().with_errors(errors);
        if let Some(engine) = engine {
            metadata = metadata.with_engine(engine);
        }
        Self::new(CompilationEventType::Error, main_file).with_metadata(metadata)
    }

    pub fn main_file_detected(main_file: String) -> Self {
        Self::new(CompilationEventType::MainFileDetected, main_file)
    }
}

impl CompilationEventMetadata {
    pub fn new() -> Self {
        Self {
            reason: None,
            pdf_path: None,
            errors: None,
            engine: None,
            duration_ms: None,
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn with_pdf_path(mut self, pdf_path: String) -> Self {
        self.pdf_path = Some(pdf_path);
        self
    }

    pub fn with_errors(mut self, errors: Vec<String>) -> Self {
        self.errors = Some(errors);
        self
    }

    pub fn with_engine(mut self, engine: String) -> Self {
        self.engine = Some(engine);
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}