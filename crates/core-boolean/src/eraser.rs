//! Eraser subtraction engine: removes material along a dynamic circular or swept stroke.

use clipper2_rs::{difference_d, inflate_paths_d, EndType, FillRule, JoinType, PathD, PointD};
use core_geom::{Point2, Polycurve};
use std::f64::consts::TAU;

use crate::convert::{paths_d_to_polycurve, polycurve_to_paths_d, DEFAULT_FLATTEN_TOLERANCE};
use crate::error::BooleanError;

/// Erases path areas under an eraser stroke of given radius (in millimeters).
///
/// Modifies and returns the updated `Polycurve` shape(s).
pub fn erase_shape(
    shape: &Polycurve,
    eraser_stroke: &[Point2],
    radius: f64,
) -> Result<Polycurve, BooleanError> {
    if eraser_stroke.is_empty() || radius <= 0.0 {
        return Ok(shape.clone());
    }

    let precision = 4;
    let fill_rule = FillRule::NonZero;

    // 1. Build eraser footprint polygon
    let eraser_paths = if eraser_stroke.len() == 1 {
        // Single point click: generate circle polygon
        let center = eraser_stroke[0];
        let steps = 32;
        let mut circle_path = PathD::new();
        for i in 0..steps {
            let theta = (i as f64) * TAU / (steps as f64);
            circle_path.push(PointD::new(
                center.x + radius * theta.cos(),
                center.y + radius * theta.sin(),
            ));
        }
        vec![circle_path]
    } else {
        // Swept line stroke: inflate using Round joins & Round ends
        let mut stroke_path = PathD::new();
        for p in eraser_stroke {
            stroke_path.push(PointD::new(p.x, p.y));
        }
        let raw_paths = vec![stroke_path];

        inflate_paths_d(
            &raw_paths,
            radius,
            JoinType::Round,
            EndType::Round,
            2.0,
            precision,
            0.02,
        )
        .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?
    };

    // 2. Perform difference: subject minus eraser
    let subject = polycurve_to_paths_d(shape, DEFAULT_FLATTEN_TOLERANCE);
    let erased = difference_d(&subject, &eraser_paths, fill_rule, precision)
        .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

    if erased.is_empty() {
        // Entire shape erased
        return Ok(Polycurve {
            subpaths: Vec::new(),
        });
    }

    let is_closed = shape.subpaths.iter().any(|s| s.closed);
    let result = paths_d_to_polycurve(&erased, is_closed, true);
    Ok(result)
}
