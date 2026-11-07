//! Contour / Parallel curve offset engine for kerf compensation and insets/outsets.

use clipper2_rs::{inflate_paths_d, EndType, JoinType, PathsD};
use core_geom::Polycurve;

use crate::convert::{paths_d_to_polycurve, polycurve_to_paths_d, DEFAULT_FLATTEN_TOLERANCE};
use crate::error::BooleanError;

/// Join style for corners of offset contours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContourJoin {
    /// Smooth rounded corners.
    Round,
    /// Sharp mitered corners up to given miter limit ratio.
    Miter(f64),
    /// Chamfered / beveled corners.
    Bevel,
}

/// Generates a parallel offset contour of a `Polycurve`.
///
/// Positive `distance_mm` expands outward; negative `distance_mm` insets inward.
pub fn offset_shape(
    shape: &Polycurve,
    distance_mm: f64,
    join: ContourJoin,
) -> Result<Polycurve, BooleanError> {
    if distance_mm.abs() < 1e-6 {
        return Err(BooleanError::ZeroOffset);
    }

    let precision = 4;
    let (join_type, miter_limit) = match join {
        ContourJoin::Round => (JoinType::Round, 2.0),
        ContourJoin::Miter(limit) => (JoinType::Miter, limit),
        ContourJoin::Bevel => (JoinType::Bevel, 2.0),
    };

    let is_closed = shape.subpaths.iter().any(|s| s.closed);
    let end_type = if is_closed {
        EndType::Polygon
    } else {
        EndType::Round
    };

    let subject = polycurve_to_paths_d(shape, DEFAULT_FLATTEN_TOLERANCE);

    let offset_paths: PathsD = inflate_paths_d(
        &subject,
        distance_mm,
        join_type,
        end_type,
        miter_limit,
        precision,
        0.02,
    )
    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

    if offset_paths.is_empty() {
        return Ok(Polycurve {
            subpaths: Vec::new(),
        });
    }

    let result = paths_d_to_polycurve(&offset_paths, is_closed, true);
    Ok(result)
}
