//! Boolean polygon operations: Weld, Trim, Intersect, Exclude, Front-Minus-Back,
//! Back-Minus-Front, Create Boundary, and Simplify.

use clipper2_rs::{
    area, area_paths, difference_d, intersect_d, simplify_paths, union_d, union_self_d, xor_d,
    FillRule, PathsD,
};
use core_geom::{Polycurve, TransformApply};

use crate::contracts::{BoolCancelToken, BoolError, BoolOp, BoolOptions};
use crate::convert::{
    paths_d_to_polycurve, polycurve_to_paths_d_checked, DEFAULT_FLATTEN_TOLERANCE,
};
use crate::error::BooleanError;

/// The type of boolean operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    /// Weld (Union): Merges all selected shapes into one unified shape.
    Weld,
    /// Trim: Subtracts the cutting tool shape (`shapes[1]`) from the target shape (`shapes[0]`).
    Trim,
    /// Intersect: Keeps only the overlapping area common to all shapes.
    Intersect,
    /// Exclude (XOR): Keeps non-overlapping areas and removes overlapping portions.
    Exclude,
    /// Front-Minus-Back: Subtracts back shape(s) (`shapes[0..last]`) from the front-most shape (`shapes.last()`).
    FrontMinusBack,
    /// Back-Minus-Front: Subtracts front shape(s) (`shapes[1..]`) from the back-most shape (`shapes[0]`).
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
            let mut acc_paths =
                polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;

            for next_shape in &shapes[1..] {
                let next_paths =
                    polycurve_to_paths_d_checked(next_shape, DEFAULT_FLATTEN_TOLERANCE)?;
                acc_paths = union_d(&acc_paths, &next_paths, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let result_poly = paths_d_to_polycurve(&acc_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Trim => {
            // Trim: Target shape (shapes[0]) trimmed by cutting tool (shapes[1])
            let subject = polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;
            let clip = polycurve_to_paths_d_checked(&shapes[1], DEFAULT_FLATTEN_TOLERANCE)?;

            let res_paths = difference_d(&subject, &clip, fill_rule, precision)
                .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

            let result_poly = paths_d_to_polycurve(&res_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::FrontMinusBack => {
            // Front shape is highest z-index (shapes.last()), Back shapes are shapes[..last]
            let front_idx = shapes.len() - 1;
            let front_paths =
                polycurve_to_paths_d_checked(&shapes[front_idx], DEFAULT_FLATTEN_TOLERANCE)?;

            let mut back_union =
                polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;
            for back_shape in &shapes[1..front_idx] {
                let p = polycurve_to_paths_d_checked(back_shape, DEFAULT_FLATTEN_TOLERANCE)?;
                back_union = union_d(&back_union, &p, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let res_paths = difference_d(&front_paths, &back_union, fill_rule, precision)
                .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

            let result_poly = paths_d_to_polycurve(&res_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::BackMinusFront => {
            // Back shape is lowest z-index (shapes[0]), Front shapes are shapes[1..]
            let back_paths = polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;

            let mut front_union =
                polycurve_to_paths_d_checked(&shapes[1], DEFAULT_FLATTEN_TOLERANCE)?;
            for front_shape in &shapes[2..] {
                let p = polycurve_to_paths_d_checked(front_shape, DEFAULT_FLATTEN_TOLERANCE)?;
                front_union = union_d(&front_union, &p, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let res_paths = difference_d(&back_paths, &front_union, fill_rule, precision)
                .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;

            let result_poly = paths_d_to_polycurve(&res_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Intersect => {
            let mut acc_paths =
                polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;

            for next_shape in &shapes[1..] {
                let next_paths =
                    polycurve_to_paths_d_checked(next_shape, DEFAULT_FLATTEN_TOLERANCE)?;
                acc_paths = intersect_d(&acc_paths, &next_paths, fill_rule, precision)
                    .map_err(|e| BooleanError::ClipperError(format!("{:?}", e)))?;
            }

            let result_poly = paths_d_to_polycurve(&acc_paths, true, refit_curves);
            Ok(vec![result_poly])
        }

        BooleanOp::Exclude => {
            let mut acc_paths =
                polycurve_to_paths_d_checked(&shapes[0], DEFAULT_FLATTEN_TOLERANCE)?;

            for next_shape in &shapes[1..] {
                let next_paths =
                    polycurve_to_paths_d_checked(next_shape, DEFAULT_FLATTEN_TOLERANCE)?;
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
    let paths = polycurve_to_paths_d_checked(shape, DEFAULT_FLATTEN_TOLERANCE)?;

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

#[inline]
fn check_cancelled(cancel: Option<&BoolCancelToken>) -> Result<(), BoolError> {
    if cancel.is_some_and(BoolCancelToken::is_cancelled) {
        Err(BoolError::Cancelled)
    } else {
        Ok(())
    }
}

#[inline]
fn report_progress(
    progress: Option<&dyn Fn(f32)>,
    cancel: Option<&BoolCancelToken>,
    fraction: f32,
) -> Result<(), BoolError> {
    if let Some(cb) = progress {
        cb(fraction);
    }
    check_cancelled(cancel)
}

pub(crate) fn boolean_with_options_impl(
    shapes: &[core_doc::Shape],
    op: BoolOp,
    opts: &BoolOptions,
    cancel: Option<&BoolCancelToken>,
    progress: Option<&dyn Fn(f32)>,
) -> Result<core_doc::Shape, BoolError> {
    check_cancelled(cancel)?;
    if shapes.len() < 2 {
        return Err(BoolError::EmptyInput(format!(
            "{op:?} requires at least 2 shapes"
        )));
    }
    report_progress(progress, cancel, 0.0)?;

    let precision = 4; // 4 decimal places (~0.1 micron in mm space)
    let fill_rule = FillRule::NonZero;
    let flatten_tol = if opts.tolerance.is_finite() && opts.tolerance > 0.0 {
        opts.tolerance.clamp(1e-4, 0.5)
    } else {
        DEFAULT_FLATTEN_TOLERANCE
    };
    let simplify_eps = (flatten_tol * 0.25).clamp(1e-5, 0.05);

    // Phase 1: Convert input shapes into world-space Polycurves and Clipper PathsD.
    let mut converted_inputs: Vec<PathsD> = Vec::with_capacity(shapes.len());
    for (idx, shape) in shapes.iter().enumerate() {
        check_cancelled(cancel)?;
        let mut poly = shape.to_polycurve();
        if poly.is_empty() {
            return Err(BoolError::EmptyInput(format!(
                "shape at index {idx} has empty geometry"
            )));
        }
        if !poly.subpaths.iter().any(|s| s.closed) {
            return Err(BoolError::EmptyInput(format!(
                "shape at index {idx} has no closed contours"
            )));
        }
        let _ = poly.apply(&shape.transform);
        let paths = polycurve_to_paths_d_checked(&poly, flatten_tol)?;
        if paths.is_empty() {
            return Err(BoolError::EmptyInput(format!(
                "shape at index {idx} produced no valid closed contours"
            )));
        }
        converted_inputs.push(paths);
    }
    report_progress(progress, cancel, 0.25)?;

    // Phase 2: Self-intersection normalization & contour winding normalization.
    let mut normalized_inputs: Vec<PathsD> = Vec::with_capacity(converted_inputs.len());
    for (idx, paths) in converted_inputs.into_iter().enumerate() {
        check_cancelled(cancel)?;
        let mut norm = union_self_d(&paths, fill_rule, precision)
            .map_err(|e| BoolError::SelfIntersection(format!("{e:?}")))?;
        norm.retain(|p| p.len() >= 3 && area(p).abs() >= 1e-9);
        if norm.is_empty() {
            return Err(BoolError::DegenerateInput(format!(
                "shape at index {idx} collapsed to zero area after self-intersection normalization"
            )));
        }
        normalized_inputs.push(norm);
    }
    report_progress(progress, cancel, 0.50)?;

    // Phase 3: N-ary boolean evaluation.
    let mut iter = normalized_inputs.into_iter();
    let mut acc = iter
        .next()
        .ok_or_else(|| BoolError::EmptyInput(format!("{op:?} requires at least 2 shapes")))?;
    for next_paths in iter {
        check_cancelled(cancel)?;
        acc = match op {
            BoolOp::Union => union_d(&acc, &next_paths, fill_rule, precision),
            BoolOp::Difference => difference_d(&acc, &next_paths, fill_rule, precision),
            BoolOp::Intersection => intersect_d(&acc, &next_paths, fill_rule, precision),
        }
        .map_err(|e| BoolError::KernelError(format!("{e:?}")))?;
    }
    report_progress(progress, cancel, 0.75)?;

    // Phase 4: Optional polygon cleaning & winding/area check.
    check_cancelled(cancel)?;
    if opts.clean_after && !acc.is_empty() {
        acc = simplify_paths(&acc, simplify_eps, true);
    }
    acc.retain(|p| p.len() >= 3 && area(p).abs() >= 1e-9);
    if acc.is_empty() || area_paths(&acc).abs() < 1e-9 {
        return Err(BoolError::EmptyResult);
    }
    report_progress(progress, cancel, 0.90)?;

    // Phase 5: Curve refit & wrap in Shape preserving primary style.
    check_cancelled(cancel)?;
    let result_poly = paths_d_to_polycurve(&acc, true, true);
    if result_poly.is_empty() || result_poly.subpaths.is_empty() {
        return Err(BoolError::EmptyResult);
    }
    report_progress(progress, cancel, 1.0)?;

    let mut out = core_doc::Shape::path(shapes[0].id, result_poly);
    out.style = shapes[0].style.clone();
    Ok(out)
}
