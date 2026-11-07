//! Knife tool for slicing vector shapes across a cut path.

use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_boolean::apply_knife_to_document;
use core_doc::{Document, UndoStack};
use core_geom::Point2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnifeMode {
    Straight,
    Freehand,
}

#[derive(Debug)]
pub struct KnifeTool {
    pub mode: KnifeMode,
    pub auto_close: bool,
    start_pos: Option<Point2>,
    current_pos: Option<Point2>,
    stroke_points: Vec<Point2>,
    is_dragging: bool,
}

impl Default for KnifeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl KnifeTool {
    pub fn new() -> Self {
        Self {
            mode: KnifeMode::Straight,
            auto_close: true,
            start_pos: None,
            current_pos: None,
            stroke_points: Vec::new(),
            is_dragging: false,
        }
    }

    pub fn cut_line(&self) -> Vec<Point2> {
        match self.mode {
            KnifeMode::Straight => {
                if let (Some(s), Some(c)) = (self.start_pos, self.current_pos) {
                    vec![s, c]
                } else {
                    Vec::new()
                }
            }
            KnifeMode::Freehand => self.stroke_points.clone(),
        }
    }
}

impl Tool for KnifeTool {
    fn id(&self) -> ToolId {
        ToolId::Knife
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
    ) -> bool {
        let pos = event.pos();
        self.start_pos = Some(pos);
        self.current_pos = Some(pos);
        self.stroke_points = vec![pos];
        self.is_dragging = true;
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if self.is_dragging {
            let pos = event.pos();
            self.current_pos = Some(pos);
            if self.mode == KnifeMode::Freehand {
                if let Some(last) = self.stroke_points.last() {
                    if last.distance(pos) > 1.0 {
                        self.stroke_points.push(pos);
                    }
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
        _event: &PointerEvent,
    ) -> bool {
        if self.is_dragging {
            let cut = self.cut_line();
            if cut.len() >= 2 {
                let _ = apply_knife_to_document(doc, undo, &cut, self.auto_close);
            }
        }

        self.start_pos = None;
        self.current_pos = None;
        self.stroke_points.clear();
        self.is_dragging = false;
        true
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.start_pos = None;
        self.current_pos = None;
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
        if self.is_dragging {
            let cut = self.cut_line();
            if cut.len() >= 2 {
                let stroke = egui::Stroke::new(1.8, egui::Color32::from_rgb(0xFF, 0x33, 0x44));
                for w in cut.windows(2) {
                    painter.line_segment([to_screen(w[0]), to_screen(w[1])], stroke);
                }
                let p0 = to_screen(cut[0]);
                let p1 = to_screen(*cut.last().unwrap());
                painter.circle_filled(p0, 4.0, egui::Color32::from_rgb(0xFF, 0x33, 0x44));
                painter.circle_stroke(p0, 5.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
                painter.circle_filled(p1, 4.0, egui::Color32::from_rgb(0xFF, 0x33, 0x44));
                painter.circle_stroke(p1, 5.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
            }
        }
    }
}
