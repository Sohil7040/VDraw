//! High-fidelity UI screenshot generator for documentation and headless verification.

use crate::tokens::{SemanticColors, ThemeTokens};
use std::path::Path;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

fn skia_rect(x: f32, y: f32, w: f32, h: f32) -> Result<Rect, String> {
    Rect::from_xywh(x, y, w, h).ok_or_else(|| format!("Invalid rect ({x}, {y}, {w}, {h})"))
}

pub fn render_ui_screenshot(output_path: &Path) -> Result<(), String> {
    let width = 1440;
    let height = 900;
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "Failed to allocate pixmap".to_string())?;

    let tokens = ThemeTokens::for_dark(true);
    let c = &tokens.color;

    let bg_base = SemanticColors::to_skia_color(c.surface);
    let bg_panel = SemanticColors::to_skia_color(c.surface_elevated);
    let bg_widget = SemanticColors::to_skia_color(c.surface_interactive);
    let bg_active = SemanticColors::to_skia_color(c.surface_active);
    let accent = SemanticColors::to_skia_color(c.accent);
    let border = SemanticColors::to_skia_color(c.border);
    let canvas_bg = SemanticColors::to_skia_color(c.canvas_bg);
    let page_bg = SemanticColors::to_skia_color(c.page_bg);
    let page_border = SemanticColors::to_skia_color(c.page_border);
    let text_secondary = SemanticColors::to_skia_color(c.on_surface_muted);
    let grid_line = SemanticColors::to_skia_alpha(c.grid_line, 120);
    let laser_red = SemanticColors::to_skia_color(c.laser_cut);
    let laser_score = SemanticColors::to_skia_color(c.laser_score);
    let laser_active = SemanticColors::to_skia_color(c.laser_active);
    let teal_guide = SemanticColors::to_skia_color(c.guide_line);
    let magenta_pink = SemanticColors::to_skia_color(c.align_guide);
    let cyan_guide = SemanticColors::to_skia_color(c.polar_guide);
    let amber_warn = SemanticColors::to_skia_color(c.warning);
    let emerald_ok = SemanticColors::to_skia_color(c.success);
    let shadow_col = SemanticColors::to_skia_alpha(c.page_shadow, 110);

    // 1. Fill entire window with base background
    pixmap.fill(bg_base);

    // 2. Menu bar & Mode Switcher (Height 32)
    let mut paint = Paint::default();
    paint.set_color(bg_panel);
    let menu_rect = skia_rect(0.0, 0.0, width as f32, 32.0)?;
    pixmap.fill_rect(menu_rect, &paint, Transform::identity(), None);

    // Menu border bottom
    paint.set_color(border);
    let line_rect = skia_rect(0.0, 31.0, width as f32, 1.0)?;
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    let stroke_1px = Stroke {
        width: 1.0,
        ..Default::default()
    };

    // Mode Switcher in Header (Center-Right: Design | Laser CAM | Toolpath Preview)
    let mode_tabs = [
        ("Design", bg_widget, false, 880.0, 75.0),
        (
            "Laser CAM",
            SemanticColors::to_skia_alpha(c.laser_active, 220),
            true,
            960.0,
            95.0,
        ),
        ("Preview", bg_widget, false, 1060.0, 80.0),
    ];
    for (_label, fill_col, is_active, mx, mw) in mode_tabs {
        paint.set_color(fill_col);
        let m_pill = skia_rect(mx, 5.0, mw, 22.0)?;
        pixmap.fill_rect(m_pill, &paint, Transform::identity(), None);
        if is_active {
            paint.set_color(teal_guide);
            let p = PathBuilder::from_rect(m_pill);
            let s_active = Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s_active, Transform::identity(), None);
        }
    }

    // 3. Property bar (Height 36, Y = 32..68)
    paint.set_color(bg_panel);
    let prop_rect = skia_rect(0.0, 32.0, width as f32, 36.0)?;
    pixmap.fill_rect(prop_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = skia_rect(0.0, 67.0, width as f32, 1.0)?;
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    paint.set_color(bg_widget);
    let center_readout = skia_rect(60.0, 38.0, 190.0, 24.0)?;
    pixmap.fill_rect(center_readout, &paint, Transform::identity(), None);
    paint.set_color(SemanticColors::to_skia_alpha(c.polar_guide, 180));
    let p_c_box = PathBuilder::from_rect(center_readout);
    pixmap.stroke_path(&p_c_box, &paint, &stroke_1px, Transform::identity(), None);

    let bool_btn_w = 68.0;
    let bool_btn_h = 24.0;
    let mut btn_x = 260.0;
    for &btn_col in &[
        SemanticColors::to_skia_alpha(c.accent, 200),
        bg_hover_skia(c),
        bg_hover_skia(c),
        bg_hover_skia(c),
        bg_hover_skia(c),
        SemanticColors::to_skia_alpha(c.guide_line, 180),
    ] {
        paint.set_color(btn_col);
        let b_rect = skia_rect(btn_x, 38.0, bool_btn_w, bool_btn_h)?;
        pixmap.fill_rect(b_rect, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p_btn = PathBuilder::from_rect(b_rect);
        pixmap.stroke_path(&p_btn, &paint, &stroke_1px, Transform::identity(), None);
        btn_x += bool_btn_w + 6.0;
    }

    // 4. Status bar (Height 24, Y = 876..900)
    paint.set_color(bg_panel);
    let status_rect = skia_rect(0.0, 876.0, width as f32, 24.0)?;
    pixmap.fill_rect(status_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = skia_rect(0.0, 876.0, width as f32, 1.0)?;
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    let snap_pill_colors = [
        (SemanticColors::to_skia_alpha(c.guide_line, 60), 860.0, 75.0),
        (SemanticColors::to_skia_alpha(c.guide_line, 60), 945.0, 70.0),
        (
            SemanticColors::to_skia_alpha(c.guide_line, 60),
            1025.0,
            80.0,
        ),
        (SemanticColors::to_skia_alpha(c.accent, 60), 1115.0, 95.0),
    ];
    for (bg_col, pill_x, pill_w) in snap_pill_colors {
        paint.set_color(bg_col);
        let pill_r = skia_rect(pill_x, 879.0, pill_w, 18.0)?;
        pixmap.fill_rect(pill_r, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p = PathBuilder::from_rect(pill_r);
        pixmap.stroke_path(&p, &paint, &stroke_1px, Transform::identity(), None);
    }

    // 5. Left Toolbar (Width 48, Y = 68..876)
    paint.set_color(bg_panel);
    let tool_rect = skia_rect(0.0, 68.0, 48.0, 808.0)?;
    pixmap.fill_rect(tool_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = skia_rect(47.0, 68.0, 1.0, 808.0)?;
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    let tool_y_positions = [
        (72.0, false),
        (116.0, false),
        (160.0, false),
        (204.0, false),
        (248.0, false),
        (292.0, false),
        (336.0, false),
        (380.0, true),
        (424.0, false),
        (468.0, false),
        (512.0, false),
    ];
    for &(ty, is_active) in &tool_y_positions {
        let fill = if is_active { bg_active } else { bg_panel };
        paint.set_color(fill);
        let tb = skia_rect(4.0, ty, 40.0, 40.0)?;
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
    let dock_rect = skia_rect(1160.0, 68.0, 280.0, 808.0)?;
    pixmap.fill_rect(dock_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = skia_rect(1160.0, 68.0, 1.0, 808.0)?;
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    paint.set_color(bg_widget);
    let tabs_rect = skia_rect(1168.0, 76.0, 264.0, 32.0)?;
    pixmap.fill_rect(tabs_rect, &paint, Transform::identity(), None);

    paint.set_color(teal_guide);
    let active_tab_line = skia_rect(1348.0, 106.0, 78.0, 2.0)?;
    pixmap.fill_rect(active_tab_line, &paint, Transform::identity(), None);

    paint.set_color(bg_widget);
    let mat_box = skia_rect(1172.0, 118.0, 256.0, 118.0)?;
    pixmap.fill_rect(mat_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_matbox = PathBuilder::from_rect(mat_box);
    pixmap.stroke_path(&p_matbox, &paint, &stroke_1px, Transform::identity(), None);

    paint.set_color(bg_panel);
    let preset_dd = skia_rect(1184.0, 142.0, 232.0, 26.0)?;
    pixmap.fill_rect(preset_dd, &paint, Transform::identity(), None);
    paint.set_color(SemanticColors::to_skia_alpha(c.laser_active, 180));
    let p_pdd = PathBuilder::from_rect(preset_dd);
    pixmap.stroke_path(&p_pdd, &paint, &stroke_1px, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.danger, 40));
    let pwr_pill = skia_rect(1184.0, 178.0, 112.0, 22.0)?;
    pixmap.fill_rect(pwr_pill, &paint, Transform::identity(), None);
    paint.set_color(SemanticColors::to_skia_alpha(c.guide_line, 40));
    let spd_pill = skia_rect(1304.0, 178.0, 112.0, 22.0)?;
    pixmap.fill_rect(spd_pill, &paint, Transform::identity(), None);

    paint.set_color(bg_panel);
    let kerf_pill = skia_rect(1184.0, 206.0, 232.0, 20.0)?;
    pixmap.fill_rect(kerf_pill, &paint, Transform::identity(), None);

    paint.set_color(bg_widget);
    let opt_box = skia_rect(1172.0, 246.0, 256.0, 95.0)?;
    pixmap.fill_rect(opt_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_optbox = PathBuilder::from_rect(opt_box);
    pixmap.stroke_path(&p_optbox, &paint, &stroke_1px, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.guide_line, 220));
    let chk1 = skia_rect(1184.0, 268.0, 14.0, 14.0)?;
    pixmap.fill_rect(chk1, &paint, Transform::identity(), None);
    paint.set_color(bg_panel);
    let chk1_lbl = skia_rect(1206.0, 267.0, 206.0, 16.0)?;
    pixmap.fill_rect(chk1_lbl, &paint, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.guide_line, 220));
    let chk2 = skia_rect(1184.0, 296.0, 14.0, 14.0)?;
    pixmap.fill_rect(chk2, &paint, Transform::identity(), None);
    paint.set_color(bg_panel);
    let chk2_lbl = skia_rect(1206.0, 295.0, 206.0, 16.0)?;
    pixmap.fill_rect(chk2_lbl, &paint, Transform::identity(), None);

    paint.set_color(bg_widget);
    let layer_box = skia_rect(1172.0, 350.0, 256.0, 196.0)?;
    pixmap.fill_rect(layer_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_lbox = PathBuilder::from_rect(layer_box);
    pixmap.stroke_path(&p_lbox, &paint, &stroke_1px, Transform::identity(), None);

    let layer_rows = [
        ("Layer 1 (Cut)", laser_red, 374.0),
        ("Layer 2 (Score)", laser_score, 420.0),
        ("Layer 3 (Engrave)", emerald_ok, 466.0),
    ];
    for (_lname, dot_col, y_row) in layer_rows {
        paint.set_color(bg_panel);
        let row_r = skia_rect(1180.0, y_row, 240.0, 40.0)?;
        pixmap.fill_rect(row_r, &paint, Transform::identity(), None);
        let mut pb_dot = PathBuilder::new();
        pb_dot.push_circle(1194.0, y_row + 20.0, 7.0);
        if let Some(p) = pb_dot.finish() {
            paint.set_color(dot_col);
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }
        paint.set_color(bg_widget);
        let op_pill = skia_rect(1210.0, y_row + 8.0, 200.0, 24.0)?;
        pixmap.fill_rect(op_pill, &paint, Transform::identity(), None);
    }

    paint.set_color(bg_widget);
    let exp_box = skia_rect(1172.0, 556.0, 256.0, 160.0)?;
    pixmap.fill_rect(exp_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_ebox = PathBuilder::from_rect(exp_box);
    pixmap.stroke_path(&p_ebox, &paint, &stroke_1px, Transform::identity(), None);

    paint.set_color(bg_panel);
    let metrics_r = skia_rect(1184.0, 574.0, 232.0, 42.0)?;
    pixmap.fill_rect(metrics_r, &paint, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.laser_active, 240));
    let gcode_btn = skia_rect(1184.0, 626.0, 232.0, 34.0)?;
    pixmap.fill_rect(gcode_btn, &paint, Transform::identity(), None);
    paint.set_color(teal_guide);
    let p_gc = PathBuilder::from_rect(gcode_btn);
    pixmap.stroke_path(&p_gc, &paint, &stroke_1px, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.surface_hover, 240));
    let plt_btn = skia_rect(1184.0, 668.0, 232.0, 30.0)?;
    pixmap.fill_rect(plt_btn, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_plt = PathBuilder::from_rect(plt_btn);
    pixmap.stroke_path(&p_plt, &paint, &stroke_1px, Transform::identity(), None);

    // 7. Central Canvas Area
    let canvas_x = 48.0;
    let canvas_y = 68.0;
    let canvas_w = 1112.0;
    let canvas_h = 808.0;

    paint.set_color(canvas_bg);
    let canvas_rect = skia_rect(canvas_x, canvas_y, canvas_w, canvas_h)?;
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

    // 9. A4 Document Page
    let page_w = 540.0;
    let page_h = 720.0;
    let page_x = canvas_x + (canvas_w - page_w) * 0.5 - 30.0;
    let page_y = canvas_y + (canvas_h - page_h) * 0.5 + 10.0;

    paint.set_color(shadow_col);
    let shadow_rect = skia_rect(page_x + 8.0, page_y + 8.0, page_w, page_h)?;
    pixmap.fill_rect(shadow_rect, &paint, Transform::identity(), None);

    paint.set_color(page_bg);
    let page_rect = skia_rect(page_x, page_y, page_w, page_h)?;
    pixmap.fill_rect(page_rect, &paint, Transform::identity(), None);

    paint.set_color(page_border);
    let path = PathBuilder::from_rect(page_rect);
    pixmap.stroke_path(&path, &paint, &stroke_1px, Transform::identity(), None);

    // 10. Vector Shapes
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
        paint.set_color(SemanticColors::to_skia_alpha(c.accent, 45));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(accent);
        let s = Stroke {
            width: 2.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    let mut pb_hole = PathBuilder::new();
    pb_hole.push_circle(gear_cx, gear_cy, 22.0);
    if let Some(p) = pb_hole.finish() {
        paint.set_color(page_bg);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(laser_red);
        let s = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    let mut pb_c_outer = PathBuilder::new();
    pb_c_outer.push_circle(gear_cx, gear_cy, 10.0);
    if let Some(p) = pb_c_outer.finish() {
        paint.set_color(cyan_guide);
        let s_cross = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_cross, Transform::identity(), None);
    }
    let mut pb_cross_lines = PathBuilder::new();
    pb_cross_lines.move_to(gear_cx - 18.0, gear_cy);
    pb_cross_lines.line_to(gear_cx - 3.0, gear_cy);
    pb_cross_lines.move_to(gear_cx + 3.0, gear_cy);
    pb_cross_lines.line_to(gear_cx + 18.0, gear_cy);
    pb_cross_lines.move_to(gear_cx, gear_cy - 18.0);
    pb_cross_lines.line_to(gear_cx, gear_cy - 3.0);
    pb_cross_lines.move_to(gear_cx, gear_cy + 3.0);
    pb_cross_lines.line_to(gear_cx, gear_cy + 18.0);
    if let Some(p) = pb_cross_lines.finish() {
        paint.set_color(cyan_guide);
        let s_cross = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_cross, Transform::identity(), None);
    }
    let mut pb_c_dot = PathBuilder::new();
    pb_c_dot.push_circle(gear_cx, gear_cy, 2.5);
    if let Some(p) = pb_c_dot.finish() {
        paint.set_color(page_bg);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }

    let mut pb_align_v = PathBuilder::new();
    pb_align_v.move_to(gear_cx, page_y + 20.0);
    pb_align_v.line_to(gear_cx, page_y + page_h - 20.0);
    if let Some(p) = pb_align_v.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.polar_guide, 190));
        let s_align = Stroke {
            width: 1.4,
            dash: tiny_skia::StrokeDash::new(vec![6.0, 4.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_align, Transform::identity(), None);
    }
    paint.set_color(SemanticColors::to_skia_alpha(c.polar_guide, 230));
    let hud_c2c = skia_rect(gear_cx + 8.0, gear_cy - 70.0, 110.0, 18.0)?;
    pixmap.fill_rect(hud_c2c, &paint, Transform::identity(), None);

    let mut pb_align_h = PathBuilder::new();
    pb_align_h.move_to(page_x + 20.0, gear_cy + gear_r_outer);
    pb_align_h.line_to(page_x + page_w - 20.0, gear_cy + gear_r_outer);
    if let Some(p) = pb_align_h.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.warning, 180));
        let s_align = Stroke {
            width: 1.4,
            dash: tiny_skia::StrokeDash::new(vec![6.0, 4.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_align, Transform::identity(), None);
    }
    paint.set_color(SemanticColors::to_skia_alpha(c.warning, 230));
    let hud_e2e = skia_rect(gear_cx + 90.0, gear_cy + gear_r_outer + 4.0, 88.0, 18.0)?;
    pixmap.fill_rect(hud_e2e, &paint, Transform::identity(), None);

    let mut pb_rapid1 = PathBuilder::new();
    pb_rapid1.move_to(page_x + 20.0, page_y + 20.0);
    pb_rapid1.line_to(gear_cx, gear_cy - 22.0);
    if let Some(p) = pb_rapid1.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.success, 200));
        let s_rapid = Stroke {
            width: 1.5,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 4.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_rapid, Transform::identity(), None);
    }
    let mut pb_rapid2 = PathBuilder::new();
    pb_rapid2.move_to(gear_cx, gear_cy - 22.0);
    pb_rapid2.line_to(gear_cx, gear_cy - gear_r_outer);
    if let Some(p) = pb_rapid2.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.success, 200));
        let s_rapid = Stroke {
            width: 1.5,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 4.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_rapid, Transform::identity(), None);
    }

    let mut pb_badge1 = PathBuilder::new();
    pb_badge1.push_circle(gear_cx - 28.0, gear_cy - 16.0, 10.0);
    if let Some(p) = pb_badge1.finish() {
        paint.set_color(laser_red);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(page_bg);
        let s_bw = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_bw, Transform::identity(), None);
    }

    let mut pb_badge2 = PathBuilder::new();
    pb_badge2.push_circle(gear_cx + gear_r_outer + 8.0, gear_cy - 10.0, 10.0);
    if let Some(p) = pb_badge2.finish() {
        paint.set_color(laser_red);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(page_bg);
        let s_bw = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_bw, Transform::identity(), None);
    }

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
        paint.set_color(SemanticColors::to_skia_alpha(c.warning, 35));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(amber_warn);
        let s = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

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
            paint.set_color(page_bg);
            let s_white = Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&pins, &paint, &s_white, Transform::identity(), None);
        }
    }

    let mut pb_erase_trail = PathBuilder::new();
    let e_start = (page_x + 90.0, page_y + 400.0);
    let e_ctrl = (page_x + 220.0, page_y + 360.0);
    let e_end = (page_x + 320.0, page_y + 420.0);
    pb_erase_trail.move_to(e_start.0, e_start.1);
    pb_erase_trail.quad_to(e_ctrl.0, e_ctrl.1, e_end.0, e_end.1);
    if let Some(p) = pb_erase_trail.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.danger, 35));
        let s_trail = Stroke {
            width: 32.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_trail, Transform::identity(), None);
    }

    let mut pb_brush = PathBuilder::new();
    pb_brush.push_circle(e_end.0, e_end.1, 16.0);
    if let Some(p) = pb_brush.finish() {
        paint.set_color(SemanticColors::to_skia_alpha(c.danger, 30));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(magenta_pink);
        let s_brush = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s_brush, Transform::identity(), None);
    }

    let text_origin_x = page_x + 60.0;
    let text_origin_y = page_y + 540.0;
    let glyph_data: &[&[(f32, f32)]] = &[
        &[(0.0, 0.0), (16.0, 48.0), (32.0, 0.0)],
        &[
            (44.0, 48.0),
            (44.0, 0.0),
            (62.0, 0.0),
            (74.0, 14.0),
            (74.0, 34.0),
            (62.0, 48.0),
            (44.0, 48.0),
        ],
        &[
            (86.0, 48.0),
            (86.0, 0.0),
            (106.0, 0.0),
            (116.0, 10.0),
            (106.0, 24.0),
            (86.0, 24.0),
        ],
        &[(126.0, 48.0), (140.0, 0.0), (154.0, 48.0)],
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
            paint.set_color(SemanticColors::to_skia_alpha(c.accent, 30));
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            paint.set_color(accent);
            let s_text = Stroke {
                width: 2.0,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s_text, Transform::identity(), None);
        }
    }

    let toast_w = 320.0;
    let toast_h = 36.0;
    let toast_x = canvas_x + canvas_w - toast_w - 24.0;
    let toast_y = canvas_y + canvas_h - toast_h - 20.0;

    paint.set_color(SemanticColors::to_skia_alpha(c.page_shadow, 160));
    let toast_s_rect = skia_rect(toast_x + 2.0, toast_y + 4.0, toast_w, toast_h)?;
    pixmap.fill_rect(toast_s_rect, &paint, Transform::identity(), None);

    paint.set_color(SemanticColors::to_skia_alpha(c.surface_elevated, 248));
    let toast_rect = skia_rect(toast_x, toast_y, toast_w, toast_h)?;
    pixmap.fill_rect(toast_rect, &paint, Transform::identity(), None);

    paint.set_color(accent);
    let p_toast = PathBuilder::from_rect(toast_rect);
    let s_toast = Stroke {
        width: 1.2,
        ..Default::default()
    };
    pixmap.stroke_path(&p_toast, &paint, &s_toast, Transform::identity(), None);

    let mut pb_tdot = PathBuilder::new();
    pb_tdot.push_circle(toast_x + 16.0, toast_y + 18.0, 4.0);
    if let Some(p) = pb_tdot.finish() {
        paint.set_color(teal_guide);
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }

    // 12. Rulers
    paint.set_color(bg_panel);
    let top_ruler = skia_rect(canvas_x, canvas_y, canvas_w, 20.0)?;
    let left_ruler = skia_rect(canvas_x, canvas_y, 20.0, canvas_h)?;
    let ruler_corner = skia_rect(canvas_x, canvas_y, 20.0, 20.0)?;

    pixmap.fill_rect(top_ruler, &paint, Transform::identity(), None);
    pixmap.fill_rect(left_ruler, &paint, Transform::identity(), None);
    paint.set_color(bg_widget);
    pixmap.fill_rect(ruler_corner, &paint, Transform::identity(), None);

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

    paint.set_color(border);
    let top_border = skia_rect(canvas_x, canvas_y + 19.0, canvas_w, 1.0)?;
    let left_border = skia_rect(canvas_x + 19.0, canvas_y, 1.0, canvas_h)?;
    pixmap.fill_rect(top_border, &paint, Transform::identity(), None);
    pixmap.fill_rect(left_border, &paint, Transform::identity(), None);

    let _ = laser_active;
    pixmap.save_png(output_path).map_err(|e| e.to_string())
}

fn bg_hover_skia(c: &SemanticColors) -> tiny_skia::Color {
    SemanticColors::to_skia_color(c.surface_hover)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_render_screenshot() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join(format!("vdraw_test_screenshot_{}.png", std::process::id()));
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
