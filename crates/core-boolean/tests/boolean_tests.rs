//! Unit and property tests for VDraw core-boolean operations:
//! Weld, Trim, Intersect, Exclude, Simplify, Knife, Eraser, Offset, Virtual Segment Delete, Text to Outlines.

use core_boolean::command::apply_boolean_to_document;
use core_boolean::convert::polygon_area;
use core_boolean::error::BooleanError;
use core_boolean::{
    boolean_with_options, default_text_to_polycurve, delete_virtual_segment, erase_shape,
    execute_boolean, find_virtual_segments, offset_shape, slice_shape, BoolCancelToken, BoolError,
    BoolOp, BoolOptions, BooleanOp, ContourJoin, TextOptions,
};
use core_doc::color::Color;
use core_doc::command::UndoStack;
use core_doc::document::Document;
use core_doc::shape::{Shape, ShapeId};
use core_doc::style::{Stroke, Style};
use core_geom::node::{BezierNode, NodeType};
use core_geom::polycurve::{CornerStyle, Subpath};
use core_geom::rect::Rect;
use core_geom::{Affine2, Point2, Polycurve};
use proptest::prelude::*;
use std::cell::RefCell;

fn make_rect_poly(x: f64, y: f64, w: f64, h: f64) -> Polycurve {
    let r = Rect::new(x, y, x + w, y + h);
    Polycurve::rectangle(r, 0.0, CornerStyle::Round)
}

#[test]
fn test_weld_overlapping_rectangles() {
    let r1 = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let r2 = make_rect_poly(50.0, 0.0, 100.0, 100.0);

    let res = execute_boolean(BooleanOp::Weld, &[r1, r2], false).expect("Weld failed");
    assert_eq!(res.len(), 1);

    let bounds = res[0].bounds().expect("Valid bounds");
    assert!((bounds.min().x - 0.0).abs() < 1e-2);
    assert!((bounds.max().x - 150.0).abs() < 1e-2);
    assert!((bounds.min().y - 0.0).abs() < 1e-2);
    assert!((bounds.max().y - 100.0).abs() < 1e-2);
}

#[test]
fn test_trim_difference() {
    let big = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let notch = make_rect_poly(50.0, 0.0, 60.0, 50.0);

    let res = execute_boolean(BooleanOp::Trim, &[big, notch], false).expect("Trim failed");
    assert_eq!(res.len(), 1);

    let bounds = res[0].bounds().expect("Valid bounds");
    assert!((bounds.min().x - 0.0).abs() < 1e-2);
    assert!((bounds.max().x - 100.0).abs() < 1e-2);
}

#[test]
fn test_intersect_shapes() {
    let r1 = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let r2 = make_rect_poly(50.0, 50.0, 100.0, 100.0);

    let res = execute_boolean(BooleanOp::Intersect, &[r1, r2], false).expect("Intersect failed");
    assert_eq!(res.len(), 1);

    let bounds = res[0].bounds().expect("Valid bounds");
    assert!((bounds.min().x - 50.0).abs() < 1e-2);
    assert!((bounds.max().x - 100.0).abs() < 1e-2);
    assert!((bounds.min().y - 50.0).abs() < 1e-2);
    assert!((bounds.max().y - 100.0).abs() < 1e-2);
}

#[test]
fn test_exclude_xor() {
    let r1 = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let r2 = make_rect_poly(50.0, 0.0, 100.0, 100.0);

    let res = execute_boolean(BooleanOp::Exclude, &[r1, r2], false).expect("Exclude failed");
    assert_eq!(res.len(), 1);
    // XOR produces 2 distinct subpath islands
    assert_eq!(res[0].subpaths.len(), 2);
}

#[test]
fn test_create_boundary() {
    let r1 = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let r2 = make_rect_poly(50.0, 50.0, 100.0, 100.0);

    let res = execute_boolean(BooleanOp::CreateBoundary, &[r1, r2], false)
        .expect("CreateBoundary failed");
    assert_eq!(res.len(), 1);
    assert!(!res[0].subpaths.is_empty());
}

#[test]
fn test_knife_slice_circle() {
    let circle = Polycurve::ellipse(Point2::new(100.0, 100.0), 50.0, 50.0);
    let cut_line = [Point2::new(30.0, 100.0), Point2::new(170.0, 100.0)];

    let pieces = slice_shape(&circle, &cut_line, true).expect("Knife slice failed");
    assert_eq!(pieces.len(), 2, "Expected circle to be cut into 2 pieces");

    for p in &pieces {
        assert!(p.subpaths[0].closed, "Expected cut piece to be closed");
    }
}

#[test]
fn test_knife_slice_open_path() {
    let p0 = Point2::new(0.0, 50.0);
    let p1 = Point2::new(100.0, 50.0);
    let line = Polycurve::line(p0, p1);

    let cut_line = [Point2::new(50.0, 0.0), Point2::new(50.0, 100.0)];

    let pieces = slice_shape(&line, &cut_line, false).expect("Knife open path slice failed");
    assert_eq!(
        pieces.len(),
        2,
        "Expected open line to be split into 2 lines"
    );
}

#[test]
fn test_eraser_subtraction() {
    let r = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    // Erase a vertical stripe through center
    let eraser_stroke = [Point2::new(50.0, -10.0), Point2::new(50.0, 110.0)];
    let radius = 5.0; // 10mm wide cut

    let erased = erase_shape(&r, &eraser_stroke, radius).expect("Eraser failed");
    assert_eq!(
        erased.subpaths.len(),
        2,
        "Expected rect to be split into 2 pieces"
    );
}

#[test]
fn test_virtual_segment_detection_and_delete() {
    let line1 = Polycurve::line(Point2::new(0.0, 50.0), Point2::new(100.0, 50.0));
    let line2 = Polycurve::line(Point2::new(50.0, 0.0), Point2::new(50.0, 100.0));

    let segments = find_virtual_segments(&line1, std::slice::from_ref(&line2));
    assert_eq!(
        segments.len(),
        2,
        "Cross intersection should form 2 virtual segments on line1"
    );

    // Click near left half
    let click = Point2::new(25.0, 50.0);
    let trimmed = delete_virtual_segment(&line1, &[line2], click, 5.0)
        .expect("Delete virtual segment failed");

    assert_eq!(trimmed.subpaths.len(), 1);
    // Remaining half should be from (50, 50) to (100, 50)
    let b = trimmed.bounds().unwrap();
    assert!((b.min().x - 50.0).abs() < 1.0);
    assert!((b.max().x - 100.0).abs() < 1.0);
}

#[test]
fn test_offset_outward_and_inward() {
    let r = make_rect_poly(10.0, 10.0, 80.0, 80.0);

    // Outward by 5 mm
    let expanded = offset_shape(&r, 5.0, ContourJoin::Round).expect("Offset outward failed");
    let eb = expanded.bounds().unwrap();
    assert!((eb.min().x - 5.0).abs() < 1e-1);
    assert!((eb.max().x - 95.0).abs() < 1e-1);

    // Inward by 5 mm
    let inset = offset_shape(&r, -5.0, ContourJoin::Miter(2.0)).expect("Offset inward failed");
    let ib = inset.bounds().unwrap();
    assert!((ib.min().x - 15.0).abs() < 1e-1);
    assert!((ib.max().x - 85.0).abs() < 1e-1);
}

#[test]
fn test_text_to_polycurve_bundled_font() {
    let opts = TextOptions {
        font_size_mm: 25.0,
        letter_spacing_mm: 1.0,
        origin: Point2::new(10.0, 50.0),
    };

    let text_poly = default_text_to_polycurve("VDRAW", &opts).expect("Text to curves failed");
    assert!(!text_poly.subpaths.is_empty(), "Expected glyph contours");

    let b = text_poly.bounds().expect("Valid text bounds");
    assert!(b.width() > 20.0, "Expected non-zero text width");
    assert!(b.height() > 10.0, "Expected non-zero text height");
}

#[test]
fn test_front_minus_back_and_back_minus_front() {
    let r_back = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let r_front = make_rect_poly(50.0, 0.0, 100.0, 100.0);

    // Front-Minus-Back: front (50..150) minus back (0..100) = (100..150)
    let fmb = execute_boolean(
        BooleanOp::FrontMinusBack,
        &[r_back.clone(), r_front.clone()],
        false,
    )
    .expect("FrontMinusBack failed");
    assert_eq!(fmb.len(), 1);
    let b_fmb = fmb[0].bounds().expect("Valid bounds");
    assert!(
        (b_fmb.min().x - 100.0).abs() < 1e-1,
        "Expected min_x near 100, got {}",
        b_fmb.min().x
    );
    assert!(
        (b_fmb.max().x - 150.0).abs() < 1e-1,
        "Expected max_x near 150, got {}",
        b_fmb.max().x
    );

    // Back-Minus-Front: back (0..100) minus front (50..150) = (0..50)
    let bmf = execute_boolean(BooleanOp::BackMinusFront, &[r_back, r_front], false)
        .expect("BackMinusFront failed");
    assert_eq!(bmf.len(), 1);
    let b_bmf = bmf[0].bounds().expect("Valid bounds");
    assert!(
        (b_bmf.min().x - 0.0).abs() < 1e-1,
        "Expected min_x near 0, got {}",
        b_bmf.min().x
    );
    assert!(
        (b_bmf.max().x - 50.0).abs() < 1e-1,
        "Expected max_x near 50, got {}",
        b_bmf.max().x
    );
}

#[test]
fn test_coincident_edges_weld() {
    let r1 = make_rect_poly(0.0, 0.0, 50.0, 100.0);
    let r2 = make_rect_poly(50.0, 0.0, 50.0, 100.0);

    let res =
        execute_boolean(BooleanOp::Weld, &[r1, r2], false).expect("Coincident edge weld failed");
    assert_eq!(res.len(), 1);
    let b = res[0].bounds().expect("Valid bounds");
    assert!((b.min().x - 0.0).abs() < 1e-2);
    assert!((b.max().x - 100.0).abs() < 1e-2);
    assert!((b.width() - 100.0).abs() < 1e-2);
}

#[test]
fn test_nested_polygons_boolean() {
    let outer = make_rect_poly(0.0, 0.0, 100.0, 100.0);
    let inner = make_rect_poly(25.0, 25.0, 50.0, 50.0);

    // Trim creates a donut (outer with inner cut out)
    let donut = execute_boolean(BooleanOp::Trim, &[outer.clone(), inner.clone()], false)
        .expect("Donut trim failed");
    assert_eq!(donut.len(), 1);
    assert_eq!(
        donut[0].subpaths.len(),
        2,
        "Expected outer and hole subpath"
    );

    // Intersect yields the inner shape exactly
    let inter = execute_boolean(BooleanOp::Intersect, &[outer, inner], false)
        .expect("Nested intersect failed");
    assert_eq!(inter.len(), 1);
    let ib = inter[0].bounds().expect("Valid bounds");
    assert!((ib.min().x - 25.0).abs() < 1e-1);
    assert!((ib.max().x - 75.0).abs() < 1e-1);
}

#[test]
fn test_coordinate_out_of_range_rejected() {
    let mut poly = Polycurve::new();
    let mut sub = Subpath::new(true);
    sub.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(150_000.0, 0.0), NodeType::Cusp)); // > 100,000 mm
    sub.push_node(BezierNode::new(
        Point2::new(150_000.0, 100.0),
        NodeType::Cusp,
    ));
    poly.subpaths.push(sub);

    let r = make_rect_poly(0.0, 0.0, 10.0, 10.0);
    let err = execute_boolean(BooleanOp::Weld, &[poly, r], false).unwrap_err();
    assert!(matches!(err, BooleanError::CoordinateOutOfRange(_)));
}

#[test]
fn test_non_finite_coordinate_rejected() {
    let mut poly = Polycurve::new();
    let mut sub = Subpath::new(true);
    sub.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(f64::NAN, 0.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(10.0, 10.0), NodeType::Cusp));
    poly.subpaths.push(sub);

    let r = make_rect_poly(0.0, 0.0, 10.0, 10.0);
    let err = execute_boolean(BooleanOp::Weld, &[poly, r], false).unwrap_err();
    assert!(matches!(err, BooleanError::NonFiniteCoordinate));
}

#[test]
fn test_empty_intersection_result() {
    let r1 = make_rect_poly(0.0, 0.0, 10.0, 10.0);
    let r2 = make_rect_poly(100.0, 100.0, 10.0, 10.0);

    let res =
        execute_boolean(BooleanOp::Intersect, &[r1, r2], false).expect("Disjoint intersect ok");
    assert_eq!(res.len(), 1);
    assert!(
        res[0].subpaths.is_empty(),
        "Expected empty geometry for disjoint intersection"
    );
}

#[test]
fn test_repeated_boolean_operations() {
    let mut current = make_rect_poly(0.0, 0.0, 20.0, 20.0);
    for i in 1..6 {
        let step = make_rect_poly(i as f64 * 10.0, 0.0, 20.0, 20.0);
        let res = execute_boolean(BooleanOp::Weld, &[current, step], false).expect("Repeated weld");
        assert_eq!(res.len(), 1);
        current = res.into_iter().next().unwrap();
    }
    let b = current.bounds().expect("Valid bounds after 5 welds");
    assert!((b.min().x - 0.0).abs() < 1e-1);
    assert!((b.max().x - 70.0).abs() < 1e-1);
}

#[test]
fn test_boolean_failure_document_integrity_rollback() {
    let mut doc = Document::new("Test Doc");
    let mut undo = UndoStack::new();

    let s1 = Shape::path(
        core_doc::shape::ShapeId(1),
        make_rect_poly(0.0, 0.0, 10.0, 10.0),
    );
    let mut invalid_poly = Polycurve::new();
    let mut sub = Subpath::new(true);
    sub.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(f64::NAN, 0.0), NodeType::Cusp));
    invalid_poly.subpaths.push(sub);
    let s2 = Shape::path(core_doc::shape::ShapeId(2), invalid_poly);

    doc.add_shape_to_active_layer(s1);
    doc.add_shape_to_active_layer(s2);

    doc.selection.add(core_doc::shape::ShapeId(1));
    doc.selection.add(core_doc::shape::ShapeId(2));

    let shape_count_before = doc.active_page().unwrap().layers[0].shapes.len();
    assert_eq!(shape_count_before, 2);

    // Apply boolean with invalid NaN geometry -> should fail
    let res = apply_boolean_to_document(&mut doc, &mut undo, BooleanOp::Weld, false);
    assert!(res.is_err(), "Expected failure on invalid geometry");

    // Verify document was untouched
    let shape_count_after = doc.active_page().unwrap().layers[0].shapes.len();
    assert_eq!(
        shape_count_after, 2,
        "Document must not be modified on failure"
    );
    assert_eq!(
        doc.selection.len(),
        2,
        "Selection must be preserved on failure"
    );
}

// ── Property Test: Idempotent Weld on identical shape ──────────────────────
proptest! {
    #[test]
    fn prop_weld_identical_shape_area_invariant(
        x in 0.0..500.0f64,
        y in 0.0..500.0f64,
        w in 10.0..100.0f64,
        h in 10.0..100.0f64,
    ) {
        let r1 = make_rect_poly(x, y, w, h);
        let r2 = make_rect_poly(x, y, w, h);

        let res = execute_boolean(BooleanOp::Weld, &[r1, r2], false).unwrap();
        let b = res[0].bounds().unwrap();

        prop_assert!((b.width() - w).abs() < 1e-2);
        prop_assert!((b.height() - h).abs() < 1e-2);
    }
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

proptest! {
    #[test]
    fn prop_boolean_with_options_area_invariants(
        ax in 0.0..50.0f64,
        ay in 0.0..50.0f64,
        aw in 30.0..80.0f64,
        ah in 30.0..80.0f64,
        dx in 5.0..25.0f64,
        dy in 5.0..25.0f64,
        bw in 30.0..80.0f64,
        bh in 30.0..80.0f64,
    ) {
        let s1 = Shape::rectangle(ShapeId(1), Rect::new(ax, ay, ax + aw, ay + ah), 0.0);
        let s2 = Shape::rectangle(
            ShapeId(2),
            Rect::new(ax + dx, ay + dy, ax + dx + bw, ay + dy + bh),
            0.0,
        );
        let area_a = shape_unsigned_area(&s1);
        let area_b = shape_unsigned_area(&s2);
        let opts = BoolOptions::default();

        let union_shape = boolean_with_options(
            &[s1.clone(), s2.clone()],
            BoolOp::Union,
            &opts,
            None,
            None,
        )
        .expect("Union should succeed");
        let area_u = shape_unsigned_area(&union_shape);
        prop_assert!(area_u <= area_a + area_b + 1e-2);
        prop_assert!(area_u + 1e-2 >= area_a.max(area_b));

        let inter_shape = boolean_with_options(
            &[s1.clone(), s2.clone()],
            BoolOp::Intersection,
            &opts,
            None,
            None,
        )
        .expect("Overlapping intersection should succeed");
        let area_i = shape_unsigned_area(&inter_shape);
        prop_assert!(area_i <= area_a.min(area_b) + 1e-2);

        // Inclusion-exclusion invariant: area(A U B) + area(A ∩ B) == area(A) + area(B)
        prop_assert!(((area_u + area_i) - (area_a + area_b)).abs() < 1e-1);

        if let Ok(diff_shape) = boolean_with_options(
            &[s1, s2],
            BoolOp::Difference,
            &opts,
            None,
            None,
        ) {
            let area_d = shape_unsigned_area(&diff_shape);
            prop_assert!(area_d <= area_a + 1e-2);
        }
    }
}

#[test]
fn test_boolean_with_options_coincident_union_and_style() {
    let mut s1 = Shape::rectangle(ShapeId(10), Rect::new(0.0, 0.0, 50.0, 100.0), 0.0);
    s1.style = Style {
        fill: Some(Color::RED),
        stroke: Some(Stroke::default()),
    };
    let s2 = Shape::rectangle(ShapeId(11), Rect::new(50.0, 0.0, 100.0, 100.0), 0.0);

    let res = boolean_with_options(
        &[s1.clone(), s2],
        BoolOp::Union,
        &BoolOptions::default(),
        None,
        None,
    )
    .expect("Coincident-edge union should succeed");

    assert_eq!(res.style, s1.style);
    let b = res.world_bounds().expect("Valid bounds");
    assert!((b.min_x - 0.0).abs() < 1e-3);
    assert!((b.max_x - 100.0).abs() < 1e-3);
    assert!((b.min_y - 0.0).abs() < 1e-3);
    assert!((b.max_y - 100.0).abs() < 1e-3);
    assert!((shape_unsigned_area(&res) - 10_000.0).abs() < 1e-1);
}

#[test]
fn test_boolean_with_options_empty_result_and_validation() {
    let s1 = Shape::rectangle(ShapeId(1), Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
    let s2 = Shape::rectangle(ShapeId(2), Rect::new(50.0, 50.0, 60.0, 60.0), 0.0);
    let opts = BoolOptions::default();

    // Disjoint intersection -> EmptyResult
    let err_inter =
        boolean_with_options(&[s1.clone(), s2], BoolOp::Intersection, &opts, None, None)
            .unwrap_err();
    assert_eq!(err_inter, BoolError::EmptyResult);

    // Total subtraction -> EmptyResult
    let cover = Shape::rectangle(ShapeId(3), Rect::new(-10.0, -10.0, 20.0, 20.0), 0.0);
    let err_diff =
        boolean_with_options(&[s1.clone(), cover], BoolOp::Difference, &opts, None, None)
            .unwrap_err();
    assert_eq!(err_diff, BoolError::EmptyResult);

    // Insufficient shapes -> EmptyInput
    let err_one = boolean_with_options(std::slice::from_ref(&s1), BoolOp::Union, &opts, None, None)
        .unwrap_err();
    assert!(matches!(err_one, BoolError::EmptyInput(_)));

    // Open line -> EmptyInput
    let line = Shape::line(ShapeId(4), Point2::new(0.0, 0.0), Point2::new(10.0, 10.0));
    let err_open = boolean_with_options(&[s1, line], BoolOp::Union, &opts, None, None).unwrap_err();
    assert!(matches!(err_open, BoolError::EmptyInput(_)));
}

#[test]
fn test_boolean_with_options_cancellation_and_no_input_mutation() {
    let mut s1 = Shape::ellipse(ShapeId(1), Point2::new(50.0, 50.0), 30.0, 30.0);
    s1.transform = Affine2::translate(Point2::new(5.0, 0.0));
    let s2 = Shape::rectangle(ShapeId(2), Rect::new(40.0, 20.0, 90.0, 80.0), 0.0);
    let inputs = vec![s1, s2];
    let snapshot = inputs.clone();
    let opts = BoolOptions::default();

    // 1. Pre-cancelled token
    let pre_token = BoolCancelToken::new();
    pre_token.cancel();
    let err_pre =
        boolean_with_options(&inputs, BoolOp::Union, &opts, Some(&pre_token), None).unwrap_err();
    assert_eq!(err_pre, BoolError::Cancelled);
    assert_eq!(inputs, snapshot);

    // 2. Mid-evaluation cancellation from progress callback
    let mid_token = BoolCancelToken::new();
    let seen_fractions = RefCell::new(Vec::new());
    let err_mid = boolean_with_options(
        &inputs,
        BoolOp::Union,
        &opts,
        Some(&mid_token),
        Some(&|frac| {
            seen_fractions.borrow_mut().push(frac);
            if frac >= 0.25 {
                mid_token.cancel();
            }
        }),
    )
    .unwrap_err();
    assert_eq!(err_mid, BoolError::Cancelled);
    assert_eq!(inputs, snapshot, "Input shapes must never be mutated");
    assert!(seen_fractions.borrow().contains(&0.25));

    // 3. Successful run reports monotonic progress up to 1.0
    let full_fractions = RefCell::new(Vec::new());
    let ok = boolean_with_options(
        &inputs,
        BoolOp::Union,
        &opts,
        None,
        Some(&|frac| full_fractions.borrow_mut().push(frac)),
    );
    assert!(ok.is_ok());
    let fracs = full_fractions.borrow();
    assert_eq!(fracs.first().copied(), Some(0.0));
    assert_eq!(fracs.last().copied(), Some(1.0));
    assert!(fracs.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(inputs, snapshot);
}

#[test]
fn test_boolean_degenerate_zero_area_input_errors() {
    // Policy: Closed contours with zero area (collinear vertices or duplicate points)
    // are rejected during input validation as `BoolError::DegenerateInput` rather than
    // silently returning an empty shape.
    let mut collinear = Subpath::new(true);
    collinear.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    collinear.push_node(BezierNode::new(Point2::new(25.0, 0.0), NodeType::Cusp));
    collinear.push_node(BezierNode::new(Point2::new(50.0, 0.0), NodeType::Cusp));
    let collinear_shape = Shape::path(ShapeId(10), Polycurve::from_subpath(collinear));

    let mut duplicate_pts = Subpath::new(true);
    for _ in 0..4 {
        duplicate_pts.push_node(BezierNode::new(Point2::new(15.0, 15.0), NodeType::Cusp));
    }
    let duplicate_shape = Shape::path(ShapeId(11), Polycurve::from_subpath(duplicate_pts));
    let valid_rect = Shape::rectangle(ShapeId(12), Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);

    let opts = BoolOptions::default();

    let err_collinear = boolean_with_options(
        &[collinear_shape, valid_rect.clone()],
        BoolOp::Union,
        &opts,
        None,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err_collinear, BoolError::DegenerateInput(_)),
        "Expected DegenerateInput for collinear zero-area polygon, got {err_collinear:?}"
    );

    let err_dup = boolean_with_options(
        &[duplicate_shape, valid_rect],
        BoolOp::Union,
        &opts,
        None,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(err_dup, BoolError::DegenerateInput(_)),
        "Expected DegenerateInput for duplicate-point polygon, got {err_dup:?}"
    );
}

#[test]
fn test_boolean_self_intersecting_input_policy() {
    // Policy frozen for fuzzing (Agent E):
    // `clipper2_rs::union_self_d` silently heals self-intersecting non-zero-area contours
    // (such as an asymmetric bow-tie) into clean simple subpaths, whereas `BoolError::SelfIntersection`
    // is a defensive variant surfaced only if Clipper returns an error during `union_self_d`.
    let mut bowtie = Subpath::new(true);
    bowtie.push_node(BezierNode::new(Point2::new(0.0, 0.0), NodeType::Cusp));
    bowtie.push_node(BezierNode::new(Point2::new(40.0, 30.0), NodeType::Cusp));
    bowtie.push_node(BezierNode::new(Point2::new(40.0, 0.0), NodeType::Cusp));
    bowtie.push_node(BezierNode::new(Point2::new(0.0, 10.0), NodeType::Cusp));
    let bowtie_shape = Shape::path(ShapeId(20), Polycurve::from_subpath(bowtie));
    let clip_rect = Shape::rectangle(ShapeId(21), Rect::new(10.0, 0.0, 50.0, 30.0), 0.0);

    let opts = BoolOptions::default();
    let res = boolean_with_options(&[bowtie_shape, clip_rect], BoolOp::Union, &opts, None, None);
    match res {
        Ok(merged) => {
            assert!(
                shape_unsigned_area(&merged) > 0.0,
                "Healed bow-tie union must have strictly positive area"
            );
        }
        Err(BoolError::SelfIntersection(_)) => {}
        Err(other) => {
            panic!("Self-intersecting non-zero-area input must either heal cleanly or yield SelfIntersection, got {other:?}");
        }
    }
}
