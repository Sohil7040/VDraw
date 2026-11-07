//! Conversion between VDraw Polycurve geometry and Clipper2 / Cavalier data structures,
//! including Schneider cubic Bézier curve re-fitting.

use clipper2_rs::{PathD, PathsD, PointD};
use core_geom::curve_fit::fit_curve;
use core_geom::{BezierNode, NodeType, Point2, Polycurve, Subpath};

/// Default curve flattening tolerance in millimeters for boolean operations.
pub const DEFAULT_FLATTEN_TOLERANCE: f64 = 0.02;

/// Default error tolerance for Schneider curve re-fitting.
pub const DEFAULT_REFIT_TOLERANCE: f64 = 0.05;

/// Converts a `Polycurve` into `clipper2_rs::PathsD`.
pub fn polycurve_to_paths_d(poly: &Polycurve, tolerance: f64) -> PathsD {
    let mut paths = PathsD::new();

    for sub in &poly.subpaths {
        if sub.nodes.is_empty() {
            continue;
        }

        let pts = sub.flatten(tolerance);
        if pts.len() < 2 {
            continue;
        }

        let mut path: PathD = pts.into_iter().map(|p| PointD::new(p.x, p.y)).collect();

        // Clipper expects closed paths to NOT duplicate the first point as the last point
        if sub.closed && path.len() > 1 {
            let first = path[0];
            let last = *path.last().unwrap();
            if (first.x - last.x).abs() < 1e-6 && (first.y - last.y).abs() < 1e-6 {
                path.pop();
            }
        }

        if !path.is_empty() {
            paths.push(path);
        }
    }

    paths
}

/// Converts `clipper2_rs::PathsD` back into a `Polycurve`.
///
/// If `refit_curves` is true, smooth curved sections are re-fitted into minimal
/// cubic Bézier splines using Schneider's algorithm, preserving straight sections.
pub fn paths_d_to_polycurve(paths: &PathsD, is_closed: bool, refit_curves: bool) -> Polycurve {
    let mut subpaths = Vec::new();

    for path in paths {
        if path.len() < 2 {
            continue;
        }

        let pts: Vec<Point2> = path.iter().map(|p| Point2::new(p.x, p.y)).collect();

        let subpath = if refit_curves && pts.len() > 6 {
            // Check if it's purely straight polygon (e.g. box, triangle, hex)
            if is_pure_polyline(&pts, 0.05) {
                polyline_to_subpath(&pts, is_closed)
            } else {
                refit_closed_polyline(&pts, DEFAULT_REFIT_TOLERANCE, is_closed)
            }
        } else {
            polyline_to_subpath(&pts, is_closed)
        };

        subpaths.push(subpath);
    }

    Polycurve { subpaths }
}

/// Converts a slice of points into a discrete linear `Subpath` with Cusp nodes.
pub fn polyline_to_subpath(pts: &[Point2], is_closed: bool) -> Subpath {
    let nodes: Vec<BezierNode> = pts
        .iter()
        .map(|&p| BezierNode::new(p, NodeType::Cusp))
        .collect();

    Subpath {
        nodes,
        closed: is_closed,
    }
}

/// Checks if a polyline consists entirely of a few long straight segments.
fn is_pure_polyline(pts: &[Point2], tolerance: f64) -> bool {
    let n = pts.len();
    if n < 4 {
        return true;
    }

    // Douglas-Peucker simplification check: if simplifying within tolerance
    // collapses 90% of the points, it was mostly straight segments
    let simplified = ramer_douglas_peucker(pts, tolerance);
    simplified.len() <= 8 && simplified.len() * 4 < n
}

/// Re-fits a polyline into smooth cubic Bézier curves.
fn refit_closed_polyline(pts: &[Point2], tolerance: f64, is_closed: bool) -> Subpath {
    if !is_closed {
        return fit_curve(pts, tolerance);
    }

    // For closed contours, ensure loop continuity
    let mut closed_pts = pts.to_vec();
    if let Some(&first) = pts.first() {
        closed_pts.push(first);
    }

    let mut sub = fit_curve(&closed_pts, tolerance);
    sub.closed = true;

    // Remove duplicate end node if fit_curve created one for closed loop
    if sub.nodes.len() > 2 {
        let first_pt = sub.nodes[0].point;
        let last_pt = sub.nodes.last().unwrap().point;
        if first_pt.distance(last_pt) < 1e-4 {
            let last_node = sub.nodes.pop().unwrap();
            if let Some(first_node) = sub.nodes.first_mut() {
                first_node.handle_in = last_node.handle_in;
                first_node.node_type = NodeType::Smooth;
            }
        }
    }

    sub
}

/// Simple Ramer-Douglas-Peucker simplification for straight-segment detection.
pub fn ramer_douglas_peucker(pts: &[Point2], epsilon: f64) -> Vec<Point2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }

    let first = pts[0];
    let last = *pts.last().unwrap();

    let mut max_dist = 0.0;
    let mut index = 0;

    for (i, &pt) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = perpendicular_distance(pt, first, last);
        if d > max_dist {
            max_dist = d;
            index = i;
        }
    }

    if max_dist > epsilon {
        let mut rec1 = ramer_douglas_peucker(&pts[..=index], epsilon);
        let rec2 = ramer_douglas_peucker(&pts[index..], epsilon);

        rec1.pop(); // Remove duplicate at junction
        rec1.extend(rec2);
        rec1
    } else {
        vec![first, last]
    }
}

fn perpendicular_distance(pt: Point2, line_start: Point2, line_end: Point2) -> f64 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let mag = (dx * dx + dy * dy).sqrt();
    if mag < 1e-10 {
        return pt.distance(line_start);
    }
    ((dy * pt.x - dx * pt.y + line_end.x * line_start.y - line_end.y * line_start.x).abs()) / mag
}
