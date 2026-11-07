use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::Point2;

#[derive(Debug, Default)]
pub struct EllipseTool {
    drag_start: Option<Point2>,
    current_pt: Option<Point2>,
}

impl EllipseTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn calculate_ellipse(
        &self,
        start: Point2,
        current: Point2,
        shift: bool,
        ctrl: bool,
    ) -> (Point2, f64, f64) {
        if shift {
            let mut rx = (current.x - start.x).abs();
            let mut ry = (current.y - start.y).abs();
            if ctrl {
                let r = rx.max(ry);
                rx = r;
                ry = r;
            }
            (start, rx, ry)
        } else {
            let center = Point2::new((start.x + current.x) * 0.5, (start.y + current.y) * 0.5);
            let mut rx = (current.x - start.x).abs() * 0.5;
            let mut ry = (current.y - start.y).abs() * 0.5;
            if ctrl {
                let r = rx.max(ry);
                rx = r;
                ry = r;
            }
            (center, rx, ry)
        }
    }
}

impl Tool for EllipseTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Ellipse
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        if event.primary {
            self.drag_start = Some(event.pos());
            self.current_pt = Some(event.pos());
            true
        } else {
            false
        }
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        if self.drag_start.is_some() {
            self.current_pt = Some(event.pos());
            true
        } else {
            false
        }
    }

    fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if let Some(start) = self.drag_start.take() {
            let (center, rx, ry) = self.calculate_ellipse(
                start,
                event.pos(),
                event.modifiers.shift,
                event.modifiers.ctrl,
            );

            if rx > 0.1 && ry > 0.1 {
                let id = doc.allocate_id();
                let shape = Shape::ellipse(id, center, rx, ry);
                undo.apply(Box::new(AddShapeCommand::new(shape, 0)), doc);
                doc.selection.select_single(id);
            }
            self.current_pt = None;
            true
        } else {
            false
        }
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.drag_start = None;
        self.current_pt = None;
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
        if let (Some(start), Some(curr)) = (self.drag_start, self.current_pt) {
            let (center, rx, ry) = self.calculate_ellipse(start, curr, false, false);
            let s_center = to_screen(center);
            let s_edge = to_screen(Point2::new(center.x + rx, center.y + ry));
            let s_rx = (s_edge.x - s_center.x).abs();
            let s_ry = (s_edge.y - s_center.y).abs();
            let rect = egui::Rect::from_center_size(s_center, egui::vec2(s_rx * 2.0, s_ry * 2.0));

            // Approximate circle / ellipse preview in egui painter
            if (s_rx - s_ry).abs() < 1.0 {
                painter.circle_filled(
                    s_center,
                    s_rx,
                    egui::Color32::from_rgba_unmultiplied(124, 92, 255, 30),
                );
                painter.circle_stroke(
                    s_center,
                    s_rx,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
                );
            } else {
                painter.rect_stroke(
                    rect,
                    egui::CornerRadius::same(6),
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(124, 92, 255)),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
}
