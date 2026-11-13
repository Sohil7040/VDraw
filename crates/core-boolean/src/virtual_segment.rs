//! Virtual Segment Delete engine: detects segments bounded by intersection points
//! and cleanly removes targeted segments.

use core_geom::rect::Rect;
use core_geom::{Point2, Polycurve, Subpath};

use crate::convert::{polyline_to_subpath, DEFAULT_FLATTEN_TOLERANCE};
use crate::error::BooleanError;

/// A candidate segment bounded by two intersection points or an endpoint and an intersection.
#[derive(Debug, Clone)]
pub struct VirtualSegment {
    /// Index of the subpath this segment belongs to.
    pub subpath_idx: usize,
    /// Points forming the segment.
    pub points: Vec<Point2>,
    /// Bounding box of the segment for fast culling.
    pub bounds: Rect,
    /// Midpoint along the segment for hit testing.
    pub midpoint: Point2,
}

/// Identifies all virtual segments in a Polycurve formed by intersections with other shapes
/// or self-intersections.
pub fn find_virtual_segments(
    target_shape: &Polycurve,
    all_shapes: &[Polycurve],
) -> Vec<VirtualSegment> {
    let mut segments = Vec::new();

    // Collect all other flattened polylines
    let mut other_polylines: Vec<Vec<Point2>> = Vec::new();
    for s in all_shapes {
        for sub in &s.subpaths {
            let pts = sub.flatten(DEFAULT_FLATTEN_TOLERANCE);
            if pts.len() >= 2 {
                other_polylines.push(pts);
            }
        }
    }

    for (sub_idx, sub) in target_shape.subpaths.iter().enumerate() {
        let pts = sub.flatten(DEFAULT_FLATTEN_TOLERANCE);
        if pts.len() < 2 {
            continue;
        }

        let mut current_pts = vec![pts[0]];

        for i in 0..pts.len() - 1 {
            let p1 = pts[i];
            let p2 = pts[i + 1];

            // Collect all intersections on segment i
            let mut seg_intersections = Vec::new();

            for other in &other_polylines {
                for j in 0..other.len() - 1 {
                    let o1 = other[j];
                    let o2 = other[j + 1];

                    // Skip identical segment comparison
                    if (p1.distance_squared(o1) < 1e-8 && p2.distance_squared(o2) < 1e-8)
                        || (p1.distance_squared(o2) < 1e-8 && p2.distance_squared(o1) < 1e-8)
                    {
                        continue;
                    }

                    if let Some((u, inter)) = segment_intersection_param(p1, p2, o1, o2) {
                        seg_intersections.push((u, inter));
                    }
                }
            }

            // Sort intersections along the segment by u parameter (p1 -> p2)
            seg_intersections
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            for (_u, inter) in seg_intersections {
                current_pts.push(inter);
                if let Some(seg) = make_virtual_segment(sub_idx, current_pts) {
                    segments.push(seg);
                }
                current_pts = vec![inter];
            }

            current_pts.push(p2);
        }

        if let Some(seg) = make_virtual_segment(sub_idx, current_pts) {
            segments.push(seg);
        }
    }

    segments
}

/// Deletes the virtual segment closest to `click_pos` within `tolerance_mm`.
pub fn delete_virtual_segment(
    target_shape: &Polycurve,
    all_shapes: &[Polycurve],
    click_pos: Point2,
    tolerance_mm: f64,
) -> Result<Polycurve, BooleanError> {
    let segments = find_virtual_segments(target_shape, all_shapes);
    if segments.is_empty() {
        return Err(BooleanError::VirtualSegmentNotFound);
    }

    // Find closest segment to click_pos
    let mut best_dist = f64::INFINITY;
    let mut best_idx = None;

    for (idx, seg) in segments.iter().enumerate() {
        // Quick AABB reject
        if seg.bounds.distance_to_point(click_pos) > tolerance_mm {
            continue;
        }

        // Measure distance to segment points
        for i in 0..seg.points.len() - 1 {
            let p1 = seg.points[i];
            let p2 = seg.points[i + 1];
            let d = distance_point_to_line_segment(click_pos, p1, p2);
            if d < best_dist {
                best_dist = d;
                best_idx = Some(idx);
            }
        }
    }

    let target_idx = match best_idx {
        Some(idx) if best_dist <= tolerance_mm => idx,
        _ => return Err(BooleanError::VirtualSegmentNotFound),
    };

    // Reconstruct shape keeping all segments EXCEPT the clicked one
    let mut new_subpaths: Vec<Subpath> = Vec::new();

    for (idx, seg) in segments.into_iter().enumerate() {
        if idx == target_idx {
            continue; // Skip clicked virtual segment!
        }

        if seg.points.len() >= 2 {
            new_subpaths.push(polyline_to_subpath(&seg.points, false));
        }
    }

    Ok(Polycurve {
        subpaths: new_subpaths,
    })
}

fn make_virtual_segment(subpath_idx: usize, points: Vec<Point2>) -> Option<VirtualSegment> {
    if points.len() < 2 {
        return None;
    }

    let mid_idx = points.len() / 2;
    let midpoint = points[mid_idx];

    let mut b = Rect::from_point(points[0]);
    for p in &points[1..] {
        b.include_point(*p);
    }

    Some(VirtualSegment {
        subpath_idx,
        points,
        bounds: b,
        midpoint,
    })
}

fn segment_intersection_param(
    p1: Point2,
    p2: Point2,
    p3: Point2,
    p4: Point2,
) -> Option<(f64, Point2)> {
    let d = (p2.x - p1.x) * (p4.y - p3.y) - (p2.y - p1.y) * (p4.x - p3.x);
    if d.abs() < 1e-9 {
        return None;
    }

    let u = ((p3.x - p1.x) * (p4.y - p3.y) - (p3.y - p1.y) * (p4.x - p3.x)) / d;
    let v = ((p3.x - p1.x) * (p2.y - p1.y) - (p3.y - p1.y) * (p2.x - p1.x)) / d;

    if (0.001..=0.999).contains(&u) && (0.001..=0.999).contains(&v) {
        Some((
            u,
            Point2::new(p1.x + u * (p2.x - p1.x), p1.y + u * (p2.y - p1.y)),
        ))
    } else {
        None
    }
}

fn distance_point_to_line_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab_x = b.x - a.x;
    let ab_y = b.y - a.y;
    let len_sq = ab_x * ab_x + ab_y * ab_y;
    if len_sq < 1e-10 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * ab_x + (p.y - a.y) * ab_y) / len_sq).clamp(0.0, 1.0);
    let proj = Point2::new(a.x + t * ab_x, a.y + t * ab_y);
    p.distance(proj)
}
