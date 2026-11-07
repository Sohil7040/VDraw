use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::Point2;

/// Star interactive creation tool.
#[derive(Debug)]
pub struct StarTool {
    is_dragging: bool,
    start_pos: Option<Point2>,
    current_pos: Option<Point2>,
    pub points: u32,
    pub sharpness: f64,
}

impl Default for StarTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StarTool {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            start_pos: None,
            current_pos: None,
            points: 5,
            sharpness: 0.5, // Inner radius is 50% of outer radius
        }
    }
}

impl Tool for StarTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Star
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.is_dragging = true;
        self.start_pos = Some(event.world_pos);
        self.current_pos = Some(event.world_pos);
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if self.is_dragging {
            self.current_pos = Some(event.world_pos);
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

        let outer_radius = (event.world_pos - start).length();
        if outer_radius < 0.5 {
            return false;
        }

        let inner_radius = outer_radius * self.sharpness.clamp(0.1, 0.9);
        let id = doc.allocate_id();
        let shape = Shape::star(id, start, outer_radius, inner_radius, self.points);

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
            let outer_radius = (curr - start).length();
            if outer_radius > 0.5 {
                let inner_radius = outer_radius * self.sharpness.clamp(0.1, 0.9);
                let n = self.points.clamp(3, 50) * 2;
                let mut screen_pts = Vec::with_capacity(n as usize + 1);

                for i in 0..=n {
                    let r = if i % 2 == 0 {
                        outer_radius
                    } else {
                        inner_radius
                    };
                    let angle = (i as f64) * std::f64::consts::TAU / (n as f64)
                        - std::f64::consts::FRAC_PI_2;
                    let pt = Point2::new(start.x + r * angle.cos(), start.y + r * angle.sin());
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
