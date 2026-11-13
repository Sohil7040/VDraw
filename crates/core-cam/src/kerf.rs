//! Kerf compensation engine using parallel contour offsets.
//!
//! Compensates for the physical kerf (beam diameter / cut width) of the laser cutter.
//! Outer contours are expanded and inner holes are contracted (or vice-versa),
//! ensuring finished parts match CAD dimensions precisely.

use crate::operation::KerfMode;
use core_boolean::{offset_shape, ContourJoin};
use core_geom::{Point2, Polycurve, Subpath};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum KerfError {
    #[error("Offset geometry collapsed: feature smaller than kerf width ({kerf_mm:.3} mm)")]
    FeatureCollapsed { kerf_mm: f64 },
    #[error("Boolean offset error: {0}")]
    BooleanError(String),
}

/// Apply kerf compensation to a flattened contour.
///
/// If `is_inner_hole` is true:
/// - Under `KerfMode::Outside`, the hole is contracted by `-kerf / 2`.
/// - Under `KerfMode::Inside`, the hole is expanded by `+kerf / 2`.
///
/// If `is_inner_hole` is false (outer boundary):
/// - Under `KerfMode::Outside`, the boundary is expanded by `+kerf / 2`.
/// - Under `KerfMode::Inside`, the boundary is contracted by `-kerf / 2`.
pub fn apply_kerf_compensation(
    points: &[Point2],
    closed: bool,
    is_inner_hole: bool,
    kerf_mm: f64,
    mode: KerfMode,
) -> Result<Vec<Point2>, KerfError> {
    if mode == KerfMode::Center || kerf_mm.abs() < 1e-5 || points.len() < 3 || !closed {
        return Ok(points.to_vec());
    }

    let half_kerf = kerf_mm / 2.0;
    let distance_mm = match mode {
        KerfMode::Center => 0.0,
        KerfMode::Outside => {
            if is_inner_hole {
                -half_kerf
            } else {
                half_kerf
            }
        }
        KerfMode::Inside => {
            if is_inner_hole {
                half_kerf
            } else {
                -half_kerf
            }
        }
    };

    if distance_mm.abs() < 1e-6 {
        return Ok(points.to_vec());
    }

    // Build a Polycurve from the input points
    let mut subpath = Subpath::new(closed);
    for &pt in points {
        subpath.push_node(core_geom::BezierNode::new(pt, core_geom::NodeType::Cusp));
    }
    let poly = Polycurve {
        subpaths: vec![subpath],
    };

    let offset_poly = offset_shape(&poly, distance_mm, ContourJoin::Miter(2.0))
        .map_err(|e| KerfError::BooleanError(e.to_string()))?;

    if offset_poly.subpaths.is_empty() {
        return Err(KerfError::FeatureCollapsed { kerf_mm });
    }

    // Flatten back to points
    let flattened = offset_poly.subpaths[0].flatten(0.02);
    if flattened.len() < 3 {
        return Err(KerfError::FeatureCollapsed { kerf_mm });
    }

    Ok(flattened)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kerf_outside_expansion_and_contraction() {
        // 100 x 100 mm outer square
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
            Point2::new(0.0, 100.0),
        ];

        let kerf = 0.2; // 0.1 mm offset per side
        let outer_offset =
            apply_kerf_compensation(&square, true, false, kerf, KerfMode::Outside).unwrap();

        // Check bounding box expansion
        let min_x = outer_offset
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = outer_offset
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (min_x - (-0.1)).abs() < 1e-2,
            "Expected min_x -0.1, got {min_x}"
        );
        assert!(
            (max_x - 100.1).abs() < 1e-2,
            "Expected max_x 100.1, got {max_x}"
        );

        // Inner hole contraction
        let hole = vec![
            Point2::new(20.0, 20.0),
            Point2::new(40.0, 20.0),
            Point2::new(40.0, 40.0),
            Point2::new(20.0, 40.0),
        ];
        let inner_offset =
            apply_kerf_compensation(&hole, true, true, kerf, KerfMode::Outside).unwrap();
        let h_min_x = inner_offset
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        let h_max_x = inner_offset
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (h_min_x - 20.1).abs() < 1e-2,
            "Expected hole min_x 20.1, got {h_min_x}"
        );
        assert!(
            (h_max_x - 39.9).abs() < 1e-2,
            "Expected hole max_x 39.9, got {h_max_x}"
        );
    }

    #[test]
    fn test_kerf_collapse_detection() {
        // Tiny 0.05 x 0.05 mm square with 0.2 mm kerf (offset distance -0.1 mm)
        let tiny = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.05, 0.0),
            Point2::new(0.05, 0.05),
            Point2::new(0.0, 0.05),
        ];

        let result = apply_kerf_compensation(&tiny, true, true, 0.2, KerfMode::Outside);
        assert!(matches!(result, Err(KerfError::FeatureCollapsed { .. })));
    }
}
