use core_boolean::convert::polygon_area;
use core_boolean::{BooleanOp, ContourJoin};
use core_doc::{Document, Page, Shape, ShapeId, ShapeKind, UndoStack};
use core_geom::{Point2, Rect};
use core_snap::contracts::{HandleMode, SnapCtx, SnapKind, SnapMask, View};
use core_tools::{
    AppCommand, CommandError, DispatchCtx, Dispatcher, DragSession, DragState, SnapController,
    ThemePref, ToolId, ToolManager,
};

fn make_test_view() -> View {
    View::new(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 1000.0)),
        egui::Vec2::ZERO,
        2.0, // 2 screen px per mm
    )
}

fn make_doc_with_rect_and_line() -> (Document, ShapeId, ShapeId) {
    let mut doc = Document::new("Test");
    let rect_id = doc.allocate_id();
    let line_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        rect_id,
        Rect::new(10.0, 10.0, 50.0, 40.0),
        0.0,
    ));
    doc.add_shape_to_active_layer(Shape::line(
        line_id,
        Point2::new(60.0, 20.0),
        Point2::new(120.0, 80.0),
    ));
    (doc, rect_id, line_id)
}

fn shape_area(shape: &Shape) -> f64 {
    let poly = shape.to_polycurve().transform(&shape.transform);
    poly.subpaths
        .iter()
        .map(|sub| polygon_area(&sub.flatten(0.02)))
        .sum::<f64>()
        .abs()
}

fn doc_pages_snapshot(doc: &Document) -> Vec<Page> {
    doc.pages.clone()
}

#[test]
fn test_dispatch_covers_every_app_command_variant() {
    let all_commands = vec![
        AppCommand::NewDocument,
        AppCommand::OpenDocument,
        AppCommand::SaveDocument,
        AppCommand::SaveDocumentAs,
        AppCommand::ImportSvg,
        AppCommand::ExportSvg,
        AppCommand::ExportDxf,
        AppCommand::ExportPng,
        AppCommand::ExportGcode,
        AppCommand::Undo,
        AppCommand::Redo,
        AppCommand::JumpToHistoryIndex(0),
        AppCommand::DuplicateSelection,
        AppCommand::DeleteSelection,
        AppCommand::SelectAll,
        AppCommand::DeselectAll,
        AppCommand::ActivateTool(ToolId::Pick),
        AppCommand::ActivateTool(ToolId::Rectangle),
        AppCommand::TranslateSelection {
            delta: Point2::new(5.0, -3.0),
        },
        AppCommand::RotateSelection {
            pivot: None,
            angle_deg: 30.0,
        },
        AppCommand::MirrorSelectionHorizontal,
        AppCommand::MirrorSelectionVertical,
        AppCommand::MirrorSelectionLine {
            point: Point2::ZERO,
            dir: Point2::new(1.0, 1.0),
        },
        AppCommand::ScaleSelection {
            pivot: None,
            sx: 1.5,
            sy: 0.75,
        },
        AppCommand::BooleanOp(BooleanOp::Weld),
        AppCommand::BooleanOp(BooleanOp::Trim),
        AppCommand::BooleanOp(BooleanOp::Intersect),
        AppCommand::BooleanOp(BooleanOp::Exclude),
        AppCommand::BooleanOp(BooleanOp::FrontMinusBack),
        AppCommand::BooleanOp(BooleanOp::BackMinusFront),
        AppCommand::BooleanOp(BooleanOp::CreateBoundary),
        AppCommand::BooleanOp(BooleanOp::Simplify),
        AppCommand::OffsetSelection {
            distance_mm: 2.0,
            join: ContourJoin::Round,
        },
        AppCommand::ConvertTextToPath {
            text: "VDraw".to_string(),
            size_mm: 12.0,
        },
        AppCommand::GroupSelection,
        AppCommand::UngroupSelection,
        AppCommand::CombinePaths,
        AppCommand::BreakApartPaths,
        AppCommand::ToggleSnapMaster,
        AppCommand::ToggleSnapKind(SnapKind::Node),
        AppCommand::SetHandleMode(HandleMode::Center),
        AppCommand::ToggleSnapOrtho,
        AppCommand::ToggleSnapPolar,
        AppCommand::SetPolarStepDeg(30.0),
        AppCommand::ToggleAlignmentGuides,
        AppCommand::ToggleGuidelineSnap,
        AppCommand::ClearGuidelines,
        AppCommand::ZoomIn,
        AppCommand::ZoomOut,
        AppCommand::FitPage,
        AppCommand::ZoomToSelection,
        AppCommand::ToggleGrid,
        AppCommand::ToggleRulers,
        AppCommand::SetTheme(ThemePref::default()),
        AppCommand::SwitchWorkflowDesign,
        AppCommand::SwitchWorkflowLaserCam,
        AppCommand::SwitchWorkflowPreview,
        AppCommand::OpenCommandPalette,
        AppCommand::OpenHelp,
        AppCommand::StartCoachMarks,
    ];

    for select_all in [false, true] {
        for cmd in &all_commands {
            let (mut doc, _rect_id, _line_id) = make_doc_with_rect_and_line();
            if select_all {
                for s in doc
                    .active_page()
                    .unwrap()
                    .all_shapes()
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    doc.selection.add(s.id);
                }
            }
            let mut undo = UndoStack::new();
            let mut snap = SnapController::default();
            snap.update_index(&doc);
            let mut tools = ToolManager::default();
            let mut dispatcher = Dispatcher::new();
            let mut ctx = DispatchCtx::new(&mut snap, &mut tools);

            let res = dispatcher.dispatch(cmd.clone(), &mut doc, &mut undo, &mut ctx);
            match res {
                Ok(()) => {}
                Err(
                    CommandError::Unsupported
                    | CommandError::NoSelection
                    | CommandError::TransformFailed(_)
                    | CommandError::BooleanFailed(_)
                    | CommandError::InvalidParameters(_)
                    | CommandError::Cancelled,
                ) => {}
            }
        }
    }
}

#[test]
fn test_dispatch_transform_rotate_mutates_document_and_pushes_undo() {
    let (mut doc, rect_id, _line_id) = make_doc_with_rect_and_line();
    doc.selection.select(rect_id);
    let initial_transform = doc.find_shape(rect_id).unwrap().transform;

    let mut undo = UndoStack::new();
    let mut snap = SnapController::default();
    let mut tools = ToolManager::default();
    let mut dispatcher = Dispatcher::new();
    let mut ctx = DispatchCtx::new(&mut snap, &mut tools);

    dispatcher
        .dispatch(
            AppCommand::RotateSelection {
                pivot: None,
                angle_deg: 45.0,
            },
            &mut doc,
            &mut undo,
            &mut ctx,
        )
        .expect("rotate dispatch should succeed");

    let rotated_transform = doc.find_shape(rect_id).unwrap().transform;
    assert_ne!(rotated_transform, initial_transform);

    let history = undo.history();
    assert_eq!(history.len(), 1);
    assert!(history[0].is_applied);
    assert!(!history[0].label.is_empty());
    assert_ne!(history[0].label, "Transform");
    assert!(history[0].label.contains("45.0"));

    // Undo and redo restore and reapply exact transforms
    assert!(undo.undo(&mut doc));
    assert_eq!(
        doc.find_shape(rect_id).unwrap().transform,
        initial_transform
    );
    assert!(undo.redo(&mut doc));
    assert_eq!(
        doc.find_shape(rect_id).unwrap().transform,
        rotated_transform
    );
}

#[test]
fn test_dispatch_boolean_union_produces_valid_shape() {
    let mut doc = Document::new("BoolUnion");
    let id1 = doc.allocate_id();
    let id2 = doc.allocate_id();
    let rect1 = Shape::rectangle(id1, Rect::new(0.0, 0.0, 20.0, 20.0), 0.0);
    let rect2 = Shape::rectangle(id2, Rect::new(10.0, 0.0, 30.0, 20.0), 0.0);
    let area1 = shape_area(&rect1);
    let area2 = shape_area(&rect2);
    doc.add_shape_to_active_layer(rect1);
    doc.add_shape_to_active_layer(rect2);
    doc.selection.add(id1);
    doc.selection.add(id2);

    let mut undo = UndoStack::new();
    let mut snap = SnapController::default();
    let mut tools = ToolManager::default();
    let mut dispatcher = Dispatcher::new();
    let mut ctx = DispatchCtx::new(&mut snap, &mut tools);

    dispatcher
        .dispatch(
            AppCommand::BooleanOp(BooleanOp::Weld),
            &mut doc,
            &mut undo,
            &mut ctx,
        )
        .expect("weld boolean should succeed");

    let shapes: Vec<&Shape> = doc.active_page().unwrap().all_shapes().collect();
    assert_eq!(shapes.len(), 1);
    let union_area = shape_area(shapes[0]);
    assert!(union_area > area1);
    assert!(union_area > area2);
    assert!((union_area - 600.0).abs() < 1.0);
}

#[test]
fn test_dispatch_snap_toggle_flips_mask_bit() {
    let (mut doc, _rect_id, _line_id) = make_doc_with_rect_and_line();
    let mut undo = UndoStack::new();
    let mut snap = SnapController::new(SnapMask::NODE | SnapMask::CENTER);
    let mut tools = ToolManager::default();
    let mut dispatcher = Dispatcher::new();
    let mut ctx = DispatchCtx::new(&mut snap, &mut tools);

    assert!(ctx.snap_controller.mask.contains(SnapMask::NODE));
    dispatcher
        .dispatch(
            AppCommand::ToggleSnapKind(SnapKind::Node),
            &mut doc,
            &mut undo,
            &mut ctx,
        )
        .unwrap();
    assert!(!ctx.snap_controller.mask.contains(SnapMask::NODE));

    dispatcher
        .dispatch(
            AppCommand::ToggleSnapKind(SnapKind::Node),
            &mut doc,
            &mut undo,
            &mut ctx,
        )
        .unwrap();
    assert!(ctx.snap_controller.mask.contains(SnapMask::NODE));
}

#[test]
fn test_drag_state_transitions_all_paths() {
    let view = make_test_view();
    let mut doc = Document::new("DragPaths");
    let mover_id = doc.allocate_id();
    let target_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        mover_id,
        Rect::new(0.0, 0.0, 20.0, 20.0),
        0.0,
    ));
    doc.add_shape_to_active_layer(Shape::rectangle(
        target_id,
        Rect::new(100.0, 100.0, 120.0, 120.0),
        0.0,
    ));

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);
    let mut undo = UndoStack::new();
    let mut session = DragSession::new();

    // Path 1: Idle -> Armed -> Idle (click without >3 px movement)
    assert!(matches!(session.state, DragState::Idle));
    let start_screen = view.world_to_screen(Point2::new(0.0, 0.0));
    session.arm(
        HandleMode::Node,
        mover_id,
        Point2::new(0.0, 0.0),
        start_screen,
    );
    assert!(matches!(session.state, DragState::Armed { .. }));
    session.update(
        egui::pos2(start_screen.x + 2.0, start_screen.y + 1.0),
        &view,
        10.0,
        snap.get_engine(),
        &doc,
    );
    assert!(matches!(session.state, DragState::Armed { .. }));
    let after_click = session
        .release(
            egui::pos2(start_screen.x + 2.0, start_screen.y + 1.0),
            &view,
            10.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();
    assert!(matches!(after_click, DragState::Idle));
    assert!(!undo.can_undo());

    // Path 2: Idle -> Armed -> Dragging -> Cancelled
    session.arm(
        HandleMode::Node,
        mover_id,
        Point2::new(0.0, 0.0),
        start_screen,
    );
    let drag_screen = view.world_to_screen(Point2::new(100.0, 100.0));
    session.update(drag_screen, &view, 10.0, snap.get_engine(), &doc);
    assert!(matches!(session.state, DragState::Dragging { .. }));
    assert!(session.preview_shape().is_some());
    session.cancel();
    assert!(matches!(session.state, DragState::Cancelled));
    assert!(session.preview_shape().is_none());
    assert!(!undo.can_undo());

    // Path 3: Idle -> Armed -> Dragging -> Committed
    session.arm(
        HandleMode::Node,
        mover_id,
        Point2::new(0.0, 0.0),
        start_screen,
    );
    session.update(drag_screen, &view, 10.0, snap.get_engine(), &doc);
    assert!(matches!(session.state, DragState::Dragging { .. }));
    let committed = session
        .release(
            drag_screen,
            &view,
            10.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();
    assert!(matches!(committed, DragState::Committed));
    assert!(session.preview_shape().is_none());
    assert!(undo.can_undo());
}

#[test]
fn test_drag_preview_does_not_mutate_document() {
    let view = make_test_view();
    let mut doc = Document::new("PreviewNoMutate");
    let mover_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        mover_id,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        0.0,
    ));

    let mut target_points = Vec::new();
    for i in 1..=20 {
        let p = Point2::new(i as f64 * 15.0, i as f64 * 10.0);
        let id = doc.allocate_id();
        doc.add_shape_to_active_layer(Shape::rectangle(
            id,
            Rect::new(p.x, p.y, p.x + 8.0, p.y + 8.0),
            0.0,
        ));
        target_points.push(p);
    }

    let snapshot_pages = doc_pages_snapshot(&doc);
    let snapshot_next_id = doc.next_shape_id;

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);
    let mut undo = UndoStack::new();
    let mut session = DragSession::new();

    let start_screen = view.world_to_screen(Point2::ZERO);
    session.arm(HandleMode::Node, mover_id, Point2::ZERO, start_screen);

    for pt in &target_points {
        let screen = view.world_to_screen(*pt);
        let state = session.update(screen, &view, 12.0, snap.get_engine(), &doc);
        assert!(matches!(state, DragState::Dragging { .. }));
        assert!(session.preview_shape().is_some());
        assert_eq!(doc.pages, snapshot_pages);
        assert_eq!(doc.next_shape_id, snapshot_next_id);
        assert!(!undo.can_undo());
    }

    let final_screen = view.world_to_screen(target_points[19]);
    let res = session
        .release(
            final_screen,
            &view,
            12.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();
    assert!(matches!(res, DragState::Committed));
    assert_ne!(doc.pages, snapshot_pages);
}

#[test]
fn test_drag_commit_pushes_undo_entry_with_description() {
    let view = make_test_view();
    let mut doc = Document::new("DragUndoDesc");
    let mover_id = doc.allocate_id();
    let target_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        mover_id,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        0.0,
    ));
    doc.add_shape_to_active_layer(Shape::rectangle(
        target_id,
        Rect::new(25.0, -10.0, 40.0, 5.0),
        0.0,
    ));

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);
    let mut undo = UndoStack::new();
    let mut session = DragSession::new();

    let start_screen = view.world_to_screen(Point2::ZERO);
    let dest_screen = view.world_to_screen(Point2::new(25.0, -10.0));
    session.arm(HandleMode::Node, mover_id, Point2::ZERO, start_screen);
    session.update(dest_screen, &view, 10.0, snap.get_engine(), &doc);
    session
        .release(
            dest_screen,
            &view,
            10.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();

    let desc = undo.top_description().expect("expected undo description");
    assert!(!desc.is_empty());
    assert_ne!(desc, "Transform");
    assert!(desc.starts_with("Move Δ="));
}

#[test]
fn test_drag_release_without_snap_does_not_commit() {
    let view = make_test_view();
    let mut doc = Document::new("DragNoSnap");
    let mover_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        mover_id,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        0.0,
    ));
    let snapshot_pages = doc_pages_snapshot(&doc);

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);
    let mut undo = UndoStack::new();
    let mut session = DragSession::new();

    let start_screen = view.world_to_screen(Point2::ZERO);
    let empty_space_screen = view.world_to_screen(Point2::new(250.0, 250.0));
    session.arm(HandleMode::Node, mover_id, Point2::ZERO, start_screen);
    session.update(empty_space_screen, &view, 8.0, snap.get_engine(), &doc);
    assert!(matches!(
        session.state,
        DragState::Dragging { snap: None, .. }
    ));

    let released = session
        .release(
            empty_space_screen,
            &view,
            8.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();
    assert!(matches!(released, DragState::Idle));
    assert_eq!(doc.pages, snapshot_pages);
    assert_eq!(undo.history().len(), 0);
}

#[test]
fn test_snap_controller_mask_toggle_changes_resolution() {
    let view = make_test_view();
    let mut doc = Document::new("SnapMaskToggle");
    let id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(id, Rect::new(20.0, 20.0, 60.0, 60.0), 0.0));

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);

    let vertex_screen = view.world_to_screen(Point2::new(20.0, 20.0));
    let ctx = SnapCtx {
        cursor_screen: vertex_screen,
        view: &view,
        threshold_px: 10.0,
    };
    let hit = snap.get_engine().resolve(&ctx);
    assert!(matches!(hit, Some((pt, _)) if pt.kind == SnapKind::Node));

    snap.toggle_bit(SnapMask::NODE);
    assert!(!snap.mask.contains(SnapMask::NODE));
    let miss = snap.get_engine().resolve(&ctx);
    assert_eq!(miss, None);

    snap.set_mask(SnapMask::CENTER);
    let center_miss_at_vertex = snap.get_engine().resolve(&ctx);
    assert!(center_miss_at_vertex.is_none());

    snap.set_mask(SnapMask::empty());
    assert_eq!(snap.get_engine().resolve(&ctx), None);
}

#[test]
fn test_snap_controller_provider_table_is_exhaustive() {
    let bits = [
        SnapMask::NODE,
        SnapMask::ENDPOINT,
        SnapMask::MIDPOINT,
        SnapMask::CENTER,
        SnapMask::QUADRANT,
        SnapMask::INTERSECTION,
        SnapMask::GRID,
        SnapMask::ORTHO,
        SnapMask::POLAR,
    ];

    let mut snap = SnapController::new(SnapMask::empty());
    let base_contextual_count = snap.provider_count();

    for bit in bits {
        snap.set_mask(bit);
        assert!(
            snap.provider_count() > base_contextual_count,
            "expected table-driven provider for {bit:?}"
        );
        assert!(!snap.active_providers().is_empty());
    }

    // NODE | ENDPOINT deduplicates NodeSnapProvider to a single instance
    snap.set_mask(SnapMask::NODE);
    let node_only_count = snap.provider_count();
    snap.set_mask(SnapMask::NODE | SnapMask::ENDPOINT);
    assert_eq!(snap.provider_count(), node_only_count);
}

#[test]
fn test_e2e_snap_drag_commit_undo_redo() {
    let view = make_test_view();
    let mut doc = Document::new("E2ESnapDrag");
    let mover_id = doc.allocate_id();
    let target_id = doc.allocate_id();
    doc.add_shape_to_active_layer(Shape::rectangle(
        mover_id,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        0.0,
    ));
    doc.add_shape_to_active_layer(Shape::rectangle(
        target_id,
        Rect::new(80.0, 50.0, 100.0, 70.0),
        0.0,
    ));

    let original_bounds = doc.find_shape(mover_id).unwrap().world_bounds().unwrap();

    let mut snap = SnapController::new(SnapMask::NODE);
    snap.update_index(&doc);
    let mut undo = UndoStack::new();
    let mut session = DragSession::new();

    let anchor = Point2::new(0.0, 0.0);
    let target_node = Point2::new(80.0, 50.0);
    let start_screen = view.world_to_screen(anchor);
    let end_screen = view.world_to_screen(target_node);

    session.arm(HandleMode::Node, mover_id, anchor, start_screen);
    session.update(end_screen, &view, 10.0, snap.get_engine(), &doc);
    let committed = session
        .release(
            end_screen,
            &view,
            10.0,
            snap.get_engine(),
            &mut doc,
            &mut undo,
        )
        .unwrap();
    assert!(matches!(committed, DragState::Committed));

    let moved_bounds = doc.find_shape(mover_id).unwrap().world_bounds().unwrap();
    assert!((moved_bounds.min_x - 80.0).abs() < 1e-9);
    assert!((moved_bounds.min_y - 50.0).abs() < 1e-9);
    assert!((moved_bounds.max_x - 90.0).abs() < 1e-9);
    assert!((moved_bounds.max_y - 60.0).abs() < 1e-9);

    // Undo restores original shape bounds
    assert!(undo.undo(&mut doc));
    let undone_bounds = doc.find_shape(mover_id).unwrap().world_bounds().unwrap();
    assert_eq!(undone_bounds, original_bounds);

    // Redo reapplies snapped transform
    assert!(undo.redo(&mut doc));
    let redone_bounds = doc.find_shape(mover_id).unwrap().world_bounds().unwrap();
    assert!((redone_bounds.min_x - 80.0).abs() < 1e-9);
    assert!((redone_bounds.min_y - 50.0).abs() < 1e-9);
}

#[test]
fn test_mirror_transform_record_preserves_winding_and_restores_kind_on_undo() {
    let (mut doc, rect_id, _line_id) = make_doc_with_rect_and_line();
    doc.selection.select(rect_id);
    let orig_shape = doc.find_shape(rect_id).unwrap().clone();
    assert!(matches!(orig_shape.kind, ShapeKind::Rect { .. }));
    let orig_signed_area = polygon_area(&orig_shape.to_polycurve().subpaths[0].flatten(0.02));

    let mut undo = UndoStack::new();
    let mut snap = SnapController::default();
    let mut tools = ToolManager::default();
    let mut dispatcher = Dispatcher::new();
    let mut ctx = DispatchCtx::new(&mut snap, &mut tools);

    dispatcher
        .dispatch(
            AppCommand::MirrorSelectionHorizontal,
            &mut doc,
            &mut undo,
            &mut ctx,
        )
        .expect("mirror horizontal should succeed");

    let mirrored = doc.find_shape(rect_id).unwrap();
    assert!(matches!(mirrored.kind, ShapeKind::Path(_)));
    let mirrored_poly = mirrored.to_polycurve().transform(&mirrored.transform);
    let mirrored_signed_area = polygon_area(&mirrored_poly.subpaths[0].flatten(0.02));
    assert!(
        orig_signed_area.signum() == mirrored_signed_area.signum(),
        "mirroring must preserve contour winding sign: before={orig_signed_area}, after={mirrored_signed_area}"
    );
    assert_eq!(undo.top_description(), Some("Mirror Horizontal"));

    // Undo restores exact ShapeKind::Rect and Affine2
    assert!(undo.undo(&mut doc));
    let restored = doc.find_shape(rect_id).unwrap();
    assert_eq!(restored.kind, orig_shape.kind);
    assert_eq!(restored.transform, orig_shape.transform);
}
