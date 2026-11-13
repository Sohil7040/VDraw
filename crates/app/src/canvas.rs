//! Canvas widget: pan/zoom, rulers, grid, guidelines, DragSession & SnapEngine HUD overlays, and context menu.

use crate::command_palette::{build_command_catalog, PaletteEntry};
use crate::theme::RULER_SIZE;
use crate::tokens::{RadiusTokens, SemanticColors, ThemeTokens, TypographyTokens};
use core_doc::{Document, Shape, ShapeKind, UndoStack};
use core_geom::{build_mirror_record, Point2};
use core_snap::contracts::{SnapCtx, SnapFeedback, SnapKind, View};
use core_snap::{Guideline, GuidelineOrientation, SnapResult, SnapSolver};
use core_tools::{
    AppCommand, CommandError, CommandKind, DragSession, DragState, ModifiersState, PointerEvent,
    SnapController, ToolId, ToolManager,
};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// Screen pixels per millimetre at 96 DPI.
/// 1 inch = 25.4 mm, 96 px/inch -> 96/25.4 ≈ 3.7795 px/mm.
pub const PX_PER_MM: f32 = 96.0 / 25.4;

/// A4 dimensions in mm.
const A4_W_MM: f32 = 210.0;
const A4_H_MM: f32 = 297.0;

/// Snap threshold in logical pixels for live HUD and [`DragSession`] resolution.
const SNAP_THRESHOLD_PX: f32 = 12.0;

// ─── Snap HUD styling contract ───────────────────────────────────────────────

/// Halo stroke style used by the on-canvas snap HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudStrokeStyle {
    Solid,
    Dashed,
}

/// Visual differentiation between active-mask snap targets and always-on contextual hints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudStyle {
    pub filled_dot: bool,
    pub stroke_style: HudStrokeStyle,
    pub italic_label: bool,
    pub is_contextual_hint: bool,
}

/// Classifies [`SnapKind`] into either an active-mask solid HUD indicator or a
/// hollow/dashed contextual hint (`Perpendicular`, `Tangent`).
pub fn hud_style(kind: SnapKind) -> HudStyle {
    match kind {
        SnapKind::Perpendicular | SnapKind::Tangent => HudStyle {
            filled_dot: false,
            stroke_style: HudStrokeStyle::Dashed,
            italic_label: true,
            is_contextual_hint: true,
        },
        SnapKind::Node
        | SnapKind::Center
        | SnapKind::Endpoint
        | SnapKind::Midpoint
        | SnapKind::Quadrant
        | SnapKind::Intersection
        | SnapKind::Grid => HudStyle {
            filled_dot: true,
            stroke_style: HudStrokeStyle::Solid,
            italic_label: false,
            is_contextual_hint: false,
        },
    }
}

/// Returns the contextual right-click menu entries derived from the canonical [`AppCommand`] catalog.
pub fn context_menu_entries() -> Vec<PaletteEntry> {
    build_command_catalog()
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                CommandKind::Transform | CommandKind::Boolean | CommandKind::Edit
            )
        })
        .collect()
}

/// Returns the contextual right-click [`AppCommand`] list derived from the catalog.
pub fn context_menu_commands() -> Vec<AppCommand> {
    context_menu_entries()
        .into_iter()
        .map(|entry| entry.command)
        .collect()
}

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
    /// Legacy solver result for tool drawing fallback.
    pub last_snap: Option<SnapResult>,
    /// Live [`SnapFeedback`] resolved from [`SnapController`] / [`DragSession`].
    pub last_snap_feedback: Option<SnapFeedback>,
    /// Last viewport rect rendered, used by view commands (`ZoomToSelection`, `FitPage`).
    pub last_viewport: Option<Rect>,
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
            last_snap_feedback: None,
            last_viewport: None,
            cam_config: core_cam::LaserProductionConfig::default(),
        }
    }
}

impl CanvasState {
    pub const MIN_ZOOM: f32 = 0.01;
    pub const MAX_ZOOM: f32 = 64.0;

    /// Builds the canonical [`View`] projection shared with `core-snap` and `core-tools`.
    pub fn to_view(&self, viewport: Rect) -> View {
        View::new(viewport, self.pan, (self.zoom * PX_PER_MM) as f64)
    }

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
        if !scale.is_finite() || scale.abs() <= 1e-6 {
            return Point2::ZERO;
        }
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
        let avail_w = (viewport.width() - padding * 2.0).max(32.0);
        let avail_h = (viewport.height() - padding * 2.0).max(32.0);
        let scale_x = avail_w / page_w;
        let scale_y = avail_h / page_h;
        self.zoom = scale_x.min(scale_y).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let page_w_s = A4_W_MM * PX_PER_MM * self.zoom;
        let page_h_s = A4_H_MM * PX_PER_MM * self.zoom;
        self.pan = Vec2::new(
            (viewport.width() - page_w_s) * 0.5,
            (viewport.height() - page_h_s) * 0.5,
        );
    }

    /// Zooms the viewport around a world-space bounding rect.
    pub fn zoom_to_rect(&mut self, world_rect: core_geom::Rect, viewport: Rect) {
        let w_mm = world_rect.width().max(1.0) as f32;
        let h_mm = world_rect.height().max(1.0) as f32;
        let padding = 48.0_f32;
        let avail_w = (viewport.width() - padding * 2.0).max(32.0);
        let avail_h = (viewport.height() - padding * 2.0).max(32.0);
        let scale_x = avail_w / (w_mm * PX_PER_MM);
        let scale_y = avail_h / (h_mm * PX_PER_MM);
        self.zoom = scale_x.min(scale_y).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let scale = self.zoom * PX_PER_MM;
        let center = world_rect.center();
        self.pan = Vec2::new(
            viewport.width() * 0.5 - (center.x as f32) * scale,
            viewport.height() * 0.5 - (center.y as f32) * scale,
        );
    }
}

/// Output produced by [`show_canvas`] for dispatch by [`crate::app::VDrawApp`].
#[derive(Debug, Default)]
pub struct CanvasOutput {
    pub commands: Vec<AppCommand>,
    pub drag_error: Option<CommandError>,
}

// ─── Canvas widget ────────────────────────────────────────────────────────────

/// Shows the central canvas, handling [`DragSession`] / [`SnapController`] / tools
/// and rendering document shapes, mirror preview, snap HUD, and context menu.
#[allow(clippy::too_many_arguments)]
pub fn show_canvas(
    ui: &mut egui::Ui,
    state: &mut CanvasState,
    doc: &mut Document,
    undo: &mut UndoStack,
    tools: &mut ToolManager,
    snap_solver: &mut SnapSolver,
    snap_controller: &mut SnapController,
    drag_session: &mut DragSession,
    mirror_preview: Option<(Point2, Point2)>,
) -> CanvasOutput {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let avail = ui.available_rect_before_wrap();
    let (resp, painter) = ui.allocate_painter(avail.size(), egui::Sense::click_and_drag());

    let ruler_size = if state.show_rulers { RULER_SIZE } else { 0.0 };

    let viewport = Rect::from_min_size(
        avail.min + Vec2::new(ruler_size, ruler_size),
        (avail.size() - Vec2::new(ruler_size, ruler_size)).max(Vec2::splat(1.0)),
    );
    state.last_viewport = Some(viewport);

    // 1. Canvas background
    painter.rect_filled(avail, 0.0, tokens.color.canvas_bg);

    // 2. Handle input (DragSession + SnapController + legacy drawing tools)
    let drag_error = handle_input(
        ui,
        &resp,
        state,
        doc,
        undo,
        tools,
        snap_solver,
        snap_controller,
        drag_session,
        avail,
        viewport,
    );

    // 3. Grid
    if state.show_grid {
        draw_grid(&painter, state, viewport, &tokens);
    }

    // 4. Page boundary & shadow
    draw_page(&painter, state, viewport, &tokens);

    // 5. Document shapes
    draw_shapes(&painter, state, doc, viewport);

    // 6. Non-mutating DragSession preview shape
    if let Some(preview_shape) = drag_session.preview_shape() {
        draw_preview_shape(
            &painter,
            state,
            preview_shape,
            viewport,
            tokens.color.selection_stroke,
        );
    }

    // 7. Live Mirror Custom-Line preview
    if let Some((line_pt, line_dir)) = mirror_preview {
        draw_mirror_preview(&painter, state, doc, viewport, line_pt, line_dir, &tokens);
    }

    // 8. Guidelines (user-placed and dragging)
    draw_guidelines(&painter, state, doc, viewport, &tokens);

    // 9. Active tool overlay (selection box, handles, drawing preview)
    let origin = viewport.min;
    let to_screen = |p: Point2| state.world_to_screen(p, origin);
    tools.draw_overlay(&painter, doc, &to_screen);

    // 10. Two-tier Snap HUD & legacy alignment/polar guides
    draw_snap_overlays(&painter, state, viewport, &tokens);

    // 11. Empty-state onboarding hint when document has 0 shapes
    if total_shape_count(doc) == 0 {
        draw_empty_state_hint(&painter, viewport, &tokens);
    }

    // 12. Laser Toolpath Simulation & Preview Overlay
    if state.cam_config.preview {
        draw_toolpath_preview(&painter, state, doc, viewport, &tokens);
    }

    // 13. Rulers
    if state.show_rulers {
        draw_rulers(&painter, state, avail, viewport, &tokens);
    }

    // 14. Cursor icon
    if state.panning {
        ui.ctx().set_cursor_icon(if resp.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(tools.cursor());
    }

    // 15. Contextual right-click menu derived from AppCommand catalog
    let mut commands = Vec::new();
    resp.context_menu(|ui| {
        let sel_count = doc.selection.ids.len();
        let entries = context_menu_entries();
        let mut last_kind: Option<CommandKind> = None;
        for item in entries {
            if last_kind.is_some_and(|k| k != item.kind) {
                ui.separator();
            }
            last_kind = Some(item.kind);

            let enabled = match item.kind {
                CommandKind::Boolean => {
                    if matches!(item.command, AppCommand::OffsetSelection { .. }) {
                        sel_count >= 1
                    } else {
                        sel_count >= 2
                    }
                }
                CommandKind::Transform => sel_count >= 1,
                CommandKind::Edit => true,
                _ => true,
            };

            let label = if item.shortcut.is_empty() {
                format!("{}  {}", item.icon, item.label)
            } else {
                format!("{}  {}   ({})", item.icon, item.label, item.shortcut)
            };
            let btn = ui.add_enabled(enabled, egui::Button::new(label));
            if btn.clicked() {
                commands.push(item.command);
                ui.close_menu();
            }
        }
    });

    CanvasOutput {
        commands,
        drag_error,
    }
}

fn total_shape_count(doc: &Document) -> usize {
    doc.active_page()
        .map(|p| p.layers.iter().map(|l| l.shapes.len()).sum())
        .unwrap_or(0)
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
    snap_controller: &mut SnapController,
    drag_session: &mut DragSession,
    full_rect: Rect,
    viewport: Rect,
) -> Option<CommandError> {
    let canvas_origin = viewport.min;
    let mut drag_error = None;

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
        return None;
    }

    // Ruler guideline dragging
    let h_ruler = Rect::from_min_size(
        Pos2::new(viewport.min.x, full_rect.min.y),
        Vec2::new(viewport.width(), RULER_SIZE),
    );
    let v_ruler = Rect::from_min_size(
        Pos2::new(full_rect.min.x, viewport.min.y),
        Vec2::new(RULER_SIZE, viewport.height()),
    );

    let current_pos = resp
        .hover_pos()
        .or_else(|| ui.input(|i| i.pointer.latest_pos()))
        .unwrap_or(viewport.center());
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
                        snap_controller.update_index(doc);
                    }
                }
                GuidelineDrag::Vertical(x) => {
                    if viewport.contains(current_pos) {
                        let id = doc.add_guideline(GuidelineOrientation::Vertical(x));
                        snap_solver.add_guideline(Guideline::vertical(id.0, x));
                        snap_controller.update_index(doc);
                    }
                }
            }
            state.active_guideline_drag = None;
        }
        return None;
    }

    // Cancel active DragSession on Escape
    if ui.input(|i| i.key_pressed(egui::Key::Escape))
        && !matches!(drag_session.state, DragState::Idle)
    {
        drag_session.cancel();
        snap_controller.set_reference_origin(None);
        state.last_snap_feedback = None;
        return None;
    }

    let view = state.to_view(viewport);

    // Evaluate live SnapController hover feedback when pointer is over the canvas
    if resp.hovered() || matches!(drag_session.state, DragState::Dragging { .. }) {
        let snap_ctx = SnapCtx {
            cursor_screen: current_pos,
            view: &view,
            threshold_px: SNAP_THRESHOLD_PX,
        };
        state.last_snap_feedback = snap_controller
            .get_engine()
            .resolve(&snap_ctx)
            .map(|(_, fb)| fb);
    } else if !matches!(drag_session.state, DragState::Dragging { .. }) {
        state.last_snap_feedback = None;
    }

    let drag_active = matches!(
        drag_session.state,
        DragState::Armed { .. } | DragState::Dragging { .. }
    );

    // Arm DragSession for Pick-tool body drags (keeping handles on PickTool)
    let pressed_primary = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    if tools.active_id() == ToolId::Pick && pressed_primary && resp.hovered() && !drag_active {
        let on_handle = doc
            .selection_bounds()
            .and_then(|b| doc.selection.hit_test_handle(b, world_pos, 3.0))
            .is_some();

        if !on_handle {
            if let Some(hit_id) = doc.hit_test(world_pos, 2.0) {
                let shift = ui.input(|i| i.modifiers.shift);
                if !shift && !doc.selection.ids.contains(&hit_id) {
                    doc.selection.select_single(hit_id);
                }
                let anchor = doc
                    .find_shape(hit_id)
                    .and_then(|s| s.world_bounds())
                    .map(|b| b.center())
                    .unwrap_or(world_pos);
                drag_session.arm(snap_controller.handle_mode(), hit_id, anchor, current_pos);
                snap_controller.set_reference_origin(Some(anchor));
                return None;
            }
        }
    }

    // Drive active DragSession
    if matches!(
        drag_session.state,
        DragState::Armed { .. } | DragState::Dragging { .. }
    ) {
        let released_primary = resp.drag_stopped_by(egui::PointerButton::Primary)
            || ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));

        if released_primary {
            match drag_session.release(
                current_pos,
                &view,
                SNAP_THRESHOLD_PX,
                snap_controller.get_engine(),
                doc,
                undo,
            ) {
                Ok(_) => {
                    snap_controller.update_index(doc);
                }
                Err(err) => {
                    drag_error = Some(err);
                }
            }
            snap_controller.set_reference_origin(None);
            return drag_error;
        }

        let next_state = drag_session.update(
            current_pos,
            &view,
            SNAP_THRESHOLD_PX,
            snap_controller.get_engine(),
            doc,
        );
        if let DragState::Dragging { snap, .. } = next_state {
            if snap.is_some() {
                state.last_snap_feedback = snap.clone();
            }
        }
        return None;
    }

    // Legacy tool dispatch (drawing tools, marquee select, resize/rotate handles)
    let modifiers = ui.input(|i| ModifiersState {
        shift: i.modifiers.shift,
        ctrl: i.modifiers.ctrl,
        alt: i.modifiers.alt,
    });

    let primary = resp.dragged_by(egui::PointerButton::Primary)
        || resp.clicked_by(egui::PointerButton::Primary);

    let page_dim = doc.active_page().map(|p| (p.width_mm, p.height_mm));
    let snap_res = snap_solver.snap_point(world_pos, (state.zoom * PX_PER_MM) as f64, None, &[]);
    let resolved_world = state
        .last_snap_feedback
        .as_ref()
        .map(|fb| fb.target.position)
        .unwrap_or(snap_res.point);

    let event = PointerEvent::new(world_pos, current_pos, modifiers, primary)
        .with_snapped_pos(resolved_world)
        .with_viewport((state.zoom * PX_PER_MM) as f64, page_dim);

    state.last_snap = if snap_res.snapped {
        Some(snap_res)
    } else {
        None
    };

    if resp.drag_started_by(egui::PointerButton::Primary)
        || resp.clicked_by(egui::PointerButton::Primary)
    {
        tools.on_pointer_down(doc, undo, &event, Some(snap_solver));
        snap_controller.update_index(doc);
    }

    if resp.dragged_by(egui::PointerButton::Primary) || resp.hovered() {
        tools.on_pointer_move(doc, undo, &event, Some(snap_solver));
    }

    if resp.drag_stopped_by(egui::PointerButton::Primary)
        || (resp.hovered() && ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)))
    {
        tools.on_pointer_up(doc, undo, &event);
        snap_controller.update_index(doc);
    }

    drag_error
}

// ─── Document shape rendering ────────────────────────────────────────────────

fn to_color32(c: core_doc::Color) -> Color32 {
    SemanticColors::from_doc_color(c)
}

/// Triangulates a 2D simple polygon (supporting concave shapes) using ear clipping.
fn triangulate_polygon(pts: &[Pos2]) -> Vec<[u32; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }

    let mut signed_area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    let ccw = signed_area > 0.0;

    let is_convex = |prev: Pos2, curr: Pos2, next: Pos2| -> bool {
        let cross = (curr.x - prev.x) * (next.y - curr.y) - (curr.y - prev.y) * (next.x - curr.x);
        if ccw {
            cross > 0.0
        } else {
            cross < 0.0
        }
    };

    let point_in_tri = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| -> bool {
        let sign = |p1: Pos2, p2: Pos2, p3: Pos2| -> f32 {
            (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
        };
        let d1 = sign(p, a, b);
        let d2 = sign(p, b, c);
        let d3 = sign(p, c, a);
        let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
        let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
        !(has_neg && has_pos)
    };

    let mut indices: Vec<u32> = (0..n as u32).collect();
    let mut triangles = Vec::with_capacity(n - 2);

    let mut count = indices.len();
    let mut attempts = 0;
    let max_attempts = count * count;

    let mut i = 0;
    while count > 3 && attempts < max_attempts {
        attempts += 1;
        let prev_idx = indices[(i + count - 1) % count];
        let curr_idx = indices[i % count];
        let next_idx = indices[(i + 1) % count];

        let prev = pts[prev_idx as usize];
        let curr = pts[curr_idx as usize];
        let next = pts[next_idx as usize];

        if is_convex(prev, curr, next) {
            let mut is_ear = true;
            for &k in &indices {
                if k != prev_idx
                    && k != curr_idx
                    && k != next_idx
                    && point_in_tri(pts[k as usize], prev, curr, next)
                {
                    is_ear = false;
                    break;
                }
            }

            if is_ear {
                triangles.push([prev_idx, curr_idx, next_idx]);
                indices.remove(i % count);
                count -= 1;
                continue;
            }
        }
        i = (i + 1) % count;
    }

    if count == 3 {
        triangles.push([indices[0], indices[1], indices[2]]);
    }

    triangles
}

fn draw_shapes(painter: &Painter, state: &CanvasState, doc: &Document, viewport: Rect) {
    let Some(page) = doc.active_page() else {
        return;
    };
    let origin = viewport.min;

    let top_left_world = state.screen_to_world(viewport.min, origin);
    let bottom_right_world = state.screen_to_world(viewport.max, origin);
    let world_viewport = core_geom::Rect::from_points(top_left_world, bottom_right_world);

    for layer in &page.layers {
        if !layer.visible {
            continue;
        }

        for shape in &layer.shapes {
            if !shape.visible {
                continue;
            }

            if let Some(wb) = shape.world_bounds() {
                if !world_viewport.inflate(5.0, 5.0).intersects(&wb) {
                    continue;
                }
            }

            draw_single_shape(painter, state, shape, viewport, None);
        }
    }
}

fn draw_preview_shape(
    painter: &Painter,
    state: &CanvasState,
    shape: &Shape,
    viewport: Rect,
    outline_color: Color32,
) {
    draw_single_shape(painter, state, shape, viewport, Some(outline_color));
}

fn draw_single_shape(
    painter: &Painter,
    state: &CanvasState,
    shape: &Shape,
    viewport: Rect,
    override_stroke: Option<Color32>,
) {
    let origin = viewport.min;
    let fill_col = if override_stroke.is_some() {
        shape
            .style
            .fill
            .map(|c| SemanticColors::with_alpha(to_color32(c), 0.45))
    } else {
        shape.style.fill.map(to_color32)
    };
    let stroke_prop = &shape.style.stroke;

    match &shape.kind {
        ShapeKind::Line { start, end } => {
            let p0 = shape.transform.transform_point(*start);
            let p1 = shape.transform.transform_point(*end);
            let s0 = state.world_to_screen(p0, origin);
            let s1 = state.world_to_screen(p1, origin);

            if let Some(col) = override_stroke {
                painter.line_segment([s0, s1], Stroke::new(1.8, col));
            } else if let Some(st) = stroke_prop {
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

                if let Some(fc) = fill_col {
                    if sub.closed && screen_pts.len() >= 3 {
                        let triangles = triangulate_polygon(&screen_pts);
                        let mut mesh = egui::epaint::Mesh::default();
                        for &pt in &screen_pts {
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: pt,
                                uv: egui::epaint::WHITE_UV,
                                color: fc,
                            });
                        }
                        for tri in triangles {
                            mesh.indices.extend_from_slice(&tri);
                        }
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }

                let stroke_style = if let Some(col) = override_stroke {
                    Some((1.5_f32, col))
                } else {
                    stroke_prop.as_ref().map(|st| {
                        let stroke_w = if st.hairline {
                            1.0
                        } else {
                            (st.width as f32 * state.zoom * PX_PER_MM).max(1.0)
                        };
                        (stroke_w, to_color32(st.color))
                    })
                };

                if let Some((stroke_w, col)) = stroke_style {
                    if sub.closed {
                        for w in screen_pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(stroke_w, col));
                        }
                        if let (Some(first), Some(last)) = (screen_pts.first(), screen_pts.last()) {
                            painter.line_segment([*last, *first], Stroke::new(stroke_w, col));
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

fn draw_mirror_preview(
    painter: &Painter,
    state: &CanvasState,
    doc: &Document,
    viewport: Rect,
    line_point: Point2,
    line_dir: Point2,
    tokens: &ThemeTokens,
) {
    let origin = viewport.min;
    let len = (line_dir.x * line_dir.x + line_dir.y * line_dir.y).sqrt();
    if !len.is_finite() || len <= 1e-6 {
        return;
    }
    let unit = Point2::new(line_dir.x / len, line_dir.y / len);
    let p_a = state.world_to_screen(line_point - unit * 500.0, origin);
    let p_b = state.world_to_screen(line_point + unit * 500.0, origin);
    draw_dashed_segment(
        painter,
        p_a,
        p_b,
        6.0,
        4.0,
        Stroke::new(1.5, tokens.color.align_guide),
    );

    if let Ok(rec) = build_mirror_record(line_point, line_dir) {
        let preview_col = SemanticColors::with_alpha(tokens.color.align_guide, 0.75);
        for &id in &doc.selection.ids {
            if let Some(shape) = doc.find_shape(id) {
                let mut ghost = shape.clone();
                ghost.transform = rec.matrix * ghost.transform;
                draw_preview_shape(painter, state, &ghost, viewport, preview_col);
            }
        }
    }
}

// ─── Guidelines drawing ─────────────────────────────────────────────────────

fn draw_guidelines(
    painter: &Painter,
    state: &CanvasState,
    doc: &Document,
    viewport: Rect,
    tokens: &ThemeTokens,
) {
    let origin = viewport.min;
    let guide_stroke = Stroke::new(1.0, tokens.color.guide_line);

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
                        tokens.color.guide_line,
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
                        tokens.color.guide_line,
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

    if let Some(drag) = state.active_guideline_drag {
        let active_stroke = Stroke::new(1.5, tokens.color.accent_hover);
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
                    tokens.color.accent_hover,
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
                    tokens.color.accent_hover,
                );
            }
        }
    }
}

// ─── Snapping overlays & two-tier HUD ────────────────────────────────────────

fn draw_snap_overlays(
    painter: &Painter,
    state: &CanvasState,
    viewport: Rect,
    tokens: &ThemeTokens,
) {
    // 1. Primary two-tier SnapFeedback HUD from SnapController / DragSession
    if let Some(fb) = &state.last_snap_feedback {
        let screen_pt = state.world_to_screen(fb.target.position, viewport.min);
        if viewport.contains(screen_pt) {
            draw_snap_feedback_hud(painter, screen_pt, fb, tokens);
        }
    }

    // 2. Dynamic polar / alignment lines from legacy SnapResult when active
    if let Some(snap_res) = &state.last_snap {
        if snap_res.snapped {
            let origin = viewport.min;
            let screen_pt = state.world_to_screen(snap_res.point, origin);
            if viewport.contains(screen_pt) {
                if let Some(ray) = &snap_res.dynamic_ray {
                    let ray_start = state.world_to_screen(ray.origin, origin);
                    let ray_dir = (screen_pt - ray_start).normalized();
                    let ray_end = ray_start + ray_dir * 3000.0;
                    painter.line_segment(
                        [ray_start, ray_end],
                        Stroke::new(1.0, tokens.color.polar_guide),
                    );
                    if let Some(tick_pt) = ray.tick_point {
                        let s_tick = state.world_to_screen(tick_pt, origin);
                        let perp = Vec2::new(-ray_dir.y, ray_dir.x) * 4.0;
                        painter.line_segment(
                            [s_tick - perp, s_tick + perp],
                            Stroke::new(1.5, tokens.color.polar_guide),
                        );
                    }
                }

                for line in &snap_res.alignment_lines {
                    let s_start = state.world_to_screen(line.start, origin);
                    let s_end = state.world_to_screen(line.end, origin);
                    painter
                        .line_segment([s_start, s_end], Stroke::new(1.0, tokens.color.align_guide));

                    let mid = (s_start + s_end.to_vec2()) * 0.5;
                    let badge_rect = Rect::from_center_size(mid, Vec2::new(90.0, 16.0));
                    painter.rect_filled(badge_rect, 3.0, tokens.color.hud_pill_bg);
                    painter.rect_stroke(
                        badge_rect,
                        3.0,
                        Stroke::new(1.0, tokens.color.align_guide),
                        egui::StrokeKind::Outside,
                    );
                    painter.text(
                        mid,
                        egui::Align2::CENTER_CENTER,
                        &line.label,
                        egui::FontId::proportional(10.0),
                        tokens.color.on_surface,
                    );
                }
            }
        }
    }
}

fn draw_snap_feedback_hud(
    painter: &Painter,
    screen_pt: Pos2,
    fb: &SnapFeedback,
    tokens: &ThemeTokens,
) {
    let style = hud_style(fb.target.kind);
    let snap_col = tokens.color.snap_color(fb.target.kind);
    let halo_col = SemanticColors::with_alpha(snap_col, 0.78);
    let halo_r = fb.glow_radius_px.max(8.5);

    // Outer halo: solid ring for active-mask snaps, dashed ring for contextual hints
    match style.stroke_style {
        HudStrokeStyle::Solid => {
            painter.circle_stroke(screen_pt, halo_r, Stroke::new(1.5, halo_col));
        }
        HudStrokeStyle::Dashed => {
            draw_dashed_circle(painter, screen_pt, halo_r, 8, Stroke::new(1.5, halo_col));
        }
    }

    // Center indicator: filled dot for active snaps, hollow ring for contextual hints
    if style.filled_dot {
        painter.circle_filled(screen_pt, 4.0, snap_col);
    } else {
        painter.circle_stroke(screen_pt, 4.5, Stroke::new(1.8, snap_col));
    }

    // Label pill: upright for active snaps, italic for contextual hints
    let label_text = format!("{} · {}", fb.label, fb.handle_mode.label());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &label_text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(TypographyTokens::CAPTION),
            color: tokens.color.on_surface,
            italics: style.italic_label,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    let padding = Vec2::new(6.0, 3.0);
    let hud_pos = screen_pt + Vec2::new(12.0, -16.0);
    let pill_rect = Rect::from_min_size(hud_pos, galley.size() + padding * 2.0);
    painter.rect_filled(pill_rect, RadiusTokens::SM as f32, tokens.color.hud_pill_bg);
    painter.rect_stroke(
        pill_rect,
        RadiusTokens::SM as f32,
        Stroke::new(1.0, snap_col),
        egui::StrokeKind::Outside,
    );
    painter.galley(hud_pos + padding, galley, tokens.color.on_surface);
}

fn draw_dashed_circle(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    segments: usize,
    stroke: Stroke,
) {
    let segs = segments.max(4);
    let step = std::f32::consts::TAU / (segs as f32);
    for i in 0..segs {
        let a0 = (i as f32) * step;
        let a1 = a0 + step * 0.55;
        let p0 = center + Vec2::new(a0.cos(), a0.sin()) * radius;
        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * radius;
        painter.line_segment([p0, p1], stroke);
    }
}

fn draw_dashed_segment(
    painter: &Painter,
    start: Pos2,
    end: Pos2,
    dash_px: f32,
    gap_px: f32,
    stroke: Stroke,
) {
    let delta = end - start;
    let total = delta.length();
    if total <= 1e-3 {
        return;
    }
    let dir = delta / total;
    let mut t = 0.0_f32;
    let period = (dash_px + gap_px).max(2.0);
    while t < total {
        let t_end = (t + dash_px).min(total);
        painter.line_segment([start + dir * t, start + dir * t_end], stroke);
        t += period;
    }
}

fn draw_empty_state_hint(painter: &Painter, viewport: Rect, tokens: &ThemeTokens) {
    let center = viewport.center();
    let msg = "Pick a tool to begin";
    let font = egui::FontId::proportional(TypographyTokens::BODY);
    let galley = painter.layout_no_wrap(msg.to_string(), font, tokens.color.on_surface_muted);
    let pad = Vec2::new(14.0, 8.0);
    let rect = Rect::from_center_size(center, galley.size() + pad * 2.0);
    painter.rect_filled(rect, RadiusTokens::LG as f32, tokens.color.hud_pill_bg);
    painter.rect_stroke(
        rect,
        RadiusTokens::LG as f32,
        Stroke::new(1.0, tokens.color.border),
        egui::StrokeKind::Outside,
    );
    painter.galley(rect.min + pad, galley, tokens.color.on_surface_muted);
}

// ─── Grid drawing ────────────────────────────────────────────────────────────

fn draw_grid(painter: &Painter, state: &CanvasState, viewport: Rect, tokens: &ThemeTokens) {
    let scale = state.zoom * PX_PER_MM;
    if !scale.is_finite() || scale <= 1e-6 {
        return;
    }
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
    if minor_px < 4.0 {
        return;
    }

    let page_origin_screen = state.world_to_screen(Point2::ZERO, origin);
    let offset_x = ((page_origin_screen.x - origin.x) % minor_px + minor_px) % minor_px;
    let offset_y = ((page_origin_screen.y - origin.y) % minor_px + minor_px) % minor_px;

    let minor_col = tokens.color.grid_line;
    let major_col = tokens.color.grid_major_line;
    let clip = viewport;

    let mut x = origin.x + offset_x;
    while x <= clip.max.x {
        let rel = x - page_origin_screen.x;
        let is_major = ((rel / major_px).round() * major_px - rel).abs() < 0.5;
        let col = if is_major { major_col } else { minor_col };
        painter.line_segment(
            [Pos2::new(x, clip.min.y), Pos2::new(x, clip.max.y)],
            Stroke::new(1.0, col),
        );
        x += minor_px;
    }

    let mut y = origin.y + offset_y;
    while y <= clip.max.y {
        let rel = y - page_origin_screen.y;
        let is_major = ((rel / major_px).round() * major_px - rel).abs() < 0.5;
        let col = if is_major { major_col } else { minor_col };
        painter.line_segment(
            [Pos2::new(clip.min.x, y), Pos2::new(clip.max.x, y)],
            Stroke::new(1.0, col),
        );
        y += minor_px;
    }
}

// ─── Page drawing ────────────────────────────────────────────────────────────

fn draw_page(painter: &Painter, state: &CanvasState, viewport: Rect, tokens: &ThemeTokens) {
    let origin = viewport.min;
    let tl = state.world_to_screen(Point2::ZERO, origin);
    let br = state.world_to_screen(Point2::new(A4_W_MM as f64, A4_H_MM as f64), origin);
    let page_rect = Rect::from_min_max(tl, br);

    let shadow_rect = page_rect.translate(Vec2::splat(4.0));
    painter.rect_filled(shadow_rect, 2.0, tokens.color.page_shadow);
    painter.rect_filled(page_rect, 0.0, tokens.color.page_bg);
    painter.rect_stroke(
        page_rect,
        0.0,
        Stroke::new(1.0, tokens.color.page_border),
        egui::StrokeKind::Outside,
    );
}

// ─── Ruler drawing ───────────────────────────────────────────────────────────

fn draw_rulers(
    painter: &Painter,
    state: &CanvasState,
    full_rect: Rect,
    viewport: Rect,
    tokens: &ThemeTokens,
) {
    let rs = RULER_SIZE;

    let corner = Rect::from_min_size(full_rect.min, Vec2::splat(rs));
    painter.rect_filled(corner, 0.0, tokens.color.surface_elevated);

    let h_ruler = Rect::from_min_size(
        Pos2::new(viewport.min.x, full_rect.min.y),
        Vec2::new(viewport.width(), rs),
    );
    painter.rect_filled(h_ruler, 0.0, tokens.color.surface_elevated);

    let v_ruler = Rect::from_min_size(
        Pos2::new(full_rect.min.x, viewport.min.y),
        Vec2::new(rs, viewport.height()),
    );
    painter.rect_filled(v_ruler, 0.0, tokens.color.surface_elevated);

    let scale = state.zoom * PX_PER_MM;
    if !scale.is_finite() || scale <= 1e-6 {
        return;
    }
    let (minor_mm, major_mm): (f32, f32) = if scale >= 6.0 {
        (1.0, 10.0)
    } else if scale >= 2.0 {
        (5.0, 50.0)
    } else {
        (10.0, 100.0)
    };
    let minor_px = minor_mm * scale;
    if minor_px < 2.0 {
        return;
    }

    let page_origin_screen = state.world_to_screen(Point2::ZERO, viewport.min);

    {
        let start_mm = state
            .screen_to_world(Pos2::new(viewport.min.x, 0.0), viewport.min)
            .x as f32;
        let start_tick = (start_mm / minor_mm).floor() * minor_mm;
        let mut x = page_origin_screen.x + start_tick * scale;
        let mut cur_mm = start_tick;

        while x <= viewport.max.x {
            if x >= viewport.min.x {
                let is_major = ((cur_mm / major_mm).round() * major_mm - cur_mm).abs() < 0.01
                    || cur_mm.abs() < 0.001;
                let tick_h = if is_major { rs * 0.6 } else { rs * 0.3 };
                painter.line_segment(
                    [
                        Pos2::new(x, h_ruler.max.y - tick_h),
                        Pos2::new(x, h_ruler.max.y),
                    ],
                    Stroke::new(1.0, tokens.color.ruler_tick),
                );
                if is_major {
                    let label = format!("{}", cur_mm as i32);
                    painter.text(
                        Pos2::new(x + 2.0, h_ruler.min.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::proportional(9.0),
                        tokens.color.on_surface_muted,
                    );
                }
            }
            x += minor_px;
            cur_mm += minor_mm;
        }
    }

    {
        let start_mm = state
            .screen_to_world(Pos2::new(0.0, viewport.min.y), viewport.min)
            .y as f32;
        let start_tick = (start_mm / minor_mm).floor() * minor_mm;
        let mut y = page_origin_screen.y + start_tick * scale;
        let mut cur_mm = start_tick;

        while y <= viewport.max.y {
            if y >= viewport.min.y {
                let is_major = ((cur_mm / major_mm).round() * major_mm - cur_mm).abs() < 0.01
                    || cur_mm.abs() < 0.001;
                let tick_w = if is_major { rs * 0.6 } else { rs * 0.3 };
                painter.line_segment(
                    [
                        Pos2::new(v_ruler.max.x - tick_w, y),
                        Pos2::new(v_ruler.max.x, y),
                    ],
                    Stroke::new(1.0, tokens.color.ruler_tick),
                );
                if is_major {
                    let label = format!("{}", cur_mm as i32);
                    painter.text(
                        Pos2::new(v_ruler.min.x + 1.0, y - 1.0),
                        egui::Align2::LEFT_BOTTOM,
                        label,
                        egui::FontId::proportional(9.0),
                        tokens.color.on_surface_muted,
                    );
                }
            }
            y += minor_px;
            cur_mm += minor_mm;
        }
    }

    if let Some(cur) = state.cursor_mm {
        let sx = page_origin_screen.x + (cur.x as f32) * scale;
        let sy = page_origin_screen.y + (cur.y as f32) * scale;
        let indicator_col = tokens.color.accent;

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

fn draw_toolpath_preview(
    painter: &Painter,
    state: &CanvasState,
    doc: &Document,
    viewport: Rect,
    tokens: &ThemeTokens,
) {
    let plan = core_cam::generate_job_plan(doc, &state.cam_config);
    let origin = viewport.min;

    let rapid_stroke = Stroke::new(1.2, SemanticColors::with_alpha(tokens.color.warning, 0.85));
    for (from, to) in &plan.rapid_moves {
        let s_from = state.world_to_screen(*from, origin);
        let s_to = state.world_to_screen(*to, origin);

        painter.line_segment([s_from, s_to], rapid_stroke);

        let dir = (s_to - s_from).normalized();
        if dir.length_sq() > 0.01 {
            let mid = (s_from + s_to.to_vec2()) * 0.5;
            let perp = Vec2::new(-dir.y, dir.x) * 3.5;
            let tip = mid + dir * 4.5;
            painter.line_segment([mid - dir * 4.5 + perp, tip], rapid_stroke);
            painter.line_segment([mid - dir * 4.5 - perp, tip], rapid_stroke);
        }
    }

    for tp in &plan.toolpaths {
        if tp.points.len() < 2 {
            continue;
        }
        let col = tp.mode.color_rgb();
        let stroke_col = SemanticColors::from_rgb_bytes(col);
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
            if let (Some(&last), Some(&first)) = (screen_pts.last(), screen_pts.first()) {
                painter.line_segment([last, first], cut_stroke);
            }
        }

        if state.cam_config.show_sequence_badges {
            let start_s = screen_pts[0];
            let badge_radius = 8.5;
            painter.circle_filled(start_s, badge_radius, tokens.color.hud_pill_bg);
            painter.circle_stroke(start_s, badge_radius, Stroke::new(1.5, stroke_col));

            let num_text = format!("{}", tp.sequence_index);
            painter.text(
                start_s,
                egui::Align2::CENTER_CENTER,
                num_text,
                egui::FontId::proportional(10.0),
                tokens.color.on_surface,
            );
        }
    }

    let banner_rect =
        Rect::from_min_size(viewport.min + Vec2::new(16.0, 16.0), Vec2::new(280.0, 48.0));
    painter.rect_filled(banner_rect, 6.0, tokens.color.hud_pill_bg);
    painter.rect_stroke(
        banner_rect,
        6.0,
        Stroke::new(1.0, tokens.color.laser_active),
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
        tokens.color.on_surface,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_hud_distinguishes_active_and_hint_kinds() {
        let active_kinds = [
            SnapKind::Node,
            SnapKind::Center,
            SnapKind::Endpoint,
            SnapKind::Midpoint,
            SnapKind::Quadrant,
            SnapKind::Intersection,
            SnapKind::Grid,
        ];
        for kind in active_kinds {
            let style = hud_style(kind);
            assert!(style.filled_dot, "{kind:?} should use filled_dot");
            assert_eq!(style.stroke_style, HudStrokeStyle::Solid);
            assert!(!style.italic_label, "{kind:?} should use upright label");
            assert!(!style.is_contextual_hint);
        }

        for kind in [SnapKind::Perpendicular, SnapKind::Tangent] {
            let style = hud_style(kind);
            assert!(!style.filled_dot, "{kind:?} should use hollow ring");
            assert_eq!(style.stroke_style, HudStrokeStyle::Dashed);
            assert!(style.italic_label, "{kind:?} should use italic label");
            assert!(style.is_contextual_hint);
        }
    }

    #[test]
    fn test_context_menu_derived_from_app_command() {
        let cmds = context_menu_commands();
        assert!(!cmds.is_empty());
        for cmd in &cmds {
            assert!(matches!(
                cmd.kind(),
                CommandKind::Transform | CommandKind::Boolean | CommandKind::Edit
            ));
        }
        assert!(cmds
            .iter()
            .any(|c| matches!(c, AppCommand::RotateSelection { .. })));
        assert!(cmds.iter().any(|c| matches!(c, AppCommand::BooleanOp(_))));
        assert!(cmds
            .iter()
            .any(|c| matches!(c, AppCommand::DeleteSelection)));
    }
}
