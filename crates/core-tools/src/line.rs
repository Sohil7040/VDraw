use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::Point2;

#[derive(Debug, Default)]
pub struct LineTool {
    drag_start: Option<Point2>,
    current_pt: Option<Point2>,
}

impl LineTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn calculate_end(&self, start: Point2, current: Point2, ctrl: bool) -> Point2 {
        if !ctrl {
            return current;
        }

        // Snap angle to 15-degree increments (PI / 12 radians)
        let diff = current - start;
        let len = diff.length();
        if len < 1e-6 {
            return current;
        }

        let angle = diff.angle();
        let step = std::f64::consts::PI / 12.0;
        let snapped_angle = (angle / step).round() * step;

        Point2::new(
            start.x + len * snapped_angle.cos(),
            start.y + len * snapped_angle.sin(),
        )
    }
}

impl Tool for LineTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Line
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if event.primary {
            self.drag_start = Some(event.world_pos);
            self.current_pt = Some(event.world_pos);
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
    ) -> bool {
        if self.drag_start.is_some() {
            self.current_pt = Some(event.world_pos);
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
            let end = self.calculate_end(start, event.world_pos, event.modifiers.ctrl);

            if start.distance_to(end) > 0.1 {
                let id = doc.allocate_id();
                let shape = Shape::line(id, start, end);
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
            let s_start = to_screen(start);
            let s_end = to_screen(curr);

            painter.line_segment(
                [s_start, s_end],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
            );
        }
    }
}
