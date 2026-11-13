use crate::contracts::{HistoryEntry, ShapeTransformEntry};
use crate::document::Document;
use crate::shape::{Shape, ShapeId, ShapeKind};
use crate::style::Style;
use core_geom::{Affine2, Polycurve, TransformRecord};
use std::collections::VecDeque;

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

/// Command to group multiple shapes into a single `ShapeKind::Group`.
#[derive(Debug)]
pub struct GroupCommand {
    target_ids: Vec<ShapeId>,
    group_id: ShapeId,
    layer_idx: usize,
    original_shapes: Vec<Shape>,
}

impl GroupCommand {
    pub fn new(target_ids: Vec<ShapeId>, group_id: ShapeId, layer_idx: usize) -> Self {
        Self {
            target_ids,
            group_id,
            layer_idx,
            original_shapes: Vec::new(),
        }
    }
}

impl Command for GroupCommand {
    fn name(&self) -> &str {
        "Group Shapes"
    }

    fn apply(&mut self, doc: &mut Document) {
        self.original_shapes.clear();
        for &id in &self.target_ids {
            if let Some((removed, _)) = doc.remove_shape(id) {
                self.original_shapes.push(removed);
            }
        }

        if !self.original_shapes.is_empty() {
            let group_shape = Shape::group(self.group_id, self.original_shapes.clone());
            if let Some(page) = doc.active_page_mut() {
                if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                    layer.shapes.push(group_shape);
                } else if let Some(layer) = page.active_layer_mut() {
                    layer.shapes.push(group_shape);
                }
            }
            doc.selection.select(self.group_id);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        doc.remove_shape(self.group_id);
        if let Some(page) = doc.active_page_mut() {
            let layer = if let Some(l) = page.layers.get_mut(self.layer_idx) {
                l
            } else if let Some(l) = page.active_layer_mut() {
                l
            } else {
                return;
            };

            for s in self.original_shapes.drain(..) {
                layer.shapes.push(s);
            }
        }
        doc.selection.clear();
        for &id in &self.target_ids {
            doc.selection.add(id);
        }
    }
}

/// Command to ungroup a `ShapeKind::Group` into its individual child shapes.
#[derive(Debug)]
pub struct UngroupCommand {
    group_id: ShapeId,
    layer_idx: usize,
    restored_group: Option<Shape>,
    unpacked_ids: Vec<ShapeId>,
}

impl UngroupCommand {
    pub fn new(group_id: ShapeId, layer_idx: usize) -> Self {
        Self {
            group_id,
            layer_idx,
            restored_group: None,
            unpacked_ids: Vec::new(),
        }
    }
}

impl Command for UngroupCommand {
    fn name(&self) -> &str {
        "Ungroup Shapes"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some((removed, idx)) = doc.remove_shape(self.group_id) {
            self.layer_idx = idx;
            if let ShapeKind::Group { shapes } = &removed.kind {
                let group_tr = removed.transform;
                self.unpacked_ids.clear();
                if let Some(page) = doc.active_page_mut() {
                    if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                        for mut child in shapes.clone() {
                            child.transform = group_tr * child.transform;
                            self.unpacked_ids.push(child.id);
                            layer.shapes.push(child);
                        }
                    }
                }
                doc.selection.clear();
                for &id in &self.unpacked_ids {
                    doc.selection.add(id);
                }
            }
            self.restored_group = Some(removed);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        for &id in &self.unpacked_ids {
            doc.remove_shape(id);
        }
        if let Some(group) = self.restored_group.take() {
            if let Some(page) = doc.active_page_mut() {
                if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                    layer.shapes.push(group);
                }
            }
            doc.selection.select(self.group_id);
        }
    }
}

/// Command to combine multiple paths into a single compound path.
#[derive(Debug)]
pub struct CombinePathsCommand {
    target_ids: Vec<ShapeId>,
    combined_id: ShapeId,
    layer_idx: usize,
    original_shapes: Vec<Shape>,
}

impl CombinePathsCommand {
    pub fn new(target_ids: Vec<ShapeId>, combined_id: ShapeId, layer_idx: usize) -> Self {
        Self {
            target_ids,
            combined_id,
            layer_idx,
            original_shapes: Vec::new(),
        }
    }
}

impl Command for CombinePathsCommand {
    fn name(&self) -> &str {
        "Combine Paths"
    }

    fn apply(&mut self, doc: &mut Document) {
        self.original_shapes.clear();
        let mut combined_subpaths = Vec::new();
        let mut primary_style = None;

        for &id in &self.target_ids {
            if let Some((removed, _)) = doc.remove_shape(id) {
                if primary_style.is_none() {
                    primary_style = Some(removed.style.clone());
                }
                let poly = removed.to_polycurve();
                let transformed = poly.transform(&removed.transform);
                combined_subpaths.extend(transformed.subpaths);
                self.original_shapes.push(removed);
            }
        }

        let mut compound = Shape::path(
            self.combined_id,
            Polycurve {
                subpaths: combined_subpaths,
            },
        );
        compound.name = "Compound Path".to_string();
        if let Some(st) = primary_style {
            compound.style = st;
        }

        if let Some(page) = doc.active_page_mut() {
            if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                layer.shapes.push(compound);
            }
        }
        doc.selection.select(self.combined_id);
    }

    fn undo(&mut self, doc: &mut Document) {
        doc.remove_shape(self.combined_id);
        if let Some(page) = doc.active_page_mut() {
            if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                for s in self.original_shapes.drain(..) {
                    layer.shapes.push(s);
                }
            }
        }
        doc.selection.clear();
        for &id in &self.target_ids {
            doc.selection.add(id);
        }
    }
}

/// Command to break apart a compound path into independent subpath shapes.
#[derive(Debug)]
pub struct BreakApartCommand {
    compound_id: ShapeId,
    layer_idx: usize,
    new_ids: Vec<ShapeId>,
    original_shape: Option<Shape>,
}

impl BreakApartCommand {
    pub fn new(compound_id: ShapeId, layer_idx: usize, new_ids: Vec<ShapeId>) -> Self {
        Self {
            compound_id,
            layer_idx,
            new_ids,
            original_shape: None,
        }
    }
}

impl Command for BreakApartCommand {
    fn name(&self) -> &str {
        "Break Apart Path"
    }

    fn apply(&mut self, doc: &mut Document) {
        if let Some((removed, idx)) = doc.remove_shape(self.compound_id) {
            self.layer_idx = idx;
            let poly = removed.to_polycurve();
            let style = removed.style.clone();

            let needed = poly.subpaths.len();
            let mut ids = self.new_ids.clone();
            while ids.len() < needed {
                ids.push(doc.allocate_id());
            }
            self.new_ids = ids.clone();

            if let Some(page) = doc.active_page_mut() {
                if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                    for (i, sub) in poly.subpaths.into_iter().enumerate() {
                        let new_id = ids[i];
                        let single_poly = Polycurve {
                            subpaths: vec![sub],
                        };
                        let mut shape = Shape::path(new_id, single_poly);
                        shape.name = format!("{} Part {}", removed.name, i + 1);
                        shape.style = style.clone();
                        shape.transform = removed.transform;
                        layer.shapes.push(shape);
                    }
                }
            }

            doc.selection.clear();
            for &id in &self.new_ids {
                doc.selection.add(id);
            }
            self.original_shape = Some(removed);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        for &id in &self.new_ids {
            doc.remove_shape(id);
        }
        if let Some(orig) = self.original_shape.take() {
            if let Some(page) = doc.active_page_mut() {
                if let Some(layer) = page.layers.get_mut(self.layer_idx) {
                    layer.shapes.push(orig);
                }
            }
            doc.selection.select(self.compound_id);
        }
    }
}

/// Reversible command applying a [`TransformRecord`] to one or more shapes,
/// preserving contour winding across reflections and restoring exact `(ShapeKind, Affine2)` on undo.
#[derive(Debug, Clone)]
pub struct TransformRecordCommand {
    entries: Vec<ShapeTransformEntry>,
    original_kinds: Vec<Option<ShapeKind>>,
    record: TransformRecord,
}

fn flip_shape_winding(shape: &mut Shape) {
    match &mut shape.kind {
        ShapeKind::Path(poly) => {
            for sub in &mut poly.subpaths {
                sub.apply_in_place(&Affine2::IDENTITY, true);
            }
        }
        ShapeKind::Group { shapes } => {
            for child in shapes {
                flip_shape_winding(child);
            }
        }
        _ => {
            let mut poly = shape.to_polycurve();
            for sub in &mut poly.subpaths {
                sub.apply_in_place(&Affine2::IDENTITY, true);
            }
            shape.kind = ShapeKind::Path(poly);
        }
    }
}

impl TransformRecordCommand {
    /// Captures pre-transform state for `ids` in `doc` and prepares a reversible [`TransformRecord`] command.
    pub fn new(doc: &Document, ids: &[ShapeId], record: TransformRecord) -> Self {
        let mut entries = Vec::with_capacity(ids.len());
        let mut original_kinds = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Some(shape) = doc.find_shape(id) {
                let before = shape.transform;
                let after = record.matrix * before;
                entries.push(ShapeTransformEntry {
                    id,
                    before,
                    after,
                    flips_winding: record.flips_winding,
                });
                original_kinds.push(if record.flips_winding {
                    Some(shape.kind.clone())
                } else {
                    None
                });
            }
        }
        Self {
            entries,
            original_kinds,
            record,
        }
    }

    /// Constructs a [`TransformRecordCommand`] from pre-built [`ShapeTransformEntry`] items.
    pub fn from_entries(entries: Vec<ShapeTransformEntry>, record: TransformRecord) -> Self {
        let original_kinds = vec![None; entries.len()];
        Self {
            entries,
            original_kinds,
            record,
        }
    }

    /// Returns the underlying [`TransformRecord`].
    pub fn record(&self) -> &TransformRecord {
        &self.record
    }

    /// Returns the per-shape transform entries recorded by this command.
    pub fn entries(&self) -> &[ShapeTransformEntry] {
        &self.entries
    }
}

impl Command for TransformRecordCommand {
    fn name(&self) -> &str {
        &self.record.description
    }

    fn apply(&mut self, doc: &mut Document) {
        for (entry, orig_kind) in self.entries.iter().zip(self.original_kinds.iter_mut()) {
            if let Some(shape) = doc.find_shape_mut(entry.id) {
                if entry.flips_winding {
                    if orig_kind.is_none() {
                        *orig_kind = Some(shape.kind.clone());
                    }
                    flip_shape_winding(shape);
                }
                shape.transform = entry.after;
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        for (entry, orig_kind) in self.entries.iter().zip(self.original_kinds.iter()) {
            if let Some(shape) = doc.find_shape_mut(entry.id) {
                if entry.flips_winding {
                    if let Some(kind) = orig_kind {
                        shape.kind = kind.clone();
                    } else {
                        flip_shape_winding(shape);
                    }
                }
                shape.transform = entry.before;
            }
        }
    }
}

/// An entry in the undo stack tracking document generation for clean/dirty state.
struct UndoEntry {
    cmd: Box<dyn Command>,
    pre_gen: u64,
    post_gen: u64,
}

impl std::fmt::Debug for UndoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoEntry")
            .field("cmd", &self.cmd.name())
            .field("pre_gen", &self.pre_gen)
            .field("post_gen", &self.post_gen)
            .finish()
    }
}

/// Transaction-based undo and redo history stack manager with O(1) ring pruning
/// and dirty-state tracking.
pub struct UndoStack {
    undo_stack: VecDeque<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    history_cache: Vec<HistoryEntry>,
    max_depth: usize,
    current_generation: u64,
    saved_generation: u64,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("undo_count", &self.undo_stack.len())
            .field("redo_count", &self.redo_stack.len())
            .field("dirty", &self.is_dirty())
            .finish()
    }
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            history_cache: Vec::new(),
            max_depth: 100,
            current_generation: 0,
            saved_generation: 0,
        }
    }

    pub fn with_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            history_cache: Vec::new(),
            max_depth,
            current_generation: 0,
            saved_generation: 0,
        }
    }

    fn rebuild_history_cache(&mut self) {
        self.history_cache.clear();
        for entry in &self.undo_stack {
            let index = self.history_cache.len();
            self.history_cache.push(HistoryEntry {
                index,
                label: entry.cmd.name().to_string(),
                is_applied: true,
            });
        }
        for entry in self.redo_stack.iter().rev() {
            let index = self.history_cache.len();
            self.history_cache.push(HistoryEntry {
                index,
                label: entry.cmd.name().to_string(),
                is_applied: false,
            });
        }
    }

    /// Returns `true` if the document has unsaved modifications.
    pub fn is_dirty(&self) -> bool {
        self.current_generation != self.saved_generation
    }

    /// Marks the current document state as clean/saved.
    pub fn mark_saved(&mut self) {
        self.saved_generation = self.current_generation;
    }

    /// Execute command and record it onto the undo stack with O(1) amortized pruning.
    pub fn apply(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        let pre_gen = self.current_generation;
        self.current_generation += 1;
        let post_gen = self.current_generation;

        cmd.apply(doc);

        self.undo_stack.push_back(UndoEntry {
            cmd,
            pre_gen,
            post_gen,
        });

        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
        self.rebuild_history_cache();
    }

    /// Undo the most recent command, restoring document dirty generation.
    pub fn undo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut entry) = self.undo_stack.pop_back() {
            entry.cmd.undo(doc);
            self.current_generation = entry.pre_gen;
            self.redo_stack.push(entry);
            self.rebuild_history_cache();
            true
        } else {
            false
        }
    }

    /// Redo the previously undone command, restoring document dirty generation.
    pub fn redo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut entry) = self.redo_stack.pop() {
            entry.cmd.apply(doc);
            self.current_generation = entry.post_gen;
            self.undo_stack.push_back(entry);
            self.rebuild_history_cache();
            true
        } else {
            false
        }
    }

    /// Returns the combined undo/redo timeline entries in chronological order.
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history_cache
    }

    /// Returns the description label of the most recently applied command, if any.
    pub fn top_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|e| e.cmd.name())
    }

    /// Jumps along the linear undo/redo timeline so that `target_applied_count` entries remain applied.
    /// Applying any new command after undoing drops the redo tail (non-linear branches are pruned).
    pub fn jump_to(&mut self, target_applied_count: usize, doc: &mut Document) {
        let total = self.undo_stack.len() + self.redo_stack.len();
        let target = target_applied_count.min(total);
        while self.undo_stack.len() > target {
            if !self.undo(doc) {
                break;
            }
        }
        while self.undo_stack.len() < target {
            if !self.redo(doc) {
                break;
            }
        }
    }

    /// Jumps along the linear undo/redo timeline to the specified `history_index` applied depth.
    /// Applying any new command after undoing drops the redo tail (non-linear branches are pruned).
    pub fn undo_to(&mut self, history_index: usize, doc: &mut Document) {
        self.jump_to(history_index, doc);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.undo_stack.back().map(|c| c.cmd.name())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.cmd.name())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.history_cache.clear();
        self.current_generation = 0;
        self.saved_generation = 0;
    }
}
