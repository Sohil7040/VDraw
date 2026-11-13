//! Frozen document identity, transform history, and undo timeline contracts.

use crate::ShapeId;
use core_geom::Affine2;
use serde::{Deserialize, Serialize};

/// Canonical entity identifier alias shared across snapping, tools, and UI.
pub type EntityId = ShapeId;

/// Snapshot of an undoable or redoable command entry shown in UI history dropdowns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// 0-based position in the combined undo/redo timeline.
    pub index: usize,
    /// Human-readable command label.
    pub label: String,
    /// `true` if this entry is currently applied on the undo stack.
    pub is_applied: bool,
}

/// Before/after transform state for a single shape, including winding flip tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeTransformEntry {
    /// Target shape identifier.
    pub id: EntityId,
    /// Transform prior to the operation.
    pub before: Affine2,
    /// Transform after the operation.
    pub after: Affine2,
    /// `true` when reflection inverted orientation so undo reverses winding too.
    pub flips_winding: bool,
}
