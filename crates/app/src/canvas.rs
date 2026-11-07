//! Canvas widget: pan/zoom, rulers, grid, guidelines, snapping overlays, and tool interaction.

use core_doc::{Color, Document, ShapeKind, UndoStack};
use core_geom::Point2;
use core_snap::{Guideline, GuidelineOrientation, SnapResult, SnapSolver, SnapTargetKind};
use core_tools::{ModifiersState, PointerEvent, ToolManager};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

use crate::theme;

/// Screen pixels per millimetre at 96 DPI.
/// 1 inch = 25.4 mm, 96 px/inch → 96/25.4 ≈ 3.7795 px/mm
pub const PX_PER_MM: f32 = 96.0 / 25.4;

/// A4 dimensions in mm.
const A4_W_MM: f32 = 210.0;
const A4_H_MM: f32 = 297.0;

// ─── Guideline drag state ───────────────────────────────────────────────────

/// Interactive guideline dragging out from rulers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuidelineDrag {
    Horizontal(f64),
    Vertical(f64),
}

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
    /// Active guideline drag from ruler.
    pub active_guideline_drag: Option<GuidelineDrag>,
    /// Cached latest snap result for visual rendering.
    pub last_snap: Option<SnapResult>,
    /// Laser CAM production and toolpath simulation configuration.
    pub cam_config: core_cam::LaserProductionConfig,
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
            active_guideline_drag: None,
            last_snap: None,
            cam_config: core_cam::LaserProductionConfig::default(),
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

    /// Zoom in or out centered at `anchor_screen`.
    pub fn zoom_at(&mut self, anchor_screen: Pos2, canvas_origin: Pos2, factor: f32) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if (new_zoom - old_zoom).abs() < 1e-6 {
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

/// Show the canvas, updating `state`, dispatching to `tools`, and rendering `doc` and snapping overlays.
pub fn show_canvas(
    ui: &mut egui::Ui,
    state: &mut CanvasState,
    doc: &mut Document,
    undo: &mut UndoStack,
    tools: &mut ToolManager,
    snap_solver: &mut SnapSolver,
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

    // 2. Handle input and dispatch to tools with snapping
    handle_input(
        ui,
        &resp,
        state,
        doc,
        undo,
        tools,
        snap_solver,
        avail,
        viewport,
    );

    // 3. Grid
    if state.show_grid {
        draw_grid(&painter, state, viewport);
    }

    // 4. Page boundary & shadow
    draw_page(&painter, state, viewport);

    // 5. Document shapes
    draw_shapes(&painter, state, doc, viewport);

    // 6. Guidelines (user-placed and dragging)
    draw_guidelines(&painter, state, doc, viewport);

    // 7. Active tool overlay (selection box, handles, drawing preview)
    let origin = viewport.min;
    let to_screen = |p: Point2| state.world_to_screen(p, origin);
    tools.draw_overlay(&painter, doc, &to_screen);

    // 8. Snapping indicators, dynamic guides, and alignment lines
    draw_snap_overlays(&painter, state, viewport);

    // 9. Laser Toolpath Simulation & Preview Overlay
    if state.cam_config.preview {
        draw_toolpath_preview(&painter, state, doc, viewport);
    }

    // 10. Rulers
    if state.show_rulers {
        draw_rulers(&painter, state, avail, viewport);
    }

    // 10. Update cursor icon
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

#[allow(clippy::too_many_arguments)]
fn handle_input(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    state: &mut CanvasState,
    doc: &mut Document,
    undo: &mut UndoStack,
    tools: &mut ToolManager,
    snap_solver: &mut SnapSolver,
    full_rect: Rect,
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

    // ─── Ruler Guideline Dragging ─────────────────────────────────────────────
    let rs = theme::RULER_SIZE;
    let h_ruler = Rect::from_min_size(
        Pos2::new(viewport.min.x, full_rect.min.y),
        Vec2::new(viewport.width(), rs),
    );
    let v_ruler = Rect::from_min_size(
        Pos2::new(full_rect.min.x, viewport.min.y),
        Vec2::new(rs, viewport.height()),
    );

    let current_pos = resp.hover_pos().unwrap_or(viewport.center());
    let world_pos = state.screen_to_world(current_pos, canvas_origin);

    if state.show_rulers && resp.drag_started_by(egui::PointerButton::Primary) {
        if let Some(pos) = resp.hover_pos() {
            if h_ruler.contains(pos) {
                state.active_guideline_drag = Some(GuidelineDrag::Horizontal(world_pos.y));
            } else if v_ruler.contains(pos) {
                state.active_guideline_drag = Some(GuidelineDrag::Vertical(world_pos.x));
            }
        }
    }

    if let Some(drag) = state.active_guideline_drag {
        if resp.dragged_by(egui::PointerButton::Primary) {
            match drag {
                GuidelineDrag::Horizontal(_) => {
                    state.active_guideline_drag = Some(GuidelineDrag::Horizontal(world_pos.y));
                }
                GuidelineDrag::Vertical(_) => {
                    state.active_guideline_drag = Some(GuidelineDrag::Vertical(world_pos.x));
                }
            }
        } else if resp.drag_stopped_by(egui::PointerButton::Primary) || !resp.dragged() {
            match drag {
                GuidelineDrag::Horizontal(y) => {
                    if viewport.contains(current_pos) {
                        let id = doc.add_guideline(GuidelineOrientation::Horizontal(y));
                        snap_solver.add_guideline(Guideline::horizontal(id.0, y));
                    }
                }
                GuidelineDrag::Vertical(x) => {
                    if viewport.contains(current_pos) {
                        let id = doc.add_guideline(GuidelineOrientation::Vertical(x));
                        snap_solver.add_guideline(Guideline::vertical(id.0, x));
                    }
                }
            }
            state.active_guideline_drag = None;
        }
        return;
    }

    // ─── Snapping Evaluation ─────────────────────────────────────────────────
    let modifiers = ui.input(|i| ModifiersState {
        shift: i.modifiers.shift,
        ctrl: i.modifiers.ctrl,
        alt: i.modifiers.alt,
    });

    let primary = resp.dragged_by(egui::PointerButton::Primary)
        || resp.clicked_by(egui::PointerButton::Primary);

    let page_dim = doc.active_page().map(|p| (p.width_mm, p.height_mm));
    let snap_res = snap_solver.snap_point(world_pos, (state.zoom * PX_PER_MM) as f64, None, &[]);

    let event = PointerEvent::new(world_pos, current_pos, modifiers, primary)
        .with_snapped_pos(snap_res.point)
        .with_viewport((state.zoom * PX_PER_MM) as f64, page_dim);

    state.last_snap = if snap_res.snapped {
        Some(snap_res)
    } else {
        None
    };

    // Dispatch to tools
    if resp.drag_started_by(egui::PointerButton::Primary)
        || resp.clicked_by(egui::PointerButton::Primary)
    {
        tools.on_pointer_down(doc, undo, &event, Some(snap_solver));
    }

    if resp.dragged_by(egui::PointerButton::Primary) || resp.hovered() {
        tools.on_pointer_move(doc, undo, &event, Some(snap_solver));
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

// ─── Guidelines drawing ─────────────────────────────────────────────────────

fn draw_guidelines(painter: &Painter, state: &CanvasState, doc: &Document, viewport: Rect) {
    let origin = viewport.min;
    let guide_stroke = Stroke::new(1.0, theme::GUIDE_LINE);

    for g in &doc.guidelines {
        match g.orientation {
            GuidelineOrientation::Horizontal(y) => {
                let sy = state.world_to_screen(Point2::new(0.0, y), origin).y;
                if sy >= viewport.min.y && sy <= viewport.max.y {
                    painter.line_segment(
                        [Pos2::new(viewport.min.x, sy), Pos2::new(viewport.max.x, sy)],
                        guide_stroke,
                    );
                    painter.text(
                        Pos2::new(viewport.min.x + 4.0, sy - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:.1} mm", y),
                        egui::FontId::monospace(9.0),
                        theme::GUIDE_LINE,
                    );
                }
            }
            GuidelineOrientation::Vertical(x) => {
                let sx = state.world_to_screen(Point2::new(x, 0.0), origin).x;
                if sx >= viewport.min.x && sx <= viewport.max.x {
                    painter.line_segment(
                        [Pos2::new(sx, viewport.min.y), Pos2::new(sx, viewport.max.y)],
                        guide_stroke,
                    );
                    painter.text(
                        Pos2::new(sx + 2.0, viewport.min.y + 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.1} mm", x),
                        egui::FontId::monospace(9.0),
                        theme::GUIDE_LINE,
                    );
                }
            }
            GuidelineOrientation::Angled {
                origin: o,
                angle_rad,
            } => {
                let s_origin = state.world_to_screen(o, origin);
                let dir = Vec2::new(angle_rad.cos() as f32, angle_rad.sin() as f32) * 5000.0;
                painter.line_segment([s_origin - dir, s_origin + dir], guide_stroke);
            }
        }
    }

    // Active dragging guideline preview
    if let Some(drag) = state.active_guideline_drag {
        let active_stroke = Stroke::new(1.5, theme::ACCENT_HOVER);
        match drag {
            GuidelineDrag::Horizontal(y) => {
                let sy = state.world_to_screen(Point2::new(0.0, y), origin).y;
                painter.line_segment(
                    [Pos2::new(viewport.min.x, sy), Pos2::new(viewport.max.x, sy)],
                    active_stroke,
                );
                painter.text(
                    Pos2::new(viewport.min.x + 20.0, sy - 4.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("Guideline Y: {:.1} mm", y),
                    egui::FontId::monospace(10.0),
                    theme::ACCENT_HOVER,
                );
            }
            GuidelineDrag::Vertical(x) => {
                let sx = state.world_to_screen(Point2::new(x, 0.0), origin).x;
                painter.line_segment(
                    [Pos2::new(sx, viewport.min.y), Pos2::new(sx, viewport.max.y)],
                    active_stroke,
                );
                painter.text(
                    Pos2::new(sx + 4.0, viewport.min.y + 20.0),
                    egui::Align2::LEFT_TOP,
                    format!("Guideline X: {:.1} mm", x),
                    egui::FontId::monospace(10.0),
                    theme::ACCENT_HOVER,
                );
            }
        }
    }
}

// ─── Snapping overlays & HUD badges ──────────────────────────────────────────

fn draw_snap_overlays(painter: &Painter, state: &CanvasState, viewport: Rect) {
    let Some(snap_res) = &state.last_snap else {
        return;
    };
    if !snap_res.snapped {
        return;
    }

    let origin = viewport.min;
    let screen_pt = state.world_to_screen(snap_res.point, origin);
    if !viewport.contains(screen_pt) {
        return;
    }

    let Some(target) = &snap_res.target else {
        return;
    };

    let marker_col = theme::SNAP_MARKER;
    let marker_stroke = Stroke::new(1.5, marker_col);

    // 1. Draw Geometric Snap Marker Glyph
    match target.kind {
        SnapTargetKind::Node => {
            // Square (■)
            let half = 4.0;
            let r = Rect::from_center_size(screen_pt, Vec2::splat(half * 2.0));
            painter.rect_filled(r, 1.0, theme::BG_PANEL);
            painter.rect_stroke(r, 1.0, marker_stroke, egui::StrokeKind::Outside);
        }
        SnapTargetKind::Midpoint => {
            // Diamond (◆)
            let half = 5.0;
            let pts = vec![
                screen_pt + Vec2::new(0.0, -half),
                screen_pt + Vec2::new(half, 0.0),
                screen_pt + Vec2::new(0.0, half),
                screen_pt + Vec2::new(-half, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                theme::BG_PANEL,
                marker_stroke,
            ));
        }
        SnapTargetKind::Center => {
            // Circle with crosshair (⊕)
            painter.circle_stroke(screen_pt, 4.5, marker_stroke);
            painter.line_segment(
                [
                    screen_pt - Vec2::new(6.0, 0.0),
                    screen_pt + Vec2::new(6.0, 0.0),
                ],
                marker_stroke,
            );
            painter.line_segment(
                [
                    screen_pt - Vec2::new(0.0, 6.0),
                    screen_pt + Vec2::new(0.0, 6.0),
                ],
                marker_stroke,
            );
        }
        SnapTargetKind::Quadrant => {
            // Small diamond (◇)
            let half = 4.0;
            let pts = [
                screen_pt + Vec2::new(0.0, -half),
                screen_pt + Vec2::new(half, 0.0),
                screen_pt + Vec2::new(0.0, half),
                screen_pt + Vec2::new(-half, 0.0),
            ];
            painter.line_segment([pts[0], pts[1]], marker_stroke);
            painter.line_segment([pts[1], pts[2]], marker_stroke);
            painter.line_segment([pts[2], pts[3]], marker_stroke);
            painter.line_segment([pts[3], pts[0]], marker_stroke);
        }

        SnapTargetKind::Intersection => {
            // Diagonal 'X' crosshair (✕)
            let d = 4.5;
            painter.line_segment(
                [screen_pt - Vec2::splat(d), screen_pt + Vec2::splat(d)],
                marker_stroke,
            );
            painter.line_segment(
                [screen_pt + Vec2::new(-d, d), screen_pt + Vec2::new(d, -d)],
                marker_stroke,
            );
        }
        SnapTargetKind::Edge => {
            // Parallel bars (∥)
            let h = 4.5;
            painter.line_segment(
                [
                    screen_pt + Vec2::new(-2.5, -h),
                    screen_pt + Vec2::new(-2.5, h),
                ],
                marker_stroke,
            );
            painter.line_segment(
                [
                    screen_pt + Vec2::new(2.5, -h),
                    screen_pt + Vec2::new(2.5, h),
                ],
                marker_stroke,
            );
        }
        SnapTargetKind::Grid => {
            // Small '+' cross
            let d = 4.0;
            painter.line_segment(
                [screen_pt - Vec2::new(d, 0.0), screen_pt + Vec2::new(d, 0.0)],
                marker_stroke,
            );
            painter.line_segment(
                [screen_pt - Vec2::new(0.0, d), screen_pt + Vec2::new(0.0, d)],
                marker_stroke,
            );
        }
        _ => {
            painter.circle_filled(screen_pt, 3.5, marker_col);
        }
    }

    // 2. Draw Dynamic Polar Ray (if active)
    if let Some(ray) = &snap_res.dynamic_ray {
        let ray_start = state.world_to_screen(ray.origin, origin);
        let ray_dir = (screen_pt - ray_start).normalized();
        let ray_end = ray_start + ray_dir * 3000.0;
        painter.line_segment([ray_start, ray_end], Stroke::new(1.0, theme::DYNAMIC_GUIDE));

        if let Some(tick_pt) = ray.tick_point {
            let s_tick = state.world_to_screen(tick_pt, origin);
            let perp = Vec2::new(-ray_dir.y, ray_dir.x) * 4.0;
            painter.line_segment(
                [s_tick - perp, s_tick + perp],
                Stroke::new(1.5, theme::DYNAMIC_GUIDE),
            );
        }
    }

    // 3. Draw Smart Alignment Lines (if active)
    for line in &snap_res.alignment_lines {
        let s_start = state.world_to_screen(line.start, origin);
        let s_end = state.world_to_screen(line.end, origin);
        painter.line_segment([s_start, s_end], Stroke::new(1.0, theme::ALIGN_GUIDE));

        let mid = (s_start + s_end.to_vec2()) * 0.5;
        let badge_rect = Rect::from_center_size(mid, Vec2::new(90.0, 16.0));
        painter.rect_filled(badge_rect, 3.0, Color32::from_black_alpha(200));
        painter.rect_stroke(
            badge_rect,
            3.0,
            Stroke::new(1.0, theme::ALIGN_GUIDE),
            egui::StrokeKind::Outside,
        );
        painter.text(
            mid,
            egui::Align2::CENTER_CENTER,
            &line.label,
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }

    // 4. Floating HUD Pill Badge near Cursor
    let hud_pos = screen_pt + Vec2::new(12.0, -14.0);
    let hud_text = &target.label;
    let font = egui::FontId::proportional(11.0);
    let galley = painter.layout_no_wrap(hud_text.clone(), font, theme::TEXT_PRIMARY);
    let padding = Vec2::new(6.0, 3.0);
    let pill_rect = Rect::from_min_size(hud_pos, galley.size() + padding * 2.0);

    painter.rect_filled(
        pill_rect,
        4.0,
        Color32::from_rgba_unmultiplied(0x1E, 0x21, 0x28, 0xEE),
    );
    painter.rect_stroke(
        pill_rect,
        4.0,
        Stroke::new(1.0, marker_col),
        egui::StrokeKind::Outside,
    );
    painter.galley(hud_pos + padding, galley, theme::TEXT_PRIMARY);
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

// ─── Laser CAM Toolpath Simulation & Preview Overlay ─────────────────────────

fn draw_toolpath_preview(painter: &Painter, state: &CanvasState, doc: &Document, viewport: Rect) {
    let plan = core_cam::generate_job_plan(doc, &state.cam_config);
    let origin = viewport.min;

    // 1. Draw Rapid Travel Vectors (laser off - dashed amber lines)
    for (from, to) in &plan.rapid_moves {
        let s_from = state.world_to_screen(*from, origin);
        let s_to = state.world_to_screen(*to, origin);

        let stroke = Stroke::new(1.2, Color32::from_rgba_unmultiplied(245, 158, 11, 200));
        painter.line_segment([s_from, s_to], stroke);

        // Direction arrowhead halfway along vector
        let dir = (s_to - s_from).normalized();
        if dir.length_sq() > 0.01 {
            let mid = (s_from + s_to.to_vec2()) * 0.5;
            let perp = Vec2::new(-dir.y, dir.x) * 3.5;
            let tip = mid + dir * 4.5;
            painter.line_segment([mid - dir * 4.5 + perp, tip], stroke);
            painter.line_segment([mid - dir * 4.5 - perp, tip], stroke);
        }
    }

    // 2. Draw Machining Toolpaths & Sequence Badges
    for tp in &plan.toolpaths {
        if tp.points.len() < 2 {
            continue;
        }
        let col = tp.mode.color_rgb();
        let stroke_col = Color32::from_rgb(col[0], col[1], col[2]);
        let cut_stroke = Stroke::new(2.4, stroke_col);

        let screen_pts: Vec<Pos2> = tp
            .points
            .iter()
            .map(|&p| state.world_to_screen(p, origin))
            .collect();

        for i in 1..screen_pts.len() {
            painter.line_segment([screen_pts[i - 1], screen_pts[i]], cut_stroke);
        }
        if tp.closed && screen_pts.len() >= 3 {
            painter.line_segment([*screen_pts.last().unwrap(), screen_pts[0]], cut_stroke);
        }

        // Draw Sequence Badge (1, 2, 3...) at cut start point
        if state.cam_config.show_sequence_badges {
            let start_s = screen_pts[0];
            let badge_radius = 8.5;
            painter.circle_filled(start_s, badge_radius, Color32::from_black_alpha(230));
            painter.circle_stroke(start_s, badge_radius, Stroke::new(1.5, stroke_col));

            let num_text = format!("{}", tp.sequence_index);
            painter.text(
                start_s,
                egui::Align2::CENTER_CENTER,
                num_text,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }

    // 3. Floating HUD Banner: Laser Preview Info
    let banner_rect =
        Rect::from_min_size(viewport.min + Vec2::new(16.0, 16.0), Vec2::new(280.0, 48.0));
    painter.rect_filled(banner_rect, 6.0, Color32::from_black_alpha(225));
    painter.rect_stroke(
        banner_rect,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(15, 147, 136)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        banner_rect.min + Vec2::new(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        format!(
            "▷ TOOLPATH PREVIEW ({} paths)\nCut: {:.1} mm · Rapid: {:.1} mm · Est: {:.1}s",
            plan.toolpaths.len(),
            plan.total_cut_length_mm,
            plan.total_rapid_length_mm,
            plan.estimated_duration_s
        ),
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );
}
