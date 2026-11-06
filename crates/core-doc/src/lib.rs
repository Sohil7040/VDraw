//! Document model, shapes, styling, selection, and undo/redo architecture for VDraw.

pub mod color;
pub mod command;
pub mod document;
pub mod selection;
pub mod shape;
pub mod style;
pub mod units;

pub use color::Color;
pub use command::{
    AddShapeCommand, Command, CompoundCommand, RemoveShapesCommand, TransformShapesCommand,
    UndoStack,
};
pub use document::{Document, Layer, Page};
pub use selection::{HandleId, HandleInfo, Selection, TransformMode};
pub use shape::{Shape, ShapeId, ShapeKind};
pub use style::{LineCap, LineJoin, Stroke, Style};
pub use units::Unit;
