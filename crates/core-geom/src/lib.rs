//! Core geometry primitives, Bézier algorithms, and shape normalization for VDraw.

pub mod affine;
pub mod bezier;
pub mod contracts;
pub mod curve_fit;
pub mod node;
pub mod point;
pub mod polycurve;
pub mod rect;

pub use affine::Affine2;
pub use bezier::CubicBezier;
pub use contracts::{
    build_mirror_record, build_rotate_record, build_scale_record, build_translate_record,
    TransformApply, TransformBuild, TransformError, TransformRecord, SINGULAR_DET_EPSILON,
};
pub use curve_fit::fit_curve;
pub use node::{BezierNode, NodeType};
pub use point::Point2;
pub use polycurve::{CornerStyle, Polycurve, Subpath, KAPPA};
pub use rect::Rect;
