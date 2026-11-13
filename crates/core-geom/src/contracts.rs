//! Frozen 2D geometric transformation contracts for VDraw.

use crate::affine::Affine2;
use crate::point::Point2;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Tolerance below which matrix determinants are treated as singular and non-invertible.
pub const SINGULAR_DET_EPSILON: f64 = 1e-12;

/// Errors returned by 2D geometric transform constructors and applications.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TransformError {
    /// Mirror line direction vector has zero or non-finite length.
    #[error("Mirror line direction cannot be zero or non-finite")]
    DegenerateLineDirection,

    /// Scale factor contains zero or non-finite components.
    #[error("Scale factor must be non-zero and finite")]
    ZeroScaleFactor,

    /// Input coordinates or angle are NaN or infinite.
    #[error("Transform inputs must be finite floating-point numbers")]
    NonFiniteInput,

    /// Resulting matrix is singular (`|det| < 1e-12`) and cannot be inverted for undo.
    #[error("Resulting transformation matrix is singular and cannot be inverted for undo")]
    SingularTransform,
}

/// Reversible audit record produced by applying 2D geometric transforms for undo/redo.
///
/// Invariants:
/// - `matrix * inverse` approximates identity within floating-point tolerance.
/// - `flips_winding == (matrix.determinant() < 0.0)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformRecord {
    /// Batched 2D affine matrix applied in document space.
    pub matrix: Affine2,
    /// Exact inverse transform used for undo restoration.
    pub inverse: Affine2,
    /// Whether the transform inverted orientation (`det < 0`), requiring contour reversal.
    pub flips_winding: bool,
    /// Human-readable operation description for undo history menus.
    pub description: String,
}

/// Pure, allocation-free, side-effect-free 2D transform matrix constructor contract.
pub trait TransformBuild {
    /// Builds a pure translation matrix.
    fn translate(&self, delta: Point2) -> Affine2;

    /// Builds a rotation matrix counter-clockwise around `pivot` by `angle_rad`.
    fn rotate(&self, pivot: Point2, angle_rad: f64) -> Result<Affine2, TransformError>;

    /// Builds a reflection matrix across `(line_point, line_dir)` plus `flips_winding` (`true`).
    fn mirror(
        &self,
        line_point: Point2,
        line_dir: Point2,
    ) -> Result<(Affine2, bool), TransformError>;

    /// Builds a non-uniform scaling matrix around `pivot`.
    fn scale(&self, pivot: Point2, factor: (f64, f64)) -> Result<Affine2, TransformError>;
}

/// Entity-level transform application contract returning an undoable [`TransformRecord`].
pub trait TransformApply: TransformBuild {
    /// Applies a pre-computed affine matrix `m` to `self` and returns a [`TransformRecord`].
    fn apply(&mut self, m: &Affine2) -> TransformRecord;
}

fn finalize_record(
    matrix: Affine2,
    flips_winding: bool,
    description: impl Into<String>,
) -> Result<TransformRecord, TransformError> {
    if !matrix.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    let det = matrix.determinant();
    if !det.is_finite() || det.abs() < SINGULAR_DET_EPSILON {
        return Err(TransformError::SingularTransform);
    }
    let inverse = matrix.inverse().ok_or(TransformError::SingularTransform)?;
    Ok(TransformRecord {
        matrix,
        inverse,
        flips_winding,
        description: description.into(),
    })
}

/// Builds a batched translation [`TransformRecord`] rejecting non-finite or singular matrices.
pub fn build_translate_record(delta: Point2) -> Result<TransformRecord, TransformError> {
    if !delta.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    let m = Affine2::translate(delta);
    finalize_record(
        m,
        false,
        format!("Translate ({:.2}, {:.2})", delta.x, delta.y),
    )
}

/// Builds a batched rotation [`TransformRecord`] around `pivot` by `angle_rad`.
pub fn build_rotate_record(
    pivot: Point2,
    angle_rad: f64,
) -> Result<TransformRecord, TransformError> {
    if !pivot.is_finite() || !angle_rad.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    let m = Affine2::rotate_around(angle_rad, pivot);
    finalize_record(m, false, format!("Rotate {:.1}°", angle_rad.to_degrees()))
}

/// Builds a batched reflection [`TransformRecord`] across the 2D line `(line_point, line_dir)`.
///
/// Unconditionally sets `flips_winding = true`.
pub fn build_mirror_record(
    line_point: Point2,
    line_dir: Point2,
) -> Result<TransformRecord, TransformError> {
    if !line_point.is_finite() || !line_dir.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    let len = line_dir.length();
    if !len.is_finite() || len <= SINGULAR_DET_EPSILON {
        return Err(TransformError::DegenerateLineDirection);
    }
    let ux = line_dir.x / len;
    let uy = line_dir.y / len;
    let reflect_origin = Affine2::new(
        ux * ux - uy * uy,
        2.0 * ux * uy,
        2.0 * ux * uy,
        uy * uy - ux * ux,
        0.0,
        0.0,
    );
    let m = Affine2::translate(line_point) * reflect_origin * Affine2::translate(-line_point);
    finalize_record(m, true, "Mirror")
}

/// Builds a batched non-uniform scaling [`TransformRecord`] around `pivot`.
pub fn build_scale_record(
    pivot: Point2,
    factor: (f64, f64),
) -> Result<TransformRecord, TransformError> {
    if !pivot.is_finite() || !factor.0.is_finite() || !factor.1.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    if factor.0 == 0.0 || factor.1 == 0.0 {
        return Err(TransformError::ZeroScaleFactor);
    }
    let m = Affine2::scale_around(factor.0, factor.1, pivot);
    if m.determinant().abs() < SINGULAR_DET_EPSILON {
        return Err(TransformError::SingularTransform);
    }
    let flips_winding = m.determinant() < 0.0;
    finalize_record(
        m,
        flips_winding,
        format!("Scale ({:.2}, {:.2})", factor.0, factor.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_record_inverse_is_identity(rec: &TransformRecord) {
        let product = rec.matrix * rec.inverse;
        assert_relative_eq!(product.matrix[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product.matrix[1], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product.matrix[2], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product.matrix[3], 1.0, epsilon = 1e-10);
        assert_relative_eq!(product.translation[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(product.translation[1], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn inverse_identity_for_all_builders() {
        let t = build_translate_record(Point2::new(12.5, -7.25)).unwrap();
        assert!(!t.flips_winding);
        assert_record_inverse_is_identity(&t);

        let r = build_rotate_record(Point2::new(10.0, 20.0), std::f64::consts::FRAC_PI_3).unwrap();
        assert!(!r.flips_winding);
        assert_record_inverse_is_identity(&r);

        let m = build_mirror_record(Point2::new(5.0, 8.0), Point2::new(3.0, 4.0)).unwrap();
        assert!(m.flips_winding);
        assert_record_inverse_is_identity(&m);

        let s = build_scale_record(Point2::new(-3.0, 4.0), (2.5, -1.5)).unwrap();
        assert!(s.flips_winding);
        assert_record_inverse_is_identity(&s);

        // Singularity rejection check
        assert_eq!(
            build_scale_record(Point2::ZERO, (1e-13, 1.0)),
            Err(TransformError::SingularTransform)
        );
        assert_eq!(
            build_mirror_record(Point2::ZERO, Point2::ZERO),
            Err(TransformError::DegenerateLineDirection)
        );
    }
}
