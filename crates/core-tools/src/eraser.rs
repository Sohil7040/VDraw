//! Eraser tool for interactively subtracting material with dynamic circular brush.

use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_boolean::apply_eraser_to_document;
use core_doc::{Document, UndoStack};
use core_geom::Point2;

#[derive(Debug)]
pub struct EraserTool {
    /// Eraser brush radius in millimeters.
    pub radius: f64,
    /// Hover position for cursor overlay.
    pub cursor_pos: Option<Point2>,
    /// Accumulated stroke points while dragging.
    pub stroke_points: Vec<Point2>,
    pub is_dragging: bool,
}

impl Default for EraserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EraserTool {
    pub fn new() -> Self {
        Self {
            radius: 5.0, // 10mm default diameter
            cursor_pos: None,
            stroke_points: Vec::new(),
            is_dragging: false,
        }
    }
}

impl Tool for EraserTool {
    fn id(&self) -> ToolId {
        ToolId::Eraser
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        let pos = event.pos();
        self.cursor_pos = Some(pos);
        self.stroke_points = vec![pos];
        self.is_dragging = true;
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        let pos = event.pos();
        self.cursor_pos = Some(pos);

        if self.is_dragging {
            if let Some(last) = self.stroke_points.last() {
                if last.distance(pos) > 1.0 {
                    self.stroke_points.push(pos);
                }
            }
        }
        true
    }

    fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        _event: &PointerEvent,
    ) -> bool {
        if self.is_dragging && !self.stroke_points.is_empty() {
            let _ = apply_eraser_to_document(doc, undo, &self.stroke_points, self.radius);
        }

        self.stroke_points.clear();
        self.is_dragging = false;
        true
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.stroke_points.clear();
        self.is_dragging = false;
    }

    fn cursor(&self) -> egui::CursorIcon {
        egui::CursorIcon::Crosshair
    }

    fn draw_overlay(
        &self,
        painter: &egui::Painter,
        _doc: &Document,
        to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
        if let Some(pos) = self.cursor_pos {
            let center = to_screen(pos);
            let edge = to_screen(Point2::new(pos.x + self.radius, pos.y));
            let radius_px = (edge.x - center.x).abs().max(4.0);

            let stroke =
                egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(255, 64, 96, 220));
            let fill = egui::Color32::from_rgba_unmultiplied(255, 64, 96, 35);
            painter.circle(center, radius_px, fill, stroke);

            if self.is_dragging && self.stroke_points.len() >= 2 {
                let trail_stroke = egui::Stroke::new(
                    radius_px * 2.0,
                    egui::Color32::from_rgba_unmultiplied(255, 64, 96, 30),
                );
                for w in self.stroke_points.windows(2) {
                    painter.line_segment([to_screen(w[0]), to_screen(w[1])], trail_stroke);
                }
            }
        }
    }
}
