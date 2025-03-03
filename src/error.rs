use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("Missing fragment name")]
    MissingName,
}

pub type Result<T> = std::result::Result<T, CodegenError>;
