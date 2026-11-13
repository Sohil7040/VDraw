use crate::point::Point2;
use crate::rect::Rect;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

/// A 2D affine transformation matrix:
/// ```text
/// [ x' ] = [ a  c  tx ] [ x ]
/// [ y' ] = [ b  d  ty ] [ y ]
/// [ 1  ] = [ 0  0  1  ] [ 1 ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Affine2 {
    /// Column-major 2x2 matrix: [a, b, c, d]
    pub matrix: [f64; 4],
    /// Translation: [tx, ty]
    pub translation: [f64; 2],
}

impl Default for Affine2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine2 {
    pub const IDENTITY: Self = Self {
        matrix: [1.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0],
    };

    pub fn is_identity(&self) -> bool {
        (self.matrix[0] - 1.0).abs() < 1e-12
            && self.matrix[1].abs() < 1e-12
            && self.matrix[2].abs() < 1e-12
            && (self.matrix[3] - 1.0).abs() < 1e-12
            && self.translation[0].abs() < 1e-12
            && self.translation[1].abs() < 1e-12
    }

    pub fn new(a: f64, b: f64, c: f64, d: f64, tx: f64, ty: f64) -> Self {
        Self {
            matrix: [a, b, c, d],
            translation: [tx, ty],
        }
    }

    #[inline]
    pub fn translate(delta: Point2) -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 1.0],
            translation: [delta.x, delta.y],
        }
    }

    #[inline]
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            matrix: [sx, 0.0, 0.0, sy],
            translation: [0.0, 0.0],
        }
    }

    #[inline]
    pub fn scale_around(sx: f64, sy: f64, pivot: Point2) -> Self {
        Self::translate(pivot) * Self::scale(sx, sy) * Self::translate(-pivot)
    }

    #[inline]
    pub fn rotate(radians: f64) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            matrix: [cos, sin, -sin, cos],
            translation: [0.0, 0.0],
        }
    }

    #[inline]
    pub fn rotate_around(radians: f64, pivot: Point2) -> Self {
        Self::translate(pivot) * Self::rotate(radians) * Self::translate(-pivot)
    }

    #[inline]
    pub fn skew(radians_x: f64, radians_y: f64) -> Self {
        Self {
            matrix: [1.0, radians_y.tan(), radians_x.tan(), 1.0],
            translation: [0.0, 0.0],
        }
    }

    #[inline]
    pub fn skew_around(radians_x: f64, radians_y: f64, pivot: Point2) -> Self {
        Self::translate(pivot) * Self::skew(radians_x, radians_y) * Self::translate(-pivot)
    }

    /// Builds and validates a counter-clockwise rotation matrix around `pivot` by `angle_rad`.
    pub fn try_rotate_around(
        pivot: Point2,
        angle_rad: f64,
    ) -> Result<Self, crate::contracts::TransformError> {
        if !pivot.is_finite() || !angle_rad.is_finite() {
            return Err(crate::contracts::TransformError::NonFiniteInput);
        }
        let m = Self::rotate_around(angle_rad, pivot);
        Self::validate_non_singular(m)
    }

    /// Builds and validates a reflection matrix across the 2D line `(line_point, line_dir)`.
    pub fn try_mirror(
        line_point: Point2,
        line_dir: Point2,
    ) -> Result<Self, crate::contracts::TransformError> {
        if !line_point.is_finite() || !line_dir.is_finite() {
            return Err(crate::contracts::TransformError::NonFiniteInput);
        }
        let len = line_dir.length();
        if !len.is_finite() || len <= crate::contracts::SINGULAR_DET_EPSILON {
            return Err(crate::contracts::TransformError::DegenerateLineDirection);
        }
        let ux = line_dir.x / len;
        let uy = line_dir.y / len;
        let reflect_origin = Self::new(
            ux * ux - uy * uy,
            2.0 * ux * uy,
            2.0 * ux * uy,
            uy * uy - ux * ux,
            0.0,
            0.0,
        );
        let m = Self::translate(line_point) * reflect_origin * Self::translate(-line_point);
        Self::validate_non_singular(m)
    }

    /// Builds and validates a non-uniform scaling matrix around `pivot`.
    pub fn try_scale_around(
        pivot: Point2,
        factor: (f64, f64),
    ) -> Result<Self, crate::contracts::TransformError> {
        if !pivot.is_finite() || !factor.0.is_finite() || !factor.1.is_finite() {
            return Err(crate::contracts::TransformError::NonFiniteInput);
        }
        if factor.0 == 0.0 || factor.1 == 0.0 {
            return Err(crate::contracts::TransformError::ZeroScaleFactor);
        }
        let m = Self::scale_around(factor.0, factor.1, pivot);
        Self::validate_non_singular(m)
    }

    #[inline]
    fn validate_non_singular(m: Self) -> Result<Self, crate::contracts::TransformError> {
        if !m.is_finite() {
            return Err(crate::contracts::TransformError::NonFiniteInput);
        }
        let det = m.determinant();
        if !det.is_finite() || det.abs() < crate::contracts::SINGULAR_DET_EPSILON {
            return Err(crate::contracts::TransformError::SingularTransform);
        }
        Ok(m)
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.matrix.iter().all(|x| x.is_finite()) && self.translation.iter().all(|x| x.is_finite())
    }

    #[inline]
    pub fn determinant(&self) -> f64 {
        let [a, b, c, d] = self.matrix;
        a * d - b * c
    }

    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if !det.is_finite() || det.abs() < 1e-15 {
            return None;
        }
        let inv_det = 1.0 / det;
        let [a, b, c, d] = self.matrix;
        let [tx, ty] = self.translation;

        let inv_a = d * inv_det;
        let inv_b = -b * inv_det;
        let inv_c = -c * inv_det;
        let inv_d = a * inv_det;

        let inv_tx = (c * ty - d * tx) * inv_det;
        let inv_ty = (b * tx - a * ty) * inv_det;

        let inv = Self {
            matrix: [inv_a, inv_b, inv_c, inv_d],
            translation: [inv_tx, inv_ty],
        };
        if inv.is_finite() {
            Some(inv)
        } else {
            None
        }
    }

    #[inline]
    pub fn transform_point(&self, p: Point2) -> Point2 {
        let [a, b, c, d] = self.matrix;
        let [tx, ty] = self.translation;
        Point2 {
            x: a * p.x + c * p.y + tx,
            y: b * p.x + d * p.y + ty,
        }
    }

    #[inline]
    pub fn transform_vector(&self, v: Point2) -> Point2 {
        let [a, b, c, d] = self.matrix;
        Point2 {
            x: a * v.x + c * v.y,
            y: b * v.x + d * v.y,
        }
    }

    pub fn transform_rect(&self, r: &Rect) -> Rect {
        let p0 = self.transform_point(Point2::new(r.min_x, r.min_y));
        let p1 = self.transform_point(Point2::new(r.max_x, r.min_y));
        let p2 = self.transform_point(Point2::new(r.max_x, r.max_y));
        let p3 = self.transform_point(Point2::new(r.min_x, r.max_y));

        let mut out = Rect::from_point(p0);
        out.include_point(p1);
        out.include_point(p2);
        out.include_point(p3);
        out
    }
}

impl Mul for Affine2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let [a1, b1, c1, d1] = self.matrix;
        let [tx1, ty1] = self.translation;

        let [a2, b2, c2, d2] = rhs.matrix;
        let [tx2, ty2] = rhs.translation;

        Self {
            matrix: [
                a1 * a2 + c1 * b2,
                b1 * a2 + d1 * b2,
                a1 * c2 + c1 * d2,
                b1 * c2 + d1 * d2,
            ],
            translation: [a1 * tx2 + c1 * ty2 + tx1, b1 * tx2 + d1 * ty2 + ty1],
        }
    }
}

// ─── Kurbo interop ───────────────────────────────────────────────────────────

impl From<kurbo::Affine> for Affine2 {
    fn from(a: kurbo::Affine) -> Self {
        let coeffs = a.as_coeffs();
        Self {
            matrix: [coeffs[0], coeffs[1], coeffs[2], coeffs[3]],
            translation: [coeffs[4], coeffs[5]],
        }
    }
}

impl From<Affine2> for kurbo::Affine {
    fn from(a: Affine2) -> Self {
        kurbo::Affine::new([
            a.matrix[0],
            a.matrix[1],
            a.matrix[2],
            a.matrix[3],
            a.translation[0],
            a.translation[1],
        ])
    }
}
