//! Property-based fuzz tests for `core-boolean` (`boolean_with_options`).

use core_boolean::convert::polygon_area;
use core_boolean::{boolean_with_options, BoolError, BoolOp, BoolOptions};
use core_doc::{Shape, ShapeId};
use core_geom::{BezierNode, NodeType, Point2, Polycurve, Rect, Subpath};
use proptest::prelude::*;

fn polygon_shape(id: u64, pts: &[Point2]) -> Shape {
    let mut sub = Subpath::new(true);
    for &p in pts {
        sub.push_node(BezierNode::new(p, NodeType::Cusp));
    }
    Shape::path(ShapeId(id), Polycurve::from_subpath(sub))
}

fn shape_unsigned_area(shape: &Shape) -> f64 {
    let poly = shape.to_polycurve().transform(&shape.transform);
    let signed: f64 = poly
        .subpaths
        .iter()
        .filter(|s| s.closed)
        .map(|s| polygon_area(&s.flatten(0.01)))
        .sum();
    signed.abs()
}

fn clean_ring(pts: &[Point2]) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::with_capacity(pts.len());
    for &p in pts {
        if let Some(&last) = out.last() {
            if p.distance(last) < 1e-9 {
                continue;
            }
        }
        out.push(p);
    }
    if out.len() >= 2 && out.first().unwrap().distance(*out.last().unwrap()) < 1e-9 {
        out.pop();
    }
    out
}

fn orient(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Returns `true` if open/interior parts of segments `(a, b)` and `(c, d)` properly intersect
/// or overlap collinearly. Shared endpoints on adjacent segments are handled by callers.
fn segments_properly_intersect(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let eps = 1e-7;
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);

    // Strict crossing: endpoints lie strictly on opposite sides of each other's line.
    if ((o1 > eps && o2 < -eps) || (o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps) || (o3 < -eps && o4 > eps))
    {
        return true;
    }

    // Collinear overlap check (strictly overlapping interior).
    if o1.abs() <= eps && o2.abs() <= eps && o3.abs() <= eps && o4.abs() <= eps {
        let ab_len_sq = a.distance_squared_to(b);
        if ab_len_sq <= eps * eps {
            return false;
        }
        let ab = b - a;
        let t0 = (c - a).dot(ab) / ab_len_sq;
        let t1 = (d - a).dot(ab) / ab_len_sq;
        let lo = t0.min(t1);
        let hi = t0.max(t1);
        // Overlap length > eps along normalized [0, 1]
        if hi.min(1.0) - lo.max(0.0) > 1e-5 {
            return true;
        }
    }

    false
}

fn contour_self_intersects(ring: &[Point2]) -> bool {
    let n = ring.len();
    if n < 4 {
        return false;
    }
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        for j in (i + 1)..n {
            // Skip identical or adjacent edges (including closing wrap-around (0, n-1))
            if j == i || j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let c = ring[j];
            let d = ring[(j + 1) % n];
            if segments_properly_intersect(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

fn arb_polygon_points() -> impl Strategy<Value = Vec<Point2>> {
    let random_poly =
        prop::collection::vec((-50.0..50.0f64, -50.0..50.0f64), 3..=7).prop_map(|coords| {
            coords
                .into_iter()
                .map(|(x, y)| {
                    // Quantize to 0.01 mm to avoid noisy near-zero slivers while still exercising general angles
                    Point2::new((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0)
                })
                .collect()
        });
    // Explicit bow-tie self-intersecting quad generator
    let bowtie_poly = (
        -30.0..30.0f64,
        -30.0..30.0f64,
        10.0..40.0f64,
        10.0..40.0f64,
        5.0..25.0f64,
    )
        .prop_map(|(ox, oy, w, h, asym)| {
            let q = |v: f64| (v * 100.0).round() / 100.0;
            vec![
                Point2::new(q(ox), q(oy)),
                Point2::new(q(ox + w), q(oy + h)),
                Point2::new(q(ox + w), q(oy)),
                Point2::new(q(ox), q(oy + asym)),
            ]
        });
    prop_oneof![
        3 => random_poly,
        1 => bowtie_poly,
    ]
}

fn arb_bool_op() -> impl Strategy<Value = BoolOp> {
    prop::sample::select(vec![
        BoolOp::Union,
        BoolOp::Difference,
        BoolOp::Intersection,
    ])
}

fn eval_area_or_empty(
    shapes: &[Shape],
    op: BoolOp,
    opts: &BoolOptions,
) -> Result<f64, TestCaseError> {
    match boolean_with_options(shapes, op, opts, None, None) {
        Ok(s) => Ok(shape_unsigned_area(&s)),
        Err(BoolError::EmptyResult) => Ok(0.0),
        Err(other) => Err(TestCaseError::fail(format!(
            "Unexpected {op:?} error: {other:?}"
        ))),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_boolean_result_never_self_intersects(
        pts_a in arb_polygon_points(),
        pts_b in arb_polygon_points(),
        op in arb_bool_op(),
    ) {
        let clean_a = clean_ring(&pts_a);
        let clean_b = clean_ring(&pts_b);
        let simple_a = clean_a.len() >= 3 && !contour_self_intersects(&clean_a);
        let simple_b = clean_b.len() >= 3 && !contour_self_intersects(&clean_b);

        let shape_a = polygon_shape(1, &pts_a);
        let shape_b = polygon_shape(2, &pts_b);
        let area_a = shape_unsigned_area(&shape_a);
        let area_b = shape_unsigned_area(&shape_b);

        let opts = BoolOptions::default();
        let res = boolean_with_options(&[shape_a, shape_b], op, &opts, None, None);

        match res {
            Ok(merged) => {
                let poly = merged.to_polycurve();
                prop_assert!(!poly.subpaths.is_empty());

                let mut total_abs_area = 0.0;
                for sub in &poly.subpaths {
                    prop_assert!(sub.closed, "boolean output contour must be closed");
                    let flattened = sub.flatten(0.01);
                    let ring = clean_ring(&flattened);
                    prop_assert!(ring.len() >= 3, "resulting contour must have >= 3 distinct vertices");
                    let area = polygon_area(&ring).abs();
                    prop_assert!(area > 1e-8, "resulting contour must have non-zero area, got {area}");
                    total_abs_area += area;

                    prop_assert!(
                        !contour_self_intersects(&ring),
                        "boolean result contour self-intersects for op={op:?}: ring={ring:?}"
                    );
                }

                let res_area = shape_unsigned_area(&merged);
                prop_assert!(total_abs_area > 0.0 && res_area > 0.0);

                if op == BoolOp::Union {
                    if simple_a && simple_b && area_a > 1e-4 && area_b > 1e-4 {
                        prop_assert!(
                            res_area + 0.5 >= area_a.max(area_b),
                            "Union area ({res_area}) unexpectedly smaller than max(A={area_a}, B={area_b})"
                        );
                    } else {
                        prop_assert!(
                            res_area > 0.0,
                            "Post-heal union of self-intersecting inputs must have positive area"
                        );
                    }
                }
            }
            Err(err) => {
                prop_assert!(
                    matches!(
                        err,
                        BoolError::EmptyResult
                            | BoolError::DegenerateInput(_)
                            | BoolError::SelfIntersection(_)
                            | BoolError::Cancelled
                    ),
                    "Unexpected BoolError variant: {err:?}"
                );
            }
        }
    }

    #[test]
    fn prop_boolean_difference_area_invariant(
        ax_q in -100i32..100i32,
        ay_q in -100i32..100i32,
        aw_q in 2i32..120i32,
        ah_q in 2i32..120i32,
        bx_q in -100i32..100i32,
        by_q in -100i32..100i32,
        bw_q in 2i32..120i32,
        bh_q in 2i32..120i32,
    ) {
        // Use quarter-millimetre integer grid so Clipper2's 1e-4 mm fixed-point scaling
        // and axis-aligned rectangle flattening introduce zero discretization error.
        let ax = ax_q as f64 * 0.25;
        let ay = ay_q as f64 * 0.25;
        let aw = aw_q as f64 * 0.25;
        let ah = ah_q as f64 * 0.25;
        let bx = bx_q as f64 * 0.25;
        let by = by_q as f64 * 0.25;
        let bw = bw_q as f64 * 0.25;
        let bh = bh_q as f64 * 0.25;

        let shape_a = Shape::rectangle(ShapeId(1), Rect::new(ax, ay, ax + aw, ay + ah), 0.0);
        let shape_b = Shape::rectangle(ShapeId(2), Rect::new(bx, by, bx + bw, by + bh), 0.0);
        let area_a = aw * ah;
        let opts = BoolOptions::default();

        let area_diff = eval_area_or_empty(
            &[shape_a.clone(), shape_b.clone()],
            BoolOp::Difference,
            &opts,
        )?;
        let area_inter = eval_area_or_empty(&[shape_a, shape_b], BoolOp::Intersection, &opts)?;

        prop_assert!(
            ((area_diff + area_inter) - area_a).abs() <= 1e-6,
            "Exact rect invariant violated: diff({area_diff}) + inter({area_inter}) != A({area_a})"
        );
    }

    #[test]
    fn prop_boolean_difference_area_invariant_general_polygons(
        ax in -40.0..40.0f64,
        ay in -40.0..40.0f64,
        r_a in 10.0..35.0f64,
        rot_a in 0.0..std::f64::consts::TAU,
        bx in -40.0..40.0f64,
        by in -40.0..40.0f64,
        r_b in 10.0..35.0f64,
        rot_b in 0.0..std::f64::consts::TAU,
    ) {
        // Simple convex/star-free 5-gons so curve-refit (triggered at >6 pts) does not distort corners.
        let make_pentagon = |cx: f64, cy: f64, r: f64, rot: f64| -> Vec<Point2> {
            (0..5)
                .map(|i| {
                    let ang = rot + (i as f64) * std::f64::consts::TAU / 5.0;
                    Point2::new(
                        ((cx + r * ang.cos()) * 10_000.0).round() / 10_000.0,
                        ((cy + r * ang.sin()) * 10_000.0).round() / 10_000.0,
                    )
                })
                .collect()
        };

        let shape_a = polygon_shape(1, &make_pentagon(ax, ay, r_a, rot_a));
        let shape_b = polygon_shape(2, &make_pentagon(bx, by, r_b, rot_b));
        let area_a = shape_unsigned_area(&shape_a);
        let opts = BoolOptions {
            clean_after: false,
            tolerance: 1e-4,
        };

        let area_diff = eval_area_or_empty(
            &[shape_a.clone(), shape_b.clone()],
            BoolOp::Difference,
            &opts,
        )?;
        let area_inter = eval_area_or_empty(&[shape_a, shape_b], BoolOp::Intersection, &opts)?;

        // Note: when output contours exceed 6 vertices, `boolean_with_options` runs Schneider curve
        // re-fitting with DEFAULT_REFIT_TOLERANCE=0.05 mm; when polygonal (<=6 vertices), error is <=1e-4.
        let tol = 1.0_f64.max(area_a * 1e-3);
        prop_assert!(
            ((area_diff + area_inter) - area_a).abs() <= tol,
            "Polygon invariant violated: diff({area_diff}) + inter({area_inter}) vs A({area_a}), tol={tol}"
        );
    }
}
