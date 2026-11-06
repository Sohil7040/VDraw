//! Core geometry primitives, Bézier algorithms, and shape normalization for VDraw.

pub mod affine;
pub mod bezier;
pub mod node;
pub mod point;
pub mod polycurve;
pub mod rect;

pub use affine::Affine2;
pub use bezier::CubicBezier;
pub use node::{BezierNode, NodeType};
pub use point::Point2;
pub use polycurve::{CornerStyle, Polycurve, Subpath, KAPPA};
pub use rect::Rect;
