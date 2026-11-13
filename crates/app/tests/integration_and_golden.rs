//! Full-app headless acceptance test (`[F-E1]`) and `insta` + `tiny-skia` golden image tests.

mod golden;

use app::app::VDrawApp;
use app::canvas::{hud_style, HudStrokeStyle};
use app::tokens::ThemeTokens;
use core_doc::{Document, Shape, ShapeId};
use core_geom::{Point2, Rect};
use core_snap::contracts::{SnapCtx, SnapKind, View};
use core_tools::AppCommand;
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, RawInput};
use std::path::PathBuf;
use tiny_skia::Pixmap;

fn test_raw_input(screen_rect: egui::Rect, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(screen_rect),
        events,
        ..Default::default()
    }
}

/// Locates the rendered white A4 page rectangle (`tokens.color.page_bg`) inside egui's output shapes
/// so pointer events map deterministically from world millimetres to screen coordinates.
fn find_page_screen_rect(
    shapes: &[egui::epaint::ClippedShape],
    page_bg: Color32,
) -> Option<egui::Rect> {
    fn search_shape(shape: &egui::epaint::Shape, page_bg: Color32) -> Option<egui::Rect> {
        match shape {
            egui::epaint::Shape::Rect(r) if r.fill == page_bg && r.rect.width() > 50.0 => {
                Some(r.rect)
            }
            egui::epaint::Shape::Vec(items) => items.iter().find_map(|s| search_shape(s, page_bg)),
            _ => None,
        }
    }
    shapes
        .iter()
        .find_map(|cs| search_shape(&cs.shape, page_bg))
}

fn world_mm_to_screen(page_rect: egui::Rect, world_mm: Point2) -> Pos2 {
    const A4_W_MM: f32 = 210.0;
    const A4_H_MM: f32 = 297.0;
    Pos2::new(
        page_rect.min.x + (world_mm.x as f32 / A4_W_MM) * page_rect.width(),
        page_rect.min.y + (world_mm.y as f32 / A4_H_MM) * page_rect.height(),
    )
}

fn shapes_contain_snap_hud(
    shapes: &[egui::epaint::ClippedShape],
    expected_pos: Pos2,
    snap_color: Color32,
) -> bool {
    fn search(shape: &egui::epaint::Shape, expected_pos: Pos2, snap_color: Color32) -> bool {
        match shape {
            egui::epaint::Shape::Circle(c) => {
                c.center.distance(expected_pos) <= 4.0
                    && (c.fill == snap_color || c.stroke.color == snap_color)
            }
            egui::epaint::Shape::Vec(items) => {
                items.iter().any(|s| search(s, expected_pos, snap_color))
            }
            _ => false,
        }
    }
    shapes
        .iter()
        .any(|cs| search(&cs.shape, expected_pos, snap_color))
}

/// `[F-E1]` Full-app headless acceptance test:
/// Drives `VDrawApp::new_headless` + `render_ui` via real `egui::RawInput` pointer press,
/// 24 intermediate pointer moves (crossing snap targets), pointer release commit,
/// `SnapFeedback` / `hud_style` HUD verification, `UndoStack::top_description()`, undo, and redo.
#[test]
fn test_full_app_headless_snap_drag_commit_undo_redo() {
    let ctx = egui::Context::default();
    let screen_rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0));
    let mut app = VDrawApp::new_headless(&ctx);

    // Load a Document containing two rectangles and one line.
    let mut doc = Document::new("Acceptance E2E");
    let mover_id = ShapeId(1);
    let target_id = ShapeId(2);
    let line_id = ShapeId(3);
    let mut mover = Shape::rectangle(mover_id, Rect::new(20.0, 20.0, 50.0, 40.0), 0.0);
    mover.style.fill = Some(core_doc::Color::rgba(60, 120, 220, 160));
    doc.add_shape_to_active_layer(mover);
    doc.add_shape_to_active_layer(Shape::rectangle(
        target_id,
        Rect::new(100.0, 80.0, 150.0, 120.0),
        0.0,
    ));
    doc.add_shape_to_active_layer(Shape::line(
        line_id,
        Point2::new(65.0, 55.0),
        Point2::new(150.0, 55.0),
    ));
    app.load_document(doc);

    let initial_bounds = app
        .doc()
        .find_shape(mover_id)
        .and_then(|s| s.world_bounds())
        .expect("mover shape initial bounds");

    // Warm up two frames so egui panel sizes settle before reading `page_rect`.
    let _ = ctx.run(test_raw_input(screen_rect, Vec::new()), |ctx| {
        app.render_ui(ctx);
    });
    let warmup_out = ctx.run(test_raw_input(screen_rect, Vec::new()), |ctx| {
        app.render_ui(ctx);
    });
    let tokens = ThemeTokens::from_ctx(&ctx);
    let page_rect = find_page_screen_rect(&warmup_out.shapes, tokens.color.page_bg)
        .expect("page_bg rect must be rendered on canvas");

    // Drag mover rectangle from its interior body (29, 28) across the line endpoint (65, 55)
    // onto target rectangle corner (100, 80) (mover anchor is its center (35, 30)).
    let start_world = Point2::new(29.0, 28.0);
    let mid_snap_world = Point2::new(65.0, 55.0);
    let end_world = Point2::new(100.0, 80.0);
    let start_screen = world_mm_to_screen(page_rect, start_world);
    let end_screen = world_mm_to_screen(page_rect, end_world);

    // Move pointer onto mover shape body so `resp.hovered()` is active.
    let _ = ctx.run(
        test_raw_input(screen_rect, vec![Event::PointerMoved(start_screen)]),
        |ctx| app.render_ui(ctx),
    );

    // Primary pointer press -> arms DragSession on mover_id.
    let _ = ctx.run(
        test_raw_input(
            screen_rect,
            vec![
                Event::PointerMoved(start_screen),
                Event::PointerButton {
                    pos: start_screen,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
        ),
        |ctx| app.render_ui(ctx),
    );

    // >= 20 intermediate PointerMoved frames, passing near line endpoint (65, 55) and
    // target rectangle corner (100, 80).
    let total_move_frames = 24usize;
    let mut snap_frames_seen = 0usize;
    let mut rendered_hud_frames = 0usize;

    for step in 1..=total_move_frames {
        let pos_world = if step <= 12 {
            let t = (step as f64) / 12.0;
            start_world.lerp(mid_snap_world, t)
        } else {
            let t = ((step - 12) as f64) / 12.0;
            mid_snap_world.lerp(end_world, t)
        };
        let pos_screen = world_mm_to_screen(page_rect, pos_world);

        let out = ctx.run(
            test_raw_input(screen_rect, vec![Event::PointerMoved(pos_screen)]),
            |ctx| app.render_ui(ctx),
        );

        if let Some(fb) = app.last_snap_feedback().cloned() {
            snap_frames_seen += 1;
            let style = hud_style(fb.target.kind);
            assert!(
                style.filled_dot || style.is_contextual_hint,
                "hud_style must classify active vs contextual snap feedback"
            );
            let snap_col = tokens.color.snap_color(fb.target.kind);
            if shapes_contain_snap_hud(&out.shapes, fb.screen_pos, snap_col) {
                rendered_hud_frames += 1;
            }
        }
    }

    assert!(
        snap_frames_seen >= 3,
        "Expected >= 3 snap frames during drag sweep, observed {snap_frames_seen}; page_rect={page_rect:?}, drag_state={:?}, mask={:?}",
        app.drag_session().state,
        app.snap_controller().mask
    );
    assert!(
        rendered_hud_frames >= 3,
        "Expected >= 3 frames with rendered SnapFeedback HUD shapes, observed {rendered_hud_frames}"
    );

    // Release primary button at target rectangle corner (100, 80) -> commits drag.
    let _ = ctx.run(
        test_raw_input(
            screen_rect,
            vec![
                Event::PointerMoved(end_screen),
                Event::PointerButton {
                    pos: end_screen,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        ),
        |ctx| app.render_ui(ctx),
    );

    // Assert document state updated accurately: mover center snapped to (100, 80).
    let moved_bounds = app
        .doc()
        .find_shape(mover_id)
        .and_then(|s| s.world_bounds())
        .expect("mover bounds after drag commit");
    assert!(
        (moved_bounds.min_x - 85.0).abs() < 1e-6 && (moved_bounds.max_x - 115.0).abs() < 1e-6,
        "Expected mover X bounds [85, 115], got [{}, {}]",
        moved_bounds.min_x,
        moved_bounds.max_x
    );
    assert!(
        (moved_bounds.min_y - 70.0).abs() < 1e-6 && (moved_bounds.max_y - 90.0).abs() < 1e-6,
        "Expected mover Y bounds [70, 90], got [{}, {}]",
        moved_bounds.min_y,
        moved_bounds.max_y
    );

    // Assert UndoStack::top_description() is populated and != "Transform".
    let desc = app
        .undo_stack()
        .top_description()
        .expect("undo stack must contain committed drag command");
    assert!(!desc.is_empty());
    assert_ne!(desc, "Transform");

    // Undo restores exact original state.
    app.dispatch_command(AppCommand::Undo)
        .expect("undo command must succeed");
    let undone_bounds = app
        .doc()
        .find_shape(mover_id)
        .and_then(|s| s.world_bounds())
        .expect("mover bounds after undo");
    assert!(
        (undone_bounds.min_x - initial_bounds.min_x).abs() < 1e-9
            && (undone_bounds.min_y - initial_bounds.min_y).abs() < 1e-9
            && (undone_bounds.max_x - initial_bounds.max_x).abs() < 1e-9
            && (undone_bounds.max_y - initial_bounds.max_y).abs() < 1e-9,
        "Undo must restore exact initial bounds"
    );

    // Redo reapplies exact target state.
    app.dispatch_command(AppCommand::Redo)
        .expect("redo command must succeed");
    let redone_bounds = app
        .doc()
        .find_shape(mover_id)
        .and_then(|s| s.world_bounds())
        .expect("mover bounds after redo");
    assert!(
        (redone_bounds.min_x - moved_bounds.min_x).abs() < 1e-9
            && (redone_bounds.min_y - moved_bounds.min_y).abs() < 1e-9,
        "Redo must reapply exact committed bounds"
    );
}

// ─── Golden Image Tests (`insta` + `tiny-skia`) ──────────────────────────────

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("__snapshots__")
}

fn verify_golden_pixmap(name: &str, pixmap: &Pixmap, extra_metadata: &str) {
    assert_eq!(pixmap.width(), golden::GOLDEN_WIDTH);
    assert_eq!(pixmap.height(), golden::GOLDEN_HEIGHT);

    let png_bytes = pixmap.encode_png().expect("encode golden PNG");
    let png_path = snapshot_dir().join(format!("{name}.png"));
    if std::env::var("INSTA_UPDATE").as_deref() == Ok("always") || !png_path.exists() {
        std::fs::create_dir_all(snapshot_dir()).expect("create golden __snapshots__ directory");
        std::fs::write(&png_path, &png_bytes).expect("write golden reference PNG");
    } else {
        let expected_png = std::fs::read(&png_path)
            .unwrap_or_else(|e| panic!("failed to read golden PNG {}: {e}", png_path.display()));
        assert_eq!(
            png_bytes, expected_png,
            "Golden PNG bytes drifted for {name} (run with INSTA_UPDATE=always to update)"
        );
    }

    let digest = golden::fnv1a_rgba_digest(pixmap);
    let manifest = format!(
        "scenario: {name}\ndimensions: {}x{}\nrgba_fnv1a: 0x{digest:016x}\npng_len: {}\n{extra_metadata}",
        pixmap.width(),
        pixmap.height(),
        png_bytes.len(),
    );

    insta::with_settings!({
        snapshot_path => "golden/__snapshots__",
        prepend_module_to_snapshot => false,
    }, {
        insta::assert_snapshot!(name, manifest);
    });
}

#[test]
fn golden_empty_state_overlay() {
    let tokens = ThemeTokens::for_dark(true);
    let pixmap = golden::render_empty_state_overlay(&tokens);
    let accent_pixels = golden::count_matching_pixels(&pixmap, tokens.color.accent, 6);
    assert!(
        accent_pixels > 2000,
        "expected accent CTA button in overlay"
    );
    let center = golden::sample_rgba(&pixmap, 258, 200);
    verify_golden_pixmap(
        "empty_state_overlay",
        &pixmap,
        &format!("center_rgba: {center:?}\naccent_pixels: {accent_pixels}\n"),
    );
}

#[test]
fn golden_snap_node_active() {
    let tokens = ThemeTokens::for_dark(true);
    let style = hud_style(SnapKind::Node);
    assert!(style.filled_dot);
    assert_eq!(style.stroke_style, HudStrokeStyle::Solid);
    assert!(!style.is_contextual_hint);

    let pixmap = golden::render_snap_node_active(&tokens);
    let snap_col = tokens.color.snap_color(SnapKind::Node);
    let center_rgba = golden::sample_rgba(&pixmap, 250, 150);
    // Active Node center must be filled with the Okabe-Ito snap_node color.
    assert_eq!(center_rgba[0], snap_col.r());
    assert_eq!(center_rgba[1], snap_col.g());
    assert_eq!(center_rgba[2], snap_col.b());

    let node_pixels = golden::count_matching_pixels(&pixmap, snap_col, 12);
    verify_golden_pixmap(
        "snap_node_active",
        &pixmap,
        &format!(
            "hud_style: filled_dot={}, stroke={:?}\ncenter_rgba: {center_rgba:?}\nsnap_pixels: {node_pixels}\n",
            style.filled_dot, style.stroke_style
        ),
    );
}

#[test]
fn golden_snap_perpendicular_hint() {
    let tokens = ThemeTokens::for_dark(true);
    let style = hud_style(SnapKind::Perpendicular);
    assert!(!style.filled_dot);
    assert_eq!(style.stroke_style, HudStrokeStyle::Dashed);
    assert!(style.is_contextual_hint);

    let pixmap = golden::render_snap_perpendicular_hint(&tokens);
    let snap_col = tokens.color.snap_color(SnapKind::Perpendicular);
    let center_rgba = golden::sample_rgba(&pixmap, 260, 220);
    // Hollow center ring must NOT be filled solid with snap_perpendicular color.
    assert_ne!(
        [center_rgba[0], center_rgba[1], center_rgba[2]],
        [snap_col.r(), snap_col.g(), snap_col.b()]
    );
    // Ring and dashed halo are anti-aliased over the white page background, so allow AA blend margin.
    let ring_pixels = golden::count_matching_pixels(&pixmap, snap_col, 110);
    assert!(
        ring_pixels > 20,
        "expected dashed halo and hollow ring pixels, got {ring_pixels}"
    );

    verify_golden_pixmap(
        "snap_perpendicular_hint",
        &pixmap,
        &format!(
            "hud_style: filled_dot={}, stroke={:?}, contextual={}\ncenter_rgba: {center_rgba:?}\nsnap_pixels: {ring_pixels}\n",
            style.filled_dot, style.stroke_style, style.is_contextual_hint
        ),
    );
}

#[test]
fn golden_command_palette_rot() {
    let tokens = ThemeTokens::for_dark(true);
    let pixmap = golden::render_command_palette_rot(&tokens);
    let focus_pixels = golden::count_matching_pixels(&pixmap, tokens.color.border_focus, 10);
    assert!(focus_pixels > 200, "expected focused search input border");
    let modal_top = golden::sample_rgba(&pixmap, 320, 70);
    verify_golden_pixmap(
        "command_palette_rot",
        &pixmap,
        &format!("modal_top_rgba: {modal_top:?}\nfocus_pixels: {focus_pixels}\n"),
    );
}

#[test]
fn golden_properties_inspector_panels() {
    let tokens = ThemeTokens::for_dark(true);
    let pixmap = golden::render_properties_inspector_panels(&tokens);
    let sel_pixels = golden::count_matching_pixels(&pixmap, tokens.color.selection_stroke, 10);
    assert!(sel_pixels > 200, "expected selection bbox on canvas");
    let inspector_card = golden::sample_rgba(&pixmap, 520, 60);
    verify_golden_pixmap(
        "properties_inspector_panels",
        &pixmap,
        &format!("inspector_card_rgba: {inspector_card:?}\nsel_pixels: {sel_pixels}\n"),
    );
}

/// `[WAVE3B-HOTFIX-1]` Regression test:
/// Proves that shape-count-preserving mutations via Align/Distribute route through
/// `Dispatcher::dispatch`, update the `DirtySignal`, and trigger `sync_spatial_index_if_dirty`
/// so snapping resolves against the fresh post-alignment coordinates instead of stale indices.
#[test]
fn test_align_then_snap_uses_fresh_index() {
    let ctx = egui::Context::default();
    let mut app = VDrawApp::new_headless(&ctx);

    // 1. Create two rectangle shapes on the active page:
    // Rect 1: bounds [10, 10] to [30, 30] (left edge = 10)
    // Rect 2: bounds [50, 10] to [70, 30] (left edge = 50)
    let r1 = Shape::rectangle(ShapeId(1), Rect::new(10.0, 10.0, 30.0, 30.0), 0.0);
    let r2 = Shape::rectangle(ShapeId(2), Rect::new(50.0, 10.0, 70.0, 30.0), 0.0);
    let r1_id = r1.id;
    let r2_id = r2.id;

    let mut doc = Document::new("AlignSnapRegressionTest");
    if let Some(page) = doc.active_page_mut() {
        if let Some(layer) = page.layers.first_mut() {
            layer.shapes.push(r1);
            layer.shapes.push(r2);
        }
    }
    app.load_document(doc);

    // 2. Mark document dirty with an initial operation so undo_stack.is_dirty() becomes true
    app.doc_mut().selection.select(r1_id);
    let _ = app.dispatch_with_ctx(
        &ctx,
        AppCommand::TranslateSelection {
            delta: Point2::new(1.0, 0.0),
        },
    );
    let _ = app.dispatch_with_ctx(
        &ctx,
        AppCommand::TranslateSelection {
            delta: Point2::new(-1.0, 0.0),
        },
    );
    assert!(
        app.undo_stack().is_dirty(),
        "Document must be dirty prior to align"
    );

    // Sync initial spatial index
    app.sync_spatial_index_for_test();

    // 3. Select both shapes
    app.doc_mut().selection.select(r1_id);
    app.doc_mut().selection.add(r2_id);

    let view = View::new(
        egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 1000.0)),
        egui::Vec2::ZERO,
        1.0,
    );

    // Assert initial snap near Rect 2 pre-alignment left edge (50.0, 10.0):
    // Snap query at (50.5, 10.5) should find a snap target
    let pre_snap = app.snap_controller().get_engine().resolve(&SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(50.5, 10.5)),
        view: &view,
        threshold_px: 5.0,
    });
    assert!(
        pre_snap.is_some(),
        "Pre-alignment: expected snap near Rect 2 initial position (50, 10)"
    );

    // 4. Perform Align Left using dispatch_transform_shapes:
    // Collective bounds min_x = 10.0.
    // Rect 1 min_x = 10.0 -> dx = 0.0
    // Rect 2 min_x = 50.0 -> dx = 10.0 - 50.0 = -40.0
    // Rect 2 is translated by (-40, 0) to [10.0, 10.0]..[30.0, 30.0]
    let changes = vec![
        (r1_id, Point2::new(0.0, 0.0)),
        (r2_id, Point2::new(-40.0, 0.0)),
    ];
    let (doc, undo) = app.doc_and_undo_mut();
    app::panels::align::dispatch_transform_shapes(doc, undo, changes);

    // 5. Trigger spatial index synchronization: DirtySignal detects undo depth / top command change
    app.sync_spatial_index_for_test();

    // 6. Verify fresh index:
    // Query near OLD coordinates (50.5, 10.5): must NOT find any shape because Rect 2 moved away!
    let old_pos_snap = app.snap_controller().get_engine().resolve(&SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(50.5, 10.5)),
        view: &view,
        threshold_px: 5.0,
    });
    assert!(
        old_pos_snap.is_none(),
        "Post-alignment: old position (50, 10) must not resolve any snap point!"
    );

    // Query near NEW coordinates (10.2, 10.2): must resolve snap target at (10.0, 10.0)!
    let new_pos_snap = app.snap_controller().get_engine().resolve(&SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(10.2, 10.2)),
        view: &view,
        threshold_px: 5.0,
    });
    assert!(
        new_pos_snap.is_some(),
        "Post-alignment: new position (10, 10) must resolve snap point from fresh spatial index!"
    );
}
