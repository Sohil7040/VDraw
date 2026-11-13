//! Frozen 2D boolean kernel contracts across `core-boolean`, `core-tools`, and `app`.

use crate::boolean::BooleanOp;
use crate::error::BooleanError;
use core_doc::Shape;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// Canonical 2D boolean operation contract mapped onto VDraw's `clipper2-rs` backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoolOp {
    /// Merges shapes into one unified contour.
    Union,
    /// Subtracts trailing shapes from the primary shape.
    Difference,
    /// Keeps only the overlapping area common to all shapes.
    Intersection,
}

impl BoolOp {
    /// All canonical boolean contract variants.
    pub const ALL: [Self; 3] = [Self::Union, Self::Difference, Self::Intersection];
}

impl From<BoolOp> for BooleanOp {
    fn from(op: BoolOp) -> Self {
        match op {
            BoolOp::Union => BooleanOp::Weld,
            BoolOp::Difference => BooleanOp::Trim,
            BoolOp::Intersection => BooleanOp::Intersect,
        }
    }
}

impl TryFrom<BooleanOp> for BoolOp {
    type Error = BooleanOp;

    fn try_from(op: BooleanOp) -> Result<Self, Self::Error> {
        match op {
            BooleanOp::Weld => Ok(BoolOp::Union),
            BooleanOp::Trim => Ok(BoolOp::Difference),
            BooleanOp::Intersect => Ok(BoolOp::Intersection),
            other => Err(other),
        }
    }
}

/// Detailed boolean error contract supporting non-blocking UI toasts and "copy details".
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BoolError {
    /// Input geometry was empty or open where closed regions are required.
    #[error("Empty boolean input geometry: {0}")]
    EmptyInput(String),

    /// Operation produced zero area (disjoint intersection or total subtraction).
    #[error("Boolean operation produced an empty result")]
    EmptyResult,

    /// Coincident edges or degenerate zero-area contours prevented valid evaluation.
    #[error("Degenerate or coincident boolean input: {0}")]
    DegenerateInput(String),

    /// Self-intersecting path could not be normalized cleanly.
    #[error("Self-intersecting contour: {0}")]
    SelfIntersection(String),

    /// Underlying `clipper2-rs` failure.
    #[error("Boolean kernel failure: {0}")]
    KernelError(String),

    /// Coordinate exceeded fixed-point scaling limits (`±100,000 mm`).
    #[error("Coordinate {0} mm exceeds supported fixed-point precision bounds")]
    CoordinateOverflow(f64),

    /// Operation aborted cooperatively via [`BoolCancelToken`].
    #[error("Boolean operation cancelled")]
    Cancelled,
}

impl From<BooleanError> for BoolError {
    fn from(err: BooleanError) -> Self {
        match err {
            BooleanError::InsufficientShapes(msg) => BoolError::EmptyInput(msg),
            BooleanError::InvalidGeometry => BoolError::EmptyInput("empty polycurve".to_string()),
            BooleanError::DegeneratePolygon => {
                BoolError::DegenerateInput("zero-area or collinear polygon".to_string())
            }
            BooleanError::CoordinateOutOfRange(v) => BoolError::CoordinateOverflow(v),
            BooleanError::NonFiniteCoordinate => {
                BoolError::DegenerateInput("non-finite coordinate".to_string())
            }
            BooleanError::ClipperError(msg) | BooleanError::CurveFitError(msg) => {
                BoolError::KernelError(msg)
            }
            BooleanError::ZeroOffset
            | BooleanError::NoKnifeIntersection
            | BooleanError::VirtualSegmentNotFound => BoolError::KernelError(err.to_string()),
        }
    }
}

/// Cooperative cancellation handle for boolean operations.
#[derive(Debug, Clone, Default)]
pub struct BoolCancelToken(Arc<AtomicBool>);

impl BoolCancelToken {
    /// Creates an active, uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation of an in-flight boolean operation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns `true` if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Frozen boolean solver options.
#[derive(Debug, Clone, PartialEq)]
pub struct BoolOptions {
    /// Run Clipper's polygon cleaning / simplification after boolean evaluation.
    pub clean_after: bool,
    /// Simplification and curve-flattening tolerance in millimetres (default `1e-9`).
    pub tolerance: f64,
}

impl Default for BoolOptions {
    fn default() -> Self {
        Self {
            clean_after: true,
            tolerance: 1e-9,
        }
    }
}

/// Evaluates an N-ary 2D boolean operation on document [`Shape`]s with winding normalization,
/// coincident/self-intersection cleanup, cooperative cancellation, and progress reporting.
pub fn boolean_with_options(
    shapes: &[Shape],
    op: BoolOp,
    opts: &BoolOptions,
    cancel: Option<&BoolCancelToken>,
    progress: Option<&dyn Fn(f32)>,
) -> Result<Shape, BoolError> {
    crate::boolean::boolean_with_options_impl(shapes, op, opts, cancel, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_op_bijection_roundtrip() {
        for op in BoolOp::ALL {
            let legacy: BooleanOp = op.into();
            let roundtrip = BoolOp::try_from(legacy);
            assert_eq!(roundtrip, Ok(op));
        }
    }

    #[test]
    fn extended_boolean_ops_report_unmapped_cleanly() {
        assert_eq!(
            BoolOp::try_from(BooleanOp::Exclude),
            Err(BooleanOp::Exclude)
        );
        assert_eq!(
            BoolOp::try_from(BooleanOp::Simplify),
            Err(BooleanOp::Simplify)
        );
    }
}
