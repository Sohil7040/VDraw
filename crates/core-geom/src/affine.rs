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

    #[inline]
    pub fn determinant(&self) -> f64 {
        let [a, b, c, d] = self.matrix;
        a * d - b * c
    }

    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-15 {
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

        Some(Self {
            matrix: [inv_a, inv_b, inv_c, inv_d],
            translation: [inv_tx, inv_ty],
        })
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
