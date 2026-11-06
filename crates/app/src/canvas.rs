//! Canvas widget: pan/zoom, rulers, grid, page boundary, shape rendering, and tool interaction.

use core_doc::{Color, Document, ShapeKind, UndoStack};
use core_geom::Point2;
use core_tools::{ModifiersState, PointerEvent, ToolManager};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

use crate::theme;

/// Screen pixels per millimetre at 96 DPI.
/// 1 inch = 25.4 mm, 96 px/inch → 96/25.4 ≈ 3.7795 px/mm
pub const PX_PER_MM: f32 = 96.0 / 25.4;

/// A4 dimensions in mm.
const A4_W_MM: f32 = 210.0;
const A4_H_MM: f32 = 297.0;

// ─── Canvas state ────────────────────────────────────────────────────────────

/// Persistent state for the canvas viewport.
#[derive(Debug, Clone)]
pub struct CanvasState {
    /// Pan offset: how many screen-pixels the origin has been shifted.
    pub pan: Vec2,
    /// Zoom factor (1.0 = 100 %).
    pub zoom: f32,
    /// Whether the user is currently in pan-mode (Space held or MMB pressed).
    pub panning: bool,
    /// Last known cursor position in document space (mm).
    pub cursor_mm: Option<Point2>,
    /// Whether to show the grid.
    pub show_grid: bool,
    /// Whether to show rulers.
    pub show_rulers: bool,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            panning: false,
            cursor_mm: None,
            show_grid: true,
            show_rulers: true,
        }
    }
}

impl CanvasState {
    pub const MIN_ZOOM: f32 = 0.01;
    pub const MAX_ZOOM: f32 = 64.0;

    /// Convert a document-space point (mm) to screen-space position.
    pub fn world_to_screen(&self, p: Point2, canvas_origin: Pos2) -> Pos2 {
        let scale = self.zoom * PX_PER_MM;
        Pos2::new(
            canvas_origin.x + (p.x as f32) * scale + self.pan.x,
            canvas_origin.y + (p.y as f32) * scale + self.pan.y,
        )
    }

    /// Convert a screen-space position to document-space (mm).
    pub fn screen_to_world(&self, p: Pos2, canvas_origin: Pos2) -> Point2 {
        let scale = self.zoom * PX_PER_MM;
        Point2::new(
            ((p.x - canvas_origin.x - self.pan.x) / scale) as f64,
            ((p.y - canvas_origin.y - self.pan.y) / scale) as f64,
        )
    }

    /// Zoom centred on `anchor_screen` by `factor`.
    pub fn zoom_at(&mut self, anchor_screen: Pos2, canvas_origin: Pos2, factor: f32) {
        let old_zoom = self.zoom;
        let new_zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let anchor_rel = anchor_screen - canvas_origin;
        let world_before = (anchor_rel - self.pan) / (old_zoom * PX_PER_MM);
        self.zoom = new_zoom;
        self.pan = anchor_rel - world_before * (new_zoom * PX_PER_MM);
    }

    /// Fit the A4 page into the given viewport rect with padding.
    pub fn fit_page(&mut self, viewport: Rect) {
        let page_w = A4_W_MM * PX_PER_MM;
        let page_h = A4_H_MM * PX_PER_MM;
        let padding = 40.0_f32;
        let scale_x = (viewport.width() - padding * 2.0) / page_w;
        let scale_y = (viewport.height() - padding * 2.0) / page_h;
        self.zoom = scale_x.min(scale_y).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let page_w_s = A4_W_MM * PX_PER_MM * self.zoom;
        let page_h_s = A4_H_MM * PX_PER_MM * self.zoom;
        self.pan = Vec2::new(
            (viewport.width() - page_w_s) * 0.5,
            (viewport.height() - page_h_s) * 0.5,
        );
    }
}

// ─── Canvas widget ────────────────────────────────────────────────────────────

/// Show the canvas, updating `state`, dispatching to `tools`, and rendering `doc`.
pub fn show_canvas(
    ui: &mut egui::Ui,
    state: &mut CanvasState,
    doc: &mut Document,
    undo: &mut UndoStack,
    tools: &mut ToolManager,
) {
    let avail = ui.available_rect_before_wrap();
    let (resp, painter) = ui.allocate_painter(avail.size(), egui::Sense::click_and_drag());

    let ruler_size = if state.show_rulers {
        theme::RULER_SIZE
    } else {
        0.0
    };

    // Viewport is the canvas area minus rulers
    let viewport = Rect::from_min_size(
        avail.min + Vec2::new(ruler_size, ruler_size),
        avail.size() - Vec2::new(ruler_size, ruler_size),
    );

    // 1. Canvas background
    painter.rect_filled(avail, 0.0, theme::CANVAS_BG);

    // 2. Handle input and dispatch to tools
    handle_input(ui, &resp, state, doc, undo, tools, viewport);

    // 3. Grid
    if state.show_grid {
        draw_grid(&painter, state, viewport);
    }

    // 4. Page boundary & shadow
    draw_page(&painter, state, viewport);

    // 5. Document shapes
    draw_shapes(&painter, state, doc, viewport);

    // 6. Active tool overlay (selection box, handles, drawing preview)
    let origin = viewport.min;
    let to_screen = |p: Point2| state.world_to_screen(p, origin);
    tools.draw_overlay(&painter, doc, &to_screen);

    // 7. Rulers
    if state.show_rulers {
        draw_rulers(&painter, state, avail, viewport);
    }

    // 8. Update cursor icon
    if state.panning {
        ui.ctx().set_cursor_icon(if resp.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(tools.cursor());
    }
}

// ─── Input handling ──────────────────────────────────────────────────────────

fn handle_input(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    state: &mut CanvasState,
    doc: &mut Document,
    undo: &mut UndoStack,
    tools: &mut ToolManager,
    viewport: Rect,
) {
    let canvas_origin = viewport.min;

    // Track cursor position in document space
    if let Some(pos) = resp.hover_pos() {
        state.cursor_mm = Some(state.screen_to_world(pos, canvas_origin));
    } else {
        state.cursor_mm = None;
    }

    // Scroll-to-zoom
    let scroll = ui.input(|i| i.smooth_scroll_delta);
    if resp.hovered() && scroll.y != 0.0 {
        let anchor = resp.hover_pos().unwrap_or(viewport.center());
        let factor = 1.1_f32.powf(scroll.y / 20.0);
        state.zoom_at(anchor, canvas_origin, factor);
    }

    // Pan state: Space held or middle mouse drag
    let space_held = ui.input(|i| i.key_down(egui::Key::Space));
    let mmb_held = ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
    state.panning = space_held || mmb_held;

    if state.panning {
        if resp.dragged() {
            state.pan += resp.drag_delta();
        }
        return;
    }

    // Build PointerEvent
    let modifiers = ui.input(|i| ModifiersState {
        shift: i.modifiers.shift,
        ctrl: i.modifiers.ctrl,
        alt: i.modifiers.alt,
    });

    let current_pos = resp.hover_pos().unwrap_or(viewport.center());
    let world_pos = state.screen_to_world(current_pos, canvas_origin);
    let event = PointerEvent {
        world_pos,
        screen_pos: current_pos,
        modifiers,
        primary: resp.dragged_by(egui::PointerButton::Primary)
            || resp.clicked_by(egui::PointerButton::Primary),
    };

    if resp.drag_started_by(egui::PointerButton::Primary)
        || resp.clicked_by(egui::PointerButton::Primary)
    {
        tools.on_pointer_down(doc, undo, &event);
    }

    if resp.dragged_by(egui::PointerButton::Primary) || resp.hovered() {
        tools.on_pointer_move(doc, undo, &event);
    }

    if resp.drag_stopped_by(egui::PointerButton::Primary)
        || (resp.hovered() && ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)))
    {
        tools.on_pointer_up(doc, undo, &event);
    }
}

// ─── Document shape rendering ────────────────────────────────────────────────

fn to_color32(c: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

fn draw_shapes(painter: &Painter, state: &CanvasState, doc: &Document, viewport: Rect) {
    let Some(page) = doc.active_page() else {
        return;
    };
    let origin = viewport.min;

    for layer in &page.layers {
        if !layer.visible {
            continue;
        }

        for shape in &layer.shapes {
            if !shape.visible {
                continue;
            }

            let fill_col = shape.style.fill.map(to_color32);
            let stroke_prop = &shape.style.stroke;

            // Direct rendering for standard primitives for optimal speed & sharpness
            match &shape.kind {
                ShapeKind::Line { start, end } => {
                    let p0 = shape.transform.transform_point(*start);
                    let p1 = shape.transform.transform_point(*end);
                    let s0 = state.world_to_screen(p0, origin);
                    let s1 = state.world_to_screen(p1, origin);

                    if let Some(st) = stroke_prop {
                        let stroke_w = if st.hairline {
                            1.0
                        } else {
                            (st.width as f32 * state.zoom * PX_PER_MM).max(1.0)
                        };
                        painter.line_segment([s0, s1], Stroke::new(stroke_w, to_color32(st.color)));
                    }
                }
                _ => {
                    // Polycurve rendering
                    let poly = shape.to_polycurve();
                    let tol = (0.05 / state.zoom.max(0.1) as f64).clamp(0.01, 0.5);

                    for sub in &poly.subpaths {
                        let local_pts = sub.flatten(tol);
                        if local_pts.len() < 2 {
                            continue;
                        }

                        let screen_pts: Vec<Pos2> = local_pts
                            .iter()
                            .map(|&p| {
                                let world_p = shape.transform.transform_point(p);
                                state.world_to_screen(world_p, origin)
                            })
                            .collect();

                        // Fill
                        if let Some(fc) = fill_col {
                            if sub.closed && screen_pts.len() >= 3 {
                                painter.add(egui::Shape::convex_polygon(
                                    screen_pts.clone(),
                                    fc,
                                    Stroke::NONE,
                                ));
                            }
                        }

                        // Stroke
                        if let Some(st) = stroke_prop {
                            let stroke_w = if st.hairline {
                                1.0
                            } else {
                                (st.width as f32 * state.zoom * PX_PER_MM).max(1.0)
                            };
                            let col = to_color32(st.color);

                            if sub.closed {
                                for w in screen_pts.windows(2) {
                                    painter.line_segment([w[0], w[1]], Stroke::new(stroke_w, col));
                                }
                                if let (Some(first), Some(last)) =
                                    (screen_pts.first(), screen_pts.last())
                                {
                                    painter
                                        .line_segment([*last, *first], Stroke::new(stroke_w, col));
                                }
                            } else {
                                for w in screen_pts.windows(2) {
                                    painter.line_segment([w[0], w[1]], Stroke::new(stroke_w, col));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─── Grid drawing ────────────────────────────────────────────────────────────

fn draw_grid(painter: &Painter, state: &CanvasState, viewport: Rect) {
    let scale = state.zoom * PX_PER_MM;
    let origin = viewport.min;

    let (minor_mm, major_mm) = if scale >= 6.0 {
        (1.0_f32, 10.0_f32)
    } else if scale >= 1.5 {
        (5.0_f32, 50.0_f32)
    } else {
        (10.0_f32, 100.0_f32)
    };

    let minor_px = minor_mm * scale;
    let major_px = major_mm * scale;

    let page_origin_screen = state.world_to_screen(Point2::ZERO, origin);
    let offset_x = ((page_origin_screen.x - origin.x) % minor_px + minor_px) % minor_px;
    let offset_y = ((page_origin_screen.y - origin.y) % minor_px + minor_px) % minor_px;

    let minor_col = theme::GRID_LINE;
    let major_col = Color32::from_rgba_premultiplied(0x3E, 0x44, 0x52, 0xA0);

    let clip = viewport;

    // Vertical lines
    let mut x = origin.x + offset_x;
    while x <= clip.max.x {
        let rel = x - page_origin_screen.x;
        let is_major = (rel / major_px).round() * major_px - rel < 0.5;
        let col = if is_major { major_col } else { minor_col };
        painter.line_segment(
            [Pos2::new(x, clip.min.y), Pos2::new(x, clip.max.y)],
            Stroke::new(1.0, col),
        );
        x += minor_px;
    }

    // Horizontal lines
    let mut y = origin.y + offset_y;
    while y <= clip.max.y {
        let rel = y - page_origin_screen.y;
        let is_major = (rel / major_px).round() * major_px - rel < 0.5;
        let col = if is_major { major_col } else { minor_col };
        painter.line_segment(
            [Pos2::new(clip.min.x, y), Pos2::new(clip.max.x, y)],
            Stroke::new(1.0, col),
        );
        y += minor_px;
    }
}

// ─── Page drawing ────────────────────────────────────────────────────────────

fn draw_page(painter: &Painter, state: &CanvasState, viewport: Rect) {
    let origin = viewport.min;
    let tl = state.world_to_screen(Point2::ZERO, origin);
    let br = state.world_to_screen(Point2::new(A4_W_MM as f64, A4_H_MM as f64), origin);
    let page_rect = Rect::from_min_max(tl, br);

    // Drop shadow
    let shadow_offset = 4.0_f32;
    let shadow_rect = page_rect.translate(Vec2::splat(shadow_offset));
    painter.rect_filled(shadow_rect, 2.0, Color32::from_black_alpha(80));

    // Page body
    painter.rect_filled(page_rect, 0.0, theme::PAGE_BG);

    // Page border
    painter.rect_stroke(
        page_rect,
        0.0,
        Stroke::new(1.0, Color32::from_gray(180)),
        egui::StrokeKind::Outside,
    );
}

// ─── Ruler drawing ───────────────────────────────────────────────────────────

fn draw_rulers(painter: &Painter, state: &CanvasState, full_rect: Rect, viewport: Rect) {
    let rs = theme::RULER_SIZE;

    // Corner box
    let corner = Rect::from_min_size(full_rect.min, Vec2::splat(rs));
    painter.rect_filled(corner, 0.0, theme::RULER_BG);

    // Top ruler
    let h_ruler = Rect::from_min_size(
        Pos2::new(viewport.min.x, full_rect.min.y),
        Vec2::new(viewport.width(), rs),
    );
    painter.rect_filled(h_ruler, 0.0, theme::RULER_BG);

    // Left ruler
    let v_ruler = Rect::from_min_size(
        Pos2::new(full_rect.min.x, viewport.min.y),
        Vec2::new(rs, viewport.height()),
    );
    painter.rect_filled(v_ruler, 0.0, theme::RULER_BG);

    // Draw ticks
    let scale = state.zoom * PX_PER_MM;
    let (minor_mm, major_mm): (f32, f32) = if scale >= 6.0 {
        (1.0, 10.0)
    } else if scale >= 2.0 {
        (5.0, 50.0)
    } else {
        (10.0, 100.0)
    };
    let minor_px = minor_mm * scale;

    let page_origin_screen = state.world_to_screen(Point2::ZERO, viewport.min);

    // Horizontal ticks
    {
        let start_mm = state
            .screen_to_world(Pos2::new(viewport.min.x, 0.0), viewport.min)
            .x as f32;
        let start_tick = (start_mm / minor_mm).floor() * minor_mm;
        let mut x = page_origin_screen.x + start_tick * scale;
        let mut cur_mm = start_tick;

        while x <= viewport.max.x {
            if x >= viewport.min.x {
                let is_major =
                    (cur_mm / major_mm).round() * major_mm == cur_mm || cur_mm.abs() < 0.001;
                let tick_h = if is_major { rs * 0.6 } else { rs * 0.3 };
                painter.line_segment(
                    [
                        Pos2::new(x, h_ruler.max.y - tick_h),
                        Pos2::new(x, h_ruler.max.y),
                    ],
                    Stroke::new(1.0, theme::RULER_TICK),
                );
                if is_major {
                    let label = format!("{}", cur_mm as i32);
                    painter.text(
                        Pos2::new(x + 2.0, h_ruler.min.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::proportional(9.0),
                        theme::RULER_TEXT,
                    );
                }
            }
            x += minor_px;
            cur_mm += minor_mm;
        }
    }

    // Vertical ticks
    {
        let start_mm = state
            .screen_to_world(Pos2::new(0.0, viewport.min.y), viewport.min)
            .y as f32;
        let start_tick = (start_mm / minor_mm).floor() * minor_mm;
        let mut y = page_origin_screen.y + start_tick * scale;
        let mut cur_mm = start_tick;

        while y <= viewport.max.y {
            if y >= viewport.min.y {
                let is_major =
                    (cur_mm / major_mm).round() * major_mm == cur_mm || cur_mm.abs() < 0.001;
                let tick_w = if is_major { rs * 0.6 } else { rs * 0.3 };
                painter.line_segment(
                    [
                        Pos2::new(v_ruler.max.x - tick_w, y),
                        Pos2::new(v_ruler.max.x, y),
                    ],
                    Stroke::new(1.0, theme::RULER_TICK),
                );
                if is_major {
                    let label = format!("{}", cur_mm as i32);
                    painter.text(
                        Pos2::new(v_ruler.min.x + 1.0, y - 1.0),
                        egui::Align2::LEFT_BOTTOM,
                        label,
                        egui::FontId::proportional(9.0),
                        theme::RULER_TEXT,
                    );
                }
            }
            y += minor_px;
            cur_mm += minor_mm;
        }
    }

    // Cursor indicator lines
    if let Some(cur) = state.cursor_mm {
        let sx = page_origin_screen.x + (cur.x as f32) * scale;
        let sy = page_origin_screen.y + (cur.y as f32) * scale;
        let indicator_col = theme::ACCENT;

        if sx >= h_ruler.min.x && sx <= h_ruler.max.x {
            painter.line_segment(
                [Pos2::new(sx, h_ruler.min.y), Pos2::new(sx, h_ruler.max.y)],
                Stroke::new(1.5, indicator_col),
            );
        }

        if sy >= v_ruler.min.y && sy <= v_ruler.max.y {
            painter.line_segment(
                [Pos2::new(v_ruler.min.x, sy), Pos2::new(v_ruler.max.x, sy)],
                Stroke::new(1.5, indicator_col),
            );
        }
    }
}
