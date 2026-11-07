use crate::document::Document;
use crate::shape::{Shape, ShapeId, ShapeKind};
use crate::style::Style;
use core_geom::Affine2;

/// A command that can be applied to and reverted from a `Document`.
pub trait Command: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn apply(&mut self, doc: &mut Document);
    fn undo(&mut self, doc: &mut Document);
}

/// Command to add a newly created shape to a layer.
#[derive(Debug)]
pub struct AddShapeCommand {
    shape: Option<Shape>,
    layer_idx: usize,
    created_id: ShapeId,
}

impl AddShapeCommand {
    pub fn new(shape: Shape, layer_idx: usize) -> Self {
        let created_id = shape.id;
        Self {
            shape: Some(shape),
            layer_idx,
            created_id,
        }
    }
}

impl Command for AddShapeCommand {
    fn name(&self) -> &str {
        "Add Shape"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(shape) = self.shape.take() {
            if let Some(page) = doc.active_page_mut() {
                if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                    layer.shapes.push(shape);
                } else if let Some(layer) = page.active_layer_mut() {
                    layer.shapes.push(shape);
                }
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some((removed, idx)) = doc.remove_shape(self.created_id) {
            self.shape = Some(removed);
            self.layer_idx = idx;
        }
    }
}

/// Command to delete shapes from the document.
#[derive(Debug)]
pub struct RemoveShapesCommand {
    removed_shapes: Vec<(Shape, usize)>,
    target_ids: Vec<ShapeId>,
}

impl RemoveShapesCommand {
    pub fn new(ids: Vec<ShapeId>) -> Self {
        Self {
            removed_shapes: Vec::new(),
            target_ids: ids,
        }
    }
}

impl Command for RemoveShapesCommand {
    fn name(&self) -> &str {
        "Delete Shapes"
    }

    fn apply(&mut self, doc: &mut Document) {
        self.removed_shapes.clear();
        for &id in &self.target_ids {
            if let Some(removed) = doc.remove_shape(id) {
                self.removed_shapes.push(removed);
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = doc.active_page_mut() {
            for (shape, layer_idx) in self.removed_shapes.drain(..) {
                if let Some(layer) = page.layers.get_mut(layer_idx) {
                    layer.shapes.push(shape);
                } else if let Some(layer) = page.active_layer_mut() {
                    layer.shapes.push(shape);
                }
            }
        }
    }
}

/// Command that applies a transform to one or more shapes.
#[derive(Debug)]
pub struct TransformShapesCommand {
    name: String,
    changes: Vec<(ShapeId, Affine2, Affine2)>, // (id, old_transform, new_transform)
}

impl TransformShapesCommand {
    pub fn new(name: impl Into<String>, changes: Vec<(ShapeId, Affine2, Affine2)>) -> Self {
        Self {
            name: name.into(),
            changes,
        }
    }
}

impl Command for TransformShapesCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn apply(&mut self, doc: &mut Document) {
        for &(id, _, new_tr) in &self.changes {
            if let Some(shape) = doc.find_shape_mut(id) {
                shape.transform = new_tr;
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        for &(id, old_tr, _) in &self.changes {
            if let Some(shape) = doc.find_shape_mut(id) {
                shape.transform = old_tr;
            }
        }
    }
}

/// Command to change a shape's style (fill, stroke, line cap/join, etc.)
#[derive(Debug)]
pub struct SetStyleCommand {
    shape_id: ShapeId,
    old_style: Style,
    new_style: Style,
}

impl SetStyleCommand {
    pub fn new(shape_id: ShapeId, old_style: Style, new_style: Style) -> Self {
        Self {
            shape_id,
            old_style,
            new_style,
        }
    }
}

impl Command for SetStyleCommand {
    fn name(&self) -> &str {
        "Change Style"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(shape) = doc.find_shape_mut(self.shape_id) {
            shape.style = self.new_style.clone();
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(shape) = doc.find_shape_mut(self.shape_id) {
            shape.style = self.old_style.clone();
        }
    }
}

/// Command to modify a shape's underlying geometry / kind (e.g. node edits, curve splits).
#[derive(Debug)]
pub struct ModifyShapeKindCommand {
    shape_id: ShapeId,
    name: String,
    old_kind: ShapeKind,
    new_kind: ShapeKind,
}

impl ModifyShapeKindCommand {
    pub fn new(
        shape_id: ShapeId,
        name: impl Into<String>,
        old_kind: ShapeKind,
        new_kind: ShapeKind,
    ) -> Self {
        Self {
            shape_id,
            name: name.into(),
            old_kind,
            new_kind,
        }
    }
}

impl Command for ModifyShapeKindCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some(shape) = doc.find_shape_mut(self.shape_id) {
            shape.kind = self.new_kind.clone();
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(shape) = doc.find_shape_mut(self.shape_id) {
            shape.kind = self.old_kind.clone();
        }
    }
}

/// Compound command executing multiple commands as an atomic unit.
#[derive(Debug)]
pub struct CompoundCommand {
    name: String,
    commands: Vec<Box<dyn Command>>,
}

impl CompoundCommand {
    pub fn new(name: impl Into<String>, commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            name: name.into(),
            commands,
        }
    }
}

impl Command for CompoundCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn apply(&mut self, doc: &mut Document) {
        for cmd in &mut self.commands {
            cmd.apply(doc);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(doc);
        }
    }
}

/// Undo and redo history stack manager.
#[derive(Default)]
pub struct UndoStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
}

impl std::fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("undo_count", &self.undo_stack.len())
            .field("redo_count", &self.redo_stack.len())
            .finish()
    }
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 100,
        }
    }

    pub fn with_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    /// Execute command and record it onto the undo stack.
    pub fn apply(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        cmd.apply(doc);
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Undo the most recent command.
    pub fn undo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(doc);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    /// Redo the previously undone command.
    pub fn redo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.apply(doc);
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.name())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.name())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
