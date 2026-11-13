//! Document model, shapes, styling, selection, and undo/redo architecture for VDraw.

pub mod color;
pub mod command;
pub mod contracts;
pub mod document;
pub mod guideline;
pub mod selection;
pub mod shape;
pub mod style;
pub mod units;
pub mod validation;

pub use color::Color;
pub use command::{
    AddShapeCommand, BreakApartCommand, CombinePathsCommand, Command, CompoundCommand,
    GroupCommand, ModifyShapeKindCommand, RemoveShapesCommand, SetStyleCommand,
    TransformShapesCommand, UndoStack, UngroupCommand,
};
pub use contracts::{EntityId, HistoryEntry, ShapeTransformEntry};
pub use document::{Document, DocumentMetadata, Layer, Page, PagePreset};
pub use guideline::{Guideline, GuidelineId, GuidelineOrientation};
pub use selection::{HandleId, HandleInfo, Selection, TransformMode};

pub use shape::{Shape, ShapeId, ShapeKind};
pub use style::{LineCap, LineJoin, Stroke, Style};
pub use units::Unit;
pub use validation::{validate_document, ValidationError, ValidationReport};
