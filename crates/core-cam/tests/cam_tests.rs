use core_cam::*;
use core_doc::Document;
use core_geom::{Point2, Rect};

#[test]
fn test_containment_and_inside_out_ordering() {
    // Outer square: (0, 0) to (100, 100)
    let outer_pts = vec![
        Point2::new(0.0, 0.0),
        Point2::new(100.0, 0.0),
        Point2::new(100.0, 100.0),
        Point2::new(0.0, 100.0),
    ];
    let outer = CamContour::new(outer_pts, true, 1, "Cut".to_string(), LaserMode::Cut);

    // Inner hole: (20, 20) to (40, 40)
    let inner_pts = vec![
        Point2::new(20.0, 20.0),
        Point2::new(40.0, 20.0),
        Point2::new(40.0, 40.0),
        Point2::new(20.0, 40.0),
    ];
    let inner = CamContour::new(inner_pts, true, 2, "Cut".to_string(), LaserMode::Cut);

    assert!(outer.contains_contour(&inner));
    assert!(!inner.contains_contour(&outer));

    let mut contours = vec![outer, inner];
    calculate_containment_depths(&mut contours);

    // Outer has depth 0, inner has depth 1
    assert_eq!(contours[0].depth, 0);
    assert_eq!(contours[1].depth, 1);
    assert!(contours[1].is_inner_hole);
    assert!(!contours[0].is_inner_hole);

    // Optimize cutting order with inside_out = true
    let ordered = optimize_cutting_order(contours, Point2::ZERO, true, true);

    // Inner hole (depth 1) MUST be cut before outer boundary (depth 0)!
    assert_eq!(ordered[0].shape_id, 2);
    assert_eq!(ordered[1].shape_id, 1);
}

#[test]
fn test_multi_level_nesting_dag_ordering() {
    // Level 0: Outer Part (0,0) to (200, 200)
    let c0 = CamContour::new(
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(200.0, 0.0),
            Point2::new(200.0, 200.0),
            Point2::new(0.0, 200.0),
        ],
        true,
        100,
        "Cut".to_string(),
        LaserMode::Cut,
    );

    // Level 1: Hole inside Outer Part (20, 20) to (180, 180)
    let c1 = CamContour::new(
        vec![
            Point2::new(20.0, 20.0),
            Point2::new(180.0, 20.0),
            Point2::new(180.0, 180.0),
            Point2::new(20.0, 180.0),
        ],
        true,
        101,
        "Cut".to_string(),
        LaserMode::Cut,
    );

    // Level 2: Island inside Hole (50, 50) to (150, 150)
    let c2 = CamContour::new(
        vec![
            Point2::new(50.0, 50.0),
            Point2::new(150.0, 50.0),
            Point2::new(150.0, 150.0),
            Point2::new(50.0, 150.0),
        ],
        true,
        102,
        "Cut".to_string(),
        LaserMode::Cut,
    );

    // Level 3: Tiny Hole inside Island (80, 80) to (120, 120)
    let c3 = CamContour::new(
        vec![
            Point2::new(80.0, 80.0),
            Point2::new(120.0, 80.0),
            Point2::new(120.0, 120.0),
            Point2::new(80.0, 120.0),
        ],
        true,
        103,
        "Cut".to_string(),
        LaserMode::Cut,
    );

    let contours = vec![c0, c1, c2, c3];
    let ordered = optimize_cutting_order(contours, Point2::ZERO, true, true);

    // Innermost hole c3 (103) must precede island c2 (102).
    // And c1/c2 must precede outer boundary c0 (100).
    let idx_c3 = ordered.iter().position(|c| c.shape_id == 103).unwrap();
    let idx_c2 = ordered.iter().position(|c| c.shape_id == 102).unwrap();
    let idx_c0 = ordered.iter().position(|c| c.shape_id == 100).unwrap();

    assert!(
        idx_c3 < idx_c2,
        "Hole inside island must be cut before island"
    );
    assert!(
        idx_c2 < idx_c0,
        "Island must be cut before enclosing outer boundary"
    );
}

#[test]
fn test_operation_priorities() {
    let p = vec![Point2::new(0.0, 0.0), Point2::new(10.0, 10.0)];
    let cut = CamContour::new(p.clone(), false, 1, "L1".to_string(), LaserMode::Cut);
    let engrave = CamContour::new(p.clone(), false, 2, "L2".to_string(), LaserMode::Engrave);
    let score = CamContour::new(p.clone(), false, 3, "L3".to_string(), LaserMode::Score);
    let dot = CamContour::new(p, false, 4, "L4".to_string(), LaserMode::Dot);

    let list = vec![cut, engrave, score, dot];
    let ordered = optimize_cutting_order(list, Point2::ZERO, false, false);

    // Standard sequence: Dot -> Engrave -> Score -> Cut
    assert_eq!(ordered[0].mode, LaserMode::Dot);
    assert_eq!(ordered[1].mode, LaserMode::Engrave);
    assert_eq!(ordered[2].mode, LaserMode::Score);
    assert_eq!(ordered[3].mode, LaserMode::Cut);
}

#[test]
fn test_engraving_precedes_enclosing_cut() {
    // Outer Cut boundary (0,0) to (100, 100)
    let cut = CamContour::new(
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
            Point2::new(0.0, 100.0),
        ],
        true,
        1,
        "Cut".to_string(),
        LaserMode::Cut,
    );

    // Engraving line inside the cut boundary: (30, 50) to (70, 50)
    let engrave = CamContour::new(
        vec![Point2::new(30.0, 50.0), Point2::new(70.0, 50.0)],
        false,
        2,
        "Engrave".to_string(),
        LaserMode::Engrave,
    );

    let contours = vec![cut, engrave];
    let ordered = optimize_cutting_order(contours, Point2::ZERO, true, true);

    // Engraving MUST be executed before cutting to avoid workpiece shift!
    assert_eq!(ordered[0].mode, LaserMode::Engrave);
    assert_eq!(ordered[1].mode, LaserMode::Cut);
}

#[test]
fn test_job_plan_generation_and_dialects() {
    let mut doc = Document::default();
    let page = doc.active_page_mut().unwrap();

    let r = core_doc::Shape::rectangle(
        core_doc::ShapeId(1),
        Rect::from_origin_size(Point2::new(10.0, 10.0), 50.0, 30.0),
        0.0,
    );
    page.layers[0].shapes.push(r);

    let config = LaserProductionConfig::default();
    let plan = generate_job_plan(&doc, &config);

    assert_eq!(plan.toolpaths.len(), 1);
    assert!(plan.total_cut_length_mm > 0.0);
    assert!(plan.estimated_duration_s > 0.0);
    assert!(plan.estimation.total_duration_s > 0.0);

    // Generic G-code
    let gcode = plan.export_gcode();
    assert!(gcode.contains("G21"));
    assert!(gcode.contains("M3"));
    assert!(gcode.contains("M5"));

    // GRBL Dialect with dynamic M4
    let grbl_cfg = MachineConfig {
        flavor: MachineFlavor::Grbl,
        homing_on_start: true,
        dynamic_power: true,
        ..Default::default()
    };
    let grbl = plan.export_dialect(&grbl_cfg).unwrap();
    assert!(grbl.contains("$32=1"));
    assert!(grbl.contains("$H"));
    assert!(grbl.contains("M4 S"));

    // Marlin Dialect
    let marlin_cfg = MachineConfig {
        flavor: MachineFlavor::Marlin,
        ..Default::default()
    };
    let marlin = plan.export_dialect(&marlin_cfg).unwrap();
    assert!(marlin.contains("Marlin 2.0"));
    assert!(marlin.contains("M3 S"));

    // HPGL Dialect
    let plt = plan.export_plt();
    assert!(plt.contains("IN;"));
    assert!(plt.contains("SP1;"));
    assert!(plt.contains("PU"));
    assert!(plt.contains("PD"));
}

#[test]
fn test_object_level_overrides() {
    let mut doc = Document::default();
    let page = doc.active_page_mut().unwrap();

    let s1 = core_doc::Shape::rectangle(
        core_doc::ShapeId(10),
        Rect::from_origin_size(Point2::new(10.0, 10.0), 20.0, 20.0),
        0.0,
    );
    let s2 = core_doc::Shape::rectangle(
        core_doc::ShapeId(20),
        Rect::from_origin_size(Point2::new(50.0, 10.0), 20.0, 20.0),
        0.0,
    );
    page.layers[0].shapes.push(s1);
    page.layers[0].shapes.push(s2);

    let mut config = LaserProductionConfig::default();
    // Default layer is Cut. Override shape 20 to be Score at 60 mm/s
    let mut override_s = LayerLaserSettings::new_score(60.0, 30.0, 2);
    override_s.speed_mm_s = 60.0;
    config.shape_overrides.insert(20, override_s);

    let plan = generate_job_plan(&doc, &config);
    assert_eq!(plan.toolpaths.len(), 2);

    let tp_s2 = plan.toolpaths.iter().find(|t| t.shape_id == 20).unwrap();
    assert_eq!(tp_s2.mode, LaserMode::Score);
    assert_eq!(tp_s2.speed_mm_s, 60.0);
}

#[test]
fn test_simulated_controller_safety_check() {
    let mut doc = Document::default();
    let page = doc.active_page_mut().unwrap();

    // Shape inside bounds
    let s_ok = core_doc::Shape::rectangle(
        core_doc::ShapeId(1),
        Rect::from_origin_size(Point2::new(10.0, 10.0), 50.0, 50.0),
        0.0,
    );
    page.layers[0].shapes.push(s_ok);

    let config = LaserProductionConfig::default();
    let plan = generate_job_plan(&doc, &config);

    let limits = MachineLimits {
        work_area: Rect::new(0.0, 0.0, 600.0, 400.0),
        max_feed_speed_mm_s: 1000.0,
        max_rapid_speed_mm_s: 2000.0,
        max_power_pct: 100.0,
    };
    let controller = SimulatedController::new(&limits);
    let report = controller.validate_job(&plan);

    assert!(report.is_safe);
    assert!(report.issues.is_empty());
}

#[test]
fn test_benchmark_cam_planning_1k_shapes() {
    use std::time::Instant;

    let mut doc = Document::new("CAM Benchmark");
    let page = doc.active_page_mut().unwrap();

    for i in 1..=1_000 {
        let x = (i % 30) as f64 * 15.0;
        let y = (i / 30) as f64 * 15.0;
        let r = core_doc::Shape::rectangle(
            core_doc::ShapeId(i as u64),
            Rect::from_origin_size(Point2::new(x, y), 10.0, 10.0),
            0.0,
        );
        page.layers[0].shapes.push(r);
    }

    let config = LaserProductionConfig {
        optimize_order: true,
        inside_out: true,
        ..Default::default()
    };

    let t0 = Instant::now();
    let plan = generate_job_plan(&doc, &config);
    let duration = t0.elapsed();

    assert_eq!(plan.toolpaths.len(), 1_000);
    println!("CAM Job Plan (1,000 shapes) generated in {:?}", duration);
}
