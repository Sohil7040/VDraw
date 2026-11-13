//! Philip J. Schneider's algorithm for automatically fitting digitized curves
//! with smooth cubic Bézier segments within a specified error tolerance.
//!
//! Based on "An Algorithm for Automatically Fitting Digitized Curves", Graphics Gems I (1990).

use crate::bezier::CubicBezier;
use crate::node::{BezierNode, NodeType};
use crate::point::Point2;
use crate::polycurve::Subpath;

/// Fits a digitized series of points with one or more cubic Bézier curves
/// within the specified Euclidean distance `tolerance` (in document units, e.g. mm).
pub fn fit_curve(points: &[Point2], tolerance: f64) -> Subpath {
    let tolerance = tolerance.max(1e-4);

    // Filter consecutive duplicate / near-zero-distance points
    let mut filtered = Vec::with_capacity(points.len());
    for &p in points {
        if let Some(&last) = filtered.last() {
            if (p - last).length() < 1e-6 {
                continue;
            }
        }
        filtered.push(p);
    }

    if filtered.is_empty() {
        return Subpath::new(false);
    }
    if filtered.len() == 1 {
        let mut sub = Subpath::new(false);
        sub.push_node(BezierNode::new(filtered[0], NodeType::Cusp));
        return sub;
    }
    if filtered.len() == 2 {
        let mut sub = Subpath::new(false);
        let p0 = filtered[0];
        let p1 = filtered[1];
        let h_out = p0.lerp(p1, 1.0 / 3.0);
        let h_in = p0.lerp(p1, 2.0 / 3.0);
        sub.push_node(BezierNode::with_handles(
            p0,
            None,
            Some(h_out),
            NodeType::Cusp,
        ));
        sub.push_node(BezierNode::with_handles(
            p1,
            Some(h_in),
            None,
            NodeType::Cusp,
        ));
        return sub;
    }

    // Detect sharp corners (deflection angle > 45°) to partition the curve into smooth segments
    let mut corner_indices = vec![0];
    for i in 1..filtered.len() - 1 {
        let v1 = filtered[i] - filtered[i - 1];
        let v2 = filtered[i + 1] - filtered[i];
        let len1 = v1.length();
        let len2 = v2.length();
        if len1 > 1e-4 && len2 > 1e-4 {
            let cos_angle = v1.dot(v2) / (len1 * len2);
            if cos_angle < 0.707 {
                corner_indices.push(i);
            }
        }
    }
    corner_indices.push(filtered.len() - 1);

    if corner_indices.len() > 2 {
        // Curve contains one or more sharp corners: fit each smooth segment independently
        let mut subpath = Subpath::new(false);
        for s in 0..corner_indices.len() - 1 {
            let start = corner_indices[s];
            let end = corner_indices[s + 1];
            let seg_points = &filtered[start..=end];

            let t_hat1 = compute_left_tangent(seg_points, 0);
            let t_hat2 = compute_right_tangent(seg_points, seg_points.len() - 1);
            let mut cubics = Vec::new();
            fit_cubic_recursive(seg_points, t_hat1, t_hat2, tolerance, &mut cubics);

            let seg_sub = cubics_to_subpath(&cubics);
            if s == 0 {
                subpath = seg_sub;
            } else {
                // Join at corner with Cusp continuity
                if let Some(first_node) = seg_sub.nodes.first() {
                    if let Some(last_node) = subpath.nodes.last_mut() {
                        last_node.handle_out = first_node.handle_out;
                        last_node.node_type = NodeType::Cusp;
                    }
                }
                for node in seg_sub.nodes.into_iter().skip(1) {
                    subpath.push_node(node);
                }
            }
        }
        return subpath;
    }

    let t_hat1 = compute_left_tangent(&filtered, 0);
    let t_hat2 = compute_right_tangent(&filtered, filtered.len() - 1);

    let mut cubics = Vec::new();
    fit_cubic_recursive(&filtered, t_hat1, t_hat2, tolerance, &mut cubics);

    cubics_to_subpath(&cubics)
}

/// Recursively fit cubic Béziers to a slice of points.
fn fit_cubic_recursive(
    points: &[Point2],
    t_hat1: Point2,
    t_hat2: Point2,
    tolerance: f64,
    cubics: &mut Vec<CubicBezier>,
) {
    let n_pts = points.len();
    if n_pts <= 2 {
        let p0 = points[0];
        let p3 = points[n_pts - 1];
        cubics.push(CubicBezier::line(p0, p3));
        return;
    }

    let mut u = chord_length_parameterize(points);
    let mut bezier = generate_bezier(points, &u, t_hat1, t_hat2);
    let (mut max_error, mut split_point) = compute_max_error(points, &u, &bezier);

    if max_error < tolerance {
        cubics.push(bezier);
        return;
    }

    // Try re-parameterizing up to 4 iterations (Newton-Raphson)
    const MAX_ITERATIONS: usize = 4;
    if max_error < tolerance * 4.0 {
        for _ in 0..MAX_ITERATIONS {
            u = reparameterize(points, &u, &bezier);
            bezier = generate_bezier(points, &u, t_hat1, t_hat2);
            let (err, split) = compute_max_error(points, &u, &bezier);
            max_error = err;
            split_point = split;
            if max_error < tolerance {
                cubics.push(bezier);
                return;
            }
        }
    }

    // Still exceeds tolerance: split at the point of maximum error
    let split_idx = split_point.clamp(1, n_pts - 2);
    let t_hat_center = compute_center_tangent(points, split_idx);

    fit_cubic_recursive(
        &points[..=split_idx],
        t_hat1,
        -t_hat_center,
        tolerance,
        cubics,
    );
    fit_cubic_recursive(
        &points[split_idx..],
        t_hat_center,
        t_hat2,
        tolerance,
        cubics,
    );
}

/// Generate a cubic Bézier curve using least-squares fit to the given parameterization.
fn generate_bezier(points: &[Point2], u: &[f64], t_hat1: Point2, t_hat2: Point2) -> CubicBezier {
    let n_pts = points.len();
    let p0 = points[0];
    let p3 = points[n_pts - 1];

    let mut c = [[0.0; 2]; 2];
    let mut x = [0.0; 2];

    for i in 0..n_pts {
        let ui = u[i];
        let b0 = (1.0 - ui).powi(3);
        let b1 = 3.0 * ui * (1.0 - ui).powi(2);
        let b2 = 3.0 * ui.powi(2) * (1.0 - ui);
        let b3 = ui.powi(3);

        let a1 = t_hat1 * b1;
        let a2 = t_hat2 * b2;

        c[0][0] += a1.dot(a1);
        c[0][1] += a1.dot(a2);
        c[1][0] = c[0][1];
        c[1][1] += a2.dot(a2);

        let pt = points[i];
        let origin = p0 * (b0 + b1) + p3 * (b2 + b3);
        let diff = pt - origin;

        x[0] += a1.dot(diff);
        x[1] += a2.dot(diff);
    }

    // Solve 2x2 linear system using determinants
    let det_c0_c1 = c[0][0] * c[1][1] - c[1][0] * c[0][1];
    let det_c0_x = c[0][0] * x[1] - c[1][0] * x[0];
    let det_x_c1 = x[0] * c[1][1] - x[1] * c[0][1];

    let (mut alpha_l, mut alpha_r) = if det_c0_c1.abs() > 1e-12 {
        (det_x_c1 / det_c0_c1, det_c0_x / det_c0_c1)
    } else {
        (0.0, 0.0)
    };

    // If alpha is negative or zero, fall back to default chord heuristic
    let chord = (p3 - p0).length();
    let eps = 1e-6 * chord;
    if alpha_l < eps || alpha_r < eps {
        let split_dist = chord / 3.0;
        alpha_l = split_dist;
        alpha_r = split_dist;
    }

    CubicBezier::new(p0, p0 + t_hat1 * alpha_l, p3 + t_hat2 * alpha_r, p3)
}

/// Compute maximum Euclidean error of points relative to the fitted curve.
/// Also inspects chord spans between sparse samples to detect curve bowing / sag.
fn compute_max_error(points: &[Point2], u: &[f64], bezier: &CubicBezier) -> (f64, usize) {
    let mut max_dist = 0.0;
    let mut split_point = points.len() / 2;

    for (i, (&pt, &ui)) in points.iter().zip(u.iter()).enumerate() {
        let p_on_curve = bezier.evaluate(ui);
        let dist = (p_on_curve - pt).length();
        if dist >= max_dist {
            max_dist = dist;
            split_point = i;
        }
    }

    // Check intermediate chord points for sparse digitized points
    for i in 0..points.len() - 1 {
        let p_a = points[i];
        let p_b = points[i + 1];
        let chord_len = (p_b - p_a).length();
        if chord_len > 0.1 {
            let u_a = u[i];
            let u_b = u[i + 1];
            for frac in [0.25, 0.5, 0.75] {
                let u_sample = u_a + (u_b - u_a) * frac;
                let p_expected = p_a.lerp(p_b, frac);
                let p_curve = bezier.evaluate(u_sample);
                let dist = (p_curve - p_expected).length();
                if dist > max_dist {
                    max_dist = dist;
                    split_point = if frac < 0.5 && i > 0 { i } else { i + 1 };
                }
            }
        }
    }

    (max_dist, split_point)
}

/// Use Newton-Raphson iteration to find a better parameter u for each digitized point.
fn reparameterize(points: &[Point2], u: &[f64], bezier: &CubicBezier) -> Vec<f64> {
    points
        .iter()
        .zip(u.iter())
        .map(|(&pt, &ui)| {
            let p_u = bezier.evaluate(ui);
            let d_u = bezier.derivative(ui);
            let dd_u = bezier.second_derivative(ui);

            let diff = p_u - pt;
            let numerator = diff.dot(d_u);
            let denominator = d_u.dot(d_u) + diff.dot(dd_u);

            if denominator.abs() > 1e-12 {
                (ui - numerator / denominator).clamp(0.0, 1.0)
            } else {
                ui
            }
        })
        .collect()
}

/// Chord-length parameterization for points.
fn chord_length_parameterize(points: &[Point2]) -> Vec<f64> {
    let mut u = Vec::with_capacity(points.len());
    u.push(0.0);

    for i in 1..points.len() {
        let dist = (points[i] - points[i - 1]).length();
        let prev = u[i - 1];
        u.push(prev + dist);
    }

    let total = *u.last().unwrap_or(&1.0);
    if total > 1e-12 {
        for val in &mut u {
            *val /= total;
        }
    } else {
        let n = points.len();
        for (i, val) in u.iter_mut().enumerate() {
            *val = i as f64 / (n - 1).max(1) as f64;
        }
    }

    u
}

/// Approximate unit tangent at the start point of a segment.
fn compute_left_tangent(points: &[Point2], start_idx: usize) -> Point2 {
    let p0 = points[start_idx];
    for &p in &points[start_idx + 1..] {
        let v = p - p0;
        if v.length() > 1e-4 {
            return v.normalize();
        }
    }
    Point2::new(1.0, 0.0)
}

/// Approximate unit tangent at the end point of a segment.
fn compute_right_tangent(points: &[Point2], end_idx: usize) -> Point2 {
    let pn = points[end_idx];
    for &p in points[..end_idx].iter().rev() {
        let v = p - pn;
        if v.length() > 1e-4 {
            return v.normalize();
        }
    }
    Point2::new(-1.0, 0.0)
}

/// Approximate unit tangent at an interior split point.
fn compute_center_tangent(points: &[Point2], center_idx: usize) -> Point2 {
    let p_c = points[center_idx];
    let mut v1 = Point2::ZERO;
    let mut v2 = Point2::ZERO;

    for &p in points[..center_idx].iter().rev() {
        let v = p_c - p;
        if v.length() > 1e-4 {
            v1 = v.normalize();
            break;
        }
    }

    for &p in &points[center_idx + 1..] {
        let v = p - p_c;
        if v.length() > 1e-4 {
            v2 = v.normalize();
            break;
        }
    }

    let v = (v1 + v2) * 0.5;
    if v.length() > 1e-6 {
        v.normalize()
    } else {
        Point2::new(1.0, 0.0)
    }
}

/// Convert a list of contiguous `CubicBezier` segments into a `Subpath` of `BezierNode`s.
///
/// Automatically classifies node continuity as `Smooth`, `Symmetrical`, or `Cusp`
/// based on the angular deflection between incoming and outgoing tangent handles.
fn cubics_to_subpath(cubics: &[CubicBezier]) -> Subpath {
    let mut sub = Subpath::new(false);
    if cubics.is_empty() {
        return sub;
    }

    // First node
    let first = &cubics[0];
    sub.push_node(BezierNode::with_handles(
        first.p0,
        None,
        Some(first.p1),
        NodeType::Cusp,
    ));

    // Intermediate nodes
    for i in 0..cubics.len() - 1 {
        let prev = &cubics[i];
        let next = &cubics[i + 1];
        let anchor = prev.p3;
        let h_in = prev.p2;
        let h_out = next.p1;

        let v_in = anchor - h_in;
        let v_out = h_out - anchor;

        let len_in = v_in.length();
        let len_out = v_out.length();

        let node_type = if len_in > 1e-6 && len_out > 1e-6 {
            let cos_angle = (v_in.dot(v_out)) / (len_in * len_out);
            if cos_angle > 0.85 {
                NodeType::Smooth
            } else {
                NodeType::Cusp
            }
        } else {
            NodeType::Cusp
        };

        sub.push_node(BezierNode::with_handles(
            anchor,
            Some(h_in),
            Some(h_out),
            node_type,
        ));
    }

    // Last node
    let last = &cubics[cubics.len() - 1];
    sub.push_node(BezierNode::with_handles(
        last.p3,
        Some(last.p2),
        None,
        NodeType::Cusp,
    ));

    sub
}
