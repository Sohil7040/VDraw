//! Unit and property tests for VDraw core-boolean operations:
//! Weld, Trim, Intersect, Exclude, Simplify, Knife, Eraser, Offset, Virtual Segment Delete, Text to Outlines.

use core_boolean::{
    default_text_to_polycurve, delete_virtual_segment, erase_shape, execute_boolean,
    find_virtual_segments, offset_shape, slice_shape, BooleanOp, ContourJoin, TextOptions,
};
use core_geom::polycurve::CornerStyle;
use core_geom::rect::Rect;
use core_geom::{Point2, Polycurve};
use proptest::prelude::*;

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
