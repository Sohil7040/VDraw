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
    let _text_primary = Color::from_rgba8(0xE8, 0xE9, 0xEC, 255);
    let text_secondary = Color::from_rgba8(0x8B, 0x91, 0xA1, 255);
    let grid_line = Color::from_rgba8(0x2E, 0x33, 0x3E, 120);
    let laser_red = Color::from_rgba8(0xFF, 0x33, 0x44, 255);
    let laser_blue = Color::from_rgba8(0x33, 0x88, 0xFF, 255);
    let teal_guide = Color::from_rgba8(0x20, 0xD3, 0xB5, 255);

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

    // 3. Property bar (Height 36, Y = 32..68)
    paint.set_color(bg_panel);
    let prop_rect = Rect::from_xywh(0.0, 32.0, width as f32, 36.0).unwrap();
    pixmap.fill_rect(prop_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 67.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Property bar buttons for Shape/Node tool (Cusp, Smooth, Symmetrical, Delete)
    let node_btn_w = 70.0;
    let node_btn_h = 24.0;
    let mut btn_x = 180.0;
    for &btn_label_col in &[
        Color::from_rgba8(124, 92, 255, 60),  // Active tool badge
        Color::from_rgba8(46, 51, 62, 255),   // Cusp
        Color::from_rgba8(124, 92, 255, 255), // Smooth (selected)
        Color::from_rgba8(46, 51, 62, 255),   // Symmetrical
        Color::from_rgba8(60, 30, 35, 255),   // Delete Node
    ] {
        paint.set_color(btn_label_col);
        let b_rect = Rect::from_xywh(btn_x, 38.0, node_btn_w, node_btn_h).unwrap();
        pixmap.fill_rect(b_rect, &paint, Transform::identity(), None);
        paint.set_color(border);
        let stroke_btn = Stroke {
            width: 1.0,
            ..Default::default()
        };
        let p_btn = PathBuilder::from_rect(b_rect);
        pixmap.stroke_path(&p_btn, &paint, &stroke_btn, Transform::identity(), None);
        btn_x += node_btn_w + 8.0;
    }

    // 4. Status bar (Height 24, Y = 876..900)
    paint.set_color(bg_panel);
    let status_rect = Rect::from_xywh(0.0, 876.0, width as f32, 24.0).unwrap();
    pixmap.fill_rect(status_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 876.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Phase 3 Status bar toggles: [⊕ Snap: ON] [⊞ Grid: ON] [⊢ Guides: ON] [∠ Dynamic: 15°]
    let snap_pill_colors = [
        (Color::from_rgba8(32, 211, 181, 60), 860.0, 75.0), // ⊕ Snap
        (Color::from_rgba8(32, 211, 181, 60), 945.0, 70.0), // ⊞ Grid
        (Color::from_rgba8(32, 211, 181, 60), 1025.0, 80.0), // ⊢ Guides
        (Color::from_rgba8(124, 92, 255, 60), 1115.0, 95.0), // ∠ Dynamic (15°)
    ];
    for (bg_col, pill_x, pill_w) in snap_pill_colors {
        paint.set_color(bg_col);
        let pill_r = Rect::from_xywh(pill_x, 879.0, pill_w, 18.0).unwrap();
        pixmap.fill_rect(pill_r, &paint, Transform::identity(), None);
        paint.set_color(border);
        let p = PathBuilder::from_rect(pill_r);
        let s1 = Stroke {
            width: 1.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s1, Transform::identity(), None);
    }

    // 5. Left Toolbar (Width 48, Y = 68..876)
    paint.set_color(bg_panel);
    let tool_rect = Rect::from_xywh(0.0, 68.0, 48.0, 808.0).unwrap();
    pixmap.fill_rect(tool_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(47.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Active tool highlight: Shape / Node Tool (Tool 2, Y = 118)
    paint.set_color(bg_active);
    let active_tool_box = Rect::from_xywh(4.0, 118.0, 40.0, 40.0).unwrap();
    pixmap.fill_rect(active_tool_box, &paint, Transform::identity(), None);
    paint.set_color(accent);
    let stroke = Stroke {
        width: 1.5,
        ..Default::default()
    };
    let path = PathBuilder::from_rect(active_tool_box);
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    // 6. Right Dock Panel (Width 280, X = 1160..1440, Y = 68..876)
    paint.set_color(bg_panel);
    let dock_rect = Rect::from_xywh(1160.0, 68.0, 280.0, 808.0).unwrap();
    pixmap.fill_rect(dock_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(1160.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Dock tab headers
    paint.set_color(bg_widget);
    let tabs_rect = Rect::from_xywh(1168.0, 76.0, 264.0, 32.0).unwrap();
    pixmap.fill_rect(tabs_rect, &paint, Transform::identity(), None);

    // Active tab indicator on Tab 3: Colors / Fill & Stroke
    paint.set_color(accent);
    let active_tab_line = Rect::from_xywh(1265.0, 106.0, 56.0, 2.0).unwrap();
    pixmap.fill_rect(active_tab_line, &paint, Transform::identity(), None);

    // ── Fill & Stroke Panel Controls in Right Dock ─────────────────────────

    // Fill Group Box (Y = 124..224)
    paint.set_color(bg_widget);
    let fill_box = Rect::from_xywh(1172.0, 124.0, 256.0, 96.0).unwrap();
    pixmap.fill_rect(fill_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let stroke_1px = Stroke {
        width: 1.0,
        ..Default::default()
    };
    let p_fbox = PathBuilder::from_rect(fill_box);
    pixmap.stroke_path(&p_fbox, &paint, &stroke_1px, Transform::identity(), None);

    // Fill color preview square (Violet)
    paint.set_color(accent);
    let fill_swatch = Rect::from_xywh(1184.0, 154.0, 24.0, 24.0).unwrap();
    pixmap.fill_rect(fill_swatch, &paint, Transform::identity(), None);

    // Quick swatches in Fill
    let swatch_colors = [
        Color::from_rgba8(255, 0, 0, 255),
        Color::from_rgba8(0, 0, 255, 255),
        Color::from_rgba8(0, 0, 0, 255),
        Color::from_rgba8(124, 92, 255, 255),
        Color::from_rgba8(32, 211, 181, 255),
        Color::from_rgba8(255, 120, 0, 255),
        Color::from_rgba8(255, 210, 0, 255),
        Color::from_rgba8(0, 200, 100, 255),
    ];
    for (si, &sc) in swatch_colors.iter().enumerate() {
        paint.set_color(sc);
        let sw_r = Rect::from_xywh(1218.0 + (si as f32) * 25.0, 156.0, 18.0, 18.0).unwrap();
        pixmap.fill_rect(sw_r, &paint, Transform::identity(), None);
    }

    // Stroke Group Box (Y = 232..400)
    paint.set_color(bg_widget);
    let stroke_box = Rect::from_xywh(1172.0, 232.0, 256.0, 168.0).unwrap();
    pixmap.fill_rect(stroke_box, &paint, Transform::identity(), None);
    paint.set_color(border);
    let p_sbox = PathBuilder::from_rect(stroke_box);
    pixmap.stroke_path(&p_sbox, &paint, &stroke_1px, Transform::identity(), None);

    // Stroke color preview square (Laser Red)
    paint.set_color(laser_red);
    let stroke_swatch = Rect::from_xywh(1184.0, 262.0, 24.0, 24.0).unwrap();
    pixmap.fill_rect(stroke_swatch, &paint, Transform::identity(), None);

    // Quick swatches in Stroke
    for (si, &sc) in swatch_colors.iter().enumerate() {
        paint.set_color(sc);
        let sw_r = Rect::from_xywh(1218.0 + (si as f32) * 25.0, 264.0, 18.0, 18.0).unwrap();
        pixmap.fill_rect(sw_r, &paint, Transform::identity(), None);
    }

    // Hairline Laser Cut Badge
    paint.set_color(Color::from_rgba8(255, 51, 68, 40));
    let badge_rect = Rect::from_xywh(1184.0, 302.0, 232.0, 26.0).unwrap();
    pixmap.fill_rect(badge_rect, &paint, Transform::identity(), None);
    paint.set_color(laser_red);
    let p_badge = PathBuilder::from_rect(badge_rect);
    pixmap.stroke_path(&p_badge, &paint, &stroke_1px, Transform::identity(), None);

    // Preset width buttons
    for &(bw_col, bi) in &[
        (laser_red, 1184.0),
        (laser_blue, 1244.0),
        (bg_active, 1304.0),
        (bg_active, 1364.0),
    ] {
        paint.set_color(bw_col);
        let pb = Rect::from_xywh(bi, 340.0, 52.0, 22.0).unwrap();
        pixmap.fill_rect(pb, &paint, Transform::identity(), None);
    }

    // Cap/Join toggle buttons
    for &(cw_col, ci) in &[(bg_active, 1184.0), (accent, 1224.0), (bg_active, 1264.0)] {
        paint.set_color(cw_col);
        let cb = Rect::from_xywh(ci, 370.0, 36.0, 20.0).unwrap();
        pixmap.fill_rect(cb, &paint, Transform::identity(), None);
    }

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

    // 9. A4 Page (Centered in canvas)
    let page_w = 480.0;
    let page_h = 680.0;
    let page_x = canvas_x + (canvas_w - page_w) * 0.5;
    let page_y = canvas_y + (canvas_h - page_h) * 0.5 + 10.0;

    // Drop shadow
    paint.set_color(Color::from_rgba8(0, 0, 0, 90));
    let shadow_rect = Rect::from_xywh(page_x + 6.0, page_y + 6.0, page_w, page_h).unwrap();
    pixmap.fill_rect(shadow_rect, &paint, Transform::identity(), None);

    // Page body
    paint.set_color(Color::WHITE);
    let page_rect = Rect::from_xywh(page_x, page_y, page_w, page_h).unwrap();
    pixmap.fill_rect(page_rect, &paint, Transform::identity(), None);

    // Page border
    paint.set_color(Color::from_rgba8(180, 185, 195, 255));
    let path = PathBuilder::from_rect(page_rect);
    pixmap.stroke_path(&path, &paint, &stroke_1px, Transform::identity(), None);

    // 10. Vector Shapes on the page

    // A. Hairline Red Laser Cut Rectangle
    paint.set_color(laser_red);
    let cut_rect = Rect::from_xywh(page_x + 40.0, page_y + 60.0, 160.0, 100.0).unwrap();
    let cut_stroke = Stroke {
        width: 1.5,
        ..Default::default()
    };
    let path = PathBuilder::from_rect(cut_rect);
    pixmap.stroke_path(&path, &paint, &cut_stroke, Transform::identity(), None);

    // B. Teal Smooth Ellipse
    paint.set_color(Color::from_rgba8(32, 211, 181, 40));
    let mut pb_ellipse = PathBuilder::new();
    let ex = page_x + 320.0;
    let ey = page_y + 110.0;
    let erx = 80.0_f32;
    let ery = 50.0_f32;
    let ekx = erx * 0.552_284_8;
    let eky = ery * 0.552_284_8;

    pb_ellipse.move_to(ex, ey - ery);
    pb_ellipse.cubic_to(ex + ekx, ey - ery, ex + erx, ey - eky, ex + erx, ey);
    pb_ellipse.cubic_to(ex + erx, ey + eky, ex + ekx, ey + ery, ex, ey + ery);
    pb_ellipse.cubic_to(ex - ekx, ey + ery, ex - erx, ey + eky, ex - erx, ey);
    pb_ellipse.cubic_to(ex - erx, ey - eky, ex - ekx, ey - ery, ex, ey - ery);
    pb_ellipse.close();

    if let Some(p) = pb_ellipse.finish() {
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(teal_guide);
        pixmap.stroke_path(&p, &paint, &cut_stroke, Transform::identity(), None);
    }

    // C. Regular Polygon (Pentagon) - Phase 2
    let mut pb_poly = PathBuilder::new();
    let poly_cx = page_x + 120.0;
    let poly_cy = page_y + 240.0;
    let poly_r = 50.0_f32;
    for i in 0..5 {
        let angle = (i as f32) * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
        let px = poly_cx + poly_r * angle.cos();
        let py = poly_cy + poly_r * angle.sin();
        if i == 0 {
            pb_poly.move_to(px, py);
        } else {
            pb_poly.line_to(px, py);
        }
    }
    pb_poly.close();
    if let Some(p) = pb_poly.finish() {
        paint.set_color(Color::from_rgba8(0, 200, 100, 35));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(Color::from_rgba8(0, 200, 100, 255));
        pixmap.stroke_path(&p, &paint, &cut_stroke, Transform::identity(), None);
    }

    // D. 5-Point Star - Phase 2
    let mut pb_star = PathBuilder::new();
    let star_cx = page_x + 280.0;
    let star_cy = page_y + 240.0;
    let star_ro = 50.0_f32;
    let star_ri = 22.0_f32;
    for i in 0..10 {
        let r = if i % 2 == 0 { star_ro } else { star_ri };
        let angle = (i as f32) * std::f32::consts::TAU / 10.0 - std::f32::consts::FRAC_PI_2;
        let px = star_cx + r * angle.cos();
        let py = star_cy + r * angle.sin();
        if i == 0 {
            pb_star.move_to(px, py);
        } else {
            pb_star.line_to(px, py);
        }
    }
    pb_star.close();
    if let Some(p) = pb_star.finish() {
        paint.set_color(Color::from_rgba8(255, 210, 0, 40));
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(Color::from_rgba8(255, 180, 0, 255));
        pixmap.stroke_path(&p, &paint, &cut_stroke, Transform::identity(), None);
    }

    // E. Archimedean Spiral - Phase 2
    let mut pb_spiral = PathBuilder::new();
    let sp_cx = page_x + 400.0;
    let sp_cy = page_y + 240.0;
    let turns = 2.5_f32;
    let total_angle = turns * std::f32::consts::TAU;
    let sp_expansion = 42.0_f32;
    let steps = 60;
    for i in 0..=steps {
        let frac = (i as f32) / (steps as f32);
        let theta = frac * total_angle;
        let r = frac * sp_expansion;
        let px = sp_cx + r * theta.cos();
        let py = sp_cy + r * theta.sin();
        if i == 0 {
            pb_spiral.move_to(px, py);
        } else {
            pb_spiral.line_to(px, py);
        }
    }
    if let Some(p) = pb_spiral.finish() {
        paint.set_color(accent);
        let sp_stroke = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &sp_stroke, Transform::identity(), None);
    }

    // F. Node / Curve Editing with ShapeTool (Anchor points + control handles) - Phase 2
    let mut pb_curve = PathBuilder::new();
    let p0 = (page_x + 80.0, page_y + 480.0);
    let p1 = (page_x + 170.0, page_y + 380.0);
    let p2 = (page_x + 290.0, page_y + 560.0);
    let p3 = (page_x + 400.0, page_y + 460.0);

    pb_curve.move_to(p0.0, p0.1);
    pb_curve.cubic_to(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
    if let Some(p) = pb_curve.finish() {
        paint.set_color(accent);
        let pen_stroke = Stroke {
            width: 2.0,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &pen_stroke, Transform::identity(), None);

        // Control handle tangent lines
        paint.set_color(teal_guide);
        let tangent_stroke = Stroke {
            width: 1.0,
            ..Default::default()
        };

        let mut pb_tangent1 = PathBuilder::new();
        pb_tangent1.move_to(p0.0, p0.1);
        pb_tangent1.line_to(p1.0, p1.1);
        if let Some(pt1) = pb_tangent1.finish() {
            pixmap.stroke_path(&pt1, &paint, &tangent_stroke, Transform::identity(), None);
        }

        let mut pb_tangent2 = PathBuilder::new();
        pb_tangent2.move_to(p3.0, p3.1);
        pb_tangent2.line_to(p2.0, p2.1);
        if let Some(pt2) = pb_tangent2.finish() {
            pixmap.stroke_path(&pt2, &paint, &tangent_stroke, Transform::identity(), None);
        }

        // Circular control handle endpoints (Teal)
        let h_circle = |px: &mut Pixmap, x: f32, y: f32, col: Color| {
            let mut paint = Paint::default();
            paint.set_color(col);
            let mut pb = PathBuilder::new();
            pb.push_circle(x, y, 4.0);
            if let Some(c) = pb.finish() {
                px.fill_path(&c, &paint, FillRule::Winding, Transform::identity(), None);
            }
        };

        h_circle(&mut pixmap, p1.0, p1.1, teal_guide);
        h_circle(&mut pixmap, p2.0, p2.1, teal_guide);

        // Square Anchor Points (7x7 px, Violet for selected, White for unselected)
        let draw_anchor = |px: &mut Pixmap, x: f32, y: f32, is_sel: bool| {
            let mut paint = Paint::default();
            let rect = Rect::from_xywh(x - 4.0, y - 4.0, 8.0, 8.0).unwrap();
            let fill_col = if is_sel { accent } else { Color::WHITE };
            paint.set_color(fill_col);
            px.fill_rect(rect, &paint, Transform::identity(), None);
            paint.set_color(Color::from_rgba8(22, 24, 29, 255));
            let s_stroke = Stroke {
                width: 1.2,
                ..Default::default()
            };
            let p = PathBuilder::from_rect(rect);
            px.stroke_path(&p, &paint, &s_stroke, Transform::identity(), None);
        };

        draw_anchor(&mut pixmap, p0.0, p0.1, false);
        draw_anchor(&mut pixmap, p3.0, p3.1, true); // Selected anchor point
    }

    // ── Phase 3 Snapping & Guide Overlays ──────────────────────────────────
    let align_magenta = Color::from_rgba8(0xF0, 0x42, 0x9B, 220); // #F0429B
    let snap_teal = Color::from_rgba8(0x20, 0xD3, 0xB5, 255); // #20D3B5
    let dynamic_violet = Color::from_rgba8(0x9D, 0x72, 0xFF, 200); // #9D72FF

    // A. Interactive Guidelines
    // 1. Vertical guideline at page_x + 40.0
    let v_guide_x = page_x + 40.0;
    paint.set_color(snap_teal);
    let mut pb_vg = PathBuilder::new();
    pb_vg.move_to(v_guide_x, canvas_y + 20.0);
    pb_vg.line_to(v_guide_x, canvas_y + canvas_h);
    if let Some(p) = pb_vg.finish() {
        let guide_stroke = Stroke {
            width: 1.0,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 3.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &guide_stroke, Transform::identity(), None);
    }

    // 2. Horizontal guideline at page_y + 240.0
    let h_guide_y = page_y + 240.0;
    let mut pb_hg = PathBuilder::new();
    pb_hg.move_to(canvas_x + 20.0, h_guide_y);
    pb_hg.line_to(canvas_x + canvas_w, h_guide_y);
    if let Some(p) = pb_hg.finish() {
        let guide_stroke = Stroke {
            width: 1.0,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 3.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &guide_stroke, Transform::identity(), None);
    }

    // B. Smart Alignment Guide: Magenta line through centers of Pentagon and Star
    paint.set_color(align_magenta);
    let mut pb_align = PathBuilder::new();
    pb_align.move_to(canvas_x + 20.0, poly_cy);
    pb_align.line_to(canvas_x + canvas_w, poly_cy);
    if let Some(p) = pb_align.finish() {
        let align_stroke = Stroke {
            width: 1.2,
            dash: tiny_skia::StrokeDash::new(vec![6.0, 4.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &align_stroke, Transform::identity(), None);
    }

    // Alignment marker ticks on pentagon and star centers
    for cx in [poly_cx, star_cx] {
        let mut pb_c = PathBuilder::new();
        pb_c.move_to(cx, poly_cy - 8.0);
        pb_c.line_to(cx, poly_cy + 8.0);
        if let Some(p) = pb_c.finish() {
            let s = Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
        }
    }

    // C. Dynamic Polar Guide: 45° Ray from Rectangle corner
    let corner_x = page_x + 200.0;
    let corner_y = page_y + 160.0;
    paint.set_color(dynamic_violet);
    let mut pb_ray = PathBuilder::new();
    let ray_len = 220.0_f32;
    let angle_rad = 45.0_f32.to_radians();
    let ray_end_x = corner_x + ray_len * angle_rad.cos();
    let ray_end_y = corner_y + ray_len * angle_rad.sin();
    pb_ray.move_to(corner_x, corner_y);
    pb_ray.line_to(ray_end_x, ray_end_y);
    if let Some(p) = pb_ray.finish() {
        let ray_stroke = Stroke {
            width: 1.2,
            dash: tiny_skia::StrokeDash::new(vec![5.0, 3.0], 0.0),
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &ray_stroke, Transform::identity(), None);
    }

    // Distance tick marks along the ray (every 40 units)
    for dist in [40.0_f32, 80.0, 120.0, 160.0, 200.0] {
        let tx = corner_x + dist * angle_rad.cos();
        let ty = corner_y + dist * angle_rad.sin();
        let nx = -angle_rad.sin() * 5.0;
        let ny = angle_rad.cos() * 5.0;
        let mut pb_t = PathBuilder::new();
        pb_t.move_to(tx - nx, ty - ny);
        pb_t.line_to(tx + nx, ty + ny);
        if let Some(p) = pb_t.finish() {
            let s = Stroke {
                width: 1.0,
                ..Default::default()
            };
            pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
        }
    }

    // D. Snap Target Glyphs
    // 1. Center Glyph on Polygon (Circle with crosshair)
    paint.set_color(snap_teal);
    let mut pb_cg = PathBuilder::new();
    pb_cg.push_circle(poly_cx, poly_cy, 6.0);
    pb_cg.move_to(poly_cx - 9.0, poly_cy);
    pb_cg.line_to(poly_cx + 9.0, poly_cy);
    pb_cg.move_to(poly_cx, poly_cy - 9.0);
    pb_cg.line_to(poly_cx, poly_cy + 9.0);
    if let Some(p) = pb_cg.finish() {
        let s = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // 2. Midpoint Glyph on Top Edge of Rectangle (Triangle)
    let mid_x = page_x + 120.0;
    let mid_y = page_y + 60.0;
    let mut pb_mg = PathBuilder::new();
    pb_mg.move_to(mid_x, mid_y - 6.0);
    pb_mg.line_to(mid_x + 6.0, mid_y + 5.0);
    pb_mg.line_to(mid_x - 6.0, mid_y + 5.0);
    pb_mg.close();
    if let Some(p) = pb_mg.finish() {
        let s = Stroke {
            width: 1.5,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // 3. Node / Corner Glyph on Active Snap Point (Diamond)
    let mut pb_ng = PathBuilder::new();
    let sz = 7.0_f32;
    pb_ng.move_to(corner_x, corner_y - sz);
    pb_ng.line_to(corner_x + sz, corner_y);
    pb_ng.line_to(corner_x, corner_y + sz);
    pb_ng.line_to(corner_x - sz, corner_y);
    pb_ng.close();
    if let Some(p) = pb_ng.finish() {
        let s = Stroke {
            width: 1.8,
            ..Default::default()
        };
        pixmap.stroke_path(&p, &paint, &s, Transform::identity(), None);
    }

    // E. Floating HUD Pill Badge near the active snap point
    let hud_x = corner_x + 14.0;
    let hud_y = corner_y - 28.0;
    let hud_w = 145.0_f32;
    let hud_h = 24.0_f32;

    // Badge shadow
    paint.set_color(Color::from_rgba8(0, 0, 0, 140));
    let hud_s_rect = Rect::from_xywh(hud_x + 2.0, hud_y + 2.0, hud_w, hud_h).unwrap();
    pixmap.fill_rect(hud_s_rect, &paint, Transform::identity(), None);

    // Badge background
    paint.set_color(Color::from_rgba8(22, 24, 29, 235));
    let hud_rect = Rect::from_xywh(hud_x, hud_y, hud_w, hud_h).unwrap();
    pixmap.fill_rect(hud_rect, &paint, Transform::identity(), None);

    // Badge border (accent cyan)
    paint.set_color(snap_teal);
    let hud_p = PathBuilder::from_rect(hud_rect);
    let hud_stroke = Stroke {
        width: 1.2,
        ..Default::default()
    };
    pixmap.stroke_path(&hud_p, &paint, &hud_stroke, Transform::identity(), None);

    // Badge indicator dot
    let mut pb_hdot = PathBuilder::new();
    pb_hdot.push_circle(hud_x + 10.0, hud_y + 12.0, 3.5);
    if let Some(p) = pb_hdot.finish() {
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }

    // 11. Rulers (Top: Y = 68..88, Left: X = 48..68)
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
        let test_path = temp_dir.join("vdraw_test_screenshot_p2.png");
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
