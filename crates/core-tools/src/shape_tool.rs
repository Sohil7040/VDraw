use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_doc::{Document, ModifyShapeKindCommand, ShapeId, ShapeKind, UndoStack};
use core_geom::{NodeType, Point2, Polycurve};
use std::time::Instant;

/// Identifies what is currently being dragged in the Node/Shape tool.
#[derive(Debug, Clone, PartialEq)]
enum DragTarget {
    None,
    Anchor {
        shape_id: ShapeId,
        subpath_idx: usize,
        node_idx: usize,
        start_point: Point2,
        initial_poly: Polycurve,
    },
    HandleIn {
        shape_id: ShapeId,
        subpath_idx: usize,
        node_idx: usize,
        initial_poly: Polycurve,
    },
    HandleOut {
        shape_id: ShapeId,
        subpath_idx: usize,
        node_idx: usize,
        initial_poly: Polycurve,
    },
}

/// Identifies a specific node on a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelectedNode {
    pub shape_id: ShapeId,
    pub subpath_idx: usize,
    pub node_idx: usize,
}

/// Shape / Node editing tool (Shortcut: F10).
///
/// Allows direct manipulation of anchor points, control handles (with Cusp, Smooth,
/// and Symmetrical constraint propagation), curve segment splitting, and node deletion.
#[derive(Debug)]
pub struct ShapeTool {
    drag_target: DragTarget,
    pub selected_nodes: Vec<SelectedNode>,
    hover_node: Option<SelectedNode>,
    hover_handle_in: Option<SelectedNode>,
    hover_handle_out: Option<SelectedNode>,
    last_click: Option<(Instant, Point2)>,
}

impl Default for ShapeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeTool {
    pub fn new() -> Self {
        Self {
            drag_target: DragTarget::None,
            selected_nodes: Vec::new(),
            hover_node: None,
            hover_handle_in: None,
            hover_handle_out: None,
            last_click: None,
        }
    }

    /// Set node type for currently selected nodes.
    pub fn set_selected_nodes_type(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        new_type: NodeType,
    ) {
        for sel in &self.selected_nodes {
            if let Some(shape) = doc.find_shape_mut(sel.shape_id) {
                let old_kind = shape.kind.clone();
                if let ShapeKind::Path(poly) = &mut shape.kind {
                    if let Some(sub) = poly.subpaths.get_mut(sel.subpath_idx) {
                        if sub.set_node_type(sel.node_idx, new_type) {
                            let new_kind = shape.kind.clone();
                            let cmd = Box::new(ModifyShapeKindCommand::new(
                                sel.shape_id,
                                format!("Set Node to {:?}", new_type),
                                old_kind,
                                new_kind,
                            ));
                            undo.apply(cmd, doc);
                        }
                    }
                }
            }
        }
    }

    /// Delete currently selected nodes.
    pub fn delete_selected_nodes(&mut self, doc: &mut Document, undo: &mut UndoStack) {
        let mut sorted = self.selected_nodes.clone();
        sorted.sort_by(|a, b| {
            b.shape_id
                .cmp(&a.shape_id)
                .then(b.subpath_idx.cmp(&a.subpath_idx))
                .then(b.node_idx.cmp(&a.node_idx))
        });

        for sel in sorted {
            if let Some(shape) = doc.find_shape_mut(sel.shape_id) {
                let old_kind = shape.kind.clone();
                if let ShapeKind::Path(poly) = &mut shape.kind {
                    if let Some(sub) = poly.subpaths.get_mut(sel.subpath_idx) {
                        if sub.remove_node(sel.node_idx) {
                            let new_kind = shape.kind.clone();
                            let cmd = Box::new(ModifyShapeKindCommand::new(
                                sel.shape_id,
                                "Delete Node",
                                old_kind,
                                new_kind,
                            ));
                            undo.apply(cmd, doc);
                        }
                    }
                }
            }
        }
        self.selected_nodes.clear();
    }
}

impl Tool for ShapeTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> ToolId {
        ToolId::Shape
    }

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        let tolerance = 3.0; // 3 mm hit radius in world units
        let click_pos = event.world_pos;

        // Check for double click (within 400ms and 2mm)
        let is_double_click = if let Some((t, p)) = self.last_click {
            t.elapsed().as_millis() < 400 && (click_pos - p).length() < 2.0
        } else {
            false
        };
        self.last_click = Some((Instant::now(), click_pos));

        // Ensure selected shapes are converted to paths for editing
        let selected_ids: Vec<ShapeId> = doc.selection.ids.iter().copied().collect();
        for id in &selected_ids {
            if let Some(shape) = doc.find_shape_mut(*id) {
                shape.convert_to_path();
            }
        }

        // 1. Check if clicking on any handle (handle_in or handle_out) of selected nodes
        for sel in &self.selected_nodes {
            if let Some(shape) = doc.find_shape(sel.shape_id) {
                if let ShapeKind::Path(poly) = &shape.kind {
                    if let Some(sub) = poly.subpaths.get(sel.subpath_idx) {
                        if let Some(node) = sub.nodes.get(sel.node_idx) {
                            let tr = shape.transform;

                            // Handle In
                            if let Some(hin) = node.handle_in {
                                let w_hin = tr.transform_point(hin);
                                if (click_pos - w_hin).length() <= tolerance {
                                    self.drag_target = DragTarget::HandleIn {
                                        shape_id: sel.shape_id,
                                        subpath_idx: sel.subpath_idx,
                                        node_idx: sel.node_idx,
                                        initial_poly: poly.clone(),
                                    };
                                    return true;
                                }
                            }

                            // Handle Out
                            if let Some(hout) = node.handle_out {
                                let w_hout = tr.transform_point(hout);
                                if (click_pos - w_hout).length() <= tolerance {
                                    self.drag_target = DragTarget::HandleOut {
                                        shape_id: sel.shape_id,
                                        subpath_idx: sel.subpath_idx,
                                        node_idx: sel.node_idx,
                                        initial_poly: poly.clone(),
                                    };
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Check if clicking on an anchor point
        for &shape_id in &selected_ids {
            if let Some(shape) = doc.find_shape(shape_id) {
                if let ShapeKind::Path(poly) = &shape.kind {
                    let tr = shape.transform;
                    for (sub_idx, sub) in poly.subpaths.iter().enumerate() {
                        for (node_idx, node) in sub.nodes.iter().enumerate() {
                            let w_pt = tr.transform_point(node.point);
                            if (click_pos - w_pt).length() <= tolerance {
                                let sel = SelectedNode {
                                    shape_id,
                                    subpath_idx: sub_idx,
                                    node_idx,
                                };
                                if !event.modifiers.shift {
                                    self.selected_nodes.clear();
                                }
                                if !self.selected_nodes.contains(&sel) {
                                    self.selected_nodes.push(sel);
                                }

                                self.drag_target = DragTarget::Anchor {
                                    shape_id,
                                    subpath_idx: sub_idx,
                                    node_idx,
                                    start_point: w_pt,
                                    initial_poly: poly.clone(),
                                };
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 3. Double click on a segment splits the curve and inserts a new node!
        if is_double_click {
            for &shape_id in &selected_ids {
                if let Some(shape) = doc.find_shape_mut(shape_id) {
                    let tr_inv = shape
                        .transform
                        .inverse()
                        .unwrap_or(core_geom::Affine2::IDENTITY);
                    let local_pt = tr_inv.transform_point(click_pos);
                    let old_kind = shape.kind.clone();
                    if let ShapeKind::Path(poly) = &mut shape.kind {
                        for (sub_idx, sub) in poly.subpaths.iter_mut().enumerate() {
                            let cubics = sub.to_cubics();
                            for (seg_idx, c) in cubics.iter().enumerate() {
                                let (t, _closest, dist) = c.closest_point(local_pt);
                                if dist <= tolerance && t > 0.05 && t < 0.95 {
                                    if let Some(new_idx) = sub.split_segment(seg_idx, t) {
                                        let new_kind = ShapeKind::Path(poly.clone());
                                        let cmd = Box::new(ModifyShapeKindCommand::new(
                                            shape_id,
                                            "Insert Node",
                                            old_kind,
                                            new_kind,
                                        ));
                                        undo.apply(cmd, doc);

                                        self.selected_nodes = vec![SelectedNode {
                                            shape_id,
                                            subpath_idx: sub_idx,
                                            node_idx: new_idx,
                                        }];
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Click empty canvas: test if clicking any other shape to select it for node editing
        if !event.modifiers.shift {
            self.selected_nodes.clear();
        }

        let mut hit_shape_id = None;
        if let Some(page) = doc.active_page() {
            for layer in page.layers.iter().rev() {
                if !layer.visible || layer.locked {
                    continue;
                }
                for shape in layer.shapes.iter().rev() {
                    if shape.hit_test(click_pos, tolerance) {
                        hit_shape_id = Some(shape.id);
                        break;
                    }
                }
                if hit_shape_id.is_some() {
                    break;
                }
            }
        }

        if let Some(shape_id) = hit_shape_id {
            doc.selection.select_single(shape_id);
            if let Some(s) = doc.find_shape_mut(shape_id) {
                s.convert_to_path();
            }
            return true;
        }

        doc.selection.clear();
        false
    }

    fn on_pointer_move(
        &mut self,
        doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
        _snap: Option<&core_snap::SnapSolver>,
    ) -> bool {
        match &self.drag_target {
            DragTarget::Anchor {
                shape_id,
                subpath_idx,
                node_idx,
                initial_poly: _,
                ..
            } => {
                let (sid, sidx, nidx) = (*shape_id, *subpath_idx, *node_idx);
                if let Some(shape) = doc.find_shape_mut(sid) {
                    let tr_inv = shape
                        .transform
                        .inverse()
                        .unwrap_or(core_geom::Affine2::IDENTITY);
                    let local_pt = tr_inv.transform_point(event.pos());
                    if let ShapeKind::Path(poly) = &mut shape.kind {
                        if let Some(sub) = poly.subpaths.get_mut(sidx) {
                            if let Some(node) = sub.nodes.get_mut(nidx) {
                                let delta = local_pt - node.point;
                                node.translate(delta);
                                return true;
                            }
                        }
                    }
                }
            }
            DragTarget::HandleIn {
                shape_id,
                subpath_idx,
                node_idx,
                ..
            } => {
                let (sid, sidx, nidx) = (*shape_id, *subpath_idx, *node_idx);
                if let Some(shape) = doc.find_shape_mut(sid) {
                    let tr_inv = shape
                        .transform
                        .inverse()
                        .unwrap_or(core_geom::Affine2::IDENTITY);
                    let local_pt = tr_inv.transform_point(event.pos());
                    if let ShapeKind::Path(poly) = &mut shape.kind {
                        if let Some(sub) = poly.subpaths.get_mut(sidx) {
                            if let Some(node) = sub.nodes.get_mut(nidx) {
                                node.set_handle_in(Some(local_pt));
                                return true;
                            }
                        }
                    }
                }
            }
            DragTarget::HandleOut {
                shape_id,
                subpath_idx,
                node_idx,
                ..
            } => {
                let (sid, sidx, nidx) = (*shape_id, *subpath_idx, *node_idx);
                if let Some(shape) = doc.find_shape_mut(sid) {
                    let tr_inv = shape
                        .transform
                        .inverse()
                        .unwrap_or(core_geom::Affine2::IDENTITY);
                    let local_pt = tr_inv.transform_point(event.pos());
                    if let ShapeKind::Path(poly) = &mut shape.kind {
                        if let Some(sub) = poly.subpaths.get_mut(sidx) {
                            if let Some(node) = sub.nodes.get_mut(nidx) {
                                node.set_handle_out(Some(local_pt));
                                return true;
                            }
                        }
                    }
                }
            }
            DragTarget::None => {
                // Update hover indicators
                let tolerance = 3.0;
                self.hover_node = None;
                self.hover_handle_in = None;
                self.hover_handle_out = None;

                for sel in &self.selected_nodes {
                    if let Some(shape) = doc.find_shape(sel.shape_id) {
                        if let ShapeKind::Path(poly) = &shape.kind {
                            if let Some(sub) = poly.subpaths.get(sel.subpath_idx) {
                                if let Some(node) = sub.nodes.get(sel.node_idx) {
                                    let tr = shape.transform;
                                    if let Some(hin) = node.handle_in {
                                        if (event.world_pos - tr.transform_point(hin)).length()
                                            <= tolerance
                                        {
                                            self.hover_handle_in = Some(*sel);
                                            return true;
                                        }
                                    }
                                    if let Some(hout) = node.handle_out {
                                        if (event.world_pos - tr.transform_point(hout)).length()
                                            <= tolerance
                                        {
                                            self.hover_handle_out = Some(*sel);
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let selected_ids: Vec<ShapeId> = doc.selection.ids.iter().copied().collect();
                for shape_id in selected_ids {
                    if let Some(shape) = doc.find_shape(shape_id) {
                        if let ShapeKind::Path(poly) = &shape.kind {
                            let tr = shape.transform;
                            for (sub_idx, sub) in poly.subpaths.iter().enumerate() {
                                for (node_idx, node) in sub.nodes.iter().enumerate() {
                                    let w_pt = tr.transform_point(node.point);
                                    if (event.world_pos - w_pt).length() <= tolerance {
                                        self.hover_node = Some(SelectedNode {
                                            shape_id,
                                            subpath_idx: sub_idx,
                                            node_idx,
                                        });
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        _event: &PointerEvent,
    ) -> bool {
        match std::mem::replace(&mut self.drag_target, DragTarget::None) {
            DragTarget::Anchor {
                shape_id,
                initial_poly,
                ..
            }
            | DragTarget::HandleIn {
                shape_id,
                initial_poly,
                ..
            }
            | DragTarget::HandleOut {
                shape_id,
                initial_poly,
                ..
            } => {
                if let Some(shape) = doc.find_shape(shape_id) {
                    let old_kind = ShapeKind::Path(initial_poly);
                    let new_kind = shape.kind.clone();
                    if old_kind != new_kind {
                        let cmd = Box::new(ModifyShapeKindCommand::new(
                            shape_id,
                            "Edit Curve Node",
                            old_kind,
                            new_kind,
                        ));
                        undo.apply(cmd, doc);
                    }
                }
                true
            }
            DragTarget::None => false,
        }
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.drag_target = DragTarget::None;
    }

    fn cursor(&self) -> egui::CursorIcon {
        if self.hover_handle_in.is_some() || self.hover_handle_out.is_some() {
            egui::CursorIcon::Crosshair
        } else if self.hover_node.is_some() {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Default
        }
    }

    fn draw_overlay(
        &self,
        painter: &egui::Painter,
        doc: &Document,
        to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
        let handle_line_color = egui::Color32::from_rgb(32, 211, 181); // Teal
        let handle_dot_color = egui::Color32::from_rgb(32, 211, 181);
        let selected_anchor_color = egui::Color32::from_rgb(124, 92, 255); // Electric violet
        let unselected_anchor_fill = egui::Color32::WHITE;
        let border_color = egui::Color32::from_rgb(22, 24, 29);

        for &shape_id in &doc.selection.ids {
            if let Some(shape) = doc.find_shape(shape_id) {
                if let ShapeKind::Path(poly) = &shape.kind {
                    let tr = shape.transform;

                    for (sub_idx, sub) in poly.subpaths.iter().enumerate() {
                        for (node_idx, node) in sub.nodes.iter().enumerate() {
                            let w_anchor = tr.transform_point(node.point);
                            let s_anchor = to_screen(w_anchor);

                            let is_node_selected = self.selected_nodes.contains(&SelectedNode {
                                shape_id,
                                subpath_idx: sub_idx,
                                node_idx,
                            });

                            // Draw handles for selected nodes
                            if is_node_selected {
                                // Handle In
                                if let Some(hin) = node.handle_in {
                                    let s_hin = to_screen(tr.transform_point(hin));
                                    painter.line_segment(
                                        [s_anchor, s_hin],
                                        egui::Stroke::new(1.0, handle_line_color),
                                    );
                                    painter.circle_filled(s_hin, 4.0, handle_dot_color);
                                }

                                // Handle Out
                                if let Some(hout) = node.handle_out {
                                    let s_hout = to_screen(tr.transform_point(hout));
                                    painter.line_segment(
                                        [s_anchor, s_hout],
                                        egui::Stroke::new(1.0, handle_line_color),
                                    );
                                    painter.circle_filled(s_hout, 4.0, handle_dot_color);
                                }
                            }

                            // Draw Anchor Point box (7x7 pixels)
                            let anchor_rect =
                                egui::Rect::from_center_size(s_anchor, egui::vec2(7.0, 7.0));
                            let fill = if is_node_selected {
                                selected_anchor_color
                            } else {
                                unselected_anchor_fill
                            };

                            painter.rect_filled(anchor_rect, 1.0, fill);
                            painter.rect_stroke(
                                anchor_rect,
                                1.0,
                                egui::Stroke::new(1.0, border_color),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
        }
    }
}
