//! Error types for boolean, knife, eraser, and offset operations.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BooleanError {
    #[error("At least two shapes are required for boolean operation '{0}'")]
    InsufficientShapes(String),

    #[error("Clipper internal error: {0}")]
    ClipperError(String),

    #[error("Empty input shape or invalid geometry")]
    InvalidGeometry,

    #[error("Failed to fit curves to result: {0}")]
    CurveFitError(String),

    #[error("Offset distance is zero")]
    ZeroOffset,

    #[error("Non-finite coordinate encountered (NaN or Infinity)")]
    NonFiniteCoordinate,

    #[error("Coordinate {0} mm exceeds maximum supported range (±100,000 mm)")]
    CoordinateOutOfRange(f64),

    #[error("Degenerate polygon with zero area or collinear points")]
    DegeneratePolygon,

    #[error("Knife cut line did not intersect any shape")]
    NoKnifeIntersection,

    #[error("Virtual segment not found")]
    VirtualSegmentNotFound,
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum TextError {
    #[error("Failed to parse font data: {0}")]
    FontParseError(String),

    #[error("Glyph not found for character: '{0}'")]
    GlyphNotFound(char),

    #[error("Empty text string provided")]
    EmptyText,
}
