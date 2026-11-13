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

    pub const TOLERANCE_DISPLAY: f64 = 0.05;
    pub const TOLERANCE_HIT_TEST: f64 = 0.5;
    pub const TOLERANCE_BOOLEAN: f64 = 0.02;
    pub const TOLERANCE_CAM: f64 = 0.01;

    /// Find the closest point on the curve to query point `p`.
    ///
    /// Algorithm:
    /// 1. Construct degree-5 1D Bernstein polynomial representing (B(t) - P) · B'(t) = 0.
    /// 2. Isolate all real roots in [0, 1] via variation-diminishing Bézier subdivision.
    /// 3. Refine roots using bracketed safeguarded root-finding.
    /// 4. Evaluate distance at all stationary points and endpoints (t=0, t=1) to guarantee
    ///    finding the global minimum distance without being trapped in local minima.
    ///
    /// Returns `(t, closest_point, distance)`.
    pub fn closest_point(&self, p: Point2) -> (f64, Point2, f64) {
        let mut candidates = Vec::with_capacity(7);
        candidates.push(0.0);
        candidates.push(1.0);

        // Compute degree-5 Bernstein coefficients of (B(t) - P) · B'(t)
        let q0 = self.p0 - p;
        let q1 = self.p1 - p;
        let q2 = self.p2 - p;
        let q3 = self.p3 - p;

        let v0 = (self.p1 - self.p0) * 3.0;
        let v1 = (self.p2 - self.p1) * 3.0;
        let v2 = (self.p3 - self.p2) * 3.0;

        let c0 = q0.dot(v0);
        let c1 = 0.6 * q1.dot(v0) + 0.4 * q0.dot(v1);
        let c2 = 0.3 * q2.dot(v0) + 0.6 * q1.dot(v1) + 0.1 * q0.dot(v2);
        let c3 = 0.1 * q3.dot(v0) + 0.6 * q2.dot(v1) + 0.3 * q1.dot(v2);
        let c4 = 0.4 * q3.dot(v1) + 0.6 * q2.dot(v2);
        let c5 = q3.dot(v2);

        find_bezier_roots_5(
            self,
            p,
            &[c0, c1, c2, c3, c4, c5],
            0.0,
            1.0,
            0,
            &mut candidates,
        );

        let mut best_t = 0.0;
        let mut best_pt = self.p0;
        let mut best_dist = self.p0.distance_to(p);

        for t in candidates {
            let t_clamped = t.clamp(0.0, 1.0);
            let pt = self.evaluate(t_clamped);
            let dist = pt.distance_to(p);
            if dist < best_dist {
                best_dist = dist;
                best_t = t_clamped;
                best_pt = pt;
            }
        }

        (best_t, best_pt, best_dist)
    }

    /// Flatten the curve into line segments within `tolerance` (default 0.02 mm).
    pub fn flatten(&self, tolerance: f64) -> Vec<Point2> {
        let tolerance = tolerance.max(1e-6);
        let mut points = Vec::new();
        points.push(self.p0);
        self.flatten_recursive(tolerance, 0, &mut points);
        points
    }

    fn flatten_recursive(&self, tolerance: f64, depth: usize, out: &mut Vec<Point2>) {
        const MAX_DEPTH: usize = 16;
        let chord = self.p3 - self.p0;
        let chord_len = chord.length();

        let flat = if depth >= MAX_DEPTH {
            true
        } else if chord_len < 1e-9 {
            (self.p1 - self.p0)
                .length()
                .max((self.p2 - self.p0).length())
                <= tolerance
        } else {
            // 1. Perpendicular distance to chord line
            let d1 = chord.cross(self.p1 - self.p0).abs() / chord_len;
            let d2 = chord.cross(self.p2 - self.p0).abs() / chord_len;

            // 2. Longitudinal check: control points must not overshoot segment bounds
            let u = chord / chord_len;
            let t1 = (self.p1 - self.p0).dot(u);
            let t2 = (self.p2 - self.p0).dot(u);

            let within_segment = t1 >= -tolerance
                && t1 <= chord_len + tolerance
                && t2 >= -tolerance
                && t2 <= chord_len + tolerance;

            d1 <= tolerance && d2 <= tolerance && within_segment
        };

        if flat {
            out.push(self.p3);
        } else {
            let (left, right) = self.split_half();
            left.flatten_recursive(tolerance, depth + 1, out);
            right.flatten_recursive(tolerance, depth + 1, out);
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

// ─── Degree-5 Bézier Clipping Nearest-Point Root Isolation ───────────────────

fn find_bezier_roots_5(
    curve: &CubicBezier,
    p: Point2,
    c: &[f64; 6],
    t_min: f64,
    t_max: f64,
    depth: usize,
    roots: &mut Vec<f64>,
) {
    let sign_changes = count_sign_changes(c);
    if sign_changes == 0 {
        return;
    }

    if sign_changes == 1 || depth >= 12 || (t_max - t_min) < 1e-5 {
        if let Some(r) = refine_root(curve, p, t_min, t_max) {
            roots.push(r);
        }
        return;
    }

    // Subdivide 1D degree-5 Bernstein polynomial at midpoint t = 0.5 using de Casteljau
    let mut left = [0.0; 6];
    let mut right = [0.0; 6];
    let mut temp = [[0.0; 6]; 6];
    temp[0] = *c;

    for i in 1..=5 {
        for j in 0..=(5 - i) {
            temp[i][j] = 0.5 * (temp[i - 1][j] + temp[i - 1][j + 1]);
        }
    }

    for i in 0..=5 {
        left[i] = temp[i][0];
        right[5 - i] = temp[i][5 - i];
    }

    let t_mid = 0.5 * (t_min + t_max);
    find_bezier_roots_5(curve, p, &left, t_min, t_mid, depth + 1, roots);
    find_bezier_roots_5(curve, p, &right, t_mid, t_max, depth + 1, roots);
}

fn count_sign_changes(c: &[f64; 6]) -> usize {
    let mut changes = 0;
    let mut prev_sign = 0;
    for &val in c {
        let sign = if val > 1e-12 {
            1
        } else if val < -1e-12 {
            -1
        } else {
            0
        };
        if sign != 0 {
            if prev_sign != 0 && sign != prev_sign {
                changes += 1;
            }
            prev_sign = sign;
        }
    }
    changes
}

fn eval_f(curve: &CubicBezier, p: Point2, t: f64) -> f64 {
    let bt = curve.evaluate(t);
    let d1 = curve.derivative(t);
    (bt - p).dot(d1)
}

fn refine_root(curve: &CubicBezier, p: Point2, mut a: f64, mut b: f64) -> Option<f64> {
    let mut fa = eval_f(curve, p, a);
    let fb = eval_f(curve, p, b);

    if fa.abs() < 1e-12 {
        return Some(a);
    }
    if fb.abs() < 1e-12 {
        return Some(b);
    }
    if fa * fb > 0.0 {
        let mid = 0.5 * (a + b);
        let fmid = eval_f(curve, p, mid);
        if fmid.abs() < fa.abs().min(fb.abs()) {
            return Some(mid);
        }
        return None;
    }

    for _ in 0..30 {
        if (b - a).abs() < 1e-8 {
            break;
        }
        let mid = 0.5 * (a + b);
        let fmid = eval_f(curve, p, mid);
        if fmid.abs() < 1e-12 {
            return Some(mid);
        }
        if fa * fmid < 0.0 {
            b = mid;
        } else {
            a = mid;
            fa = fmid;
        }
    }
    Some(0.5 * (a + b))
}
