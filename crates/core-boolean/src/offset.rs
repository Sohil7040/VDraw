//! Contour / Parallel curve offset engine for kerf compensation and insets/outsets.

use clipper2_rs::{inflate_paths_d, EndType, JoinType, PathsD};
use core_geom::Polycurve;

use crate::convert::{
    paths_d_to_polycurve, polycurve_to_paths_d_checked, DEFAULT_FLATTEN_TOLERANCE,
};
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
/// Properly handles compound shapes containing both closed and open subpaths.
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

    let (closed_subs, open_subs): (Vec<_>, Vec<_>) =
        shape.subpaths.iter().cloned().partition(|s| s.closed);

    let mut combined_offset_paths = PathsD::new();
    let has_closed = !closed_subs.is_empty();

    if has_closed {
        let closed_poly = Polycurve {
            subpaths: closed_subs,
        };
        let subject = polycurve_to_paths_d_checked(&closed_poly, DEFAULT_FLATTEN_TOLERANCE)?;
        if !subject.is_empty() {
            let offset_paths = inflate_paths_d(
                &subject,
                distance_mm,
                join_type,
                EndType::Polygon,
                miter_limit,
                precision,
                0.02,
            )
            .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            combined_offset_paths.extend(offset_paths);
        }
    }

    if !open_subs.is_empty() {
        let open_poly = Polycurve {
            subpaths: open_subs,
        };
        let subject = polycurve_to_paths_d_checked(&open_poly, DEFAULT_FLATTEN_TOLERANCE)?;
        if !subject.is_empty() {
            let offset_paths = inflate_paths_d(
                &subject,
                distance_mm,
                join_type,
                EndType::Round,
                miter_limit,
                precision,
                0.02,
            )
            .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            combined_offset_paths.extend(offset_paths);
        }
    }

    if combined_offset_paths.is_empty() {
        return Ok(Polycurve {
            subpaths: Vec::new(),
        });
    }

    let result = paths_d_to_polycurve(&combined_offset_paths, has_closed, true);
    Ok(result)
}
