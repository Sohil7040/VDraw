use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::{Point2, Rect};

#[derive(Debug, Default)]
pub struct RectangleTool {
    drag_start: Option<Point2>,
    current_pt: Option<Point2>,
}

impl RectangleTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn calculate_rect(&self, start: Point2, current: Point2, shift: bool, ctrl: bool) -> Rect {
        if shift {
            // Draw from center
            let mut dx = (current.x - start.x).abs();
            let mut dy = (current.y - start.y).abs();
            if ctrl {
                let s = dx.max(dy);
                dx = s;
                dy = s;
            }
            Rect::new(start.x - dx, start.y - dy, start.x + dx, start.y + dy)
        } else {
            // Draw from corner
            let mut w = current.x - start.x;
            let mut h = current.y - start.y;
            if ctrl {
                let s = w.abs().max(h.abs());
                w = s * w.signum();
                h = s * h.signum();
            }
            Rect::new(start.x, start.y, start.x + w, start.y + h)
        }
    }
}

impl Tool for RectangleTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Rectangle
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
            let current = event.pos();

            let rect =
                self.calculate_rect(start, current, event.modifiers.shift, event.modifiers.ctrl);

            if rect.width() > 0.1 && rect.height() > 0.1 {
                let id = doc.allocate_id();
                let shape = Shape::rectangle(id, rect, 0.0);
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
            let rect = self.calculate_rect(start, curr, false, false);
            let s_min = to_screen(Point2::new(rect.min_x, rect.min_y));
            let s_max = to_screen(Point2::new(rect.max_x, rect.max_y));
            let screen_rect = egui::Rect::from_two_pos(s_min, s_max);

            painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(124, 92, 255, 30),
            );
            painter.rect_stroke(
                screen_rect,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
                egui::StrokeKind::Outside,
            );
        }
    }
}
