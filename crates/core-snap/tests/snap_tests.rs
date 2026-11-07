use core_doc::{Shape, ShapeId};
use core_geom::{Point2, Rect};
use core_snap::guideline::Guideline;
use core_snap::target::SnapTargetKind;
use core_snap::{SnapSolver, SpatialIndex};
use proptest::prelude::*;
use std::time::Instant;

#[test]
fn test_node_and_midpoint_snapping() {
    let mut solver = SnapSolver::default();
    solver.config.snap_to_grid = false; // Isolate geometry snap

    // Create a 100x50 rectangle shape at (10, 10)
    let rect_shape = Shape::rectangle(
        ShapeId(1),
        Rect::from_origin_size(Point2::new(10.0, 10.0), 100.0, 50.0),
        0.0,
    );
    solver.spatial_index.insert(&rect_shape);

    let zoom = 1.0; // 1 screen px = 1 mm, tolerance = 8.0 mm

    // 1. Query near TopLeft corner (10, 10) -> should snap to Node
    let res = solver.snap_point(Point2::new(12.0, 11.0), zoom, None, &[]);
    assert!(res.snapped);
    let target = res.target.unwrap();
    assert_eq!(target.kind, SnapTargetKind::Node);
    assert_eq!(target.point, Point2::new(10.0, 10.0));

    // 2. Query near top edge midpoint ((10+110)/2 = 60, 10)
    let res_mid = solver.snap_point(Point2::new(59.5, 11.0), zoom, None, &[]);
    assert!(res_mid.snapped);
    assert_eq!(res_mid.target.unwrap().kind, SnapTargetKind::Midpoint);
    assert!((res_mid.point.x - 60.0).abs() < 1e-4);
    assert!((res_mid.point.y - 10.0).abs() < 1e-4);

    // 3. Query far away (300, 300) -> no snap
    let res_far = solver.snap_point(Point2::new(300.0, 300.0), zoom, None, &[]);
    assert!(!res_far.snapped);
}

#[test]
fn test_center_and_quadrant_snapping() {
    let mut solver = SnapSolver::default();
    solver.config.snap_to_grid = false;

    // Ellipse at center (100, 100) with rx = 40, ry = 20
    let ellipse = Shape::ellipse(ShapeId(2), Point2::new(100.0, 100.0), 40.0, 20.0);
    solver.spatial_index.insert(&ellipse);

    let zoom = 2.0; // tolerance = 4.0 mm

    // Near center (100, 100)
    let res_center = solver.snap_point(Point2::new(101.0, 99.5), zoom, None, &[]);
    assert!(res_center.snapped);
    assert_eq!(res_center.target.unwrap().kind, SnapTargetKind::Center);
    assert_eq!(res_center.point, Point2::new(100.0, 100.0));

    // Near Right quadrant (140, 100)
    let res_quad = solver.snap_point(Point2::new(139.0, 100.5), zoom, None, &[]);
    assert!(res_quad.snapped);
    let t = res_quad.target.unwrap();
    assert!(
        t.kind == SnapTargetKind::Quadrant
            || t.kind == SnapTargetKind::Node
            || t.kind == SnapTargetKind::BboxPoint
    );
    assert!((res_quad.point.x - 140.0).abs() < 1.0);
    assert!((res_quad.point.y - 100.0).abs() < 1.0);
}

#[test]
fn test_intersection_snapping() {
    let mut solver = SnapSolver::default();
    solver.config.snap_to_grid = false;

    // Two crossing lines:
    // Line 1: (0, 0) to (100, 100)
    // Line 2: (0, 100) to (100, 0)
    // Intersection at (50, 50)
    let line1 = Shape::line(
        ShapeId(10),
        Point2::new(0.0, 0.0),
        Point2::new(100.0, 100.0),
    );
    let line2 = Shape::line(
        ShapeId(11),
        Point2::new(0.0, 100.0),
        Point2::new(100.0, 0.0),
    );

    solver.spatial_index.insert(&line1);
    solver.spatial_index.insert(&line2);

    let zoom = 1.0;
    let res = solver.snap_point(Point2::new(51.0, 49.5), zoom, None, &[]);
    assert!(res.snapped);
    assert_eq!(res.target.unwrap().kind, SnapTargetKind::Intersection);
    assert!((res.point.x - 50.0).abs() < 0.1);
    assert!((res.point.y - 50.0).abs() < 0.1);
}

#[test]
fn test_guideline_snapping_and_intersection() {
    let mut solver = SnapSolver::default();
    solver.config.snap_to_grid = false;

    solver.add_guideline(Guideline::horizontal(1, 75.0));
    solver.add_guideline(Guideline::vertical(2, 120.0));

    let zoom = 1.0; // tolerance = 8.0 mm

    // Snap to horizontal guideline
    let res_h = solver.snap_point(Point2::new(40.0, 77.0), zoom, None, &[]);
    assert!(res_h.snapped);
    assert_eq!(res_h.target.unwrap().kind, SnapTargetKind::Guideline);
    assert_eq!(res_h.point.y, 75.0);

    // Snap to guideline intersection at (120, 75)
    let res_x = solver.snap_point(Point2::new(122.0, 76.0), zoom, None, &[]);
    assert!(res_x.snapped);
    assert_eq!(res_x.target.unwrap().kind, SnapTargetKind::Intersection);
    assert_eq!(res_x.point, Point2::new(120.0, 75.0));
}

#[test]
fn test_grid_snapping() {
    let mut solver = SnapSolver::default();
    solver.config.grid.enabled = true;
    solver.config.grid.major_spacing_mm = 10.0;
    solver.config.grid.subdivisions = 5; // 2.0 mm minor
    solver.config.grid.snap_to_minor = true;

    let zoom = 1.0; // tolerance = 8.0 mm

    // (23.8, 41.9) should snap to minor grid point (24.0, 42.0)
    let res = solver.snap_point(Point2::new(23.8, 41.9), zoom, None, &[]);
    assert!(res.snapped);
    assert_eq!(res.target.unwrap().kind, SnapTargetKind::Grid);
    assert!((res.point.x - 24.0).abs() < 1e-4);
    assert!((res.point.y - 42.0).abs() < 1e-4);
}

#[test]
fn test_dynamic_polar_guide_snapping() {
    let solver = SnapSolver::default();
    let origin = Point2::new(50.0, 50.0);

    // Point near 45° angle ray: dx = 30.0, dy = 30.5
    let cursor = Point2::new(80.0, 80.5);
    let zoom = 1.0;

    let res = solver.snap_point(cursor, zoom, Some(origin), &[]);
    assert!(res.snapped);
    let ray = res.dynamic_ray.expect("Expected dynamic ray");
    assert_eq!(ray.angle_deg, 45.0);
    assert_eq!(ray.origin, origin);
}

#[test]
fn test_smart_alignment_guides() {
    let mut solver = SnapSolver::default();

    // Existing shape at (100, 100), size (60, 50) => bounds: [100, 100] to [160, 150]
    let static_shape = Shape::rectangle(
        ShapeId(1),
        Rect::from_origin_size(Point2::new(100.0, 100.0), 60.0, 50.0),
        0.0,
    );
    solver.spatial_index.insert(&static_shape);

    // Dragged shape originally at (0, 0), size (50, 50)
    let dragged_box = Rect::from_origin_size(Point2::ZERO, 50.0, 50.0);

    // User is dragging near X = 100 (e.g. delta = (98.5, 200.0))
    let (snapped_delta, lines) =
        solver.snap_transform(dragged_box, Point2::new(98.5, 200.0), 1.0, &[]);

    // Should snap X to exact Left-to-Left alignment at 100.0
    assert_eq!(snapped_delta.x, 100.0);
    assert!(!lines.is_empty());
    assert!(lines.iter().any(|l| l.is_vertical && l.start.x == 100.0));
}

#[test]
fn test_priority_resolution_node_over_grid() {
    let mut solver = SnapSolver::default();
    solver.config.grid.enabled = true;
    solver.config.grid.major_spacing_mm = 10.0;
    solver.config.grid.subdivisions = 1;

    // Node at (12.3, 14.7) - deliberately off-grid
    let poly = Shape::line(
        ShapeId(99),
        Point2::new(12.3, 14.7),
        Point2::new(50.0, 50.0),
    );
    solver.spatial_index.insert(&poly);

    // Cursor right next to the off-grid node at (12.5, 14.8)
    // Nearest grid is (10, 10) which is distance ~5.4 mm (within 8 mm tolerance)
    // But Node is only ~0.2 mm away and has higher priority
    let res = solver.snap_point(Point2::new(12.5, 14.8), 1.0, None, &[]);
    assert!(res.snapped);
    assert_eq!(res.target.unwrap().kind, SnapTargetKind::Node);
    assert_eq!(res.point, Point2::new(12.3, 14.7));
}

/// SPEC QUALITY RULE: Benchmark snapping on 50k shapes, target < 2 ms per query.
#[test]
fn test_benchmark_50k_shapes_latency() {
    println!("Generating 50,000 shapes for spatial indexing benchmark...");
    let mut shapes = Vec::with_capacity(50_000);

    // 250 x 200 grid of shapes spaced across a 10,000 x 10,000 mm workspace
    for i in 0..250 {
        for j in 0..200 {
            let id = ShapeId((i * 200 + j + 1) as u64);
            let origin = Point2::new(i as f64 * 40.0, j as f64 * 50.0);
            shapes.push(Shape::rectangle(
                id,
                Rect::from_origin_size(origin, 20.0, 20.0),
                0.0,
            ));
        }
    }

    assert_eq!(shapes.len(), 50_000);

    let start_build = Instant::now();
    let spatial_index = SpatialIndex::bulk_load(&shapes);
    let build_duration = start_build.elapsed();
    println!("Bulk-loaded 50,000 shapes in {:?}", build_duration);

    let solver = SnapSolver {
        spatial_index,
        ..Default::default()
    };

    // Perform 1,000 snap queries across diverse locations
    let query_count = 1_000;
    let start_queries = Instant::now();

    for k in 0..query_count {
        let qx = (k as f64 * 37.17) % 10_000.0;
        let qy = (k as f64 * 53.41) % 10_000.0;
        let res = solver.snap_point(Point2::new(qx, qy), 1.0, None, &[]);
        // ensure compiler doesn't optimize away
        std::hint::black_box(res);
    }

    let total_query_duration = start_queries.elapsed();
    let avg_query_time = total_query_duration / (query_count as u32);
    let avg_query_ms = avg_query_time.as_secs_f64() * 1000.0;

    println!(
        "50k shapes benchmark: 1,000 queries in {:?} (avg: {:.4} ms per query)",
        total_query_duration, avg_query_ms
    );

    // SPEC CRITICAL TARGET: < 2.0 ms per query!
    assert!(
        avg_query_ms < 2.0,
        "Query latency exceeded target: {:.4} ms >= 2.0 ms",
        avg_query_ms
    );
}

// ─── Property Tests (proptest) ────────────────────────────────────────────────

proptest! {
    /// Property: Snapping an already snapped grid point is strictly idempotent: f(f(p)) == f(p).
    #[test]
    fn prop_grid_snap_idempotent(x in -1000.0..1000.0f64, y in -1000.0..1000.0f64) {
        let mut solver = SnapSolver::default();
        solver.config.snap_to_nodes = false;
        solver.config.snap_to_grid = true;
        solver.config.grid.major_spacing_mm = 5.0;
        solver.config.grid.subdivisions = 5;

        let p1 = Point2::new(x, y);
        let res1 = solver.snap_point(p1, 1.0, None, &[]);
        if res1.snapped {
            let res2 = solver.snap_point(res1.point, 1.0, None, &[]);
            prop_assert!(res2.snapped);
            prop_assert!((res1.point.x - res2.point.x).abs() < 1e-9);
            prop_assert!((res1.point.y - res2.point.y).abs() < 1e-9);
        }
    }

    /// Property: Any successful snap result must lie within the gravity radius in world units.
    #[test]
    fn prop_snap_within_tolerance(
        x in 0.0..500.0f64,
        y in 0.0..500.0f64,
        zoom in 0.1..10.0f64
    ) {
        let mut solver = SnapSolver::default();
        let target_pt = Point2::new(100.0, 100.0);
        solver.spatial_index.insert(&Shape::rectangle(
            ShapeId(1),
            Rect::from_origin_size(target_pt, 20.0, 20.0),
            0.0,
        ));


        let query_pt = Point2::new(x, y);
        let res = solver.snap_point(query_pt, zoom, None, &[]);
        let tolerance_world = solver.config.gravity_radius_px / zoom;

        if res.snapped {
            let dist = query_pt.distance(res.point);
            // Allow tiny epsilon for numerical float imprecision
            prop_assert!(dist <= tolerance_world + 1e-4);
        }
    }
}
