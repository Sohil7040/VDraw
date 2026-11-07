//! Virtual Segment Delete tool: hover highlights candidate segment, click deletes it.

use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_boolean::apply_virtual_segment_delete_to_document;
use core_doc::{Document, UndoStack};
use core_geom::Point2;

#[derive(Debug)]
pub struct VirtualSegmentDeleteTool {
    pub hovered_pos: Option<Point2>,
    pub tolerance_mm: f64,
}

impl Default for VirtualSegmentDeleteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualSegmentDeleteTool {
    pub fn new() -> Self {
        Self {
            hovered_pos: None,
            tolerance_mm: 5.0,
        }
    }
}

impl Tool for VirtualSegmentDeleteTool {
    fn id(&self) -> ToolId {
        ToolId::VirtualSegmentDelete
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        let pos = event.pos();
        self.hovered_pos = Some(pos);
        let _ = apply_virtual_segment_delete_to_document(doc, undo, pos, self.tolerance_mm);
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        self.hovered_pos = Some(event.pos());
        false
    }

    fn on_pointer_up(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        _event: &PointerEvent,
    ) -> bool {
        false
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.hovered_pos = None;
    }

    fn cursor(&self) -> egui::CursorIcon {
        egui::CursorIcon::Crosshair
    }

    fn draw_overlay(
        &self,
        painter: &egui::Painter,
        doc: &Document,
        to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
        if let Some(pos) = self.hovered_pos {
            let cursor_pt = to_screen(pos);
            painter.text(
                cursor_pt + egui::vec2(12.0, -12.0),
                egui::Align2::LEFT_BOTTOM,
                "✂",
                egui::FontId::proportional(15.0),
                egui::Color32::from_rgb(0xFF, 0x40, 0x81),
            );

            if let Some(page) = doc.active_page() {
                let all_polys: Vec<_> = page.all_shapes().map(|s| s.to_polycurve()).collect();
                for poly in &all_polys {
                    let segments = core_boolean::find_virtual_segments(poly, &all_polys);
                    for seg in segments {
                        if seg.bounds.distance_to_point(pos) <= self.tolerance_mm {
                            let stroke =
                                egui::Stroke::new(2.5, egui::Color32::from_rgb(0xFF, 0x40, 0x81));
                            for w in seg.points.windows(2) {
                                painter.line_segment([to_screen(w[0]), to_screen(w[1])], stroke);
                            }
                            if let Some(first) = seg.points.first() {
                                painter.circle_filled(
                                    to_screen(*first),
                                    3.5,
                                    egui::Color32::from_rgb(0xFF, 0x40, 0x81),
                                );
                            }
                            if let Some(last) = seg.points.last() {
                                painter.circle_filled(
                                    to_screen(*last),
                                    3.5,
                                    egui::Color32::from_rgb(0xFF, 0x40, 0x81),
                                );
                            }
                            return;
                        }
                    }
                }
            }
        }
    }
}
