use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::Point2;

/// Archimedean spiral interactive creation tool.
#[derive(Debug)]
pub struct SpiralTool {
    is_dragging: bool,
    start_pos: Option<Point2>,
    current_pos: Option<Point2>,
    pub turns: f64,
}

impl Default for SpiralTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiralTool {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            start_pos: None,
            current_pos: None,
            turns: 3.0,
        }
    }
}

impl Tool for SpiralTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Spiral
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

        let expansion = (event.pos() - start).length();

        if expansion < 0.5 {
            return false;
        }

        let id = doc.allocate_id();
        let shape = Shape::spiral(id, start, self.turns, expansion);

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
            let expansion = (curr - start).length();
            if expansion > 0.5 {
                let total_angle = self.turns * std::f64::consts::TAU;
                let steps = (self.turns * 60.0).max(30.0) as usize;
                let mut screen_pts = Vec::with_capacity(steps + 1);

                for i in 0..=steps {
                    let fraction = (i as f64) / (steps as f64);
                    let theta = fraction * total_angle;
                    let r = fraction * expansion;
                    let pt = Point2::new(start.x + r * theta.cos(), start.y + r * theta.sin());
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
