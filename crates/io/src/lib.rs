//! I/O layer for VDraw — Phase 0 stub.

use anyhow::Result;
use core_doc::Document;

/// Save a document to a JSON file at `path`.
pub fn save_json(doc: &Document, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(doc)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a document from a JSON file at `path`.
pub fn load_json(path: &std::path::Path) -> Result<Document> {
    let json = std::fs::read_to_string(path)?;
    let doc = serde_json::from_str(&json)?;
    Ok(doc)
}
