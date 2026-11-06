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

    // 4. Status bar (Height 24, Y = 876..900)
    paint.set_color(bg_panel);
    let status_rect = Rect::from_xywh(0.0, 876.0, width as f32, 24.0).unwrap();
    pixmap.fill_rect(status_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(0.0, 876.0, width as f32, 1.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // 5. Left Toolbar (Width 48, Y = 68..876)
    paint.set_color(bg_panel);
    let tool_rect = Rect::from_xywh(0.0, 68.0, 48.0, 808.0).unwrap();
    pixmap.fill_rect(tool_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(47.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Active tool highlight (Pick Tool)
    paint.set_color(bg_active);
    let active_tool_box = Rect::from_xywh(4.0, 74.0, 40.0, 40.0).unwrap();
    pixmap.fill_rect(active_tool_box, &paint, Transform::identity(), None);
    paint.set_color(accent);
    let stroke = Stroke {
        width: 1.5,
        ..Default::default()
    };
    let path = PathBuilder::from_rect(active_tool_box);
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    // 6. Right Dock Panel (Width 260, X = 1180..1440, Y = 68..876)
    paint.set_color(bg_panel);
    let dock_rect = Rect::from_xywh(1180.0, 68.0, 260.0, 808.0).unwrap();
    pixmap.fill_rect(dock_rect, &paint, Transform::identity(), None);
    paint.set_color(border);
    let line_rect = Rect::from_xywh(1180.0, 68.0, 1.0, 808.0).unwrap();
    pixmap.fill_rect(line_rect, &paint, Transform::identity(), None);

    // Dock tab headers
    paint.set_color(bg_widget);
    let tabs_rect = Rect::from_xywh(1188.0, 76.0, 244.0, 32.0).unwrap();
    pixmap.fill_rect(tabs_rect, &paint, Transform::identity(), None);

    // Active tab indicator (Objects)
    paint.set_color(accent);
    let active_tab_line = Rect::from_xywh(1192.0, 106.0, 50.0, 2.0).unwrap();
    pixmap.fill_rect(active_tab_line, &paint, Transform::identity(), None);

    // 7. Central Canvas Area (X = 48..1180, Y = 68..876)
    let canvas_x = 48.0;
    let canvas_y = 68.0;
    let canvas_w = 1132.0;
    let canvas_h = 808.0;

    paint.set_color(canvas_bg);
    let canvas_rect = Rect::from_xywh(canvas_x, canvas_y, canvas_w, canvas_h).unwrap();
    pixmap.fill_rect(canvas_rect, &paint, Transform::identity(), None);

    // 8. Grid lines
    paint.set_color(grid_line);
    let stroke_1px = Stroke {
        width: 1.0,
        ..Default::default()
    };
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
    let cut_rect = Rect::from_xywh(page_x + 60.0, page_y + 80.0, 180.0, 120.0).unwrap();
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
    let ey = page_y + 140.0;
    let erx = 90.0_f32;
    let ery = 60.0_f32;
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

    // C. Selected Shape with 8-Handle Selection Box
    let sel_x = page_x + 120.0;
    let sel_y = page_y + 260.0;
    let sel_w = 240.0;
    let sel_h = 160.0;
    let sel_rect = Rect::from_xywh(sel_x, sel_y, sel_w, sel_h).unwrap();

    // Soft fill + stroke for the selected shape
    paint.set_color(Color::from_rgba8(124, 92, 255, 30));
    pixmap.fill_rect(sel_rect, &paint, Transform::identity(), None);
    paint.set_color(accent);
    let sel_stroke = Stroke {
        width: 1.5,
        ..Default::default()
    };
    let path = PathBuilder::from_rect(sel_rect);
    pixmap.stroke_path(&path, &paint, &sel_stroke, Transform::identity(), None);

    // 8 Selection Handles
    let handle_coords = [
        (sel_x, sel_y),
        (sel_x + sel_w * 0.5, sel_y),
        (sel_x + sel_w, sel_y),
        (sel_x + sel_w, sel_y + sel_h * 0.5),
        (sel_x + sel_w, sel_y + sel_h),
        (sel_x + sel_w * 0.5, sel_y + sel_h),
        (sel_x, sel_y + sel_h),
        (sel_x, sel_y + sel_h * 0.5),
    ];

    paint.set_color(Color::WHITE);
    let handle_stroke = Stroke {
        width: 1.2,
        ..Default::default()
    };
    for &(hx, hy) in &handle_coords {
        let h_box = Rect::from_xywh(hx - 4.0, hy - 4.0, 8.0, 8.0).unwrap();
        pixmap.fill_rect(h_box, &paint, Transform::identity(), None);
        paint.set_color(accent);
        let p = PathBuilder::from_rect(h_box);
        pixmap.stroke_path(&p, &paint, &handle_stroke, Transform::identity(), None);
        paint.set_color(Color::WHITE);
    }

    // D. Pen Tool Bézier Curve with handles
    let mut pb_pen = PathBuilder::new();
    let p0 = (page_x + 80.0, page_y + 520.0);
    let p1 = (page_x + 180.0, page_y + 440.0);
    let p2 = (page_x + 280.0, page_y + 580.0);
    let p3 = (page_x + 380.0, page_y + 500.0);

    pb_pen.move_to(p0.0, p0.1);
    pb_pen.cubic_to(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
    if let Some(p) = pb_pen.finish() {
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

        // Handle circles
        let h_circle = |px: &mut Pixmap, x: f32, y: f32, col: Color| {
            let mut paint = Paint::default();
            paint.set_color(col);
            let mut pb = PathBuilder::new();
            pb.push_circle(x, y, 3.5);
            if let Some(c) = pb.finish() {
                px.fill_path(&c, &paint, FillRule::Winding, Transform::identity(), None);
            }
        };

        h_circle(&mut pixmap, p1.0, p1.1, teal_guide);
        h_circle(&mut pixmap, p2.0, p2.1, teal_guide);
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
        let test_path = temp_dir.join("vdraw_test_screenshot.png");
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
