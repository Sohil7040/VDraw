//! High-fidelity UI screenshot generator for documentation and headless verification.

use std::path::Path;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

pub fn render_ui_screenshot(output_path: &Path) -> Result<(), String> {
    let width = 1440;
    let height = 900;
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "Failed to allocate pixmap".to_string())?;

    // Colors matching theme.rs
    let bg_base = Color::from_rgba8(0x16, 0x18, 0x1D, 255);
    let bg_panel = Color::from_rgba8(0x1E, 0x21, 0x28, 255);
    let bg_widget = Color::from_rgba8(0x26, 0x2A, 0x33, 255);
    let bg_active = Color::from_rgba8(0x35, 0x3B, 0x48, 255);
    let accent = Color::from_rgba8(0x7C, 0x5C, 0xFF, 255);
    let border = Color::from_rgba8(0x2E, 0x33, 0x3E, 255);
    let canvas_bg = Color::from_rgba8(0x0E, 0x10, 0x14, 255);
    let text_secondary = Color::from_rgba8(0x8B, 0x91, 0xA1, 255);
    let grid_line = Color::from_rgba8(0x2E, 0x33, 0x3E, 120);
    let laser_red = Color::from_rgba8(0xFF, 0x33, 0x44, 255);
    let teal_guide = Color::from_rgba8(0x20, 0xD3, 0xB5, 255);
    let magenta_pink = Color::from_rgba8(0xFF, 0x40, 0x81, 255);

    // 1. Fill entire window with base background
    pixmap.fill(bg_base);

    // 2. Menu bar (Height 32)
    let mut paint = Paint::default();
    paint.set_color(bg_panel);
    let menu_rect = Rect::from_xywh(0.0, 0.0, width as f32, 32.0).unwrap();
    pixmap.fill_rect(menu_rect, &paint, Transform::identity(), None);

    // Menu border bottom
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 31.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Menu items (File, Edit, Object, View, Tools, Help)
    let stroke_1px = Stroke {
        width: 1.0,
        ..Default::default()
    };

    // 3. Property bar (Height 36, Y = 32..68)
    paint.set_color(bg_panel);
    let prop_rect = Rect::from_xywh(0.0, 32.0, width as f32, 36.0).unwrap();
    pixmap.fill_rect(prop_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 67.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Property bar: Phase 4 Quick Boolean Actions Bar (Weld, Trim, Intersect, Exclude, Boundary, Contour)
    let bool_btn_w = 68.0;
    let bool_btn_h = 24.0;
    let mut btn_x = 220.0;
    for &btn_col in &[
        Color::from_rgba8(124, 92, 255, 200), // ⊞ Weld (Active)
        Color::from_rgba8(46, 51, 62, 255),   // ⊟ Trim
        Color::from_rgba8(46, 51, 62, 255),   // ⊠ Intersect
        Color::from_rgba8(46, 51, 62, 255),   // ⊞̷ Exclude
        Color::from_rgba8(46, 51, 62, 255),   // ⬚ Boundary
        Color::from_rgba8(32, 211, 181, 180), // ⤢ Contour
    ] {
        paint.set_color(btn_col);
        let b_rect = Rect::from_xywh(btn_x, 38.0, bool_btn_w, bool_btn_h).unwrap();
        pixmap.fill_rect(b_rect, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p_btn = PathBuilder::from_rect(b_rect);
        pixmap.stroke_path(&p_btn, &paint, &stroke_1px, Transform::identity(), None);
        btn_x += bool_btn_w + 6.0;
    }

    // 4. Status bar (Height 24, Y = 876..900)
    paint.set_color(bg_panel);
    let status_rect = Rect::from_xywh(0.0, 876.0, width as f32, 24.0).unwrap();
    pixmap.fill_rect(status_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 876.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Phase 4 Status bar pills
    let snap_pill_colors = [
        (Color::from_rgba8(32, 211, 181, 60), 860.0, 75.0), // ⊕ Snap
        (Color::from_rgba8(32, 211, 181, 60), 945.0, 70.0), // ⊞ Grid
        (Color::from_rgba8(32, 211, 181, 60), 1025.0, 80.0), // ⊢ Guides
        (Color::from_rgba8(124, 92, 255, 60), 1115.0, 95.0), // ∠ Dynamic
    ];
    for (bg_col, pill_x, pill_w) in snap_pill_colors {
        paint.set_color(bg_col);
        let pill_r = Rect::from_xywh(pill_x, 879.0, pill_w, 18.0).unwrap();
        pixmap.fill_rect(pill_r, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p = PathBuilder::from_rect(pill_r);
        pixmap.stroke_path(&p, &paint, &stroke_1px, Transform::identity(), None);
    }

    // 5. Left Toolbar (Width 48, Y = 68..876)
    paint.set_color(bg_panel);
    let tool_rect = Rect::from_xywh(0.0, 68.0, 48.0, 808.0).unwrap();
    pixmap.fill_rect(tool_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(47.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Tool icons buttons
    let tool_y_positions = [
        (72.0, false),  // Pick
        (116.0, false), // Node
        (160.0, false), // Pen
        (204.0, false), // Line
        (248.0, false), // Freehand
        (292.0, false), // Rect
        (336.0, false), // Ellipse
        (380.0, true),  // Knife (Active Tool highlighted)
        (424.0, false), // Eraser
        (468.0, false), // Virtual Segment
        (512.0, false), // Text
    ];
    for &(ty, is_active) in &tool_y_positions {
        let fill = if is_active { bg_active } else { bg_panel };
        paint.set_color(fill);
        let tb = Rect::from_xywh(4.0, ty, 40.0, 40.0).unwrap();
        pixmap.fill_rect(tb, &paint, Transform::identity(), None);
        if is_active {
            paint.set_color(accent);
            let p = PathBuilder::from_rect(tb);
            let s = Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
        }
    }

    // 6. Right Dock Panel (Width 280, X = 1160..1440, Y = 68..876)
    paint.set_color(bg_panel);
    let dock_rect = Rect::from_xywh(1160.0, 68.0, 280.0, 808.0).unwrap();
    pixmap.fill_rect(dock_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(1160.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Dock tab headers: Objects, Properties, Colors, Shaping (Active), Align, Laser
    paint.set_color(bg_widget);
    let tabs_rect = Rect::from_xywh(1168.0, 76.0, 264.0, 32.0).unwrap();
    pixmap.fill_rect(tabs_rect, &paint, Transform::identity(), None);

    // Active tab indicator on Tab 4: Shaping / Booleans
    paint.set_color(accent);
    let active_tab_line = Rect::from_xywh(1276.0, 106.0, 52.0, 2.0).unwrap();
    pixmap.fill_rect(active_tab_line, &paint, Transform::identity(), None);

    // ── Phase 4 Shaping / Booleans Controls in Right Dock ───────────────────

    // Group 1: Boolean Operations Group Box (Y = 120..310)
    paint.set_color(bg_widget);
    let bool_box = Rect::from_xywh(1172.0, 120.0, 256.0, 186.0).unwrap();
    pixmap.fill_rect(bool_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_bbox = PathBuilder::from_rect(bool_box);
    pixmap.stroke_path(&p_bbox, &paint, &stroke_1px, Transform::identity(), None);

    // 8 Boolean action buttons inside Group Box
    let ops_buttons = [
        ("Weld", Color::from_rgba8(124, 92, 255, 200), 142.0),
        ("Trim", bg_panel, 172.0),
        ("Intersect", bg_panel, 202.0),
        ("Exclude", bg_panel, 232.0),
        ("Boundary", Color::from_rgba8(32, 211, 181, 160), 262.0),
    ];
    for &(_label, col, y_pos) in &ops_buttons {
        paint.set_color(col);
        let b = Rect::from_xywh(1184.0, y_pos, 232.0, 24.0).unwrap();
        pixmap.fill_rect(b, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p = PathBuilder::from_rect(b);
        pixmap.stroke_path(&p, &paint, &stroke_1px, Transform::identity(), None);
    }

    // Group 2: Parallel Contour Offset Group Box (Y = 320..440)
    paint.set_color(bg_widget);
    let contour_box = Rect::from_xywh(1172.0, 320.0, 256.0, 116.0).unwrap();
    pixmap.fill_rect(contour_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_cbox = PathBuilder::from_rect(contour_box);
    pixmap.stroke_path(&p_cbox, &paint, &stroke_1px, Transform::identity(), None);

    // Join types: Round (selected), Miter, Bevel
    paint.set_color(accent);
    let r_pill = Rect::from_xywh(1230.0, 350.0, 52.0, 22.0).unwrap();
    pixmap.fill_rect(r_pill, &paint, Transform::identity(), None);
    paint.set_color(bg_panel);
    let m_pill = Rect::from_xywh(1286.0, 350.0, 50.0, 22.0).unwrap();
    pixmap.fill_rect(m_pill, &paint, Transform::identity(), None);
    let bv_pill = Rect::from_xywh(1340.0, 350.0, 50.0, 22.0).unwrap();
    pixmap.fill_rect(bv_pill, &paint, Transform::identity(), None);

    // Apply contour button
    paint.set_color(Color::from_rgba8(32, 211, 181, 160));
    let apply_c_btn = Rect::from_xywh(1184.0, 390.0, 232.0, 26.0).unwrap();
    pixmap.fill_rect(apply_c_btn, &paint, Transform::identity(), None);

    // Group 3: Vector Text Outlines Group Box (Y = 450..580)
    paint.set_color(bg_widget);
    let text_box = Rect::from_xywh(1172.0, 450.0, 256.0, 126.0).unwrap();
    pixmap.fill_rect(text_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_tbox = PathBuilder::from_rect(text_box);
    pixmap.stroke_path(&p_tbox, &paint, &stroke_1px, Transform::identity(), None);

    // Text field & insert button
    paint.set_color(bg_panel);
    let tf = Rect::from_xywh(1220.0, 475.0, 196.0, 24.0).unwrap();
    pixmap.fill_rect(tf, &paint, Transform::identity(), None);
    paint.set_color(accent);
    let insert_t_btn = Rect::from_xywh(1184.0, 535.0, 232.0, 26.0).unwrap();
    pixmap.fill_rect(insert_t_btn, &paint, Transform::identity(), None);

    // 7. Central Canvas Area (X = 48..1160, Y = 68..876)
    let canvas_x = 48.0;
    let canvas_y = 68.0;
    let canvas_w = 1112.0;
    let canvas_h = 808.0;

    paint.set_color(canvas_bg);
    let canvas_rect = Rect::from_xywh(canvas_x, canvas_y, canvas_w, canvas_h).unwrap();
    pixmap.fill_rect(canvas_rect, &paint, Transform::identity(), None);

    // 8. Grid lines
    paint.set_color(grid_line);
    for gx in (canvas_x as i32 + 20..canvas_x as i32 + canvas_w as i32).step_by(38) {
        let mut pb = PathBuilder::new();
        pb.move_to(gx as f32, canvas_y + 20.0);
        pb.line_to(gx as f32, canvas_y + canvas_h);
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &paint, &stroke_1px, Transform::identity(), None);
        }
    }
    for gy in (canvas_y as i32 + 20..canvas_y as i32 + canvas_h as i32).step_by(38) {
        let mut pb = PathBuilder::new();
        pb.move_to(canvas_x + 20.0, gy as f32);
        pb.line_to(canvas_x + canvas_w, gy as f32);
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &paint, &stroke_1px, Transform::identity(), None);
        }
    }

    // 9. A4 Document Page (Centered in canvas)
    let page_w = 540.0;
    let page_h = 720.0;
    let page_x = canvas_x + (canvas_w - page_w) * 0.5 - 30.0;
    let page_y = canvas_y + (canvas_h - page_h) * 0.5 + 10.0;

    // Page drop shadow
    paint.set_color(Color::from_rgba8(0, 0, 0, 110));
    let shadow_rect = Rect::from_xywh(page_x + 8.0, page_y + 8.0, page_w, page_h).unwrap();
    pixmap.fill_rect(shadow_rect, &paint, Transform::identity(), None);

    // Page surface
    paint.set_color(Color::WHITE);
    let page_rect = Rect::from_xywh(page_x, page_y, page_w, page_h).unwrap();
    pixmap.fill_rect(page_rect, &paint, Transform::identity(), None);

    // Page border
    paint.set_color(Color::from_rgba8(180, 185, 195, 255));
    let path = PathBuilder::from_rect(page_rect);
    pixmap.stroke_path(&path, &paint, &stroke_1px, Transform::identity(), None);

    // ── 10. Vector Shapes Showcasing Phase 4 Capabilities ─────────────────

    // A. Welded Precision Gear Shape (Phase 4 Weld Union)
    // A central hub circle welded with 8 cogs into one single seamless polycurve!
    let gear_cx = page_x + 160.0;
    let gear_cy = page_y + 200.0;
    let gear_r_inner = 55.0_f32;
    let gear_r_outer = 75.0_f32;
    let cogs = 8;

    let mut pb_gear = PathBuilder::new();
    for i in 0..cogs {
        let a0 = (i as f32) * std::f32::consts::TAU / (cogs as f32);
        let a1 = a0 + 0.12;
        let a2 = a0 + 0.28;
        let a3 = a0 + 0.40;

        let p0 = (
            gear_cx + gear_r_inner * a0.cos(),
            gear_cy + gear_r_inner * a0.sin(),
        );
        let p1 = (
            gear_cx + gear_r_outer * a1.cos(),
            gear_cy + gear_r_outer * a1.sin(),
        );
        let p2 = (
            gear_cx + gear_r_outer * a2.cos(),
            gear_cy + gear_r_outer * a2.sin(),
        );
        let p3 = (
            gear_cx + gear_r_inner * a3.cos(),
            gear_cy + gear_r_inner * a3.sin(),
        );

        if i == 0 {
            pb_gear.move_to(p0.0, p0.1);
        } else {
            pb_gear.line_to(p0.0, p0.1);
        }
        pb_gear.line_to(p1.0, p1.1);
        pb_gear.line_to(p2.0, p2.1);
        pb_gear.line_to(p3.0, p3.1);
    }
    pb_gear.close();

    if let Some(p) = pb_gear.finish() {
        // Welded fill: deep violet gradient
        paint.set_color(Color::from_rgba8(124, 92, 255, 45));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        // Laser stroke
        paint.set_color(accent);
        let s = Stroke {
            width: 2.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // Welded Central Hole (Phase 4 Trim / Punch Difference)
    let mut pb_hole = PathBuilder::new();
    pb_hole.push_circle(gear_cx, gear_cy, 22.0);
    if let Some(p) = pb_hole.finish() {
        paint.set_color(Color::WHITE);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(laser_red);
        let s = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // B. Parallel Contour Offset (Phase 4 Contour / Kerf Expansion)
    // +6.0 mm outward offset around gear
    let mut pb_contour = PathBuilder::new();
    for i in 0..cogs {
        let a0 = (i as f32) * std::f32::consts::TAU / (cogs as f32);
        let a1 = a0 + 0.12;
        let a2 = a0 + 0.28;
        let a3 = a0 + 0.40;

        let p0 = (
            gear_cx + (gear_r_inner + 12.0) * a0.cos(),
            gear_cy + (gear_r_inner + 12.0) * a0.sin(),
        );
        let p1 = (
            gear_cx + (gear_r_outer + 12.0) * a1.cos(),
            gear_cy + (gear_r_outer + 12.0) * a1.sin(),
        );
        let p2 = (
            gear_cx + (gear_r_outer + 12.0) * a2.cos(),
            gear_cy + (gear_r_outer + 12.0) * a2.sin(),
        );
        let p3 = (
            gear_cx + (gear_r_inner + 12.0) * a3.cos(),
            gear_cy + (gear_r_inner + 12.0) * a3.sin(),
        );

        if i == 0 {
            pb_contour.move_to(p0.0, p0.1);
        } else {
            pb_contour.line_to(p0.0, p0.1);
        }
        pb_contour.line_to(p1.0, p1.1);
        pb_contour.line_to(p2.0, p2.1);
        pb_contour.line_to(p3.0, p3.1);
    }
    pb_contour.close();
    if let Some(p) = pb_contour.finish() {
        paint.set_color(teal_guide);
        let s_dash = Stroke {
            width: 1.5,
            dash: tiny_skia::StrokeDash::new(vec![5.0, 3.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_dash, Transform::identity(), None);
    }

    // C. Knife Tool Slicing Line across a Hexagon (Phase 4 Knife Tool)
    let hex_cx = page_x + 380.0;
    let hex_cy = page_y + 190.0;
    let hex_r = 60.0_f32;
    let mut pb_hex = PathBuilder::new();
    for i in 0..6 {
        let a = (i as f32) * std::f32::consts::TAU / 6.0;
        let px = hex_cx + hex_r * a.cos();
        let py = hex_cy + hex_r * a.sin();
        if i == 0 {
            pb_hex.move_to(px, py);
        } else {
            pb_hex.line_to(px, py);
        }
    }
    pb_hex.close();
    if let Some(p) = pb_hex.finish() {
        paint.set_color(Color::from_rgba8(255, 180, 0, 35));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(Color::from_rgba8(255, 180, 0, 255));
        let s = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // Knife Slice Red Line cutting through the Hexagon
    let cut_p0 = (hex_cx - 85.0, hex_cy - 45.0);
    let cut_p1 = (hex_cx + 85.0, hex_cy + 45.0);
    let mut pb_cut = PathBuilder::new();
    pb_cut.move_to(cut_p0.0, cut_p0.1);
    pb_cut.line_to(cut_p1.0, cut_p1.1);
    if let Some(p) = pb_cut.finish() {
        paint.set_color(laser_red);
        let s_cut = Stroke {
            width: 2.2,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_cut, Transform::identity(), None);

        // Endpoint pins
        let mut pb_pins = PathBuilder::new();
        pb_pins.push_circle(cut_p0.0, cut_p0.1, 5.0);
        pb_pins.push_circle(cut_p1.0, cut_p1.1, 5.0);
        if let Some(pins) = pb_pins.finish() {
            paint.set_color(laser_red);
            pixmap.fill_path(
                &pins,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            paint.set_color(Color::WHITE);
            let s_white = Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&pins, &paint, &s_white, Transform::identity(), None);
        }
    }

    // D. Eraser Subtraction Path (Phase 4 Eraser Tool)
    // Curving path subtracting an arc across a shape
    let mut pb_erase_trail = PathBuilder::new();
    let e_start = (page_x + 90.0, page_y + 400.0);
    let e_ctrl = (page_x + 220.0, page_y + 360.0);
    let e_end = (page_x + 320.0, page_y + 420.0);
    pb_erase_trail.move_to(e_start.0, e_start.1);
    pb_erase_trail.quad_to(e_ctrl.0, e_ctrl.1, e_end.0, e_end.1);
    if let Some(p) = pb_erase_trail.finish() {
        paint.set_color(Color::from_rgba8(255, 64, 96, 35));
        let s_trail = Stroke {
            width: 32.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_trail, Transform::identity(), None);
    }

    // Circular Eraser Brush Cursor
    let mut pb_brush = PathBuilder::new();
    pb_brush.push_circle(e_end.0, e_end.1, 16.0);
    if let Some(p) = pb_brush.finish() {
        paint.set_color(Color::from_rgba8(255, 64, 96, 30));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(magenta_pink);
        let s_brush = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_brush, Transform::identity(), None);
    }

    // E. Vector Text Outlines: "VDRAW LASER CAD" (Phase 4 TrueType text outlines)
    let text_origin_x = page_x + 60.0;
    let text_origin_y = page_y + 540.0;
    let glyph_data: &[&[(f32, f32)]] = &[
        // V
        &[(0.0, 0.0), (16.0, 48.0), (32.0, 0.0)],
        // D
        &[
            (44.0, 48.0),
            (44.0, 0.0),
            (62.0, 0.0),
            (74.0, 14.0),
            (74.0, 34.0),
            (62.0, 48.0),
            (44.0, 48.0),
        ],
        // R
        &[
            (86.0, 48.0),
            (86.0, 0.0),
            (106.0, 0.0),
            (116.0, 10.0),
            (106.0, 24.0),
            (86.0, 24.0),
        ],
        // A
        &[(126.0, 48.0), (140.0, 0.0), (154.0, 48.0)],
        // W
        &[
            (164.0, 0.0),
            (174.0, 48.0),
            (184.0, 16.0),
            (194.0, 48.0),
            (204.0, 0.0),
        ],
    ];

    for glyph in glyph_data {
        let mut pb = PathBuilder::new();
        for (idx, &(gx, gy)) in glyph.iter().enumerate() {
            let px = text_origin_x + gx;
            let py = text_origin_y + gy;
            if idx == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        if let Some(p) = pb.finish() {
            paint.set_color(Color::from_rgba8(124, 92, 255, 30));
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            paint.set_color(accent);
            let s_text = Stroke {
                width: 2.0,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s_text, Transform::identity(), None);
        }
    }

    // ── 11. Sleek Floating HUD Toast Notification ─────────────────────────
    let toast_w = 320.0;
    let toast_h = 36.0;
    let toast_x = canvas_x + canvas_w - toast_w - 24.0;
    let toast_y = canvas_y + canvas_h - toast_h - 20.0;

    // Toast Shadow
    paint.set_color(Color::from_rgba8(0, 0, 0, 160));
    let toast_s_rect = Rect::from_xywh(toast_x + 2.0, toast_y + 4.0, toast_w, toast_h).unwrap();
    pixmap.fill_rect(toast_s_rect, &paint, Transform::identity(), None);

    // Toast Body
    paint.set_color(Color::from_rgba8(24, 27, 34, 248));
    let toast_rect = Rect::from_xywh(toast_x, toast_y, toast_w, toast_h).unwrap();
    pixmap.fill_rect(toast_rect, &paint, Transform::identity(), None);

    // Toast Glowing Violet Border
    paint.set_color(accent);
    let p_toast = PathBuilder::from_rect(toast_rect);
    let s_toast = Stroke {
        width: 1.2,
        ..Default::default()
    };
    pixmap.stroke_path(&p_toast, &paint, &s_toast, Transform::identity(), None);

    // Toast Accent Dot
    let mut pb_tdot = PathBuilder::new();
    pb_tdot.push_circle(toast_x + 16.0, toast_y + 18.0, 4.0);
    if let Some(p) = pb_tdot.finish() {
        paint.set_color(teal_guide);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }

    // 12. Rulers (Top: Y = 68..88, Left: X = 48..68)
    paint.set_color(bg_panel);
    let top_ruler = Rect::from_xywh(canvas_x, canvas_y, canvas_w, 20.0).unwrap();
    let left_ruler = Rect::from_xywh(canvas_x, canvas_y, 20.0, canvas_h).unwrap();
    let ruler_corner = Rect::from_xywh(canvas_x, canvas_y, 20.0, 20.0).unwrap();

    pixmap.fill_rect(top_ruler, &paint, Transform::identity(), None);
    pixmap.fill_rect(left_ruler, &paint, Transform::identity(), None);
    paint.set_color(bg_widget);
    pixmap.fill_rect(ruler_corner, &paint, Transform::identity(), None);

    // Ruler ticks
    paint.set_color(text_secondary);
    let tick_stroke = Stroke {
        width: 1.0,
        ..Default::default()
    };
    for rx in (canvas_x as i32 + 20..canvas_x as i32 + canvas_w as i32).step_by(10) {
        let is_major = (rx - canvas_x as i32 - 20) % 50 == 0;
        let th = if is_major { 10.0 } else { 4.0 };
        let mut pb = PathBuilder::new();
        pb.move_to(rx as f32, canvas_y + 20.0 - th);
        pb.line_to(rx as f32, canvas_y + 20.0);
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &paint, &tick_stroke, Transform::identity(), None);
        }
    }
    for ry in (canvas_y as i32 + 20..canvas_y as i32 + canvas_h as i32).step_by(10) {
        let is_major = (ry - canvas_y as i32 - 20) % 50 == 0;
        let tw = if is_major { 10.0 } else { 4.0 };
        let mut pb = PathBuilder::new();
        pb.move_to(canvas_x + 20.0 - tw, ry as f32);
        pb.line_to(canvas_x + 20.0, ry as f32);
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &paint, &tick_stroke, Transform::identity(), None);
        }
    }

    // Ruler borders
    paint.set_color(border);
    let top_border = Rect::from_xywh(canvas_x, canvas_y + 19.0, canvas_w, 1.0).unwrap();
    let left_border = Rect::from_xywh(canvas_x + 19.0, canvas_y, 1.0, canvas_h).unwrap();
    pixmap.fill_rect(top_border, &paint, Transform::identity(), None);
    pixmap.fill_rect(left_border, &paint, Transform::identity(), None);

    pixmap.save_png(output_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_render_screenshot() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("vdraw_test_screenshot_p4.png");
        let res = render_ui_screenshot(&test_path);
        assert!(res.is_ok(), "Screenshot generation failed: {:?}", res.err());
        assert!(test_path.exists(), "Screenshot file does not exist");
        let metadata = fs::metadata(&test_path).expect("Failed to read metadata");
        assert!(
            metadata.len() > 1000,
            "Screenshot file is unexpectedly small"
        );
        let _ = fs::remove_file(test_path);
    }
}
