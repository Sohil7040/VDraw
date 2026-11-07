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

    // Optimize cutting order with inside_out = true
    let ordered = optimize_cutting_order(contours, Point2::ZERO, true, true);

    // Inner hole (depth 1) MUST be cut before outer boundary (depth 0)!
    assert_eq!(ordered[0].shape_id, 2);
    assert_eq!(ordered[1].shape_id, 1);
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
fn test_job_plan_generation_and_gcode_export() {
    let mut doc = Document::default();
    let page = doc.active_page_mut().unwrap();

    // Add a cut rectangle
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

    let gcode = plan.export_gcode();
    assert!(gcode.contains("G21"));
    assert!(gcode.contains("M3"));
    assert!(gcode.contains("M5"));

    let plt = plan.export_plt();
    assert!(plt.contains("IN;"));
    assert!(plt.contains("SP1;"));
    assert!(plt.contains("PU"));
    assert!(plt.contains("PD"));
}
