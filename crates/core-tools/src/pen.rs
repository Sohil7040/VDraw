use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{AddShapeCommand, Document, Shape, UndoStack};
use core_geom::{BezierNode, NodeType, Point2, Polycurve, Subpath};

#[derive(Debug, Default)]
pub struct PenTool {
    nodes: Vec<BezierNode>,
    is_dragging_handle: bool,
    current_cursor: Option<Point2>,
}

impl PenTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn finish_path(&mut self, doc: &mut Document, undo: &mut UndoStack, closed: bool) {
        if self.nodes.len() >= 2 {
            let mut sub = Subpath::new(closed);
            sub.nodes = std::mem::take(&mut self.nodes);
            let poly = Polycurve::from_subpath(sub);
            let id = doc.allocate_id();
            let shape = Shape::path(id, poly);
            undo.apply(Box::new(AddShapeCommand::new(shape, 0)), doc);
            doc.selection.select_single(id);
        } else {
            self.nodes.clear();
        }
        self.is_dragging_handle = false;
        self.current_cursor = None;
    }
}

impl Tool for PenTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Pen
    }

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        if !event.primary {
            return false;
        }

        let p = event.pos();

        // Check if user clicked near the starting node to close the path
        if let Some(first) = self.nodes.first() {
            if self.nodes.len() >= 3 && first.point.distance_to(p) < 4.0 {
                self.finish_path(doc, undo, true);
                return true;
            }
        }

        // Add a new node
        let node = BezierNode::new(p, NodeType::Smooth);
        self.nodes.push(node);
        self.is_dragging_handle = true;
        self.current_cursor = Some(p);
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        self.current_cursor = Some(event.pos());

        if self.is_dragging_handle {
            if let Some(last) = self.nodes.last_mut() {
                let drag_vec = event.pos() - last.point;

                if drag_vec.length() > 0.5 {
                    last.set_handle_out(Some(last.point + drag_vec));
                    last.set_handle_in(Some(last.point - drag_vec));
                    last.node_type = NodeType::Symmetrical;
                }
            }
            true
        } else {
            false
        }
    }

    fn on_pointer_up(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        if self.is_dragging_handle {
            self.is_dragging_handle = false;
            if let Some(last) = self.nodes.last_mut() {
                // If user didn't drag, make it a Cusp node with no handles
                if (event.world_pos - last.point).length() <= 0.5 {
                    last.handle_in = None;
                    last.handle_out = None;
                    last.node_type = NodeType::Cusp;
                }
            }
            true
        } else {
            false
        }
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        if !self.nodes.is_empty() {
            self.nodes.clear();
            self.is_dragging_handle = false;
            self.current_cursor = None;
        }
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
        let n = self.nodes.len();
        if n == 0 {
            return;
        }

        // Draw already placed segments
        for i in 0..n - 1 {
            let n0 = &self.nodes[i];
            let n1 = &self.nodes[i + 1];
            let c = core_geom::CubicBezier::new(
                n0.point,
                n0.handle_out.unwrap_or(n0.point),
                n1.handle_in.unwrap_or(n1.point),
                n1.point,
            );
            let pts = c.flatten(0.1);
            for w in pts.windows(2) {
                painter.line_segment(
                    [to_screen(w[0]), to_screen(w[1])],
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(124, 92, 255)),
                );
            }
        }

        // Draw rubber band to current cursor
        if let (Some(last), Some(cursor)) = (self.nodes.last(), self.current_cursor) {
            let s_last = to_screen(last.point);
            let s_cur = to_screen(cursor);
            painter.line_segment(
                [s_last, s_cur],
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(124, 92, 255, 160),
                ),
            );
        }

        // Draw node markers & control handles
        for node in &self.nodes {
            let s_pt = to_screen(node.point);

            // Handles
            if let Some(h_out) = node.handle_out {
                let s_out = to_screen(h_out);
                painter.line_segment(
                    [s_pt, s_out],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(32, 211, 181)),
                );
                painter.circle_filled(s_out, 3.0, egui::Color32::from_rgb(32, 211, 181));
            }
            if let Some(h_in) = node.handle_in {
                let s_in = to_screen(h_in);
                painter.line_segment(
                    [s_pt, s_in],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(32, 211, 181)),
                );
                painter.circle_filled(s_in, 3.0, egui::Color32::from_rgb(32, 211, 181));
            }

            // Anchor point
            painter.rect_filled(
                egui::Rect::from_center_size(s_pt, egui::vec2(6.0, 6.0)),
                0.0,
                egui::Color32::WHITE,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(s_pt, egui::vec2(6.0, 6.0)),
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(124, 92, 255)),
                egui::StrokeKind::Outside,
            );
        }
    }
}
