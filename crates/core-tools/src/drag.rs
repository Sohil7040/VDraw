//! Explicit drag interaction state machine with non-mutating snap preview and undoable commit.

use crate::dispatch::CommandError;
use core_doc::command::TransformRecordCommand;
use core_doc::{Document, Shape, ShapeId, UndoStack};
use core_geom::{build_translate_record, Point2, TransformApply, TransformBuild, TransformRecord};
use core_snap::contracts::{HandleMode, SnapCtx, SnapEngine, SnapFeedback, View};

/// Screen-space pointer movement threshold (in logical pixels) required to leave `Armed` and begin dragging.
pub const DRAG_ARM_THRESHOLD_PX: f32 = 3.0;

/// Explicit state machine governing interactive snap-assisted drag operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DragState {
    /// No drag interaction is active.
    Idle,
    /// Pointer pressed on a shape/handle; waiting for pointer movement exceeding [`DRAG_ARM_THRESHOLD_PX`].
    Armed {
        handle: HandleMode,
        shape: ShapeId,
        anchor: Point2,
    },
    /// Active drag in progress with an optional snap match and non-mutating preview transform.
    Dragging {
        handle: HandleMode,
        shape: ShapeId,
        anchor: Point2,
        snap: Option<SnapFeedback>,
        preview: TransformRecord,
    },
    /// Drag was released over a valid snap target and committed to the document/undo stack.
    Committed,
    /// Drag was cancelled (e.g. via Escape) with zero document mutation.
    Cancelled,
}

/// Manages an interactive drag session, keeping preview state isolated from [`Document`] until commit.
#[derive(Debug, Clone)]
pub struct DragSession {
    /// Current state of the drag state machine.
    pub state: DragState,
    armed_screen: Option<egui::Pos2>,
    preview_shape: Option<Shape>,
}

impl Default for DragSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DragSession {
    /// Creates an idle [`DragSession`].
    pub fn new() -> Self {
        Self {
            state: DragState::Idle,
            armed_screen: None,
            preview_shape: None,
        }
    }

    /// Arms a potential drag on `shape` at world `anchor` and screen position `cursor_screen`.
    pub fn arm(
        &mut self,
        handle: HandleMode,
        shape: ShapeId,
        anchor: Point2,
        cursor_screen: egui::Pos2,
    ) {
        self.state = DragState::Armed {
            handle,
            shape,
            anchor,
        };
        self.armed_screen = Some(cursor_screen);
        self.preview_shape = None;
    }

    /// Advances the drag session with a new pointer position without ever mutating `doc`.
    pub fn update(
        &mut self,
        cursor_screen: egui::Pos2,
        view: &View,
        threshold_px: f32,
        engine: &SnapEngine,
        doc: &Document,
    ) -> &DragState {
        let (handle, shape, anchor) = match self.state {
            DragState::Armed {
                handle,
                shape,
                anchor,
            } => {
                let moved = self
                    .armed_screen
                    .map_or(0.0, |start| start.distance(cursor_screen));
                if moved <= DRAG_ARM_THRESHOLD_PX {
                    return &self.state;
                }
                (handle, shape, anchor)
            }
            DragState::Dragging {
                handle,
                shape,
                anchor,
                ..
            } => (handle, shape, anchor),
            DragState::Idle | DragState::Committed | DragState::Cancelled => {
                return &self.state;
            }
        };

        let ctx = SnapCtx {
            cursor_screen,
            view,
            threshold_px,
        };
        let resolved = engine.resolve(&ctx);

        let (delta, snap_feedback) = match resolved {
            Some((target, fb)) => (SnapEngine::compute_delta(handle, anchor, &target), Some(fb)),
            None => {
                let cursor_world = view.screen_to_world(cursor_screen);
                let raw_delta = cursor_world - anchor;
                let safe_delta = if raw_delta.is_finite() {
                    raw_delta
                } else {
                    Point2::ZERO
                };
                (safe_delta, None)
            }
        };

        let preview = build_preview_record(doc, shape, delta);
        self.preview_shape = doc.find_shape(shape).map(|orig| {
            let mut cloned = orig.clone();
            cloned.transform = preview.matrix * cloned.transform;
            cloned
        });

        self.state = DragState::Dragging {
            handle,
            shape,
            anchor,
            snap: snap_feedback,
            preview,
        };
        &self.state
    }

    /// Completes the drag session on pointer release.
    ///
    /// Commits to `doc` and `undo` only when in `Dragging` over a valid snap target.
    pub fn release(
        &mut self,
        cursor_screen: egui::Pos2,
        view: &View,
        threshold_px: f32,
        engine: &SnapEngine,
        doc: &mut Document,
        undo: &mut UndoStack,
    ) -> Result<DragState, CommandError> {
        match self.state {
            DragState::Armed { .. } => {
                self.preview_shape = None;
                self.armed_screen = None;
                self.state = DragState::Idle;
                Ok(self.state.clone())
            }
            DragState::Dragging {
                handle,
                shape,
                anchor,
                ..
            } => {
                let ctx = SnapCtx {
                    cursor_screen,
                    view,
                    threshold_px,
                };
                let Some((target, _fb)) = engine.resolve(&ctx) else {
                    self.preview_shape = None;
                    self.armed_screen = None;
                    self.state = DragState::Idle;
                    return Ok(self.state.clone());
                };

                if doc.find_shape(shape).is_none() {
                    self.preview_shape = None;
                    self.armed_screen = None;
                    self.state = DragState::Idle;
                    return Err(CommandError::InvalidParameters(format!(
                        "dragged shape {shape} not found in document"
                    )));
                }

                let delta = SnapEngine::compute_delta(handle, anchor, &target);
                let record = match build_commit_record(doc, shape, delta) {
                    Ok(rec) => rec,
                    Err(err) => {
                        self.preview_shape = None;
                        self.armed_screen = None;
                        self.state = DragState::Idle;
                        return Err(err);
                    }
                };

                let cmd = TransformRecordCommand::new(doc, &[shape], record);
                undo.apply(Box::new(cmd), doc);

                self.preview_shape = None;
                self.armed_screen = None;
                self.state = DragState::Committed;
                Ok(self.state.clone())
            }
            DragState::Idle | DragState::Committed | DragState::Cancelled => {
                self.preview_shape = None;
                self.armed_screen = None;
                Ok(self.state.clone())
            }
        }
    }

    /// Cancels an armed or active drag, clearing any preview shape without mutating the document.
    pub fn cancel(&mut self) {
        if matches!(
            self.state,
            DragState::Armed { .. } | DragState::Dragging { .. }
        ) {
            self.state = DragState::Cancelled;
        }
        self.preview_shape = None;
        self.armed_screen = None;
    }

    /// Returns the non-mutating cloned preview [`Shape`] for overlay rendering, if dragging.
    pub fn preview_shape(&self) -> Option<&Shape> {
        self.preview_shape.as_ref()
    }
}

fn format_move_description(delta: Point2) -> String {
    format!("Move Δ=({:.2}, {:.2})", delta.x, delta.y)
}

fn build_preview_record(doc: &Document, shape: ShapeId, delta: Point2) -> TransformRecord {
    let mut probe = doc
        .find_shape(shape)
        .map(Shape::to_polycurve)
        .unwrap_or_default();
    let matrix = probe.translate(delta);
    match build_translate_record(delta) {
        Ok(mut record) => {
            let _ = probe.apply(&matrix);
            record.description = format_move_description(delta);
            record
        }
        Err(_) => {
            let mut record = probe.apply(&core_geom::Affine2::IDENTITY);
            record.description = format_move_description(Point2::ZERO);
            record
        }
    }
}

fn build_commit_record(
    doc: &Document,
    shape: ShapeId,
    delta: Point2,
) -> Result<TransformRecord, CommandError> {
    let mut probe = doc
        .find_shape(shape)
        .map(Shape::to_polycurve)
        .unwrap_or_default();
    let matrix = probe.translate(delta);
    let mut record = build_translate_record(delta)?;
    let _ = probe.apply(&matrix);
    record.description = format_move_description(delta);
    Ok(record)
}
