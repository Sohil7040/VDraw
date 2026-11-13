//! Deterministic `tiny-skia` golden image renderer driven by [`ThemeTokens`] and [`hud_style`].

use app::canvas::{hud_style, HudStrokeStyle};
use app::command_palette::build_command_catalog;
use app::tokens::{SemanticColors, SpacingTokens, ThemeTokens};
use core_snap::contracts::SnapKind;
use egui::Color32;
use tiny_skia::{
    FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform,
};

pub const GOLDEN_WIDTH: u32 = 640;
pub const GOLDEN_HEIGHT: u32 = 400;

fn paint_for(c: Color32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(SemanticColors::to_skia_color(c));
    paint.anti_alias = true;
    paint
}

fn fill_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color32) {
    if let Some(rect) = Rect::from_xywh(x, y, w.max(0.5), h.max(0.5)) {
        pixmap.fill_rect(rect, &paint_for(c), Transform::identity(), None);
    }
}

fn stroke_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color32, width: f32) {
    if let Some(rect) = Rect::from_xywh(x, y, w.max(1.0), h.max(1.0)) {
        let path = PathBuilder::from_rect(rect);
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint_for(c), &stroke, Transform::identity(), None);
    }
}

fn stroke_line(pixmap: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width,
            line_cap: LineCap::Round,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint_for(c), &stroke, Transform::identity(), None);
    }
}

fn circle_path(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r)
}

fn fill_circle(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color32) {
    if let Some(path) = circle_path(cx, cy, r) {
        pixmap.fill_path(
            &path,
            &paint_for(c),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn stroke_circle(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color32, width: f32) {
    if let Some(path) = circle_path(cx, cy, r) {
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint_for(c), &stroke, Transform::identity(), None);
    }
}

fn stroke_dashed_circle(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Color32, width: f32) {
    if let Some(path) = circle_path(cx, cy, r) {
        let dash = StrokeDash::new(vec![4.0, 3.0], 0.0);
        let stroke = Stroke {
            width,
            dash,
            line_cap: LineCap::Round,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint_for(c), &stroke, Transform::identity(), None);
    }
}

/// Renders the shared CAD workspace shell (top bar, left toolbar, canvas, page, right dock, statusbar).
fn draw_workspace_shell(pixmap: &mut Pixmap, tokens: &ThemeTokens, inspector_expanded: bool) {
    let c = &tokens.color;
    let w = GOLDEN_WIDTH as f32;
    let h = GOLDEN_HEIGHT as f32;

    // Base surface
    pixmap.fill(SemanticColors::to_skia_color(c.surface));

    // Top bar (32px)
    let top_h = 32.0;
    fill_rect(pixmap, 0.0, 0.0, w, top_h, c.surface_elevated);
    fill_rect(pixmap, 0.0, top_h - 1.0, w, 1.0, c.border);

    // Search trigger pill in top bar
    fill_rect(pixmap, 180.0, 6.0, 180.0, 20.0, c.surface_interactive);
    stroke_rect(pixmap, 180.0, 6.0, 180.0, 20.0, c.border, 1.0);

    // Mode pills (Design active)
    fill_rect(pixmap, 430.0, 6.0, 56.0, 20.0, c.accent);
    fill_rect(pixmap, 492.0, 6.0, 64.0, 20.0, c.surface_interactive);
    fill_rect(pixmap, 562.0, 6.0, 56.0, 20.0, c.surface_interactive);

    // Bottom status bar (24px)
    let status_h = SpacingTokens::STATUSBAR_HEIGHT;
    let status_y = h - status_h;
    fill_rect(pixmap, 0.0, status_y, w, status_h, c.surface_elevated);
    fill_rect(pixmap, 0.0, status_y, w, 1.0, c.border);
    // Status bar snap chip indicators
    for (idx, kind) in [
        SnapKind::Node,
        SnapKind::Endpoint,
        SnapKind::Center,
        SnapKind::Intersection,
    ]
    .iter()
    .enumerate()
    {
        let chip_x = 340.0 + (idx as f32) * 52.0;
        fill_rect(
            pixmap,
            chip_x,
            status_y + 4.0,
            46.0,
            16.0,
            c.surface_interactive,
        );
        stroke_rect(
            pixmap,
            chip_x,
            status_y + 4.0,
            46.0,
            16.0,
            c.snap_color(*kind),
            1.0,
        );
    }

    // Left toolbar (72px compact rail for 640px golden canvas)
    let toolbar_w = 72.0;
    let body_h = status_y - top_h;
    fill_rect(pixmap, 0.0, top_h, toolbar_w, body_h, c.surface_elevated);
    fill_rect(pixmap, toolbar_w - 1.0, top_h, 1.0, body_h, c.border);
    for idx in 0..8 {
        let ty = top_h + 10.0 + (idx as f32) * 34.0;
        let bg = if idx == 0 {
            c.accent
        } else {
            c.surface_interactive
        };
        fill_rect(pixmap, 10.0, ty, 52.0, 26.0, bg);
        stroke_rect(pixmap, 10.0, ty, 52.0, 26.0, c.border, 1.0);
    }

    // Right inspector width
    let inspector_w = if inspector_expanded { 216.0 } else { 148.0 };
    let inspector_x = w - inspector_w;
    fill_rect(
        pixmap,
        inspector_x,
        top_h,
        inspector_w,
        body_h,
        c.surface_elevated,
    );
    fill_rect(pixmap, inspector_x, top_h, 1.0, body_h, c.border);

    // Central canvas region
    let canvas_x = toolbar_w;
    let canvas_w = inspector_x - toolbar_w;
    fill_rect(pixmap, canvas_x, top_h, canvas_w, body_h, c.canvas_bg);

    // Subtle canvas grid
    let step = 24.0;
    let mut gx = canvas_x + step;
    while gx < inspector_x {
        stroke_line(pixmap, gx, top_h, gx, status_y, c.grid_line, 0.75);
        gx += step;
    }
    let mut gy = top_h + step;
    while gy < status_y {
        stroke_line(pixmap, canvas_x, gy, inspector_x, gy, c.grid_line, 0.75);
        gy += step;
    }

    // Centered A4 page sheet inside canvas
    let page_w = (canvas_w - 64.0).max(120.0);
    let page_h = (body_h - 48.0).max(120.0);
    let page_x = canvas_x + (canvas_w - page_w) * 0.5;
    let page_y = top_h + (body_h - page_h) * 0.5;
    fill_rect(
        pixmap,
        page_x + 4.0,
        page_y + 4.0,
        page_w,
        page_h,
        c.page_shadow,
    );
    fill_rect(pixmap, page_x, page_y, page_w, page_h, c.page_bg);
    stroke_rect(pixmap, page_x, page_y, page_w, page_h, c.page_border, 1.0);
}

fn draw_snap_hud(pixmap: &mut Pixmap, tokens: &ThemeTokens, cx: f32, cy: f32, kind: SnapKind) {
    let style = hud_style(kind);
    let snap_col = tokens.color.snap_color(kind);
    let halo_col = SemanticColors::with_alpha(snap_col, 0.78);
    let halo_r = 11.0;

    match style.stroke_style {
        HudStrokeStyle::Solid => stroke_circle(pixmap, cx, cy, halo_r, halo_col, 1.75),
        HudStrokeStyle::Dashed => stroke_dashed_circle(pixmap, cx, cy, halo_r, halo_col, 1.75),
    }

    if style.filled_dot {
        fill_circle(pixmap, cx, cy, 4.5, snap_col);
    } else {
        stroke_circle(pixmap, cx, cy, 4.5, snap_col, 1.8);
    }

    // HUD pill badge offset to top-right
    let pill_x = cx + 14.0;
    let pill_y = cy - 22.0;
    let pill_w = if style.is_contextual_hint {
        104.0
    } else {
        78.0
    };
    let pill_h = 20.0;
    fill_rect(
        pixmap,
        pill_x,
        pill_y,
        pill_w,
        pill_h,
        tokens.color.hud_pill_bg,
    );
    stroke_rect(pixmap, pill_x, pill_y, pill_w, pill_h, snap_col, 1.25);
    // Pill text proxy glyph bar
    fill_rect(
        pixmap,
        pill_x + 7.0,
        pill_y + 7.0,
        pill_w - 14.0,
        5.0,
        tokens.color.on_surface,
    );
}

/// Scenario 1: Empty-state overlay over blank workspace.
pub fn render_empty_state_overlay(tokens: &ThemeTokens) -> Pixmap {
    let mut pixmap = Pixmap::new(GOLDEN_WIDTH, GOLDEN_HEIGHT).expect("allocate pixmap");
    let c = &tokens.color;
    draw_workspace_shell(&mut pixmap, tokens, false);

    // Canvas scrim + centered empty-state hint card
    fill_rect(
        &mut pixmap,
        72.0,
        32.0,
        640.0 - 72.0 - 148.0,
        400.0 - 32.0 - 24.0,
        c.backdrop_scrim,
    );
    let card_w = 276.0;
    let card_h = 136.0;
    let card_x = 72.0 + ((640.0 - 72.0 - 148.0) - card_w) * 0.5;
    let card_y = 32.0 + ((400.0 - 32.0 - 24.0) - card_h) * 0.5;

    fill_rect(
        &mut pixmap,
        card_x,
        card_y,
        card_w,
        card_h,
        c.surface_elevated,
    );
    stroke_rect(&mut pixmap, card_x, card_y, card_w, card_h, c.border, 1.25);

    // Header + two subtitle lines + primary accent action button
    fill_rect(
        &mut pixmap,
        card_x + 20.0,
        card_y + 20.0,
        160.0,
        10.0,
        c.on_surface,
    );
    fill_rect(
        &mut pixmap,
        card_x + 20.0,
        card_y + 42.0,
        220.0,
        6.0,
        c.on_surface_muted,
    );
    fill_rect(
        &mut pixmap,
        card_x + 20.0,
        card_y + 56.0,
        190.0,
        6.0,
        c.on_surface_muted,
    );
    fill_rect(
        &mut pixmap,
        card_x + 20.0,
        card_y + 84.0,
        110.0,
        28.0,
        c.accent,
    );
    fill_rect(
        &mut pixmap,
        card_x + 142.0,
        card_y + 84.0,
        96.0,
        28.0,
        c.surface_interactive,
    );
    stroke_rect(
        &mut pixmap,
        card_x + 142.0,
        card_y + 84.0,
        96.0,
        28.0,
        c.border,
        1.0,
    );

    pixmap
}

/// Scenario 2: Active-mask `SnapKind::Node` snap in progress (filled dot + solid halo).
pub fn render_snap_node_active(tokens: &ThemeTokens) -> Pixmap {
    let mut pixmap = Pixmap::new(GOLDEN_WIDTH, GOLDEN_HEIGHT).expect("allocate pixmap");
    let c = &tokens.color;
    draw_workspace_shell(&mut pixmap, tokens, false);

    // Static target rectangle & line on the white page
    stroke_rect(&mut pixmap, 250.0, 150.0, 96.0, 72.0, c.surface, 1.8);
    stroke_line(&mut pixmap, 140.0, 260.0, 230.0, 310.0, c.laser_score, 1.8);

    // Dragged rectangle ghost snapping its bottom-right corner to target corner (250, 150)
    fill_rect(&mut pixmap, 174.0, 96.0, 76.0, 54.0, c.selection_fill);
    stroke_rect(
        &mut pixmap,
        174.0,
        96.0,
        76.0,
        54.0,
        c.selection_stroke,
        1.5,
    );

    // Active Node HUD at (250.0, 150.0)
    draw_snap_hud(&mut pixmap, tokens, 250.0, 150.0, SnapKind::Node);

    pixmap
}

/// Scenario 3: Contextual `SnapKind::Perpendicular` hint (hollow ring + dashed halo).
pub fn render_snap_perpendicular_hint(tokens: &ThemeTokens) -> Pixmap {
    let mut pixmap = Pixmap::new(GOLDEN_WIDTH, GOLDEN_HEIGHT).expect("allocate pixmap");
    let c = &tokens.color;
    draw_workspace_shell(&mut pixmap, tokens, false);

    // Horizontal base segment at y = 220
    stroke_line(&mut pixmap, 140.0, 220.0, 380.0, 220.0, c.surface, 2.0);
    // Anchor point at (260, 120) dropping a perpendicular ray onto (260, 220)
    fill_circle(&mut pixmap, 260.0, 120.0, 3.5, c.polar_guide);
    stroke_line(&mut pixmap, 260.0, 120.0, 260.0, 220.0, c.polar_guide, 1.4);

    // Right-angle perpendicular glyph at (260, 220)
    stroke_line(
        &mut pixmap,
        260.0,
        208.0,
        272.0,
        208.0,
        c.snap_perpendicular,
        1.2,
    );
    stroke_line(
        &mut pixmap,
        272.0,
        208.0,
        272.0,
        220.0,
        c.snap_perpendicular,
        1.2,
    );

    // Contextual Perpendicular HUD at (260.0, 220.0)
    draw_snap_hud(&mut pixmap, tokens, 260.0, 220.0, SnapKind::Perpendicular);

    pixmap
}

/// Scenario 4: Command palette open with query `"rot"` populated from the real catalog.
pub fn render_command_palette_rot(tokens: &ThemeTokens) -> Pixmap {
    let mut pixmap = Pixmap::new(GOLDEN_WIDTH, GOLDEN_HEIGHT).expect("allocate pixmap");
    let c = &tokens.color;
    draw_workspace_shell(&mut pixmap, tokens, false);

    // Full-screen dim scrim
    fill_rect(
        &mut pixmap,
        0.0,
        0.0,
        GOLDEN_WIDTH as f32,
        GOLDEN_HEIGHT as f32,
        c.backdrop_scrim,
    );

    let modal_w = 340.0;
    let modal_h = 240.0;
    let modal_x = (GOLDEN_WIDTH as f32 - modal_w) * 0.5;
    let modal_y = 64.0;
    fill_rect(
        &mut pixmap,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        c.surface_elevated,
    );
    stroke_rect(
        &mut pixmap,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        c.border,
        1.5,
    );

    // Search input box with focus ring and `"rot"` query highlight
    let input_x = modal_x + 12.0;
    let input_y = modal_y + 12.0;
    let input_w = modal_w - 24.0;
    fill_rect(&mut pixmap, input_x, input_y, input_w, 30.0, c.surface);
    stroke_rect(
        &mut pixmap,
        input_x,
        input_y,
        input_w,
        30.0,
        c.border_focus,
        1.75,
    );
    fill_rect(
        &mut pixmap,
        input_x + 10.0,
        input_y + 10.0,
        32.0,
        10.0,
        c.accent,
    );

    // Filter real command catalog with query "rot"
    let catalog = build_command_catalog();
    let matches: Vec<_> = catalog
        .iter()
        .filter(|e| e.label.to_ascii_lowercase().contains("rot"))
        .take(5)
        .collect();

    for (idx, entry) in matches.iter().enumerate() {
        let row_y = modal_y + 54.0 + (idx as f32) * 34.0;
        let row_bg = if idx == 0 { c.surface_hover } else { c.surface };
        fill_rect(&mut pixmap, input_x, row_y, input_w, 28.0, row_bg);
        if idx == 0 {
            fill_rect(&mut pixmap, input_x, row_y, 3.0, 28.0, c.accent);
        }
        let label_w = ((entry.label.len() as f32) * 7.5).min(input_w - 72.0);
        fill_rect(
            &mut pixmap,
            input_x + 12.0,
            row_y + 10.0,
            label_w,
            8.0,
            c.on_surface,
        );
        if !entry.shortcut.is_empty() {
            fill_rect(
                &mut pixmap,
                input_x + input_w - 52.0,
                row_y + 8.0,
                42.0,
                12.0,
                c.surface_interactive,
            );
        }
    }

    pixmap
}

/// Scenario 5: Right Properties Inspector with Rotate / Mirror / Scale / Boolean panels visible.
pub fn render_properties_inspector_panels(tokens: &ThemeTokens) -> Pixmap {
    let mut pixmap = Pixmap::new(GOLDEN_WIDTH, GOLDEN_HEIGHT).expect("allocate pixmap");
    let c = &tokens.color;
    draw_workspace_shell(&mut pixmap, tokens, true);

    // Selected shape on canvas with selection bounding box
    fill_rect(&mut pixmap, 170.0, 130.0, 120.0, 90.0, c.selection_fill);
    stroke_rect(
        &mut pixmap,
        170.0,
        130.0,
        120.0,
        90.0,
        c.selection_stroke,
        1.75,
    );
    for (hx, hy) in [
        (170.0, 130.0),
        (290.0, 130.0),
        (290.0, 220.0),
        (170.0, 220.0),
    ] {
        fill_rect(&mut pixmap, hx - 3.0, hy - 3.0, 6.0, 6.0, c.page_bg);
        stroke_rect(
            &mut pixmap,
            hx - 3.0,
            hy - 3.0,
            6.0,
            6.0,
            c.selection_stroke,
            1.2,
        );
    }

    // Expanded Right Inspector panel cards: Rotate, Mirror, Scale, Boolean
    let insp_x = GOLDEN_WIDTH as f32 - 216.0;
    let card_x = insp_x + 10.0;
    let card_w = 196.0;
    let panels = [
        ("Rotate", 42.0, 68.0, c.accent),
        ("Mirror", 118.0, 68.0, c.polar_guide),
        ("Scale", 194.0, 68.0, c.success),
        ("Boolean", 270.0, 92.0, c.warning),
    ];

    for (_name, py, ph, accent_col) in panels {
        fill_rect(&mut pixmap, card_x, py, card_w, ph, c.surface);
        stroke_rect(&mut pixmap, card_x, py, card_w, ph, c.border, 1.0);
        // Header strip
        fill_rect(&mut pixmap, card_x + 8.0, py + 8.0, 72.0, 7.0, c.on_surface);
        fill_rect(
            &mut pixmap,
            card_x + card_w - 44.0,
            py + 6.0,
            36.0,
            12.0,
            accent_col,
        );
        // Control rows (numeric fields / action pills)
        fill_rect(
            &mut pixmap,
            card_x + 8.0,
            py + 26.0,
            84.0,
            16.0,
            c.surface_interactive,
        );
        fill_rect(
            &mut pixmap,
            card_x + 100.0,
            py + 26.0,
            88.0,
            16.0,
            c.surface_interactive,
        );
        if ph > 75.0 {
            // Extra row of Boolean buttons (Weld / Trim / Intersect / Exclude)
            for bx in 0..3 {
                let x = card_x + 8.0 + (bx as f32) * 61.0;
                let col = if bx == 0 {
                    c.accent
                } else {
                    c.surface_interactive
                };
                fill_rect(&mut pixmap, x, py + 52.0, 56.0, 22.0, col);
                stroke_rect(&mut pixmap, x, py + 52.0, 56.0, 22.0, c.border, 1.0);
            }
        }
    }

    pixmap
}

/// Deterministic FNV-1a 64-bit hash over raw RGBA bytes.
pub fn fnv1a_rgba_digest(pixmap: &Pixmap) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in pixmap.data() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Counts pixels in `pixmap` whose unpremultiplied RGB is within Manhattan distance `tol` of `target`.
pub fn count_matching_pixels(pixmap: &Pixmap, target: Color32, tol: i32) -> usize {
    let mut count = 0usize;
    for px in pixmap.pixels() {
        let dr = (px.red() as i32 - target.r() as i32).abs();
        let dg = (px.green() as i32 - target.g() as i32).abs();
        let db = (px.blue() as i32 - target.b() as i32).abs();
        if dr + dg + db <= tol {
            count += 1;
        }
    }
    count
}

/// Samples the `[r, g, b, a]` bytes at pixel `(x, y)` in `pixmap`.
pub fn sample_rgba(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let p = pixmap
        .pixel(x, y)
        .expect("sample coordinates within pixmap bounds");
    [p.red(), p.green(), p.blue(), p.alpha()]
}
