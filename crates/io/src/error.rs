use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("File I/O error: {0}")]
    FileIo(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Document validation failed: {0}")]
    ValidationError(String),
    #[error("SVG parsing error: {0}")]
    SvgParse(String),
    #[error("DXF export error: {0}")]
    DxfExport(String),
    #[error("Unsupported format or version: {0}")]
    Unsupported(String),
}
