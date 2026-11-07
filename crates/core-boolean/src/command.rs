//! Undoable document commands for boolean, knife, eraser, contour, and text operations.

use core_doc::command::{Command, UndoStack};
use core_doc::document::Document;
use core_doc::shape::{Shape, ShapeId, ShapeKind};
use core_geom::Point2;

use crate::boolean::{execute_boolean, BooleanOp};
use crate::error::{BooleanError, TextError};
use crate::knife::slice_shape;
use crate::offset::{offset_shape, ContourJoin};
use crate::text_to_path::{default_text_to_polycurve, TextOptions};

/// An undoable command representing shape replacements from a Boolean, Knife, or Eraser operation.
#[derive(Debug)]
pub struct BooleanReplaceCommand {
    name: String,
    removed_shapes: Vec<(Shape, usize)>,
    added_shapes: Vec<Shape>,
    layer_idx: usize,
    created_ids: Vec<ShapeId>,
}

impl BooleanReplaceCommand {
    pub fn new(
        name: impl Into<String>,
        removed_shapes: Vec<(Shape, usize)>,
        added_shapes: Vec<Shape>,
        layer_idx: usize,
    ) -> Self {
        let created_ids = added_shapes.iter().map(|s| s.id).collect();
        Self {
            name: name.into(),
            removed_shapes,
            added_shapes,
            layer_idx,
            created_ids,
        }
    }
}

impl Command for BooleanReplaceCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn apply(&mut self, doc: &mut Document) {
        // Remove old shapes
        for (shape, _) in &self.removed_shapes {
            doc.remove_shape(shape.id);
        }

        // Add new shapes
        if let Some(page) = doc.active_page_mut() {
            if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                for shape in &self.added_shapes {
                    layer.shapes.push(shape.clone());
                }
            } else if let Some(layer) = page.active_layer_mut() {
                for shape in &self.added_shapes {
                    layer.shapes.push(shape.clone());
                }
            }
        }

        // Select the newly created shapes
        doc.selection.clear();
        for id in &self.created_ids {
            doc.selection.add(*id);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        // Remove added shapes
        for id in &self.created_ids {
            doc.remove_shape(*id);
        }

        // Restore removed shapes
        if let Some(page) = doc.active_page_mut() {
            for (shape, layer_idx) in &self.removed_shapes {
                if let Some(layer) = page.layers.get_mut(*layer_idx) {
                    layer.shapes.push(shape.clone());
                } else if let Some(layer) = page.active_layer_mut() {
                    layer.shapes.push(shape.clone());
                }
            }
        }

        // Reselect restored shapes
        doc.selection.clear();
        for (shape, _) in &self.removed_shapes {
            doc.selection.add(shape.id);
        }
    }
}

/// Applies a Boolean operation to currently selected shapes in the document.
pub fn apply_boolean_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    op: BooleanOp,
    refit_curves: bool,
) -> Result<String, BooleanError> {
    if doc.selection.is_empty() {
        return Err(BooleanError::InsufficientShapes(format!("{:?}", op)));
    }

    // Collect selected shapes
    let mut selected_shapes = Vec::new();
    let mut removed_shapes = Vec::new();
    let mut primary_style = None;
    let mut target_layer_idx = 0;

    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                if doc.selection.contains(shape.id) {
                    if primary_style.is_none() {
                        primary_style = Some(shape.style.clone());
                        target_layer_idx = layer_idx;
                    }
                    selected_shapes.push(shape.clone());
                    removed_shapes.push((shape.clone(), layer_idx));
                }
            }
        }
    }

    if op != BooleanOp::Simplify && selected_shapes.len() < 2 {
        return Err(BooleanError::InsufficientShapes(format!("{:?}", op)));
    }

    let polycurves: Vec<_> = selected_shapes.iter().map(|s| s.to_polycurve()).collect();

    let result_polycurves = execute_boolean(op, &polycurves, refit_curves)?;
    let style = primary_style.unwrap_or_default();

    let mut added_shapes = Vec::new();
    let op_name = format!("{:?}", op);

    for (idx, poly) in result_polycurves.into_iter().enumerate() {
        let new_id = ShapeId(doc.next_shape_id);
        doc.next_shape_id += 1;
        let name = format!("{} Result {}", op_name, idx + 1);
        let mut new_shape = Shape::path(new_id, poly);
        new_shape.name = name;
        new_shape.style = style.clone();
        added_shapes.push(new_shape);
    }

    let summary = if op == BooleanOp::CreateBoundary {
        format!("Created boundary from {} shapes", selected_shapes.len())
    } else {
        format!("{} on {} shapes", op_name, selected_shapes.len())
    };

    // If CreateBoundary, we keep source shapes and only add the new boundary!
    let cmd_removed = if op == BooleanOp::CreateBoundary {
        Vec::new()
    } else {
        removed_shapes
    };

    let cmd = BooleanReplaceCommand::new(&summary, cmd_removed, added_shapes, target_layer_idx);
    undo.apply(Box::new(cmd), doc);

    Ok(summary)
}

/// Applies Knife slicing across shapes intersecting the cut line in the document.
pub fn apply_knife_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    cut_line: &[Point2],
    auto_close: bool,
) -> Result<String, BooleanError> {
    if cut_line.len() < 2 {
        return Err(BooleanError::InvalidGeometry);
    }

    let mut cut_tasks = Vec::new();

    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                // If there's a selection, only cut selected shapes; otherwise cut all intersecting shapes
                if !doc.selection.is_empty() && !doc.selection.contains(shape.id) {
                    continue;
                }

                let poly = shape.to_polycurve();
                if let Ok(pieces) = slice_shape(&poly, cut_line, auto_close) {
                    cut_tasks.push((shape.clone(), layer_idx, pieces));
                }
            }
        }
    }

    if cut_tasks.is_empty() {
        return Err(BooleanError::NoKnifeIntersection);
    }

    let mut removed_shapes = Vec::new();
    let mut added_shapes = Vec::new();
    let target_layer_idx = cut_tasks[0].1;

    for (shape, layer_idx, pieces) in cut_tasks {
        removed_shapes.push((shape.clone(), layer_idx));
        for (idx, piece) in pieces.into_iter().enumerate() {
            let new_id = ShapeId(doc.next_shape_id);
            doc.next_shape_id += 1;
            let mut new_shape = Shape::path(new_id, piece);
            new_shape.name = format!("{} Cut {}", shape.name, idx + 1);
            new_shape.style = shape.style.clone();
            added_shapes.push(new_shape);
        }
    }

    let summary = format!(
        "Sliced {} shape(s) into {} pieces",
        removed_shapes.len(),
        added_shapes.len()
    );

    let cmd = BooleanReplaceCommand::new(&summary, removed_shapes, added_shapes, target_layer_idx);
    undo.apply(Box::new(cmd), doc);

    Ok(summary)
}

/// Applies contour offset to selected shape(s) in the document.
pub fn apply_contour_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    distance_mm: f64,
    join: ContourJoin,
) -> Result<String, BooleanError> {
    if doc.selection.is_empty() {
        return Err(BooleanError::InvalidGeometry);
    }

    let mut contour_tasks = Vec::new();

    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                if doc.selection.contains(shape.id) {
                    let poly = shape.to_polycurve();
                    let offset_poly = offset_shape(&poly, distance_mm, join)?;
                    contour_tasks.push((shape.clone(), layer_idx, offset_poly));
                }
            }
        }
    }

    if contour_tasks.is_empty() {
        return Err(BooleanError::ZeroOffset);
    }

    let target_layer_idx = contour_tasks[0].1;
    let mut added_shapes = Vec::new();

    for (shape, _layer_idx, offset_poly) in contour_tasks {
        let new_id = ShapeId(doc.next_shape_id);
        doc.next_shape_id += 1;
        let mut new_shape = Shape::path(new_id, offset_poly);
        new_shape.name = format!("{} Contour", shape.name);
        new_shape.style = shape.style.clone();
        added_shapes.push(new_shape);
    }

    let summary = format!("Offset contour ({:+.2} mm)", distance_mm);
    let cmd = BooleanReplaceCommand::new(&summary, Vec::new(), added_shapes, target_layer_idx);
    undo.apply(Box::new(cmd), doc);

    Ok(summary)
}

/// Converts text into a vector shape in the document.
pub fn apply_text_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    text: &str,
    font_size_mm: f64,
    origin: Point2,
) -> Result<String, TextError> {
    let opts = TextOptions {
        font_size_mm,
        letter_spacing_mm: 0.0,
        origin,
    };

    let poly = default_text_to_polycurve(text, &opts)?;
    let new_id = ShapeId(doc.next_shape_id);
    doc.next_shape_id += 1;

    let mut shape = Shape::new(new_id, text, ShapeKind::Path(poly));
    shape.style = core_doc::style::Style::default_outline();

    let layer_idx = doc.active_page().map(|p| p.active_layer).unwrap_or(0);
    let summary = format!("Create Text \"{}\"", text);

    let cmd = BooleanReplaceCommand::new(&summary, Vec::new(), vec![shape], layer_idx);
    undo.apply(Box::new(cmd), doc);

    Ok(summary)
}

/// Applies Eraser stroke across shapes in the document.
pub fn apply_eraser_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    eraser_stroke: &[Point2],
    radius: f64,
) -> Result<String, BooleanError> {
    if eraser_stroke.is_empty() || radius <= 0.0 {
        return Err(BooleanError::InvalidGeometry);
    }

    let mut erase_tasks = Vec::new();

    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                // If there's a selection, only erase selected; otherwise erase all
                if !doc.selection.is_empty() && !doc.selection.contains(shape.id) {
                    continue;
                }

                let poly = shape.to_polycurve();
                if let Ok(erased_poly) = crate::eraser::erase_shape(&poly, eraser_stroke, radius) {
                    if erased_poly != poly {
                        erase_tasks.push((shape.clone(), layer_idx, erased_poly));
                    }
                }
            }
        }
    }

    if erase_tasks.is_empty() {
        return Ok("No shapes erased".to_string());
    }

    let target_layer_idx = erase_tasks[0].1;
    let mut removed_shapes = Vec::new();
    let mut added_shapes = Vec::new();

    for (shape, layer_idx, erased_poly) in erase_tasks {
        removed_shapes.push((shape.clone(), layer_idx));
        if !erased_poly.subpaths.is_empty() {
            let new_id = ShapeId(doc.next_shape_id);
            doc.next_shape_id += 1;
            let mut new_shape = Shape::path(new_id, erased_poly);
            new_shape.name = shape.name.clone();
            new_shape.style = shape.style.clone();
            added_shapes.push(new_shape);
        }
    }

    let summary = format!("Erased on {} shape(s)", removed_shapes.len());
    let cmd = BooleanReplaceCommand::new(&summary, removed_shapes, added_shapes, target_layer_idx);
    undo.apply(Box::new(cmd), doc);

    Ok(summary)
}

/// Applies Virtual Segment Delete at click point in the document.
pub fn apply_virtual_segment_delete_to_document(
    doc: &mut Document,
    undo: &mut UndoStack,
    click_pos: Point2,
    tolerance_mm: f64,
) -> Result<String, BooleanError> {
    let mut all_polys = Vec::new();
    let mut candidate_shapes = Vec::new();

    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                let poly = shape.to_polycurve();
                all_polys.push(poly.clone());
                candidate_shapes.push((shape.clone(), layer_idx, poly));
            }
        }
    }

    for (shape, layer_idx, poly) in candidate_shapes {
        if let Ok(new_poly) = crate::virtual_segment::delete_virtual_segment(
            &poly,
            &all_polys,
            click_pos,
            tolerance_mm,
        ) {
            let new_id = ShapeId(doc.next_shape_id);
            doc.next_shape_id += 1;
            let mut new_shape = Shape::path(new_id, new_poly);
            new_shape.name = shape.name.clone();
            new_shape.style = shape.style.clone();

            let summary = format!("Delete virtual segment on {}", shape.name);
            let cmd = BooleanReplaceCommand::new(
                &summary,
                vec![(shape, layer_idx)],
                vec![new_shape],
                layer_idx,
            );
            undo.apply(Box::new(cmd), doc);
            return Ok(summary);
        }
    }

    Err(BooleanError::VirtualSegmentNotFound)
}
