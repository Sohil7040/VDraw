use core_doc::{Shape, ShapeId};
use core_geom::{BezierNode, NodeType, Point2, Polycurve, Rect, Subpath};
use core_snap::guideline::Guideline;
use core_snap::target::SnapTargetKind;
use core_snap::{
    CenterSnapProvider, GeometrySnapProvider, GridSnapProvider, HandleMode,
    IntersectionSnapProvider, MidpointSnapProvider, NodeSnapProvider, OrthoConstraintProvider,
    PerpendicularSnapProvider, PolarConstraintProvider, QuadrantSnapProvider, SnapCtx, SnapEngine,
    SnapKind, SnapMask, SnapPoint, SnapProvider, SnapSolver, SpatialIndex, TangentSnapProvider,
    View, DEFAULT_SNAP_MASK, POLAR_STEP_DEG,
};
use proptest::prelude::*;
use std::sync::Arc;
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

// ─── Wave 1 Deliverable 1: SnapEngine & SnapProvider Tests ───────────────────

fn test_view() -> View {
    View::new(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 800.0)),
        egui::Vec2::ZERO,
        2.0,
    )
}

struct StaticProvider(Vec<SnapPoint>);

impl SnapProvider for StaticProvider {
    fn candidates(&self, _ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        self.0.clone()
    }

    fn collect_candidates(&self, _ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        out.extend(self.0.iter().cloned());
    }
}

#[test]
fn test_engine_tie_break_node_beats_center_and_priority_chain() {
    let view = test_view();
    let mut engine = SnapEngine::new(SnapMask::all());

    // Place a Center at (102, 100) and a Node at (98, 100) — both equidistant (2.0 mm)
    // from cursor at world (100, 100). Node must win with label "Node".
    engine.providers.push(Box::new(StaticProvider(vec![
        SnapPoint::new(Point2::new(102.0, 100.0), SnapKind::Center, ShapeId(1)),
        SnapPoint::new(Point2::new(98.0, 100.0), SnapKind::Node, ShapeId(2)),
    ])));

    let ctx = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(100.0, 100.0)),
        view: &view,
        threshold_px: 10.0,
    };

    let (pt, fb) = engine.resolve(&ctx).expect("expected equidistant tie hit");
    assert_eq!(pt.kind, SnapKind::Node);
    assert_eq!(pt.source, ShapeId(2));
    assert_eq!(fb.label, "Node");

    // Verify full tie-break order:
    // Node > Endpoint > Intersection > Midpoint > Quadrant > Center > Perpendicular > Tangent > Grid
    let mut prev_winner = None;
    for (start, &kind) in SnapKind::ALL.iter().enumerate() {
        let mut chain_engine = SnapEngine::new(SnapMask::all());
        let remaining: Vec<SnapPoint> = SnapKind::ALL[start..]
            .iter()
            .copied()
            .rev()
            .enumerate()
            .map(|(idx, k)| SnapPoint::new(Point2::new(100.0, 100.0), k, ShapeId(idx as u64 + 1)))
            .collect();
        chain_engine
            .providers
            .push(Box::new(StaticProvider(remaining)));
        let (winner, fb) = chain_engine.resolve(&ctx).expect("expected winner");
        assert_eq!(winner.kind, kind);
        assert_eq!(fb.label, kind.label());
        if let Some(prev) = prev_winner {
            assert_ne!(prev, winner.kind);
        }
        prev_winner = Some(winner.kind);
    }
}

#[test]
fn test_engine_endpoint_vs_node_open_and_closed_subpaths() {
    let view = test_view();
    let mut sub = Subpath::new(false);
    sub.push_node(BezierNode::new(Point2::new(10.0, 20.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(50.0, 20.0), NodeType::Cusp));
    sub.push_node(BezierNode::new(Point2::new(90.0, 60.0), NodeType::Cusp));
    let open_poly = Shape::path(ShapeId(1), Polycurve::from_subpath(sub));
    let closed_rect = Shape::rectangle(
        ShapeId(2),
        Rect::from_origin_size(Point2::new(200.0, 200.0), 40.0, 30.0),
        0.0,
    );

    let index = Arc::new(SpatialIndex::bulk_load(&[open_poly, closed_rect]));
    let mut engine = SnapEngine::new(DEFAULT_SNAP_MASK);
    engine
        .providers
        .push(Box::new(NodeSnapProvider::new(index)));

    // First node of open subpath -> Endpoint
    let ctx_start = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(10.5, 20.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_start).expect("start endpoint");
    assert_eq!(pt.kind, SnapKind::Endpoint);
    assert_eq!(pt.position, Point2::new(10.0, 20.0));
    assert_eq!(fb.label, "Endpoint");

    // Interior node of open subpath -> Node
    let ctx_mid_node = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(50.2, 20.2)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_mid_node).expect("interior node");
    assert_eq!(pt.kind, SnapKind::Node);
    assert_eq!(pt.position, Point2::new(50.0, 20.0));
    assert_eq!(fb.label, "Node");

    // Last node of open subpath -> Endpoint
    let ctx_end = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(89.5, 59.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_end).expect("end endpoint");
    assert_eq!(pt.kind, SnapKind::Endpoint);
    assert_eq!(pt.position, Point2::new(90.0, 60.0));
    assert_eq!(fb.label, "Endpoint");

    // Corner of closed rectangle -> Node
    let ctx_rect = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(200.5, 200.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_rect).expect("closed rect corner node");
    assert_eq!(pt.kind, SnapKind::Node);
    assert_eq!(pt.position, Point2::new(200.0, 200.0));
    assert_eq!(fb.label, "Node");
}

#[test]
fn test_engine_midpoint_center_and_quadrant_providers() {
    let view = test_view();
    let rect = Shape::rectangle(
        ShapeId(10),
        Rect::from_origin_size(Point2::new(20.0, 20.0), 80.0, 40.0),
        0.0,
    );
    let ellipse = Shape::ellipse(ShapeId(11), Point2::new(200.0, 150.0), 40.0, 25.0);
    let index = Arc::new(SpatialIndex::bulk_load(&[rect, ellipse]));

    // Midpoint provider
    let mut mid_engine = SnapEngine::new(SnapMask::MIDPOINT);
    mid_engine
        .providers
        .push(Box::new(MidpointSnapProvider::new(Arc::clone(&index))));
    let mid_ctx = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(60.5, 20.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = mid_engine.resolve(&mid_ctx).expect("midpoint");
    assert_eq!(pt.kind, SnapKind::Midpoint);
    assert_eq!(fb.label, "Midpoint");
    assert!((pt.position.x - 60.0).abs() < 1e-6);
    assert!((pt.position.y - 20.0).abs() < 1e-6);

    // Center provider
    let mut center_engine = SnapEngine::new(SnapMask::CENTER);
    center_engine
        .providers
        .push(Box::new(CenterSnapProvider::new(Arc::clone(&index))));
    let center_ctx = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(201.0, 149.0)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = center_engine.resolve(&center_ctx).expect("center");
    assert_eq!(pt.kind, SnapKind::Center);
    assert_eq!(fb.label, "Center");
    assert!((pt.position.x - 200.0).abs() < 1e-6);
    assert!((pt.position.y - 150.0).abs() < 1e-6);

    // Quadrant provider (all 4 cardinal extrema of the ellipse)
    let mut quad_engine = SnapEngine::new(SnapMask::QUADRANT);
    quad_engine
        .providers
        .push(Box::new(QuadrantSnapProvider::new(index)));
    let expected_quads = [
        Point2::new(200.0, 125.0),
        Point2::new(240.0, 150.0),
        Point2::new(200.0, 175.0),
        Point2::new(160.0, 150.0),
    ];
    for expected in expected_quads {
        let q_ctx = SnapCtx {
            cursor_screen: view.world_to_screen(expected + Point2::new(1.0, -1.0)),
            view: &view,
            threshold_px: 8.0,
        };
        let (pt, fb) = quad_engine.resolve(&q_ctx).expect("quadrant");
        assert_eq!(pt.kind, SnapKind::Quadrant);
        assert_eq!(fb.label, "Quadrant");
        assert!(pt.position.distance(expected) < 1e-3);
    }
}

#[test]
fn test_engine_intersection_provider_lines_and_curves() {
    let view = test_view();
    let circle = Shape::ellipse(ShapeId(1), Point2::new(100.0, 100.0), 50.0, 50.0);
    let diag = Shape::line(
        ShapeId(2),
        Point2::new(100.0, 20.0),
        Point2::new(100.0, 180.0),
    );
    let cross = Shape::line(
        ShapeId(3),
        Point2::new(20.0, 100.0),
        Point2::new(180.0, 100.0),
    );

    let index = Arc::new(SpatialIndex::bulk_load(&[circle, diag, cross]));
    let mut engine = SnapEngine::new(SnapMask::INTERSECTION);
    engine
        .providers
        .push(Box::new(IntersectionSnapProvider::new(index)));

    // Line x Line intersection at (100, 100)
    let ctx_center = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(101.0, 99.0)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_center).expect("line-line intersection");
    assert_eq!(pt.kind, SnapKind::Intersection);
    assert_eq!(fb.label, "Intersection");
    assert!(pt.position.distance(Point2::new(100.0, 100.0)) < 1e-3);

    // Circle x Vertical Line intersection at (100, 50)
    let ctx_top = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(100.5, 50.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_top).expect("circle-line intersection");
    assert_eq!(pt.kind, SnapKind::Intersection);
    assert_eq!(fb.label, "Intersection");
    assert!(pt.position.distance(Point2::new(100.0, 50.0)) < 0.05);
}

#[test]
fn test_engine_grid_provider() {
    let view = test_view();
    let mut engine = SnapEngine::new(SnapMask::GRID);
    engine
        .providers
        .push(Box::new(GridSnapProvider::with_spacing(10.0, 5))); // 2.0 mm step

    let ctx_hit = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(23.8, 41.7)),
        view: &view,
        threshold_px: 6.0, // 3.0 mm
    };
    let (pt, fb) = engine.resolve(&ctx_hit).expect("grid snap hit");
    assert_eq!(pt.kind, SnapKind::Grid);
    assert_eq!(fb.label, "Grid");
    assert!(pt.position.distance(Point2::new(24.0, 42.0)) < 1e-6);

    // Threshold too tight to reach nearest 10mm grid point from (25, 25)
    let mut coarse_engine = SnapEngine::new(SnapMask::GRID);
    coarse_engine
        .providers
        .push(Box::new(GridSnapProvider::with_spacing(10.0, 1)));
    let ctx_miss = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(25.0, 25.0)),
        view: &view,
        threshold_px: 4.0, // 2.0 mm < sqrt(50) = 7.07 mm
    };
    assert_eq!(coarse_engine.resolve(&ctx_miss), None);
}

#[test]
fn test_engine_perpendicular_and_tangent_providers() {
    let view = test_view();
    let line = Shape::line(ShapeId(1), Point2::new(0.0, 50.0), Point2::new(100.0, 50.0));
    let circle = Shape::ellipse(ShapeId(2), Point2::new(100.0, 200.0), 50.0, 50.0);
    let index = Arc::new(SpatialIndex::bulk_load(&[line, circle]));

    // Perpendicular onto horizontal line from (37, 0) -> (37, 50)
    let mut perp_engine = SnapEngine::new(SnapMask::PERPENDICULAR);
    perp_engine
        .providers
        .push(Box::new(PerpendicularSnapProvider::new(
            Arc::clone(&index),
            Some(Point2::new(37.0, 0.0)),
        )));
    let ctx_line_perp = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(37.5, 50.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = perp_engine
        .resolve(&ctx_line_perp)
        .expect("line perpendicular");
    assert_eq!(pt.kind, SnapKind::Perpendicular);
    assert_eq!(fb.label, "Perpendicular");
    assert!(pt.position.distance(Point2::new(37.0, 50.0)) < 1e-6);

    // Perpendicular onto circle centered at (100, 200) r=50 from (0, 200) -> (50, 200)
    let mut circle_perp_engine = SnapEngine::new(SnapMask::PERPENDICULAR);
    circle_perp_engine
        .providers
        .push(Box::new(PerpendicularSnapProvider::new(
            Arc::clone(&index),
            Some(Point2::new(0.0, 200.0)),
        )));
    let ctx_circle_perp = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(50.5, 200.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = circle_perp_engine
        .resolve(&ctx_circle_perp)
        .expect("circle perpendicular");
    assert_eq!(pt.kind, SnapKind::Perpendicular);
    assert_eq!(fb.label, "Perpendicular");
    assert!(pt.position.distance(Point2::new(50.0, 200.0)) < 0.05);

    // Tangent onto circle centered at (100, 200) r=50 from (0, 200):
    // Analytic tangent points are (75, 200 - 25*sqrt(3)) and (75, 200 + 25*sqrt(3))
    let expected_tan = Point2::new(75.0, 200.0 - 25.0 * 3.0_f64.sqrt());
    let mut tan_engine = SnapEngine::new(SnapMask::TANGENT);
    tan_engine.providers.push(Box::new(TangentSnapProvider::new(
        index,
        Some(Point2::new(0.0, 200.0)),
    )));
    let ctx_tan = SnapCtx {
        cursor_screen: view.world_to_screen(expected_tan + Point2::new(0.5, -0.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = tan_engine.resolve(&ctx_tan).expect("circle tangent");
    assert_eq!(pt.kind, SnapKind::Tangent);
    assert_eq!(fb.label, "Tangent");
    assert!(pt.position.distance(expected_tan) < 0.1);
}

#[test]
fn test_engine_polar_and_ortho_constraint_providers() {
    let view = test_view();
    let origin = Point2::new(50.0, 50.0);

    // Polar constraint using POLAR_STEP_DEG (15 deg)
    assert!((POLAR_STEP_DEG - 15.0).abs() < 1e-9);
    let polar = PolarConstraintProvider::new(origin);
    let mut polar_engine = SnapEngine::new(SnapMask::GRID | SnapMask::POLAR);
    polar_engine.providers.push(Box::new(polar));

    // Cursor slightly off the 30-degree ray at radius 80 mm
    let angle_30 = 30.0_f64.to_radians();
    let on_ray = origin + Point2::new(80.0 * angle_30.cos(), 80.0 * angle_30.sin());
    let near_ray = on_ray + Point2::new(0.0, 1.0);
    let ctx_polar = SnapCtx {
        cursor_screen: view.world_to_screen(near_ray),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, _) = polar_engine.resolve(&ctx_polar).expect("polar 30 deg snap");
    let ray_vec = pt.position - origin;
    let snapped_angle_deg = ray_vec.y.atan2(ray_vec.x).to_degrees();
    assert!((snapped_angle_deg - 30.0).abs() < 1e-6);

    // Off-ray beyond threshold must return None
    let angle_37 = 37.5_f64.to_radians();
    let mid_ray = origin + Point2::new(100.0 * angle_37.cos(), 100.0 * angle_37.sin());
    let ctx_polar_miss = SnapCtx {
        cursor_screen: view.world_to_screen(mid_ray),
        view: &view,
        threshold_px: 8.0, // 4 mm < 100 * sin(7.5 deg) ~= 13.05 mm
    };
    assert_eq!(polar_engine.resolve(&ctx_polar_miss), None);

    // Ortho constraint (H and V projection)
    let ortho = OrthoConstraintProvider::new(origin);
    let mut ortho_engine = SnapEngine::new(SnapMask::GRID | SnapMask::ORTHO);
    ortho_engine.providers.push(Box::new(ortho));

    let ctx_ortho_h = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(140.0, 51.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt_h, _) = ortho_engine
        .resolve(&ctx_ortho_h)
        .expect("ortho horizontal");
    assert_eq!(pt_h.position, Point2::new(140.0, 50.0));

    let ctx_ortho_v = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(48.5, 130.0)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt_v, _) = ortho_engine.resolve(&ctx_ortho_v).expect("ortho vertical");
    assert_eq!(pt_v.position, Point2::new(50.0, 130.0));

    // Direct projection helper works for arbitrary distances
    assert_eq!(
        ortho.project(Point2::new(200.0, 60.0)),
        Some(Point2::new(200.0, 50.0))
    );
    assert_eq!(
        ortho.project(Point2::new(55.0, 200.0)),
        Some(Point2::new(50.0, 200.0))
    );
}

#[test]
fn test_compute_delta_all_handle_modes() {
    let source = Point2::new(15.0, 25.0);
    let target = SnapPoint::new(Point2::new(45.0, 10.0), SnapKind::Node, ShapeId(7));
    let expected = Point2::new(30.0, -15.0);

    for mode in [
        HandleMode::Node,
        HandleMode::Center,
        HandleMode::Insertion,
        HandleMode::Edge,
    ] {
        let delta = SnapEngine::compute_delta(mode, source, &target);
        assert_eq!(delta, expected);
    }

    // Non-finite inputs must cleanly return Point2::ZERO without panicking
    let nan_target = SnapPoint::new(Point2::new(f64::NAN, 10.0), SnapKind::Node, ShapeId(8));
    assert_eq!(
        SnapEngine::compute_delta(HandleMode::Node, source, &nan_target),
        Point2::ZERO
    );
    assert_eq!(
        SnapEngine::compute_delta(HandleMode::Center, Point2::new(f64::INFINITY, 0.0), &target),
        Point2::ZERO
    );
}

#[test]
fn test_snap_solver_as_snap_provider() {
    let view = test_view();
    let mut solver = SnapSolver::default();
    solver.config.snap_to_grid = false;
    solver.spatial_index.insert(&Shape::line(
        ShapeId(42),
        Point2::new(10.0, 10.0),
        Point2::new(110.0, 10.0),
    ));

    let mut engine = SnapEngine::new(DEFAULT_SNAP_MASK | SnapMask::MIDPOINT);
    engine.providers.push(Box::new(solver));

    let ctx_ep = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(10.5, 10.2)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_ep).expect("solver endpoint");
    assert_eq!(pt.kind, SnapKind::Endpoint);
    assert_eq!(fb.label, "Endpoint");

    let ctx_mid = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(60.2, 10.2)),
        view: &view,
        threshold_px: 8.0,
    };
    let (pt, fb) = engine.resolve(&ctx_mid).expect("solver midpoint");
    assert_eq!(pt.kind, SnapKind::Midpoint);
    assert_eq!(fb.label, "Midpoint");
}

#[test]
fn test_engine_10k_shapes_latency_and_scratch_stability() {
    let mut shapes = Vec::with_capacity(10_000);
    for i in 0..100 {
        for j in 0..100 {
            let id = ShapeId((i * 100 + j + 1) as u64);
            let origin = Point2::new(i as f64 * 40.0, j as f64 * 40.0);
            shapes.push(Shape::rectangle(
                id,
                Rect::from_origin_size(origin, 20.0, 20.0),
                0.0,
            ));
        }
    }

    let index = Arc::new(SpatialIndex::bulk_load(&shapes));
    let mut engine = SnapEngine::new(SnapMask::all());
    engine
        .providers
        .push(Box::new(GeometrySnapProvider::new(Arc::clone(&index))));
    engine.providers.push(Box::new(GridSnapProvider::default()));

    let view = test_view();
    let mut scratch = Vec::with_capacity(256);
    let warmup_ctx = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(400.5, 400.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let _ = engine.resolve_with_scratch(&warmup_ctx, &mut scratch);
    let warmed_capacity = scratch.capacity();

    let query_count = 1_000;
    let start = Instant::now();
    for k in 0..query_count {
        let qx = (k as f64 * 37.17) % 4_000.0;
        let qy = (k as f64 * 53.41) % 4_000.0;
        let ctx = SnapCtx {
            cursor_screen: view.world_to_screen(Point2::new(qx, qy)),
            view: &view,
            threshold_px: 8.0,
        };
        let res = engine.resolve_with_scratch(&ctx, &mut scratch);
        std::hint::black_box(res);
        assert_eq!(
            scratch.capacity(),
            warmed_capacity,
            "scratch buffer should not reallocate after warmup"
        );
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / (query_count as f64);
    assert!(
        avg_ms < 1.0,
        "10k-shape SnapEngine::resolve exceeded 1 ms budget: {avg_ms:.4} ms"
    );
}

#[test]
fn test_snap_feedback_labels_match_snap_kind_variants() {
    let view = test_view();
    let target_world = Point2::new(50.0, 50.0);
    let ctx = SnapCtx {
        cursor_screen: view.world_to_screen(target_world),
        view: &view,
        threshold_px: 8.0,
    };

    for kind in SnapKind::ALL {
        let mut engine = SnapEngine::new(SnapMask::all());
        engine
            .providers
            .push(Box::new(StaticProvider(vec![SnapPoint::new(
                target_world,
                kind,
                ShapeId(1),
            )])));
        let (pt, fb) = engine
            .resolve(&ctx)
            .expect("fake provider must yield snap hit for every SnapKind");
        assert_eq!(pt.kind, kind);
        assert_eq!(fb.label, format!("{kind:?}"));
    }
}

proptest! {
    #[test]
    fn prop_resolve_is_idempotent(
        rx in 0.0..200.0f64,
        ry in 0.0..200.0f64,
        rw in 20.0..100.0f64,
        rh in 20.0..100.0f64,
        cx in -20.0..320.0f64,
        cy in -20.0..320.0f64,
        mask_bits in any::<u16>(),
    ) {
        let view = test_view();
        let rect = Shape::rectangle(
            ShapeId(1),
            Rect::from_origin_size(Point2::new(rx, ry), rw, rh),
            0.0,
        );
        let ellipse = Shape::ellipse(
            ShapeId(2),
            Point2::new(rx + rw * 0.5, ry + rh * 0.5),
            rw * 0.4,
            rh * 0.4,
        );
        let line = Shape::line(
            ShapeId(3),
            Point2::new(rx, ry),
            Point2::new(rx + rw, ry + rh),
        );
        let index = Arc::new(SpatialIndex::bulk_load(&[rect, ellipse, line]));
        let mask = SnapMask::from_bits_truncate(mask_bits as u32);
        let mut engine = SnapEngine::new(mask);
        engine.providers.push(Box::new(
            GeometrySnapProvider::new(index).with_reference_origin(Some(Point2::new(rx, ry))),
        ));
        engine.providers.push(Box::new(GridSnapProvider::default()));

        let ctx = SnapCtx {
            cursor_screen: view.world_to_screen(Point2::new(cx, cy)),
            view: &view,
            threshold_px: 10.0,
        };
        let first = engine.resolve(&ctx);
        let second = engine.resolve(&ctx);
        prop_assert_eq!(first, second);
    }

    #[test]
    fn prop_delta_translation_invariance(
        rx in 10.0..150.0f64,
        ry in 10.0..150.0f64,
        rw in 30.0..120.0f64,
        rh in 30.0..120.0f64,
        dx in -1.5..1.5f64,
        dy in -1.5..1.5f64,
        vx in -120.0..120.0f64,
        vy in -120.0..120.0f64,
        ax in -100.0..100.0f64,
        ay in -100.0..100.0f64,
        handle_idx in 0usize..4usize,
    ) {
        let view = View::new(
            egui::Rect::from_min_size(egui::pos2(-500.0, -500.0), egui::vec2(3000.0, 3000.0)),
            egui::Vec2::ZERO,
            2.0,
        );
        let handle_mode = [
            HandleMode::Node,
            HandleMode::Center,
            HandleMode::Insertion,
            HandleMode::Edge,
        ][handle_idx];
        let v = Point2::new(vx, vy);
        let anchor = Point2::new(ax, ay);

        let rect_orig = Shape::rectangle(
            ShapeId(10),
            Rect::from_origin_size(Point2::new(rx, ry), rw, rh),
            0.0,
        );
        let rect_shifted = Shape::rectangle(
            ShapeId(10),
            Rect::from_origin_size(Point2::new(rx + vx, ry + vy), rw, rh),
            0.0,
        );

        let mut engine_orig = SnapEngine::new(DEFAULT_SNAP_MASK | SnapMask::MIDPOINT);
        engine_orig.handle_mode = handle_mode;
        engine_orig.providers.push(Box::new(GeometrySnapProvider::new(Arc::new(
            SpatialIndex::bulk_load(&[rect_orig]),
        ))));

        let mut engine_shifted = SnapEngine::new(DEFAULT_SNAP_MASK | SnapMask::MIDPOINT);
        engine_shifted.handle_mode = handle_mode;
        engine_shifted.providers.push(Box::new(GeometrySnapProvider::new(Arc::new(
            SpatialIndex::bulk_load(&[rect_shifted]),
        ))));

        let cursor_orig = Point2::new(rx + dx, ry + dy);
        let ctx_orig = SnapCtx {
            cursor_screen: view.world_to_screen(cursor_orig),
            view: &view,
            threshold_px: 8.0, // 4.0 mm > sqrt(1.5^2 + 1.5^2) ~= 2.12 mm
        };
        let ctx_shifted = SnapCtx {
            cursor_screen: view.world_to_screen(cursor_orig + v),
            view: &view,
            threshold_px: 8.0,
        };

        let (pt_orig, _) = engine_orig
            .resolve(&ctx_orig)
            .expect("original cursor within threshold must snap");
        let (pt_shifted, _) = engine_shifted
            .resolve(&ctx_shifted)
            .expect("shifted cursor within threshold must snap");

        prop_assert_eq!(pt_orig.kind, pt_shifted.kind);
        let delta_orig = SnapEngine::compute_delta(handle_mode, anchor, &pt_orig);
        let delta_shifted = SnapEngine::compute_delta(handle_mode, anchor + v, &pt_shifted);
        prop_assert!(
            delta_orig.distance(delta_shifted) < 1e-6,
            "delta translation invariance violated: orig={delta_orig:?}, shifted={delta_shifted:?}"
        );
    }

    #[test]
    fn prop_resolve_returns_none_beyond_threshold(
        rx in 20.0..120.0f64,
        ry in 20.0..120.0f64,
        rw in 10.0..60.0f64,
        rh in 10.0..60.0f64,
        threshold_px in 2.0..16.0f32,
        scale in 0.5..4.0f64,
        extra_margin in 2.0..50.0f64,
    ) {
        let view = View::new(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(2000.0, 2000.0)),
            egui::Vec2::ZERO,
            scale,
        );
        let rect = Shape::rectangle(
            ShapeId(1),
            Rect::from_origin_size(Point2::new(rx, ry), rw, rh),
            0.0,
        );
        let line = Shape::line(
            ShapeId(2),
            Point2::new(rx, ry),
            Point2::new(rx + rw, ry + rh),
        );
        let ellipse = Shape::ellipse(
            ShapeId(3),
            Point2::new(rx + rw * 0.5, ry + rh * 0.5),
            rw * 0.5,
            rh * 0.5,
        );
        let index = Arc::new(SpatialIndex::bulk_load(&[rect, line, ellipse]));

        let geom_mask = SnapMask::NODE
            | SnapMask::ENDPOINT
            | SnapMask::MIDPOINT
            | SnapMask::CENTER
            | SnapMask::QUADRANT
            | SnapMask::INTERSECTION
            | SnapMask::PERPENDICULAR
            | SnapMask::TANGENT;
        let mut engine = SnapEngine::new(geom_mask);
        engine.providers.push(Box::new(
            GeometrySnapProvider::new(index).with_reference_origin(Some(Point2::new(rx, ry))),
        ));

        let threshold_world = (threshold_px as f64) / scale;
        let far_cursor = Point2::new(
            rx + rw + threshold_world + extra_margin,
            ry + rh + threshold_world + extra_margin,
        );
        let far_ctx = SnapCtx {
            cursor_screen: view.world_to_screen(far_cursor),
            view: &view,
            threshold_px,
        };
        prop_assert_eq!(engine.resolve(&far_ctx), None);
    }
}
