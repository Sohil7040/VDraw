use core_doc::{
    AddShapeCommand, Color, CombinePathsCommand, Document, GroupCommand, HandleId, PagePreset,
    RemoveShapesCommand, Shape, ShapeId, TransformMode, TransformShapesCommand, UndoStack,
    UngroupCommand, ValidationError,
};
use core_geom::node::{BezierNode, NodeType};
use core_geom::polycurve::Subpath;
use core_geom::{Affine2, Point2, Polycurve, Rect};

#[test]
fn test_document_add_and_remove_shape() {
    let mut doc = Document::new("Test Doc");
    let id = doc.allocate_id();
    let rect = Rect::new(10.0, 10.0, 50.0, 40.0);
    let shape = Shape::rectangle(id, rect, 0.0);

    doc.add_shape_to_active_layer(shape);
    assert!(doc.find_shape(id).is_some());

    let (removed, layer_idx) = doc.remove_shape(id).expect("Should remove");
    assert_eq!(removed.id, id);
    assert_eq!(layer_idx, 0);
    assert!(doc.find_shape(id).is_none());
}

#[test]
fn test_undo_redo_stack() {
    let mut doc = Document::new("Test Doc");
    let mut undo_stack = UndoStack::new();

    let id = doc.allocate_id();
    let shape = Shape::rectangle(id, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0);

    // 1. Add shape via command
    let add_cmd = Box::new(AddShapeCommand::new(shape, 0));
    undo_stack.apply(add_cmd, &mut doc);
    assert!(doc.find_shape(id).is_some());
    assert!(undo_stack.can_undo());
    assert!(!undo_stack.can_redo());

    // 2. Undo addition
    assert!(undo_stack.undo(&mut doc));
    assert!(doc.find_shape(id).is_none());
    assert!(undo_stack.can_redo());

    // 3. Redo addition
    assert!(undo_stack.redo(&mut doc));
    assert!(doc.find_shape(id).is_some());

    // 4. Transform shape via command
    let old_tr = Affine2::IDENTITY;
    let new_tr = Affine2::translate(Point2::new(20.0, 30.0));
    let tr_cmd = Box::new(TransformShapesCommand::new(
        "Move Shape",
        vec![(id, old_tr, new_tr)],
    ));
    undo_stack.apply(tr_cmd, &mut doc);
    assert_eq!(doc.find_shape(id).unwrap().transform, new_tr);

    // Undo transform
    assert!(undo_stack.undo(&mut doc));
    assert_eq!(doc.find_shape(id).unwrap().transform, old_tr);

    // 5. Delete shape via command
    let del_cmd = Box::new(RemoveShapesCommand::new(vec![id]));
    undo_stack.apply(del_cmd, &mut doc);
    assert!(doc.find_shape(id).is_none());

    // Undo delete
    assert!(undo_stack.undo(&mut doc));
    assert!(doc.find_shape(id).is_some());
}

#[test]
fn test_selection_and_transform_mode_toggle() {
    let mut doc = Document::new("Test Doc");
    let id1 = doc.allocate_id();
    let id2 = doc.allocate_id();

    let s1 = Shape::rectangle(id1, Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);
    let s2 = Shape::rectangle(id2, Rect::new(50.0, 50.0, 80.0, 80.0), 0.0);
    doc.add_shape_to_active_layer(s1);
    doc.add_shape_to_active_layer(s2);

    // First click: select single shape in Scale mode
    doc.selection.select_single(id1);
    assert_eq!(doc.selection.len(), 1);
    assert_eq!(doc.selection.transform_mode, TransformMode::Scale);

    // Second click on the same shape: toggle to RotateSkew mode
    doc.selection.select_single(id1);
    assert_eq!(doc.selection.len(), 1);
    assert_eq!(doc.selection.transform_mode, TransformMode::RotateSkew);

    // Third click on the same shape: toggle back to Scale mode
    doc.selection.select_single(id1);
    assert_eq!(doc.selection.transform_mode, TransformMode::Scale);

    // Multiple selection bounds check
    doc.selection.add(id2);
    let b = doc.selection_bounds().expect("Bounds should exist");
    assert_eq!(b, Rect::new(0.0, 0.0, 80.0, 80.0));

    // Handle check
    let handles = doc.selection.handles(b);
    assert_eq!(handles.len(), 8);
    assert_eq!(handles[0].id, HandleId::TopLeft);
    assert_eq!(handles[0].point, Point2::new(0.0, 0.0));
}

#[test]
fn test_shape_hit_test() {
    let rect_shape = Shape::rectangle(ShapeId(1), Rect::new(10.0, 10.0, 50.0, 50.0), 0.0);
    assert!(rect_shape.hit_test(Point2::new(10.0, 20.0), 1.0)); // On edge
    assert!(!rect_shape.hit_test(Point2::new(30.0, 30.0), 1.0)); // Interior (no fill by default)

    let mut filled_rect = rect_shape.clone();
    filled_rect.style.fill = Some(Color::BLACK);
    assert!(filled_rect.hit_test(Point2::new(30.0, 30.0), 1.0)); // Interior with fill hits!

    let ellipse_shape = Shape::ellipse(ShapeId(2), Point2::new(100.0, 100.0), 20.0, 10.0);
    assert!(ellipse_shape.hit_test(Point2::new(120.0, 100.0), 1.0)); // Right apex
    assert!(!ellipse_shape.hit_test(Point2::new(150.0, 100.0), 1.0)); // Far away
}

#[test]
fn test_style_and_modify_kind_commands() {
    let mut doc = Document::new("Style Test");
    let mut undo_stack = UndoStack::new();

    let id = doc.allocate_id();
    let shape = Shape::rectangle(id, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0);
    let old_style = shape.style.clone();
    let mut new_style = old_style.clone();
    new_style.fill = Some(Color::rgb(255, 0, 0));

    doc.add_shape_to_active_layer(shape);

    // 1. Change style
    let cmd = Box::new(core_doc::SetStyleCommand::new(
        id,
        old_style.clone(),
        new_style.clone(),
    ));
    undo_stack.apply(cmd, &mut doc);
    assert_eq!(
        doc.find_shape(id).unwrap().style.fill,
        Some(Color::rgb(255, 0, 0))
    );

    // Undo style
    assert!(undo_stack.undo(&mut doc));
    assert_eq!(doc.find_shape(id).unwrap().style.fill, None);

    // Redo style
    assert!(undo_stack.redo(&mut doc));
    assert_eq!(
        doc.find_shape(id).unwrap().style.fill,
        Some(Color::rgb(255, 0, 0))
    );

    // 2. Convert to path & modify kind
    let old_kind = doc.find_shape(id).unwrap().kind.clone();
    let poly = doc.find_shape(id).unwrap().to_polycurve();
    let new_kind = core_doc::ShapeKind::Path(poly);

    let kind_cmd = Box::new(core_doc::ModifyShapeKindCommand::new(
        id,
        "Convert to Curves",
        old_kind.clone(),
        new_kind.clone(),
    ));
    undo_stack.apply(kind_cmd, &mut doc);
    assert!(matches!(
        doc.find_shape(id).unwrap().kind,
        core_doc::ShapeKind::Path(_)
    ));

    // Undo convert to curves
    assert!(undo_stack.undo(&mut doc));
    assert!(matches!(
        doc.find_shape(id).unwrap().kind,
        core_doc::ShapeKind::Rect { .. }
    ));
}

#[test]
fn test_parametric_shapes() {
    let poly = Shape::polygon(ShapeId(10), Point2::new(50.0, 50.0), 30.0, 6);
    assert_eq!(poly.to_polycurve().subpaths[0].nodes.len(), 6);

    let star = Shape::star(ShapeId(11), Point2::new(50.0, 50.0), 40.0, 20.0, 5);
    assert_eq!(star.to_polycurve().subpaths[0].nodes.len(), 10);

    let spiral = Shape::spiral(ShapeId(12), Point2::new(50.0, 50.0), 2.0, 40.0);
    assert!(spiral.to_polycurve().length() > 30.0);
}

#[test]
fn test_group_and_ungroup_commands() {
    let mut doc = Document::new("Group Test");
    let mut undo = UndoStack::new();

    let s1 = Shape::rectangle(ShapeId(1), Rect::new(0.0, 0.0, 20.0, 20.0), 0.0);
    let s2 = Shape::ellipse(ShapeId(2), Point2::new(50.0, 50.0), 10.0, 10.0);
    doc.add_shape_to_active_layer(s1);
    doc.add_shape_to_active_layer(s2);

    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 2);

    // Group s1 and s2
    let grp_cmd = Box::new(GroupCommand::new(
        vec![ShapeId(1), ShapeId(2)],
        ShapeId(100),
        0,
    ));
    undo.apply(grp_cmd, &mut doc);

    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 1);
    assert!(doc.find_shape(ShapeId(100)).is_some());
    assert!(doc.find_shape(ShapeId(1)).is_none());
    assert!(doc.find_shape(ShapeId(2)).is_none());
    assert!(doc.selection.contains(ShapeId(100)));

    // Ungroup
    let ungrp_cmd = Box::new(UngroupCommand::new(ShapeId(100), 0));
    undo.apply(ungrp_cmd, &mut doc);

    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 2);
    assert!(doc.find_shape(ShapeId(100)).is_none());
    assert!(doc.find_shape(ShapeId(1)).is_some());
    assert!(doc.find_shape(ShapeId(2)).is_some());

    // Undo ungroup
    assert!(undo.undo(&mut doc));
    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 1);
    assert!(doc.find_shape(ShapeId(100)).is_some());

    // Undo group
    assert!(undo.undo(&mut doc));
    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 2);
    assert!(doc.find_shape(ShapeId(1)).is_some());
    assert!(doc.find_shape(ShapeId(2)).is_some());
}

#[test]
fn test_group_bounds_and_hit_test() {
    let mut s1 = Shape::rectangle(ShapeId(1), Rect::new(0.0, 0.0, 20.0, 20.0), 0.0);
    s1.style.fill = Some(Color::BLACK);
    let mut s2 = Shape::rectangle(ShapeId(2), Rect::new(40.0, 40.0, 60.0, 60.0), 0.0);
    s2.style.fill = Some(Color::BLACK);
    let mut grp = Shape::group(ShapeId(10), vec![s1, s2]);

    let b = grp.world_bounds().expect("Group bounds");
    assert!((b.min().x - 0.0).abs() < 1e-2);
    assert!((b.max().x - 60.0).abs() < 1e-2);
    assert!((b.min().y - 0.0).abs() < 1e-2);
    assert!((b.max().y - 60.0).abs() < 1e-2);

    // Hit test s1
    assert!(grp.hit_test(Point2::new(10.0, 10.0), 1.0));
    // Hit test s2
    assert!(grp.hit_test(Point2::new(50.0, 50.0), 1.0));
    // Gap between s1 and s2 should miss
    assert!(!grp.hit_test(Point2::new(30.0, 30.0), 1.0));

    // Translate group
    grp.transform = Affine2::translate(Point2::new(100.0, 100.0));
    let tb = grp.world_bounds().expect("Transformed group bounds");
    assert!((tb.min().x - 100.0).abs() < 1e-2);
    assert!((tb.max().x - 160.0).abs() < 1e-2);
    assert!(grp.hit_test(Point2::new(110.0, 110.0), 1.0));
    assert!(!grp.hit_test(Point2::new(10.0, 10.0), 1.0));
}

#[test]
fn test_combine_and_break_apart_paths() {
    let mut doc = Document::new("Combine Test");
    let mut undo = UndoStack::new();

    let p1 = Shape::path(
        ShapeId(1),
        Polycurve::line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0)),
    );
    let p2 = Shape::path(
        ShapeId(2),
        Polycurve::line(Point2::new(20.0, 0.0), Point2::new(30.0, 0.0)),
    );
    doc.add_shape_to_active_layer(p1);
    doc.add_shape_to_active_layer(p2);

    let comb_cmd = Box::new(CombinePathsCommand::new(
        vec![ShapeId(1), ShapeId(2)],
        ShapeId(3),
        0,
    ));
    undo.apply(comb_cmd, &mut doc);

    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 1);
    let combined = doc.find_shape(ShapeId(3)).unwrap();
    assert_eq!(combined.to_polycurve().subpaths.len(), 2);

    assert!(undo.undo(&mut doc));
    assert_eq!(doc.active_page().unwrap().layers[0].shapes.len(), 2);
}

#[test]
fn test_undo_stack_dirty_tracking() {
    let mut doc = Document::new("Dirty Test");
    let mut undo = UndoStack::new();

    assert!(!undo.is_dirty());

    let s1 = Shape::rectangle(ShapeId(1), Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
    undo.apply(Box::new(AddShapeCommand::new(s1, 0)), &mut doc);
    assert!(undo.is_dirty());

    undo.mark_saved();
    assert!(!undo.is_dirty(), "Should be clean right after save");

    let s2 = Shape::rectangle(ShapeId(2), Rect::new(20.0, 0.0, 30.0, 10.0), 0.0);
    undo.apply(Box::new(AddShapeCommand::new(s2, 0)), &mut doc);
    assert!(undo.is_dirty(), "Dirty after second command");

    undo.undo(&mut doc);
    assert!(
        !undo.is_dirty(),
        "Should be clean again after undoing to save point"
    );

    undo.redo(&mut doc);
    assert!(undo.is_dirty(), "Dirty again after redo past save point");
}

#[test]
fn test_document_validation_errors() {
    let mut doc = Document::new("Validation Test");

    // Valid document should pass validation
    let report = doc.validate();
    assert!(report.is_valid());

    // 1. Duplicate ID check
    let s1 = Shape::rectangle(ShapeId(1), Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
    let s2 = Shape::rectangle(ShapeId(1), Rect::new(20.0, 0.0, 30.0, 10.0), 0.0); // Duplicate ID 1
    doc.add_shape_to_active_layer(s1);
    doc.add_shape_to_active_layer(s2);

    let report = doc.validate();
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::DuplicateShapeId(ShapeId(1)))));

    // 2. NaN coordinate check
    let mut nan_poly = Polycurve::new();
    let mut sub = Subpath::new(true);
    sub.push_node(BezierNode::new(Point2::new(f64::NAN, 0.0), NodeType::Cusp));
    nan_poly.subpaths.push(sub);
    let s3 = Shape::path(ShapeId(3), nan_poly);
    doc.add_shape_to_active_layer(s3);

    let report = doc.validate();
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::NonFiniteCoordinate(ShapeId(3), _))));
}

#[test]
fn test_page_presets() {
    assert_eq!(PagePreset::A4.dimensions_mm(), Some((210.0, 297.0)));
    assert_eq!(
        PagePreset::LaserBedMedium.dimensions_mm(),
        Some((600.0, 400.0))
    );
    assert_eq!(PagePreset::Custom.dimensions_mm(), None);
}
