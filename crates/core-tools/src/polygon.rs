use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::Point2;

/// Regular polygon interactive creation tool.
#[derive(Debug)]
pub struct PolygonTool {
    is_dragging: bool,
    start_pos: Option<Point2>,
    current_pos: Option<Point2>,
    pub sides: u32,
}

impl Default for PolygonTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PolygonTool {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            start_pos: None,
            current_pos: None,
            sides: 5,
        }
    }
}

impl Tool for PolygonTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Polygon
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.is_dragging = true;
        self.start_pos = Some(event.pos());
        self.current_pos = Some(event.pos());
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if self.is_dragging {
            self.current_pos = Some(event.pos());
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
        if !self.is_dragging {
            return false;
        }
        self.is_dragging = false;

        let start = match self.start_pos.take() {
            Some(s) => s,
            None => return false,
        };
        self.current_pos = None;

        let radius = (event.pos() - start).length();

        if radius < 0.5 {
            return false;
        }

        let id = doc.allocate_id();
        let shape = Shape::polygon(id, start, radius, self.sides);

        let cmd = Box::new(AddShapeCommand::new(shape, 0));
        undo.apply(cmd, doc);

        doc.selection.select_single(id);
        true
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.is_dragging = false;
        self.start_pos = None;
        self.current_pos = None;
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
        if let (Some(start), Some(curr)) = (self.start_pos, self.current_pos) {
            let radius = (curr - start).length();
            if radius > 0.5 {
                let n = self.sides.clamp(3, 50);
                let mut screen_pts = Vec::with_capacity(n as usize + 1);
                for i in 0..=n {
                    let angle = (i as f64) * std::f64::consts::TAU / (n as f64)
                        - std::f64::consts::FRAC_PI_2;
                    let pt = Point2::new(
                        start.x + radius * angle.cos(),
                        start.y + radius * angle.sin(),
                    );
                    screen_pts.push(to_screen(pt));
                }
                painter.add(egui::Shape::line(
                    screen_pts,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
                ));
            }
        }
    }
}
