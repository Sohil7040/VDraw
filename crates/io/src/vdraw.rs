use crate::error::IoError;
use core_doc::{validate_document, Document};
use std::path::Path;

/// Save document in native `.vdraw` format (pretty-printed JSON with schema metadata).
pub fn save_vdraw(doc: &Document, path: &Path) -> Result<(), IoError> {
    // Validate document integrity prior to saving
    let validation = validate_document(doc);
    if !validation.is_valid() {
        return Err(IoError::ValidationError(format!(
            "Document contains integrity errors: {:?}",
            validation.errors
        )));
    }

    let json = serde_json::to_string_pretty(doc)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load document from `.vdraw` format with integrity and schema validation.
pub fn load_vdraw(path: &Path) -> Result<Document, IoError> {
    let json = std::fs::read_to_string(path)?;
    let doc: Document = serde_json::from_str(&json)?;

    let validation = validate_document(&doc);
    if !validation.is_valid() {
        return Err(IoError::ValidationError(format!(
            "Loaded document failed validation: {:?}",
            validation.errors
        )));
    }

    Ok(doc)
}
