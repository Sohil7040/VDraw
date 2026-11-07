use core_doc::{
    AddShapeCommand, Color, Document, HandleId, RemoveShapesCommand, Shape, ShapeId, TransformMode,
    TransformShapesCommand, UndoStack,
};
use core_geom::{Affine2, Point2, Rect};

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
