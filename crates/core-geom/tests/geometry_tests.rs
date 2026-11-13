use core_geom::{Affine2, BezierNode, CubicBezier, NodeType, Point2, Polycurve, Rect, KAPPA};
use proptest::prelude::*;

#[test]
fn test_point_arithmetic() {
    let p1 = Point2::new(10.0, 20.0);
    let p2 = Point2::new(5.0, 8.0);

    assert_eq!(p1 + p2, Point2::new(15.0, 28.0));
    assert_eq!(p1 - p2, Point2::new(5.0, 12.0));
    assert_eq!(p1 * 2.0, Point2::new(20.0, 40.0));
    assert_eq!(p1 / 2.0, Point2::new(5.0, 10.0));
    assert_eq!(-p1, Point2::new(-10.0, -20.0));

    assert!((p1.distance_to(p2) - (25.0 + 144.0_f64).sqrt()).abs() < 1e-10);
    assert_eq!(p1.dot(p2), 10.0 * 5.0 + 20.0 * 8.0);
}

#[test]
fn test_rect_bounds_and_contains() {
    let r = Rect::new(10.0, 20.0, 50.0, 80.0);
    assert_eq!(r.width(), 40.0);
    assert_eq!(r.height(), 60.0);
    assert_eq!(r.center(), Point2::new(30.0, 50.0));

    assert!(r.contains(Point2::new(20.0, 40.0)));
    assert!(!r.contains(Point2::new(5.0, 40.0)));

    let r2 = Rect::new(30.0, 40.0, 70.0, 90.0);
    assert!(r.intersects(&r2));

    let u = r.union(&r2);
    assert_eq!(u, Rect::new(10.0, 20.0, 70.0, 90.0));
}

#[test]
fn test_affine_transformations() {
    let t = Affine2::translate(Point2::new(10.0, 20.0));
    let p = Point2::new(5.0, 5.0);
    assert_eq!(t.transform_point(p), Point2::new(15.0, 25.0));

    let s = Affine2::scale(2.0, 3.0);
    assert_eq!(s.transform_point(p), Point2::new(10.0, 15.0));

    let r = Affine2::rotate(std::f64::consts::FRAC_PI_2);
    let p_rot = r.transform_point(Point2::new(1.0, 0.0));
    assert!((p_rot.x - 0.0).abs() < 1e-10);
    assert!((p_rot.y - 1.0).abs() < 1e-10);

    // Inverse test
    let comb = t * s * r;
    let inv = comb.inverse().expect("Invertible");
    let back = inv.transform_point(comb.transform_point(p));
    assert!((back.x - p.x).abs() < 1e-9);
    assert!((back.y - p.y).abs() < 1e-9);
}

#[test]
fn test_node_mechanics() {
    let anchor = Point2::new(10.0, 10.0);

    // Cusp node: handles are independent
    let mut cusp = BezierNode::new(anchor, NodeType::Cusp);
    cusp.set_handle_out(Some(Point2::new(15.0, 10.0)));
    cusp.set_handle_in(Some(Point2::new(8.0, 12.0)));
    assert_eq!(cusp.handle_out, Some(Point2::new(15.0, 10.0)));
    assert_eq!(cusp.handle_in, Some(Point2::new(8.0, 12.0)));

    // Symmetrical node: handles mirror each other
    let mut sym = BezierNode::new(anchor, NodeType::Symmetrical);
    sym.set_handle_out(Some(Point2::new(15.0, 10.0)));
    assert_eq!(sym.handle_in, Some(Point2::new(5.0, 10.0)));

    // Smooth node: handles collinear, independent lengths
    let mut smooth = BezierNode::new(anchor, NodeType::Smooth);
    smooth.set_handle_in(Some(Point2::new(6.0, 10.0))); // length 4.0
    smooth.set_handle_out(Some(Point2::new(18.0, 10.0))); // length 8.0, direction +X
    assert_eq!(smooth.handle_in, Some(Point2::new(6.0, 10.0))); // direction -X, length 4.0
}

#[test]
fn test_de_casteljau_split() {
    let curve = CubicBezier::new(
        Point2::new(0.0, 0.0),
        Point2::new(10.0, 40.0),
        Point2::new(40.0, 40.0),
        Point2::new(50.0, 0.0),
    );

    let t = 0.35;
    let (left, right) = curve.split(t);

    let eval_t = curve.evaluate(t);
    let eval_left_1 = left.evaluate(1.0);
    let eval_right_0 = right.evaluate(0.0);

    assert!((eval_left_1.x - eval_t.x).abs() < 1e-10);
    assert!((eval_left_1.y - eval_t.y).abs() < 1e-10);
    assert!((eval_right_0.x - eval_t.x).abs() < 1e-10);
    assert!((eval_right_0.y - eval_t.y).abs() < 1e-10);

    // Midpoint check
    let mid_eval = left.evaluate(0.5);
    let orig_half_eval = curve.evaluate(t * 0.5);
    assert!((mid_eval.x - orig_half_eval.x).abs() < 1e-10);
    assert!((mid_eval.y - orig_half_eval.y).abs() < 1e-10);
}

#[test]
fn test_arc_length_straight_line() {
    let p0 = Point2::new(10.0, 20.0);
    let p3 = Point2::new(40.0, 60.0);
    let line_curve = CubicBezier::line(p0, p3);

    let expected_len = p0.distance_to(p3);
    let computed_len = line_curve.length();

    assert!((computed_len - expected_len).abs() < 1e-8);
}

#[test]
fn test_closest_point() {
    let curve = CubicBezier::new(
        Point2::new(0.0, 0.0),
        Point2::new(10.0, 20.0),
        Point2::new(30.0, 20.0),
        Point2::new(40.0, 0.0),
    );

    // Query point known to be near apex (t ≈ 0.5)
    let apex = curve.evaluate(0.5);
    let query = Point2::new(apex.x, apex.y + 5.0);

    let (t, pt, dist) = curve.closest_point(query);
    assert!((t - 0.5).abs() < 0.05);
    assert!((dist - 5.0).abs() < 0.05);
    assert!((pt.x - apex.x).abs() < 0.1);
}

#[test]
fn test_flattening_tolerance() {
    let curve = CubicBezier::new(
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 20.0),
        Point2::new(20.0, 20.0),
        Point2::new(20.0, 0.0),
    );

    let tolerance = 0.02; // 0.02 mm laser cutting tolerance
    let polyline = curve.flatten(tolerance);

    assert!(polyline.len() > 2);
    assert_eq!(polyline.first().copied(), Some(curve.p0));
    assert_eq!(polyline.last().copied(), Some(curve.p3));

    // Verify all consecutive segments deviate from the curve by <= tolerance
    for i in 0..polyline.len() - 1 {
        let mid = polyline[i].lerp(polyline[i + 1], 0.5);
        let (_, _, d) = curve.closest_point(mid);
        assert!(d <= tolerance * 2.0); // chord deviation bounded by tolerance
    }
}

#[test]
fn test_ellipse_normalization_kappa() {
    let center = Point2::new(50.0, 50.0);
    let rx = 30.0;
    let ry = 20.0;
    let poly = Polycurve::ellipse(center, rx, ry);

    let cubics = poly.subpaths[0].to_cubics();
    assert_eq!(cubics.len(), 4);

    // Quadrant points should match exactly
    assert!((cubics[0].p0.x - 50.0).abs() < 1e-10);
    assert!((cubics[0].p0.y - (50.0 - ry)).abs() < 1e-10);

    assert!((cubics[1].p0.x - (50.0 + rx)).abs() < 1e-10);
    assert!((cubics[1].p0.y - 50.0).abs() < 1e-10);

    assert!((cubics[2].p0.x - 50.0).abs() < 1e-10);
    assert!((cubics[2].p0.y - (50.0 + ry)).abs() < 1e-10);

    assert!((cubics[3].p0.x - (50.0 - rx)).abs() < 1e-10);
    assert!((cubics[3].p0.y - 50.0).abs() < 1e-10);

    // Check KAPPA value
    assert!((KAPPA - 0.5522847498307936).abs() < 1e-14);
}

// ─── Property Tests (proptest) ───────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_split_join_continuity(
        x0 in -100.0..100.0f64, y0 in -100.0..100.0f64,
        x1 in -100.0..100.0f64, y1 in -100.0..100.0f64,
        x2 in -100.0..100.0f64, y2 in -100.0..100.0f64,
        x3 in -100.0..100.0f64, y3 in -100.0..100.0f64,
        t in 0.01..0.99f64,
    ) {
        let curve = CubicBezier::new(
            Point2::new(x0, y0),
            Point2::new(x1, y1),
            Point2::new(x2, y2),
            Point2::new(x3, y3),
        );

        let (left, right) = curve.split(t);
        let eval_orig = curve.evaluate(t);
        let eval_left_end = left.evaluate(1.0);
        let eval_right_start = right.evaluate(0.0);

        prop_assert!((eval_left_end.x - eval_orig.x).abs() < 1e-8);
        prop_assert!((eval_left_end.y - eval_orig.y).abs() < 1e-8);
        prop_assert!((eval_right_start.x - eval_orig.x).abs() < 1e-8);
        prop_assert!((eval_right_start.y - eval_orig.y).abs() < 1e-8);
    }

    #[test]
    fn prop_closest_point_is_local_minimum(
        x0 in -50.0..50.0f64, y0 in -50.0..50.0f64,
        x3 in -50.0..50.0f64, y3 in -50.0..50.0f64,
        qx in -50.0..50.0f64, qy in -50.0..50.0f64,
    ) {
        let curve = CubicBezier::line(Point2::new(x0, y0), Point2::new(x3, y3));
        let q = Point2::new(qx, qy);

        let (t, _pt, dist) = curve.closest_point(q);

        // Perturb t slightly within [0, 1]
        let delta = 0.01;
        if t + delta <= 1.0 {
            let pt_p = curve.evaluate(t + delta);
            prop_assert!(pt_p.distance_to(q) >= dist - 1e-6);
        }
        if t - delta >= 0.0 {
            let pt_m = curve.evaluate(t - delta);
            prop_assert!(pt_m.distance_to(q) >= dist - 1e-6);
        }
    }

    #[test]
    fn prop_fit_curve_preserves_endpoints(
        x0 in -100.0..100.0f64, y0 in -100.0..100.0f64,
        x1 in -100.0..100.0f64, y1 in -100.0..100.0f64,
        x2 in -100.0..100.0f64, y2 in -100.0..100.0f64,
    ) {
        let pts = vec![
            Point2::new(x0, y0),
            Point2::new(x1, y1),
            Point2::new(x2, y2),
        ];
        let sub = core_geom::fit_curve(&pts, 2.0);
        if !sub.is_empty() {
            let first = sub.nodes.first().unwrap().point;
            let last = sub.nodes.last().unwrap().point;
            prop_assert!((first.x - x0).abs() < 1e-6);
            prop_assert!((first.y - y0).abs() < 1e-6);
            prop_assert!((last.x - x2).abs() < 1e-6);
            prop_assert!((last.y - y2).abs() < 1e-6);
        }
    }
}

#[test]
fn test_curve_fit_sine_wave() {
    let mut raw_pts = Vec::new();
    for i in 0..40 {
        let x = (i as f64) * 5.0;
        let y = (x * 0.05).sin() * 30.0;
        raw_pts.push(Point2::new(x, y));
    }

    let tolerance = 2.0;
    let subpath = core_geom::fit_curve(&raw_pts, tolerance);
    assert!(!subpath.is_empty());
    assert!(subpath.nodes.len() >= 2);

    let cubics = subpath.to_cubics();
    assert!(!cubics.is_empty());

    // Check that endpoints match exactly
    assert_eq!(subpath.nodes[0].point, raw_pts[0]);
    assert_eq!(
        subpath.nodes.last().unwrap().point,
        *raw_pts.last().unwrap()
    );
}

#[test]
fn test_subpath_split_segment() {
    let mut sub = core_geom::Subpath::new(false);
    sub.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(100.0, 100.0), NodeType::Cusp));

    let inserted = sub.split_segment(0, 0.5);
    assert_eq!(inserted, Some(1));
    assert_eq!(sub.nodes.len(), 3);

    let mid = sub.nodes[1].point;
    assert!((mid.x - 50.0).abs() < 1e-6);
    assert!((mid.y - 50.0).abs() < 1e-6);
}

#[test]
fn test_polygon_and_star_and_spiral() {
    let center = Point2::new(50.0, 50.0);

    // Pentagon
    let poly = Polycurve::polygon(center, 40.0, 5);
    assert_eq!(poly.subpaths.len(), 1);
    assert_eq!(poly.subpaths[0].nodes.len(), 5);
    assert!(poly.subpaths[0].closed);
    let b_poly = poly.bounds().expect("Bounds exist");
    assert!(b_poly.min_x >= 10.0 && b_poly.max_x <= 90.0);

    // 5-Point Star
    let star = Polycurve::star(center, 40.0, 20.0, 5);
    assert_eq!(star.subpaths.len(), 1);
    assert_eq!(star.subpaths[0].nodes.len(), 10);
    assert!(star.subpaths[0].closed);

    // Spiral
    let spiral = Polycurve::spiral(center, 3.0, 60.0);
    assert_eq!(spiral.subpaths.len(), 1);
    assert!(!spiral.subpaths[0].closed);
    assert!(spiral.length() > 60.0);
}

#[test]
fn test_affine_singular_and_nan_handling() {
    // 1. Singular matrix (det = 0)
    let singular = Affine2::new(2.0, 4.0, 1.0, 2.0, 10.0, 20.0);
    assert_eq!(singular.determinant(), 0.0);
    assert!(singular.inverse().is_none());

    // 2. Matrix with NaN determinant
    let nan_mat = Affine2::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0);
    assert!(!nan_mat.is_finite());
    assert!(nan_mat.inverse().is_none());

    // 3. Matrix with Infinite translation
    let inf_mat = Affine2::new(1.0, 0.0, 0.0, 1.0, f64::INFINITY, 0.0);
    assert!(!inf_mat.is_finite());
    assert!(inf_mat.inverse().is_none());

    // 4. Skew and non-uniform scaling
    let skew = Affine2::skew(0.2, 0.1);
    assert!(skew.is_finite());
    let inv_skew = skew.inverse().expect("Invertible skew");
    let pt = Point2::new(15.0, 25.0);
    let roundtrip = inv_skew.transform_point(skew.transform_point(pt));
    assert!((roundtrip.x - pt.x).abs() < 1e-10);
    assert!((roundtrip.y - pt.y).abs() < 1e-10);
}

#[test]
fn test_bezier_closest_point_difficult_cases() {
    // S-curve with inflection and multiple stationary points
    let s_curve = CubicBezier::new(
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 50.0),
        Point2::new(100.0, -50.0),
        Point2::new(100.0, 0.0),
    );

    // Query point near the first loop
    let q1 = Point2::new(10.0, 40.0);
    let (t1, pt1, dist1) = s_curve.closest_point(q1);
    assert!(t1 > 0.0 && t1 < 0.5);
    let direct_d1 = pt1.distance_to(q1);
    assert!((dist1 - direct_d1).abs() < 1e-10);

    // Check that endpoints t=0 and t=1 are properly compared
    let q_start = Point2::new(-10.0, -10.0);
    let (t_start, pt_start, dist_start) = s_curve.closest_point(q_start);
    assert_eq!(t_start, 0.0);
    assert_eq!(pt_start, s_curve.p0);
    assert!((dist_start - s_curve.p0.distance_to(q_start)).abs() < 1e-10);
}

#[test]
fn test_flatten_collinear_overshoot_safeguard() {
    // Collinear Bézier where p1 and p2 shoot out in opposite directions along chord line:
    // Infinite line cross-product is 0, but the curve loops out to -10 and +30!
    let overshoot = CubicBezier::new(
        Point2::new(0.0, 0.0),
        Point2::new(30.0, 0.0),
        Point2::new(-20.0, 0.0),
        Point2::new(10.0, 0.0),
    );

    let tolerance = 0.02;
    let polyline = overshoot.flatten(tolerance);

    // Under the old bug, polyline was only 2 points! With the fix, it must subdivide:
    assert!(
        polyline.len() > 2,
        "Collinear overshooting curve must subdivide and not falsely report flat"
    );

    // Verify all points remain bounded and start/end match
    assert_eq!(polyline.first().copied(), Some(overshoot.p0));
    assert_eq!(polyline.last().copied(), Some(overshoot.p3));
}

#[test]
fn test_rectangle_chamfer_and_scallop() {
    let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
    let r = 10.0;

    // Chamfer
    let chamfer = Polycurve::rectangle(rect, r, core_geom::CornerStyle::Chamfer);
    assert_eq!(chamfer.subpaths.len(), 1);
    assert_eq!(chamfer.subpaths[0].nodes.len(), 8);
    for node in &chamfer.subpaths[0].nodes {
        assert_eq!(node.node_type, NodeType::Cusp);
        assert!(node.handle_in.is_none());
        assert!(node.handle_out.is_none());
    }

    // Scallop (inverted rounded corners)
    let scallop = Polycurve::rectangle(rect, r, core_geom::CornerStyle::Scallop);
    assert_eq!(scallop.subpaths.len(), 1);
    assert_eq!(scallop.subpaths[0].nodes.len(), 8);
    for node in &scallop.subpaths[0].nodes {
        assert_eq!(node.node_type, NodeType::Smooth);
    }
}

#[test]
fn test_curve_fit_sharp_corner_preservation() {
    // L-shape with a sharp 90-degree corner at (50, 0)
    let mut l_shape = Vec::new();
    for x in (0..=50).step_by(5) {
        l_shape.push(Point2::new(x as f64, 0.0));
    }
    for y in (5..=50).step_by(5) {
        l_shape.push(Point2::new(50.0, y as f64));
    }

    let subpath = core_geom::fit_curve(&l_shape, 1.0);
    assert!(!subpath.is_empty());

    // The intermediate node near (50, 0) should be classified as Cusp due to 90° deflection
    let has_cusp = subpath
        .nodes
        .iter()
        .skip(1)
        .take(subpath.nodes.len().saturating_sub(2))
        .any(|n| n.node_type == NodeType::Cusp);
    assert!(
        has_cusp,
        "Sharp 90-degree corner must be preserved with a Cusp node"
    );
}
