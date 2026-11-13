//! Exhaustive application command dispatcher routing [`AppCommand`] across document,
//! undo stack, snap controller, tool manager, and deferred UI effect requests.

use crate::contracts::{AppCommand, ThemePref};
use crate::snap_controller::SnapController;
use crate::ToolManager;
use core_boolean::{
    boolean_with_options, default_text_to_polycurve, execute_boolean, offset_shape, BoolError,
    BoolOp, BoolOptions, BooleanOp, BooleanReplaceCommand, TextOptions,
};
use core_doc::command::{
    AddShapeCommand, BreakApartCommand, CombinePathsCommand, Command, CompoundCommand,
    GroupCommand, RemoveShapesCommand, TransformRecordCommand, UngroupCommand,
};
use core_doc::{Document, Shape, ShapeId, ShapeKind, UndoStack};
use core_geom::{
    build_mirror_record, build_rotate_record, build_scale_record, build_translate_record, Point2,
    TransformApply, TransformBuild, TransformError, TransformRecord,
};
use core_snap::contracts::SnapMask;

/// Errors returned by [`Dispatcher::dispatch`] and [`crate::DragSession::release`].
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// Command is not supported in the current context.
    Unsupported,
    /// Command requires one or more selected shapes, but the selection is empty.
    NoSelection,
    /// 2D geometric transform validation failed.
    TransformFailed(TransformError),
    /// 2D boolean kernel evaluation failed.
    BooleanFailed(BoolError),
    /// Command arguments or target state were invalid.
    InvalidParameters(String),
    /// Operation was cooperatively cancelled.
    Cancelled,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Command is not supported"),
            Self::NoSelection => write!(f, "No shapes selected"),
            Self::TransformFailed(err) => write!(f, "Transform failed: {err}"),
            Self::BooleanFailed(err) => write!(f, "Boolean operation failed: {err}"),
            Self::InvalidParameters(msg) => write!(f, "Invalid command parameters: {msg}"),
            Self::Cancelled => write!(f, "Command cancelled"),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TransformFailed(err) => Some(err),
            Self::BooleanFailed(err) => Some(err),
            Self::Unsupported
            | Self::NoSelection
            | Self::InvalidParameters(_)
            | Self::Cancelled => None,
        }
    }
}

impl From<TransformError> for CommandError {
    fn from(err: TransformError) -> Self {
        Self::TransformFailed(err)
    }
}

impl From<BoolError> for CommandError {
    fn from(err: BoolError) -> Self {
        if matches!(err, BoolError::Cancelled) {
            Self::Cancelled
        } else {
            Self::BooleanFailed(err)
        }
    }
}

/// Non-UI effect recorded for the application shell when a file, viewport, or theme command runs.
#[derive(Debug, Clone, PartialEq)]
pub enum UiActionRequest {
    NewDocument,
    OpenDocument,
    SaveDocument,
    SaveDocumentAs,
    ImportSvg,
    ExportSvg,
    ExportDxf,
    ExportPng,
    ExportGcode,
    ZoomIn,
    ZoomOut,
    FitPage,
    ZoomToSelection,
    ToggleGrid,
    ToggleRulers,
    SetTheme(ThemePref),
    SwitchWorkflowDesign,
    SwitchWorkflowLaserCam,
    SwitchWorkflowPreview,
    OpenCommandPalette,
    OpenHelp,
    StartCoachMarks,
}

/// Convenience alias for [`UiActionRequest`].
pub type UiEffect = UiActionRequest;

/// Mutable tool-layer context required by [`Dispatcher::dispatch`].
pub struct DispatchCtx<'a> {
    /// Active snap mask & provider controller.
    pub snap_controller: &'a mut SnapController,
    /// Active interactive tool manager.
    pub tool_manager: &'a mut ToolManager,
    /// Most recent UI-shell effect request emitted during dispatch.
    pub pending_action: Option<UiActionRequest>,
}

impl<'a> DispatchCtx<'a> {
    /// Creates a new [`DispatchCtx`] wrapping `snap_controller` and `tool_manager`.
    pub fn new(snap_controller: &'a mut SnapController, tool_manager: &'a mut ToolManager) -> Self {
        Self {
            snap_controller,
            tool_manager,
            pending_action: None,
        }
    }

    /// Consumes and returns the pending [`UiActionRequest`], if any.
    pub fn take_pending_action(&mut self) -> Option<UiActionRequest> {
        self.pending_action.take()
    }
}

/// Stateless command router dispatching every [`AppCommand`] variant exhaustively.
#[derive(Debug, Default)]
pub struct Dispatcher;

impl Dispatcher {
    /// Creates a new [`Dispatcher`].
    pub fn new() -> Self {
        Self
    }

    /// Dispatches `cmd` against `doc`, `undo`, and `ctx` with zero panics and zero wildcard arms.
    pub fn dispatch(
        &mut self,
        cmd: AppCommand,
        doc: &mut Document,
        undo: &mut UndoStack,
        ctx: &mut DispatchCtx<'_>,
    ) -> Result<(), CommandError> {
        match cmd {
            // ── File & Export ────────────────────────────────────────────────
            AppCommand::NewDocument => {
                ctx.pending_action = Some(UiActionRequest::NewDocument);
                Ok(())
            }
            AppCommand::OpenDocument => {
                ctx.pending_action = Some(UiActionRequest::OpenDocument);
                Ok(())
            }
            AppCommand::SaveDocument => {
                ctx.pending_action = Some(UiActionRequest::SaveDocument);
                Ok(())
            }
            AppCommand::SaveDocumentAs => {
                ctx.pending_action = Some(UiActionRequest::SaveDocumentAs);
                Ok(())
            }
            AppCommand::ImportSvg => {
                ctx.pending_action = Some(UiActionRequest::ImportSvg);
                Ok(())
            }
            AppCommand::ExportSvg => {
                ctx.pending_action = Some(UiActionRequest::ExportSvg);
                Ok(())
            }
            AppCommand::ExportDxf => {
                ctx.pending_action = Some(UiActionRequest::ExportDxf);
                Ok(())
            }
            AppCommand::ExportPng => {
                ctx.pending_action = Some(UiActionRequest::ExportPng);
                Ok(())
            }
            AppCommand::ExportGcode => {
                ctx.pending_action = Some(UiActionRequest::ExportGcode);
                Ok(())
            }

            // ── Edit & Undo History ──────────────────────────────────────────
            AppCommand::Undo => {
                let _ = undo.undo(doc);
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::Redo => {
                let _ = undo.redo(doc);
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::JumpToHistoryIndex(idx) => {
                if idx > undo.history().len() {
                    return Err(CommandError::InvalidParameters(format!(
                        "history index {idx} out of bounds (len {})",
                        undo.history().len()
                    )));
                }
                undo.jump_to(idx, doc);
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::DuplicateSelection => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                let mut cmds: Vec<Box<dyn Command>> = Vec::with_capacity(selected.len());
                let mut new_ids = Vec::with_capacity(selected.len());
                for (mut shape, layer_idx) in selected {
                    let new_id = doc.allocate_id();
                    shape.id = new_id;
                    shape.name = format!("{} Copy", shape.name);
                    new_ids.push(new_id);
                    cmds.push(Box::new(AddShapeCommand::new(shape, layer_idx)));
                }
                undo.apply(
                    Box::new(CompoundCommand::new("Duplicate Selection", cmds)),
                    doc,
                );
                doc.selection.clear();
                for id in new_ids {
                    doc.selection.add(id);
                }
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::DeleteSelection => {
                let ids = collect_selected_ids(doc);
                if ids.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                undo.apply(Box::new(RemoveShapesCommand::new(ids)), doc);
                doc.selection.clear();
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::SelectAll => {
                let mut ids = Vec::new();
                if let Some(page) = doc.active_page() {
                    for layer in &page.layers {
                        if !layer.visible || layer.locked {
                            continue;
                        }
                        for shape in &layer.shapes {
                            if shape.visible && !shape.locked {
                                ids.push(shape.id);
                            }
                        }
                    }
                }
                doc.selection.clear();
                for id in ids {
                    doc.selection.add(id);
                }
                Ok(())
            }
            AppCommand::DeselectAll => {
                doc.selection.clear();
                Ok(())
            }
            AppCommand::GroupSelection => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                if selected.len() < 2 {
                    return Err(CommandError::InvalidParameters(
                        "grouping requires at least 2 selected shapes".to_string(),
                    ));
                }
                let target_layer_idx = selected[0].1;
                let ids: Vec<ShapeId> = selected.into_iter().map(|(s, _)| s.id).collect();
                let group_id = doc.allocate_id();
                undo.apply(
                    Box::new(GroupCommand::new(ids, group_id, target_layer_idx)),
                    doc,
                );
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::UngroupSelection => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                let groups: Vec<(ShapeId, usize)> = selected
                    .into_iter()
                    .filter(|(s, _)| matches!(s.kind, ShapeKind::Group { .. }))
                    .map(|(s, idx)| (s.id, idx))
                    .collect();
                if groups.is_empty() {
                    return Err(CommandError::InvalidParameters(
                        "no group selected to ungroup".to_string(),
                    ));
                }
                let cmds: Vec<Box<dyn Command>> = groups
                    .into_iter()
                    .map(|(id, idx)| Box::new(UngroupCommand::new(id, idx)) as Box<dyn Command>)
                    .collect();
                if cmds.len() == 1 {
                    if let Some(single) = cmds.into_iter().next() {
                        undo.apply(single, doc);
                    }
                } else {
                    undo.apply(Box::new(CompoundCommand::new("Ungroup Shapes", cmds)), doc);
                }
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::CombinePaths => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                if selected.len() < 2 {
                    return Err(CommandError::InvalidParameters(
                        "combining paths requires at least 2 selected shapes".to_string(),
                    ));
                }
                let target_layer_idx = selected[0].1;
                let ids: Vec<ShapeId> = selected.into_iter().map(|(s, _)| s.id).collect();
                let combined_id = doc.allocate_id();
                undo.apply(
                    Box::new(CombinePathsCommand::new(ids, combined_id, target_layer_idx)),
                    doc,
                );
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::BreakApartPaths => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                let breakable: Vec<(ShapeId, usize)> = selected
                    .into_iter()
                    .filter(|(s, _)| s.to_polycurve().subpaths.len() > 1)
                    .map(|(s, idx)| (s.id, idx))
                    .collect();
                if breakable.is_empty() {
                    return Err(CommandError::InvalidParameters(
                        "no compound path with multiple subpaths selected".to_string(),
                    ));
                }
                let cmds: Vec<Box<dyn Command>> = breakable
                    .into_iter()
                    .map(|(id, idx)| {
                        Box::new(BreakApartCommand::new(id, idx, Vec::new())) as Box<dyn Command>
                    })
                    .collect();
                if cmds.len() == 1 {
                    if let Some(single) = cmds.into_iter().next() {
                        undo.apply(single, doc);
                    }
                } else {
                    undo.apply(
                        Box::new(CompoundCommand::new("Break Apart Paths", cmds)),
                        doc,
                    );
                }
                ctx.snap_controller.update_index(doc);
                Ok(())
            }
            AppCommand::ConvertTextToPath { text, size_mm } => {
                if !size_mm.is_finite() || size_mm <= 0.0 {
                    return Err(CommandError::InvalidParameters(
                        "text size_mm must be finite and positive".to_string(),
                    ));
                }
                let opts = TextOptions {
                    font_size_mm: size_mm,
                    letter_spacing_mm: 0.0,
                    origin: Point2::ZERO,
                };
                let poly = default_text_to_polycurve(&text, &opts)
                    .map_err(|e| CommandError::InvalidParameters(e.to_string()))?;
                if poly.is_empty() {
                    return Err(CommandError::InvalidParameters(
                        "text produced empty outline".to_string(),
                    ));
                }
                let new_id = doc.allocate_id();
                let mut shape = Shape::path(new_id, poly);
                shape.name = text;
                let layer_idx = doc.active_page().map(|p| p.active_layer).unwrap_or(0);
                undo.apply(Box::new(AddShapeCommand::new(shape, layer_idx)), doc);
                doc.selection.select(new_id);
                ctx.snap_controller.update_index(doc);
                Ok(())
            }

            // ── Tools ────────────────────────────────────────────────────────
            AppCommand::ActivateTool(tool_id) => {
                ctx.tool_manager.set_tool(tool_id);
                Ok(())
            }

            // ── Transforms (2D Planar) ───────────────────────────────────────
            AppCommand::TranslateSelection { delta } => {
                apply_transform_command(doc, undo, ctx, TransformRequest::Translate { delta })
            }
            AppCommand::RotateSelection { pivot, angle_deg } => apply_transform_command(
                doc,
                undo,
                ctx,
                TransformRequest::Rotate { pivot, angle_deg },
            ),
            AppCommand::MirrorSelectionHorizontal => {
                apply_transform_command(doc, undo, ctx, TransformRequest::MirrorHorizontal)
            }
            AppCommand::MirrorSelectionVertical => {
                apply_transform_command(doc, undo, ctx, TransformRequest::MirrorVertical)
            }
            AppCommand::MirrorSelectionLine { point, dir } => {
                apply_transform_command(doc, undo, ctx, TransformRequest::MirrorLine { point, dir })
            }
            AppCommand::ScaleSelection { pivot, sx, sy } => {
                apply_transform_command(doc, undo, ctx, TransformRequest::Scale { pivot, sx, sy })
            }

            // ── Boolean & Path Operations ────────────────────────────────────
            AppCommand::BooleanOp(op) => apply_boolean_command(doc, undo, ctx, op),
            AppCommand::OffsetSelection { distance_mm, join } => {
                let selected = collect_selected_shapes_with_layer(doc);
                if selected.is_empty() {
                    return Err(CommandError::NoSelection);
                }
                let mut offsets = Vec::with_capacity(selected.len());
                for (shape, _layer_idx) in &selected {
                    let world_poly = shape.to_polycurve().transform(&shape.transform);
                    let offset_poly = offset_shape(&world_poly, distance_mm, join)
                        .map_err(|e| CommandError::BooleanFailed(BoolError::from(e)))?;
                    offsets.push((shape.name.clone(), shape.style.clone(), offset_poly));
                }
                let target_layer_idx = selected[0].1;
                let mut added_shapes = Vec::with_capacity(offsets.len());
                for (name, style, poly) in offsets {
                    let new_id = doc.allocate_id();
                    let mut shape = Shape::path(new_id, poly);
                    shape.name = format!("{name} Contour");
                    shape.style = style;
                    added_shapes.push(shape);
                }
                let summary = format!("Offset ({:+.2} mm)", distance_mm);
                let cmd =
                    BooleanReplaceCommand::new(summary, Vec::new(), added_shapes, target_layer_idx);
                undo.apply(Box::new(cmd), doc);
                ctx.snap_controller.update_index(doc);
                Ok(())
            }

            // ── Snap System & Constraints ────────────────────────────────────
            AppCommand::ToggleSnapMaster => {
                let _ = ctx.snap_controller.toggle_master();
                Ok(())
            }
            AppCommand::ToggleSnapKind(kind) => {
                ctx.snap_controller.toggle_kind(kind);
                Ok(())
            }
            AppCommand::SetHandleMode(mode) => {
                ctx.snap_controller.set_handle_mode(mode);
                Ok(())
            }
            AppCommand::ToggleSnapOrtho => {
                ctx.snap_controller.toggle_bit(SnapMask::ORTHO);
                Ok(())
            }
            AppCommand::ToggleSnapPolar => {
                ctx.snap_controller.toggle_bit(SnapMask::POLAR);
                Ok(())
            }
            AppCommand::SetPolarStepDeg(deg) => {
                if !deg.is_finite() || deg <= 0.0 || deg > 360.0 {
                    return Err(CommandError::InvalidParameters(
                        "polar step angle must be finite and in (0, 360]".to_string(),
                    ));
                }
                ctx.snap_controller.set_polar_step_deg(deg);
                Ok(())
            }
            AppCommand::ToggleAlignmentGuides => {
                let _ = ctx.snap_controller.toggle_alignment_guides();
                Ok(())
            }
            AppCommand::ToggleGuidelineSnap => {
                let _ = ctx.snap_controller.toggle_guideline_snap();
                Ok(())
            }
            AppCommand::ClearGuidelines => {
                doc.clear_guidelines();
                Ok(())
            }

            // ── Viewport, Theme & Help ───────────────────────────────────────
            AppCommand::ZoomIn => {
                ctx.pending_action = Some(UiActionRequest::ZoomIn);
                Ok(())
            }
            AppCommand::ZoomOut => {
                ctx.pending_action = Some(UiActionRequest::ZoomOut);
                Ok(())
            }
            AppCommand::FitPage => {
                ctx.pending_action = Some(UiActionRequest::FitPage);
                Ok(())
            }
            AppCommand::ZoomToSelection => {
                ctx.pending_action = Some(UiActionRequest::ZoomToSelection);
                Ok(())
            }
            AppCommand::ToggleGrid => {
                ctx.pending_action = Some(UiActionRequest::ToggleGrid);
                Ok(())
            }
            AppCommand::ToggleRulers => {
                ctx.pending_action = Some(UiActionRequest::ToggleRulers);
                Ok(())
            }
            AppCommand::SetTheme(pref) => {
                ctx.pending_action = Some(UiActionRequest::SetTheme(pref));
                Ok(())
            }
            AppCommand::SwitchWorkflowDesign => {
                ctx.pending_action = Some(UiActionRequest::SwitchWorkflowDesign);
                Ok(())
            }
            AppCommand::SwitchWorkflowLaserCam => {
                ctx.pending_action = Some(UiActionRequest::SwitchWorkflowLaserCam);
                Ok(())
            }
            AppCommand::SwitchWorkflowPreview => {
                ctx.pending_action = Some(UiActionRequest::SwitchWorkflowPreview);
                Ok(())
            }
            AppCommand::OpenCommandPalette => {
                ctx.pending_action = Some(UiActionRequest::OpenCommandPalette);
                Ok(())
            }
            AppCommand::OpenHelp => {
                ctx.pending_action = Some(UiActionRequest::OpenHelp);
                Ok(())
            }
            AppCommand::StartCoachMarks => {
                ctx.pending_action = Some(UiActionRequest::StartCoachMarks);
                Ok(())
            }
        }
    }
}

enum TransformRequest {
    Translate {
        delta: Point2,
    },
    Rotate {
        pivot: Option<Point2>,
        angle_deg: f64,
    },
    MirrorHorizontal,
    MirrorVertical,
    MirrorLine {
        point: Point2,
        dir: Point2,
    },
    Scale {
        pivot: Option<Point2>,
        sx: f64,
        sy: f64,
    },
}

fn collect_selected_ids(doc: &Document) -> Vec<ShapeId> {
    let mut ids = Vec::new();
    if let Some(page) = doc.active_page() {
        for layer in &page.layers {
            for shape in &layer.shapes {
                if doc.selection.contains(shape.id) {
                    ids.push(shape.id);
                }
            }
        }
    }
    ids
}

fn collect_selected_shapes_with_layer(doc: &Document) -> Vec<(Shape, usize)> {
    let mut out = Vec::new();
    if let Some(page) = doc.active_page() {
        for (layer_idx, layer) in page.layers.iter().enumerate() {
            for shape in &layer.shapes {
                if doc.selection.contains(shape.id) {
                    out.push((shape.clone(), layer_idx));
                }
            }
        }
    }
    out
}

fn default_selection_pivot(doc: &Document) -> Point2 {
    doc.selection_bounds()
        .map(|b| doc.selection.pivot(b))
        .unwrap_or(Point2::ZERO)
}

fn build_transform_record(
    doc: &Document,
    probe_id: ShapeId,
    req: TransformRequest,
) -> Result<TransformRecord, CommandError> {
    let mut probe = doc
        .find_shape(probe_id)
        .map(Shape::to_polycurve)
        .unwrap_or_default();

    let mut record = match req {
        TransformRequest::Translate { delta } => {
            let m = probe.translate(delta);
            let mut rec = build_translate_record(delta)?;
            let _ = probe.apply(&m);
            rec.description = format!("Move Δ=({:.2}, {:.2})", delta.x, delta.y);
            rec
        }
        TransformRequest::Rotate { pivot, angle_deg } => {
            if !angle_deg.is_finite() {
                return Err(CommandError::TransformFailed(
                    TransformError::NonFiniteInput,
                ));
            }
            let p = pivot.unwrap_or_else(|| default_selection_pivot(doc));
            let angle_rad = angle_deg.to_radians();
            let m = probe.rotate(p, angle_rad)?;
            let mut rec = build_rotate_record(p, angle_rad)?;
            let _ = probe.apply(&m);
            rec.description = format!("Rotate {angle_deg:.1}°");
            rec
        }
        TransformRequest::MirrorHorizontal => {
            let p = default_selection_pivot(doc);
            let dir = Point2::new(0.0, 1.0);
            let (m, _) = probe.mirror(p, dir)?;
            let mut rec = build_mirror_record(p, dir)?;
            let _ = probe.apply(&m);
            rec.description = "Mirror Horizontal".to_string();
            rec
        }
        TransformRequest::MirrorVertical => {
            let p = default_selection_pivot(doc);
            let dir = Point2::new(1.0, 0.0);
            let (m, _) = probe.mirror(p, dir)?;
            let mut rec = build_mirror_record(p, dir)?;
            let _ = probe.apply(&m);
            rec.description = "Mirror Vertical".to_string();
            rec
        }
        TransformRequest::MirrorLine { point, dir } => {
            let (m, _) = probe.mirror(point, dir)?;
            let mut rec = build_mirror_record(point, dir)?;
            let _ = probe.apply(&m);
            rec.description = "Mirror across line".to_string();
            rec
        }
        TransformRequest::Scale { pivot, sx, sy } => {
            let p = pivot.unwrap_or_else(|| default_selection_pivot(doc));
            let m = probe.scale(p, (sx, sy))?;
            let mut rec = build_scale_record(p, (sx, sy))?;
            let _ = probe.apply(&m);
            rec.description = format!("Scale ({sx:.2}, {sy:.2})");
            rec
        }
    };

    if record.description.trim().is_empty() || record.description == "Transform" {
        record.description = "Transform Selection".to_string();
    }

    Ok(record)
}

fn apply_transform_command(
    doc: &mut Document,
    undo: &mut UndoStack,
    ctx: &mut DispatchCtx<'_>,
    req: TransformRequest,
) -> Result<(), CommandError> {
    let selected_ids = collect_selected_ids(doc);
    if selected_ids.is_empty() {
        return Err(CommandError::NoSelection);
    }
    let record = build_transform_record(doc, selected_ids[0], req)?;
    let cmd = TransformRecordCommand::new(doc, &selected_ids, record);
    undo.apply(Box::new(cmd), doc);
    ctx.snap_controller.update_index(doc);
    Ok(())
}

fn apply_boolean_command(
    doc: &mut Document,
    undo: &mut UndoStack,
    ctx: &mut DispatchCtx<'_>,
    op: BooleanOp,
) -> Result<(), CommandError> {
    let selected_with_layer = collect_selected_shapes_with_layer(doc);
    if selected_with_layer.is_empty() {
        return Err(CommandError::NoSelection);
    }

    let target_layer_idx = selected_with_layer[0].1;
    let primary_style = selected_with_layer[0].0.style.clone();
    let selected_shapes: Vec<Shape> = selected_with_layer.iter().map(|(s, _)| s.clone()).collect();

    match BoolOp::try_from(op) {
        Ok(bool_op) => {
            let mut result_shape = boolean_with_options(
                &selected_shapes,
                bool_op,
                &BoolOptions::default(),
                None,
                None,
            )?;
            let new_id = doc.allocate_id();
            result_shape.id = new_id;
            result_shape.name = format!("{op:?} Result");
            let summary = format!("{op:?} on {} shapes", selected_shapes.len());
            let cmd = BooleanReplaceCommand::new(
                summary,
                selected_with_layer,
                vec![result_shape],
                target_layer_idx,
            );
            undo.apply(Box::new(cmd), doc);
            ctx.snap_controller.update_index(doc);
            Ok(())
        }
        Err(ext_op) => {
            let world_polys: Vec<_> = selected_shapes
                .iter()
                .map(|s| s.to_polycurve().transform(&s.transform))
                .collect();
            let results = execute_boolean(ext_op, &world_polys, true)
                .map_err(|e| CommandError::BooleanFailed(BoolError::from(e)))?;
            let non_empty: Vec<_> = results
                .into_iter()
                .filter(|p| !p.is_empty() && !p.subpaths.is_empty())
                .collect();
            if non_empty.is_empty() {
                return Err(CommandError::BooleanFailed(BoolError::EmptyResult));
            }
            let mut added_shapes = Vec::with_capacity(non_empty.len());
            for (idx, poly) in non_empty.into_iter().enumerate() {
                let new_id = doc.allocate_id();
                let mut shape = Shape::path(new_id, poly);
                shape.name = format!("{ext_op:?} Result {}", idx + 1);
                shape.style = primary_style.clone();
                added_shapes.push(shape);
            }
            let removed = if ext_op == BooleanOp::CreateBoundary {
                Vec::new()
            } else {
                selected_with_layer
            };
            let summary = format!("{ext_op:?} on {} shapes", selected_shapes.len());
            let cmd = BooleanReplaceCommand::new(summary, removed, added_shapes, target_layer_idx);
            undo.apply(Box::new(cmd), doc);
            ctx.snap_controller.update_index(doc);
            Ok(())
        }
    }
}
