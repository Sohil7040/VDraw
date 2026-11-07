use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{Document, HandleId, ShapeId, TransformMode, TransformShapesCommand, UndoStack};
use core_geom::{Affine2, Point2, Rect};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragOperation {
    Move {
        start_pt: Point2,
        initial_bounds: Option<Rect>,
    },
    MovePivot {
        start_pt: Point2,
    },
    Scale {
        handle: HandleId,
        start_pt: Point2,
        initial_bounds: Rect,
        origin_pt: Point2,
    },
    Rotate {
        pivot: Point2,
        start_angle: f64,
    },
    Marquee {
        start_pt: Point2,
        current_pt: Point2,
    },
}

#[derive(Debug, Default)]
pub struct PickTool {
    active_drag: Option<DragOperation>,
    initial_transforms: HashMap<ShapeId, Affine2>,
    hovered_handle: Option<HandleId>,
    hovered_shape: Option<ShapeId>,
    pub active_alignment_lines: Vec<core_snap::AlignmentLine>,
    pub active_snap_target: Option<core_snap::SnapTarget>,
    pub snap_label: Option<String>,
}

impl PickTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn update_hover(&mut self, doc: &Document, event: &PointerEvent) {
        let tolerance = 2.0; // 2mm world tolerance for selection hit testing
        self.hovered_handle = None;
        self.hovered_shape = None;

        if let Some(bounds) = doc.selection_bounds() {
            if let Some(h) = doc.selection.hit_test_handle(bounds, event.world_pos, 3.0) {
                self.hovered_handle = Some(h);
                return;
            }
        }

        self.hovered_shape = doc.hit_test(event.world_pos, tolerance);
    }
}

impl Tool for PickTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Pick
    }

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        if !event.primary {
            return false;
        }

        let p = event.pos();
        let bounds_opt = doc.selection_bounds();

        // 1. Check if user clicked a selection handle
        if let Some(bounds) = bounds_opt {
            if let Some(handle) = doc.selection.hit_test_handle(bounds, p, 3.0) {
                // Record initial transforms
                self.initial_transforms.clear();
                for &id in &doc.selection.ids {
                    if let Some(shape) = doc.find_shape(id) {
                        self.initial_transforms.insert(id, shape.transform);
                    }
                }

                match handle {
                    HandleId::CenterPivot => {
                        self.active_drag = Some(DragOperation::MovePivot { start_pt: p });
                        return true;
                    }
                    HandleId::RotateTopLeft
                    | HandleId::RotateTopRight
                    | HandleId::RotateBottomRight
                    | HandleId::RotateBottomLeft => {
                        let pivot = doc.selection.pivot(bounds);
                        let start_angle = (p - pivot).angle();
                        self.active_drag = Some(DragOperation::Rotate { pivot, start_angle });
                        return true;
                    }
                    _ => {
                        // Opposite anchor point for scale
                        let origin_pt = match handle {
                            HandleId::TopLeft => Point2::new(bounds.max_x, bounds.max_y),
                            HandleId::Top => Point2::new(bounds.center().x, bounds.max_y),
                            HandleId::TopRight => Point2::new(bounds.min_x, bounds.max_y),
                            HandleId::Right => Point2::new(bounds.min_x, bounds.center().y),
                            HandleId::BottomRight => Point2::new(bounds.min_x, bounds.min_y),
                            HandleId::Bottom => Point2::new(bounds.center().x, bounds.min_y),
                            HandleId::BottomLeft => Point2::new(bounds.max_x, bounds.min_y),
                            HandleId::Left => Point2::new(bounds.max_x, bounds.center().y),
                            _ => bounds.center(),
                        };

                        self.active_drag = Some(DragOperation::Scale {
                            handle,
                            start_pt: p,
                            initial_bounds: bounds,
                            origin_pt,
                        });
                        return true;
                    }
                }
            }
        }

        // 2. Check if user clicked a shape
        if let Some(hit_id) = doc.hit_test(p, 2.0) {
            if event.modifiers.shift {
                doc.selection.toggle(hit_id);
            } else if !doc.selection.contains(hit_id) {
                doc.selection.select_single(hit_id);
            } else {
                // Clicking an already selected shape toggles Rotate/Skew mode if single!
                doc.selection.select_single(hit_id);
            }

            // Start move drag with initial selection bounds
            self.initial_transforms.clear();
            for &id in &doc.selection.ids {
                if let Some(shape) = doc.find_shape(id) {
                    self.initial_transforms.insert(id, shape.transform);
                }
            }
            let initial_bounds = doc.selection_bounds();
            self.active_drag = Some(DragOperation::Move {
                start_pt: p,
                initial_bounds,
            });
            return true;
        }

        // 3. Clicked empty canvas space
        if !event.modifiers.shift {
            doc.selection.clear();
        }
        self.active_drag = Some(DragOperation::Marquee {
            start_pt: p,
            current_pt: p,
        });
        true
    }

    fn on_pointer_move(
        &mut self,
        doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        self.update_hover(doc, event);

        let p = event.pos();
        match self.active_drag {
            Some(DragOperation::Move {
                start_pt,
                initial_bounds,
            }) => {
                let raw_delta = p - start_pt;
                let mut final_delta = raw_delta;
                self.active_alignment_lines.clear();
                self.active_snap_target = None;
                self.snap_label = None;

                if !event.modifiers.alt {
                    if let Some(solver) = snap {
                        if let Some(bounds) = initial_bounds {
                            let exclude_ids: Vec<_> =
                                self.initial_transforms.keys().copied().collect();
                            let snap_res = solver.snap_transform_advanced(
                                bounds,
                                raw_delta,
                                event.zoom,
                                event.page_size,
                                &exclude_ids,
                            );
                            final_delta = snap_res.delta;
                            self.active_alignment_lines = snap_res.alignment_lines;
                            self.active_snap_target = snap_res.snap_target;
                            self.snap_label = snap_res.label;
                        }
                    }
                }

                for (&id, &init_tr) in &self.initial_transforms {
                    if let Some(shape) = doc.find_shape_mut(id) {
                        shape.transform = Affine2::translate(final_delta) * init_tr;
                    }
                }
                true
            }
            Some(DragOperation::MovePivot { .. }) => {
                // Relocate rotation pivot
                let new_pivot = if !event.modifiers.alt {
                    if let Some(solver) = snap {
                        let snap_res = solver.snap_point(p, event.zoom, None, &[]);
                        if snap_res.snapped {
                            snap_res.point
                        } else {
                            p
                        }
                    } else {
                        p
                    }
                } else {
                    p
                };
                doc.selection.custom_pivot = Some(new_pivot);
                true
            }
            Some(DragOperation::Scale {
                handle,
                start_pt: _,
                initial_bounds,
                origin_pt,
            }) => {
                let mut sx = 1.0;
                let mut sy = 1.0;

                let init_w = initial_bounds.width();
                let init_h = initial_bounds.height();

                if init_w > 1e-6 && init_h > 1e-6 {
                    let cur_w = (p.x - origin_pt.x).abs();
                    let cur_h = (p.y - origin_pt.y).abs();

                    match handle {
                        HandleId::Left | HandleId::Right => {
                            sx = cur_w / init_w;
                        }
                        HandleId::Top | HandleId::Bottom => {
                            sy = cur_h / init_h;
                        }
                        _ => {
                            sx = cur_w / init_w;
                            sy = cur_h / init_h;
                            if event.modifiers.ctrl {
                                let s = sx.max(sy);
                                sx = s;
                                sy = s;
                            }
                        }
                    }
                }

                let scale_tr = Affine2::scale_around(sx, sy, origin_pt);
                for (&id, &init_tr) in &self.initial_transforms {
                    if let Some(shape) = doc.find_shape_mut(id) {
                        shape.transform = scale_tr * init_tr;
                    }
                }
                true
            }
            Some(DragOperation::Rotate { pivot, start_angle }) => {
                let cur_angle = (p - pivot).angle();
                let mut delta_angle = cur_angle - start_angle;

                if event.modifiers.ctrl {
                    // Snap to 15-degree steps
                    let step = std::f64::consts::PI / 12.0;
                    delta_angle = (delta_angle / step).round() * step;
                }

                let rot_tr = Affine2::rotate_around(delta_angle, pivot);
                for (&id, &init_tr) in &self.initial_transforms {
                    if let Some(shape) = doc.find_shape_mut(id) {
                        shape.transform = rot_tr * init_tr;
                    }
                }
                true
            }
            Some(DragOperation::Marquee { start_pt, .. }) => {
                self.active_drag = Some(DragOperation::Marquee {
                    start_pt,
                    current_pt: p,
                });
                true
            }
            None => false,
        }
    }

    fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.active_alignment_lines.clear();
        self.active_snap_target = None;
        self.snap_label = None;

        match self.active_drag.take() {
            Some(DragOperation::Move { .. })
            | Some(DragOperation::MovePivot { .. })
            | Some(DragOperation::Scale { .. })
            | Some(DragOperation::Rotate { .. }) => {
                // Record undoable transform changes
                let mut changes = Vec::new();
                for (&id, &init_tr) in &self.initial_transforms {
                    if let Some(shape) = doc.find_shape(id) {
                        if shape.transform != init_tr {
                            changes.push((id, init_tr, shape.transform));
                        }
                    }
                }

                if !changes.is_empty() {
                    undo.apply(
                        Box::new(TransformShapesCommand::new("Transform", changes)),
                        doc,
                    );
                }
                self.initial_transforms.clear();
                true
            }
            Some(DragOperation::Marquee {
                start_pt,
                current_pt: _,
            }) => {
                let marquee_rect = Rect::from_points(start_pt, event.world_pos);
                if marquee_rect.width() > 1.0 && marquee_rect.height() > 1.0 {
                    let mut matched_ids = Vec::new();
                    if let Some(page) = doc.active_page() {
                        for shape in page.all_shapes() {
                            if let Some(sb) = shape.world_bounds() {
                                if marquee_rect.intersects(&sb) {
                                    matched_ids.push(shape.id);
                                }
                            }
                        }
                    }
                    for id in matched_ids {
                        doc.selection.add(id);
                    }
                }
                true
            }
            None => false,
        }
    }

    fn on_cancel(&mut self, doc: &mut Document) {
        self.active_alignment_lines.clear();
        self.active_snap_target = None;
        self.snap_label = None;

        // Revert live preview transforms if drag was active
        if self.active_drag.is_some() {
            for (&id, &init_tr) in &self.initial_transforms {
                if let Some(shape) = doc.find_shape_mut(id) {
                    shape.transform = init_tr;
                }
            }
            self.active_drag = None;
            self.initial_transforms.clear();
        }
    }

    fn cursor(&self) -> egui::CursorIcon {
        if let Some(handle) = self.hovered_handle {
            match handle {
                HandleId::CenterPivot => egui::CursorIcon::Crosshair,
                HandleId::TopLeft | HandleId::BottomRight => egui::CursorIcon::ResizeNorthWest,
                HandleId::TopRight | HandleId::BottomLeft => egui::CursorIcon::ResizeNorthEast,
                HandleId::Top | HandleId::Bottom => egui::CursorIcon::ResizeNorth,
                HandleId::Left | HandleId::Right => egui::CursorIcon::ResizeEast,
                HandleId::RotateTopLeft
                | HandleId::RotateTopRight
                | HandleId::RotateBottomRight
                | HandleId::RotateBottomLeft => egui::CursorIcon::Crosshair,
                _ => egui::CursorIcon::Default,
            }
        } else if self.hovered_shape.is_some() {
            egui::CursorIcon::Move
        } else {
            egui::CursorIcon::Default
        }
    }

    fn draw_overlay(
        &self,
        painter: &egui::Painter,
        doc: &Document,
        to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
        // 1. Draw marquee if active
        if let Some(DragOperation::Marquee {
            start_pt,
            current_pt,
        }) = self.active_drag
        {
            let s_min = to_screen(Point2::new(
                start_pt.x.min(current_pt.x),
                start_pt.y.min(current_pt.y),
            ));
            let s_max = to_screen(Point2::new(
                start_pt.x.max(current_pt.x),
                start_pt.y.max(current_pt.y),
            ));
            let rect = egui::Rect::from_two_pos(s_min, s_max);

            painter.rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(124, 92, 255, 30),
            );
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(124, 92, 255)),
                egui::StrokeKind::Outside,
            );
        }

        // 2. Draw selection box, handles, and prominent Center Crosshair (⊕)
        if let Some(bounds) = doc.selection_bounds() {
            let s_min = to_screen(Point2::new(bounds.min_x, bounds.min_y));
            let s_max = to_screen(Point2::new(bounds.max_x, bounds.max_y));
            let s_rect = egui::Rect::from_two_pos(s_min, s_max);

            // Bounding box outline
            painter.rect_stroke(
                s_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(124, 92, 255)),
                egui::StrokeKind::Outside,
            );

            // ─── Prominent Center Crosshair (⊕) ──────────────────────────────
            let center_s = to_screen(bounds.center());
            let cyan_col = egui::Color32::from_rgb(6, 182, 212);
            let shadow_col = egui::Color32::from_black_alpha(160);

            // Subtle dark drop-shadow for high visibility against any artwork
            painter.circle_stroke(center_s, 6.0, egui::Stroke::new(2.5, shadow_col));
            painter.circle_stroke(center_s, 5.5, egui::Stroke::new(1.5, cyan_col));
            painter.circle_filled(center_s, 2.0, cyan_col);

            let ch = 9.0;
            // Horizontal crosshair with shadow
            painter.line_segment(
                [
                    center_s - egui::vec2(ch, 0.0),
                    center_s + egui::vec2(ch, 0.0),
                ],
                egui::Stroke::new(2.5, shadow_col),
            );
            painter.line_segment(
                [
                    center_s - egui::vec2(ch, 0.0),
                    center_s + egui::vec2(ch, 0.0),
                ],
                egui::Stroke::new(1.5, cyan_col),
            );
            // Vertical crosshair with shadow
            painter.line_segment(
                [
                    center_s - egui::vec2(0.0, ch),
                    center_s + egui::vec2(0.0, ch),
                ],
                egui::Stroke::new(2.5, shadow_col),
            );
            painter.line_segment(
                [
                    center_s - egui::vec2(0.0, ch),
                    center_s + egui::vec2(0.0, ch),
                ],
                egui::Stroke::new(1.5, cyan_col),
            );

            // Selection handles
            let handles = doc.selection.handles(bounds);
            for h in handles {
                let s_pt = to_screen(h.point);
                let is_hovered = self.hovered_handle == Some(h.id);
                let stroke_col = if is_hovered {
                    egui::Color32::from_rgb(155, 128, 255)
                } else {
                    egui::Color32::from_rgb(124, 92, 255)
                };

                match doc.selection.transform_mode {
                    TransformMode::Scale => {
                        let size = if is_hovered { 8.0 } else { 6.0 };
                        painter.rect_filled(
                            egui::Rect::from_center_size(s_pt, egui::vec2(size, size)),
                            0.0,
                            egui::Color32::WHITE,
                        );
                        painter.rect_stroke(
                            egui::Rect::from_center_size(s_pt, egui::vec2(size, size)),
                            0.0,
                            egui::Stroke::new(1.0, stroke_col),
                            egui::StrokeKind::Outside,
                        );
                    }
                    TransformMode::RotateSkew => {
                        if h.id == HandleId::CenterPivot {
                            // Crosshair pivot in emerald
                            let pivot_col = egui::Color32::from_rgb(16, 185, 129);
                            painter.circle_stroke(s_pt, 7.0, egui::Stroke::new(1.8, pivot_col));
                            painter.circle_filled(s_pt, 2.5, pivot_col);
                        } else {
                            // Circle rotation / skew handle
                            let radius = if is_hovered { 5.0 } else { 4.0 };
                            painter.circle_filled(s_pt, radius, egui::Color32::WHITE);
                            painter.circle_stroke(s_pt, radius, egui::Stroke::new(1.5, stroke_col));
                        }
                    }
                }
            }
        } else if let Some(hover_id) = self.hovered_shape {
            // Subtle center hint on unselected hovered shape
            if let Some(shape) = doc.find_shape(hover_id) {
                if let Some(sb) = shape.world_bounds() {
                    let sc = to_screen(sb.center());
                    painter.circle_stroke(
                        sc,
                        4.5,
                        egui::Stroke::new(
                            1.2,
                            egui::Color32::from_rgba_unmultiplied(6, 182, 212, 140),
                        ),
                    );
                    painter.circle_filled(
                        sc,
                        1.5,
                        egui::Color32::from_rgba_unmultiplied(6, 182, 212, 140),
                    );
                }
            }
        }

        // 3. Draw Live Dynamic Alignment Lines while moving
        for line in &self.active_alignment_lines {
            let s_start = to_screen(line.start);
            let s_end = to_screen(line.end);
            let line_col =
                egui::Color32::from_rgb(line.color_rgb[0], line.color_rgb[1], line.color_rgb[2]);

            painter.line_segment([s_start, s_end], egui::Stroke::new(1.5, line_col));

            let mid = (s_start + s_end.to_vec2()) * 0.5;
            let galley = painter.layout_no_wrap(
                line.label.clone(),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            let padding = egui::vec2(8.0, 3.0);
            let badge_rect = egui::Rect::from_center_size(mid, galley.size() + padding * 2.0);

            painter.rect_filled(badge_rect, 4.0, egui::Color32::from_black_alpha(220));
            painter.rect_stroke(
                badge_rect,
                4.0,
                egui::Stroke::new(1.0, line_col),
                egui::StrokeKind::Outside,
            );
            painter.galley(badge_rect.min + padding, galley, egui::Color32::WHITE);
        }

        // 4. Draw Winning Snap Target Marker while moving
        if let Some(target) = &self.active_snap_target {
            let s_pt = to_screen(target.point);
            let cyan = egui::Color32::from_rgb(6, 182, 212);

            match target.kind {
                core_snap::SnapTargetKind::Center => {
                    // Pulsing / glowing target circle with crosshair
                    painter.circle_stroke(s_pt, 8.0, egui::Stroke::new(2.0, cyan));
                    painter.circle_filled(s_pt, 3.0, cyan);
                    painter.line_segment(
                        [s_pt - egui::vec2(12.0, 0.0), s_pt + egui::vec2(12.0, 0.0)],
                        egui::Stroke::new(1.5, cyan),
                    );
                    painter.line_segment(
                        [s_pt - egui::vec2(0.0, 12.0), s_pt + egui::vec2(0.0, 12.0)],
                        egui::Stroke::new(1.5, cyan),
                    );
                }
                core_snap::SnapTargetKind::Node => {
                    let r = egui::Rect::from_center_size(s_pt, egui::vec2(10.0, 10.0));
                    painter.rect_filled(r, 1.0, egui::Color32::WHITE);
                    painter.rect_stroke(
                        r,
                        1.0,
                        egui::Stroke::new(2.0, cyan),
                        egui::StrokeKind::Outside,
                    );
                }
                core_snap::SnapTargetKind::Midpoint => {
                    let half = 6.0;
                    let pts = vec![
                        s_pt + egui::vec2(0.0, -half),
                        s_pt + egui::vec2(half, 0.0),
                        s_pt + egui::vec2(0.0, half),
                        s_pt + egui::vec2(-half, 0.0),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        egui::Color32::WHITE,
                        egui::Stroke::new(2.0, cyan),
                    ));
                }
                _ => {
                    painter.circle_stroke(s_pt, 6.0, egui::Stroke::new(1.8, cyan));
                }
            }

            // HUD Pill badge
            let hud_pos = s_pt + egui::vec2(14.0, -16.0);
            let text = format!("SNAP: {}", target.label);
            let galley = painter.layout_no_wrap(
                text,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            let padding = egui::vec2(6.0, 3.0);
            let badge_rect = egui::Rect::from_min_size(hud_pos, galley.size() + padding * 2.0);
            painter.rect_filled(badge_rect, 4.0, egui::Color32::from_black_alpha(220));
            painter.rect_stroke(
                badge_rect,
                4.0,
                egui::Stroke::new(1.0, cyan),
                egui::StrokeKind::Outside,
            );
            painter.galley(badge_rect.min + padding, galley, egui::Color32::WHITE);
        }
    }
}
