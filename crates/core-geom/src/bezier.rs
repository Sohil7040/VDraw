use crate::point::Point2;
use crate::rect::Rect;
use serde::{Deserialize, Serialize};

/// A single cubic Bézier curve segment defined by four control points:
/// `p0` (start), `p1` (control 1), `p2` (control 2), `p3` (end).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CubicBezier {
    pub p0: Point2,
    pub p1: Point2,
    pub p2: Point2,
    pub p3: Point2,
}

impl CubicBezier {
    #[inline]
    pub const fn new(p0: Point2, p1: Point2, p2: Point2, p3: Point2) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Construct a straight line segment represented as a cubic Bézier.
    #[inline]
    pub fn line(p0: Point2, p3: Point2) -> Self {
        let p1 = p0.lerp(p3, 1.0 / 3.0);
        let p2 = p0.lerp(p3, 2.0 / 3.0);
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate point on curve at parameter `t` in [0, 1].
    #[inline]
    pub fn evaluate(&self, t: f64) -> Point2 {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;

        self.p0 * mt3 + self.p1 * (3.0 * mt2 * t) + self.p2 * (3.0 * mt * t2) + self.p3 * t3
    }

    /// First derivative B'(t) vector.
    #[inline]
    pub fn derivative(&self, t: f64) -> Point2 {
        let mt = 1.0 - t;
        let c0 = (self.p1 - self.p0) * (3.0 * mt * mt);
        let c1 = (self.p2 - self.p1) * (6.0 * mt * t);
        let c2 = (self.p3 - self.p2) * (3.0 * t * t);
        c0 + c1 + c2
    }

    /// Second derivative B''(t) vector.
    #[inline]
    pub fn second_derivative(&self, t: f64) -> Point2 {
        let mt = 1.0 - t;
        let d0 = (self.p2 - 2.0 * self.p1 + self.p0) * (6.0 * mt);
        let d1 = (self.p3 - 2.0 * self.p2 + self.p1) * (6.0 * t);
        d0 + d1
    }

    /// Normalized unit tangent vector at parameter `t`.
    pub fn tangent(&self, t: f64) -> Point2 {
        let d = self.derivative(t);
        let len = d.length();
        if len > 1e-12 {
            d / len
        } else {
            // Degenerate derivative, try endpoints
            let fallback = self.p3 - self.p0;
            if fallback.length() > 1e-12 {
                fallback.normalize()
            } else {
                Point2::new(1.0, 0.0)
            }
        }
    }

    /// Normalized unit normal vector at parameter `t` (tangent rotated 90° CCW).
    #[inline]
    pub fn normal(&self, t: f64) -> Point2 {
        self.tangent(t).perpendicular()
    }

    /// Split the cubic Bézier curve at parameter `t` using de Casteljau's algorithm.
    pub fn split(&self, t: f64) -> (Self, Self) {
        let q0 = self.p0;
        let q1 = self.p0.lerp(self.p1, t);
        let q2 = self.p1.lerp(self.p2, t);
        let q3 = self.p2.lerp(self.p3, t);

        let r0 = q1.lerp(q2, t);
        let r1 = q2.lerp(q3, t);

        let s0 = r0.lerp(r1, t);

        (Self::new(q0, q1, r0, s0), Self::new(s0, r1, q3, self.p3))
    }

    /// Split at the midpoint `t = 0.5`.
    #[inline]
    pub fn split_half(&self) -> (Self, Self) {
        self.split(0.5)
    }

    /// Subdivide the curve to subsegment `[t0, t1]`.
    pub fn subsegment(&self, t0: f64, t1: f64) -> Self {
        if (t0 - 0.0).abs() < 1e-12 && (t1 - 1.0).abs() < 1e-12 {
            return *self;
        }
        let (_, right) = self.split(t0);
        let new_t1 = (t1 - t0) / (1.0 - t0);
        let (sub, _) = right.split(new_t1);
        sub
    }

    /// Compute arc length on sub-interval `[t0, t1]` using 5-point Gauss-Legendre quadrature.
    pub fn arc_length(&self, t0: f64, t1: f64) -> f64 {
        // 5-point Gauss-Legendre abscissae and weights on [-1, 1]
        const X: [f64; 5] = [
            0.0,
            -0.538_469_310_105_683_1,
            0.538_469_310_105_683_1,
            -0.906_179_845_938_664,
            0.906_179_845_938_664,
        ];
        const W: [f64; 5] = [
            0.568_888_888_888_888_9,
            0.478_628_670_499_366_5,
            0.478_628_670_499_366_5,
            0.236_926_885_056_189_1,
            0.236_926_885_056_189_1,
        ];

        let half_dt = (t1 - t0) * 0.5;
        let mid_t = (t0 + t1) * 0.5;

        let mut sum = 0.0;
        for i in 0..5 {
            let t = mid_t + half_dt * X[i];
            let speed = self.derivative(t).length();
            sum += W[i] * speed;
        }

        sum * half_dt
    }

    /// Total arc length of the curve.
    #[inline]
    pub fn length(&self) -> f64 {
        self.arc_length(0.0, 1.0)
    }

    /// Find the closest point on the curve to query point `p`.
    ///
    /// Algorithm:
    /// 1. Coarse search over 16 bisection intervals (depth 4)
    /// 2. Newton-Raphson refinement on `(B(t) - P) · B'(t) = 0`
    /// 3. Clamping and boundary checks
    ///
    /// Returns `(t, closest_point, distance)`.
    pub fn closest_point(&self, p: Point2) -> (f64, Point2, f64) {
        // 1. Initial coarse sampling
        const SAMPLES: usize = 16;
        let mut best_t = 0.0;
        let mut best_dist_sq = self.p0.distance_squared_to(p);

        for i in 1..=SAMPLES {
            let t = i as f64 / SAMPLES as f64;
            let pt = self.evaluate(t);
            let d_sq = pt.distance_squared_to(p);
            if d_sq < best_dist_sq {
                best_dist_sq = d_sq;
                best_t = t;
            }
        }

        // 2. Newton-Raphson refinement
        // f(t) = (B(t) - P) · B'(t)
        // f'(t) = |B'(t)|^2 + (B(t) - P) · B''(t)
        let mut t = best_t;
        for _ in 0..10 {
            let bt = self.evaluate(t);
            let diff = bt - p;
            let d1 = self.derivative(t);
            let d2 = self.second_derivative(t);

            let f = diff.dot(d1);
            let f_prime = d1.length_squared() + diff.dot(d2);

            if f_prime.abs() < 1e-14 {
                break;
            }

            let delta = f / f_prime;
            t = (t - delta).clamp(0.0, 1.0);

            if delta.abs() < 1e-10 {
                break;
            }
        }

        // Compare refined t with candidates
        let pt = self.evaluate(t);
        let dist = pt.distance_to(p);

        let p0_dist = self.p0.distance_to(p);
        let p3_dist = self.p3.distance_to(p);

        if p0_dist < dist && p0_dist <= p3_dist {
            (0.0, self.p0, p0_dist)
        } else if p3_dist < dist {
            (1.0, self.p3, p3_dist)
        } else {
            (t, pt, dist)
        }
    }

    /// Flatten the curve into line segments within `tolerance` (default 0.02 mm).
    pub fn flatten(&self, tolerance: f64) -> Vec<Point2> {
        let mut points = Vec::new();
        points.push(self.p0);
        self.flatten_recursive(tolerance, &mut points);
        points
    }

    fn flatten_recursive(&self, tolerance: f64, out: &mut Vec<Point2>) {
        // Flatness check: maximum deviation of p1 and p2 from line (p0 -> p3)
        let chord = self.p3 - self.p0;
        let chord_len = chord.length();

        let flat = if chord_len < 1e-9 {
            (self.p1 - self.p0)
                .length()
                .max((self.p2 - self.p0).length())
                <= tolerance
        } else {
            // Distance from point to line: |cross(chord, p - p0)| / |chord|
            let d1 = chord.cross(self.p1 - self.p0).abs() / chord_len;
            let d2 = chord.cross(self.p2 - self.p0).abs() / chord_len;
            d1 <= tolerance && d2 <= tolerance
        };

        if flat {
            out.push(self.p3);
        } else {
            let (left, right) = self.split_half();
            left.flatten_recursive(tolerance, out);
            right.flatten_recursive(tolerance, out);
        }
    }

    /// Compute exact tight bounding box by checking endpoints and derivative extrema.
    pub fn bounds(&self) -> Rect {
        let mut r = Rect::from_point(self.p0);
        r.include_point(self.p3);

        // Extrema occur where B'_x(t) = 0 and B'_y(t) = 0
        // B'(t) = a * t^2 + b * t + c
        let solve_quadratic = |p0: f64, p1: f64, p2: f64, p3: f64| -> Vec<f64> {
            let a = 3.0 * (-p0 + 3.0 * p1 - 3.0 * p2 + p3);
            let b = 6.0 * (p0 - 2.0 * p1 + p2);
            let c = 3.0 * (-p0 + p1);

            let mut roots = Vec::with_capacity(2);
            if a.abs() < 1e-12 {
                if b.abs() > 1e-12 {
                    let t = -c / b;
                    if t > 0.0 && t < 1.0 {
                        roots.push(t);
                    }
                }
            } else {
                let discr = b * b - 4.0 * a * c;
                if discr >= 0.0 {
                    let sqrt_d = discr.sqrt();
                    let t1 = (-b - sqrt_d) / (2.0 * a);
                    let t2 = (-b + sqrt_d) / (2.0 * a);
                    if t1 > 0.0 && t1 < 1.0 {
                        roots.push(t1);
                    }
                    if t2 > 0.0 && t2 < 1.0 {
                        roots.push(t2);
                    }
                }
            }
            roots
        };

        for tx in solve_quadratic(self.p0.x, self.p1.x, self.p2.x, self.p3.x) {
            r.include_point(self.evaluate(tx));
        }

        for ty in solve_quadratic(self.p0.y, self.p1.y, self.p2.y, self.p3.y) {
            r.include_point(self.evaluate(ty));
        }

        r
    }
}
