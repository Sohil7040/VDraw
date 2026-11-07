use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::{Point2, Polycurve};

/// Freehand curve drawing tool with Schneider curve fitting.
#[derive(Debug)]
pub struct FreehandTool {
    is_drawing: bool,
    points: Vec<Point2>,
    pub tolerance: f64,
}

impl Default for FreehandTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FreehandTool {
    pub fn new() -> Self {
        Self {
            is_drawing: false,
            points: Vec::new(),
            tolerance: 1.5, // 1.5 mm tolerance
        }
    }
}

impl Tool for FreehandTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Freehand
    }

    fn on_pointer_down(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.is_drawing = true;
        self.points.clear();
        self.points.push(event.world_pos);
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if self.is_drawing {
            if let Some(&last) = self.points.last() {
                if (event.world_pos - last).length() >= 0.2 {
                    self.points.push(event.world_pos);
                }
            }
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
        if !self.is_drawing {
            return false;
        }
        self.is_drawing = false;
        self.points.push(event.world_pos);

        if self.points.len() < 2 {
            self.points.clear();
            return true;
        }

        let subpath = core_geom::fit_curve(&self.points, self.tolerance);
        self.points.clear();

        if subpath.is_empty() {
            return true;
        }

        let poly = Polycurve::from_subpath(subpath);
        let id = doc.allocate_id();
        let shape = Shape::path(id, poly);

        let cmd = Box::new(AddShapeCommand::new(shape, 0));
        undo.apply(cmd, doc);

        doc.selection.select_single(id);
        true
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.is_drawing = false;
        self.points.clear();
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
        if self.is_drawing && self.points.len() >= 2 {
            let screen_pts: Vec<egui::Pos2> = self.points.iter().map(|&p| to_screen(p)).collect();
            painter.add(egui::Shape::line(
                screen_pts,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
            ));
        }
    }
}
