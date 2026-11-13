//! I/O subsystem for VDraw.
//!
//! Provides production-ready native `.vdraw` JSON format with schema verification,
//! standard SVG import/export with path decomposition and CSS styling,
//! and AutoCAD DXF R12/R14 export for CNC/CAD interoperability.

pub mod dxf_export;
pub mod error;
pub mod svg_export;
pub mod svg_import;
pub mod vdraw;

pub use dxf_export::{export_dxf, save_dxf};
pub use error::IoError;
pub use svg_export::{export_svg, save_svg};
pub use svg_import::{import_svg, load_svg};
pub use vdraw::{load_vdraw, save_vdraw};

use anyhow::Result;
use core_doc::Document;

/// Save a document to a JSON file at `path` (backward-compatible alias).
pub fn save_json(doc: &Document, path: &std::path::Path) -> Result<()> {
    save_vdraw(doc, path).map_err(Into::into)
}

/// Load a document from a JSON file at `path` (backward-compatible alias).
pub fn load_json(path: &std::path::Path) -> Result<Document> {
    load_vdraw(path).map_err(Into::into)
}
