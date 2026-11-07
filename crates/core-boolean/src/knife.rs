//! Knife slicing engine: divides vector shapes across a straight line or freehand stroke.

use clipper2_rs::{difference_d, inflate_paths_d, EndType, FillRule, JoinType, PathD, PointD};
use core_geom::rect::Rect;
use core_geom::{Point2, Polycurve};

use crate::convert::{
    paths_d_to_polycurve, polycurve_to_paths_d, polyline_to_subpath, DEFAULT_FLATTEN_TOLERANCE,
};
use crate::error::BooleanError;

/// Slices a `Polycurve` across a cut path.
///
/// Returns a list of separated `Polycurve` pieces.
pub fn slice_shape(
    shape: &Polycurve,
    cut_line: &[Point2],
    auto_close: bool,
) -> Result<Vec<Polycurve>, BooleanError> {
    if cut_line.len() < 2 {
        return Err(BooleanError::InvalidGeometry);
    }

    let bounds = shape.bounds().ok_or(BooleanError::InvalidGeometry)?;

    // Check if cut line intersects bounding box
    if !line_intersects_rect(cut_line, &bounds) {
        return Err(BooleanError::NoKnifeIntersection);
    }

    // Check if any subpath is closed
    let is_any_closed = shape.subpaths.iter().any(|s| s.closed);

    if is_any_closed {
        slice_closed_shape(shape, cut_line, auto_close)
    } else {
        slice_open_paths(shape, cut_line)
    }
}

/// Slices closed shapes by subtracting a hairline slit and separating disconnected islands.
fn slice_closed_shape(
    shape: &Polycurve,
    cut_line: &[Point2],
    auto_close: bool,
) -> Result<Vec<Polycurve>, BooleanError> {
    let precision = 4;
    let fill_rule = FillRule::NonZero;

    // Convert cut line to PathD
    let mut cut_path = PathD::new();
    for p in cut_line {
        cut_path.push(PointD::new(p.x, p.y));
    }
    let cut_paths = vec![cut_path];

    // Inflate cut line into an ultra-fine slit (0.001 mm)
    let slit_delta = 0.001;
    let slit = inflate_paths_d(
        &cut_paths,
        slit_delta,
        JoinType::Miter,
        EndType::Square,
        2.0,
        precision,
        0.001,
    )
    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

    let subject = polycurve_to_paths_d(shape, DEFAULT_FLATTEN_TOLERANCE);
    let cut_result = difference_d(&subject, &slit, fill_rule, precision)
        .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

    if cut_result.is_empty() {
        return Err(BooleanError::NoKnifeIntersection);
    }

    // Convert each resulting closed contour into its own independent Polycurve
    let mut pieces = Vec::new();
    for island in &cut_result {
        if island.len() < 3 {
            continue;
        }

        let island_paths = vec![island.clone()];

        let piece_poly = paths_d_to_polycurve(&island_paths, auto_close, true);
        if !piece_poly.subpaths.is_empty() {
            pieces.push(piece_poly);
        }
    }

    if pieces.len() <= 1 {
        // Did not cut through
        return Err(BooleanError::NoKnifeIntersection);
    }

    Ok(pieces)
}

/// Slices open paths at intersection points along the cut line.
fn slice_open_paths(
    shape: &Polycurve,
    cut_line: &[Point2],
) -> Result<Vec<Polycurve>, BooleanError> {
    let mut all_pieces = Vec::new();

    for sub in &shape.subpaths {
        let pts = sub.flatten(DEFAULT_FLATTEN_TOLERANCE);
        if pts.len() < 2 {
            continue;
        }

        let mut split_segments: Vec<Vec<Point2>> = vec![Vec::new()];

        for i in 0..pts.len() - 1 {
            let p1 = pts[i];
            let p2 = pts[i + 1];

            split_segments.last_mut().unwrap().push(p1);

            // Check intersection with any segment in cut_line
            for j in 0..cut_line.len() - 1 {
                let c1 = cut_line[j];
                let c2 = cut_line[j + 1];

                if let Some(inter) = segment_intersection(p1, p2, c1, c2) {
                    split_segments.last_mut().unwrap().push(inter);
                    // Start new segment piece
                    split_segments.push(vec![inter]);
                }
            }
        }

        if let Some(last) = pts.last() {
            split_segments.last_mut().unwrap().push(*last);
        }

        for seg in split_segments {
            if seg.len() >= 2 {
                let piece_sub = polyline_to_subpath(&seg, false);
                all_pieces.push(Polycurve {
                    subpaths: vec![piece_sub],
                });
            }
        }
    }

    if all_pieces.len() <= shape.subpaths.len() {
        return Err(BooleanError::NoKnifeIntersection);
    }

    Ok(all_pieces)
}

fn line_intersects_rect(line: &[Point2], rect: &Rect) -> bool {
    for p in line {
        if rect.contains(*p) {
            return true;
        }
    }

    // Check segment intersections with 4 rect edges
    let r_min = rect.min();
    let r_max = rect.max();
    let corners = [
        Point2::new(r_min.x, r_min.y),
        Point2::new(r_max.x, r_min.y),
        Point2::new(r_max.x, r_max.y),
        Point2::new(r_min.x, r_max.y),
    ];

    for i in 0..line.len() - 1 {
        let l1 = line[i];
        let l2 = line[i + 1];

        for edge_idx in 0..4 {
            let c1 = corners[edge_idx];
            let c2 = corners[(edge_idx + 1) % 4];
            if segment_intersection(l1, l2, c1, c2).is_some() {
                return true;
            }
        }
    }

    false
}

fn segment_intersection(p1: Point2, p2: Point2, p3: Point2, p4: Point2) -> Option<Point2> {
    let d = (p2.x - p1.x) * (p4.y - p3.y) - (p2.y - p1.y) * (p4.x - p3.x);
    if d.abs() < 1e-9 {
        return None;
    }

    let u = ((p3.x - p1.x) * (p4.y - p3.y) - (p3.y - p1.y) * (p4.x - p3.x)) / d;
    let v = ((p3.x - p1.x) * (p2.y - p1.y) - (p3.y - p1.y) * (p2.x - p1.x)) / d;

    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        Some(Point2::new(
            p1.x + u * (p2.x - p1.x),
            p1.y + u * (p2.y - p1.y),
        ))
    } else {
        None
    }
}
