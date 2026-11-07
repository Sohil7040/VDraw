//! Boolean polygon operations: Weld, Trim, Intersect, Exclude, Front-Minus-Back,
//! Back-Minus-Front, Create Boundary, and Simplify.

use clipper2_rs::{difference_d, intersect_d, union_d, union_self_d, xor_d, FillRule};
use core_geom::Polycurve;

use crate::convert::{paths_d_to_polycurve, polycurve_to_paths_d, DEFAULT_FLATTEN_TOLERANCE};
use crate::error::BooleanError;

/// The type of boolean operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    /// Weld (Union): Merges all selected shapes into one unified shape.
    Weld,
    /// Trim (Difference): Subtracts the second shape from the first shape.
    Trim,
    /// Intersect: Keeps only the overlapping area common to all shapes.
    Intersect,
    /// Exclude (XOR): Keeps non-overlapping areas and removes overlapping portions.
    Exclude,
    /// Front-Minus-Back: Subtracts back shape from front shape.
    FrontMinusBack,
    /// Back-Minus-Front: Subtracts front shape from back shape.
    BackMinusFront,
    /// Create Boundary: Generates an outline curve of the welded boundary while
    /// preserving original source shapes.
    CreateBoundary,
    /// Simplify: Cleans self-intersections and normalizes contour winding.
    Simplify,
}

/// Executes a boolean operation on a slice of `Polycurve` shapes.
///
/// Returns a list of resulting `Polycurve` shapes (and if `CreateBoundary`,
/// the boundary curve is returned without deleting inputs).
pub fn execute_boolean(
    op: BooleanOp,
    shapes: &[Polycurve],
    refit_curves: bool,
) -> Result<Vec<Polycurve>, BooleanError> {
    if shapes.is_empty() {
        return Err(BooleanError::InvalidGeometry);
    }

    if op == BooleanOp::Simplify {
        return simplify_shape(&shapes[0], refit_curves);
    }

    if shapes.len() < 2 {
        return Err(BooleanError::InsufficientShapes(format!("{:?}", op)));
    }

    let precision = 4; // 4 decimal places (~0.1 micron in mm space)
    let fill_rule = FillRule::NonZero;

    match op {
        BooleanOp::Weld | BooleanOp::CreateBoundary => {
            let mut acc_paths = polycurve_to_paths_d(&shapes[0], DEFAULT_FLATTEN_TOLERANCE);

            for next_shape in &shapes[1..] {
                let next_paths = polycurve_to_paths_d(next_shape, DEFAULT_FLATTEN_TOLERANCE);
                acc_paths = union_d(&acc_paths, &next_paths, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let result_poly = paths_d_to_polycurve(&acc_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Trim | BooleanOp::FrontMinusBack => {
            // Subject is shapes[0] (or front), Clip is shapes[1]
            let subject = polycurve_to_paths_d(&shapes[0], DEFAULT_FLATTEN_TOLERANCE);
            let clip = polycurve_to_paths_d(&shapes[1], DEFAULT_FLATTEN_TOLERANCE);

            let res_paths = difference_d(&subject, &clip, fill_rule, precision)
                .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

            let result_poly = paths_d_to_polycurve(&res_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::BackMinusFront => {
            // Subject is shapes[1] (back), Clip is shapes[0] (front)
            let subject = polycurve_to_paths_d(&shapes[1], DEFAULT_FLATTEN_TOLERANCE);
            let clip = polycurve_to_paths_d(&shapes[0], DEFAULT_FLATTEN_TOLERANCE);

            let res_paths = difference_d(&subject, &clip, fill_rule, precision)
                .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

            let result_poly = paths_d_to_polycurve(&res_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Intersect => {
            let mut acc_paths = polycurve_to_paths_d(&shapes[0], DEFAULT_FLATTEN_TOLERANCE);

            for next_shape in &shapes[1..] {
                let next_paths = polycurve_to_paths_d(next_shape, DEFAULT_FLATTEN_TOLERANCE);
                acc_paths = intersect_d(&acc_paths, &next_paths, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let result_poly = paths_d_to_polycurve(&acc_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Exclude => {
            let mut acc_paths = polycurve_to_paths_d(&shapes[0], DEFAULT_FLATTEN_TOLERANCE);

            for next_shape in &shapes[1..] {
                let next_paths = polycurve_to_paths_d(next_shape, DEFAULT_FLATTEN_TOLERANCE);
                acc_paths = xor_d(&acc_paths, &next_paths, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let result_poly = paths_d_to_polycurve(&acc_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Simplify => unreachable!(),
    }
}

/// Simplifies a single shape by resolving self-intersections and normalizing winding.
pub fn simplify_shape(
    shape: &Polycurve,
    refit_curves: bool,
) -> Result<Vec<Polycurve>, BooleanError> {
    let precision = 4;
    let fill_rule = FillRule::NonZero;
    let paths = polycurve_to_paths_d(shape, DEFAULT_FLATTEN_TOLERANCE);

    let simplified = union_self_d(&paths, fill_rule, precision)
        .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

    let result_poly = paths_d_to_polycurve(&simplified, true, refit_curves);
    Ok(vec![result_poly])
}

/// Helper function to perform a boolean operation between two Polycurves.
pub fn boolean_op(op: BooleanOp, a: &Polycurve, b: &Polycurve) -> Polycurve {
    match execute_boolean(op, &[a.clone(), b.clone()], true) {
        Ok(mut res) if !res.is_empty() => res.remove(0),
        _ => a.clone(),
    }
}
