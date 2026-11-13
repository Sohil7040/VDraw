//! Alignment & Distribution panel.

use crate::panels::toast::ToastManager;
use crate::tokens::ThemeTokens;
use core_doc::{Document, ShapeId, UndoStack};
use core_geom::{Point2, Rect};
use egui::{RichText, Ui};

/// Dispatches per-shape translations through [`core_tools::Dispatcher`], ensuring mutations
/// route through the formal command dispatcher and update the snap controller.
pub fn dispatch_transform_shapes(
    doc: &mut Document,
    undo: &mut UndoStack,
    changes: Vec<(ShapeId, Point2)>,
) {
    let original_sel = doc.selection.clone();
    let mut snap = core_tools::SnapController::new(core_snap::DEFAULT_SNAP_MASK);
    let mut tools = core_tools::ToolManager::new(core_tools::ToolId::Pick);
    let mut dctx = core_tools::DispatchCtx::new(&mut snap, &mut tools);
    let mut dispatcher = core_tools::Dispatcher;

    for (id, delta) in changes {
        if delta.x.abs() > 1e-9 || delta.y.abs() > 1e-9 {
            doc.selection.ids.clear();
            doc.selection.ids.insert(id);
            let _ = dispatcher.dispatch(
                core_tools::AppCommand::TranslateSelection { delta },
                doc,
                undo,
                &mut dctx,
            );
        }
    }
    doc.selection = original_sel;
}

pub fn show_align_panel(
    ui: &mut Ui,
    doc: &mut Document,
    undo: &mut UndoStack,
    toast: &mut ToastManager,
) {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    ui.heading("Align & Distribute");
    ui.separator();

    let sel_ids: Vec<ShapeId> = doc.selection.ids.iter().copied().collect();

    if sel_ids.len() < 2 {
        ui.label(
            RichText::new("Select 2 or more shapes to align or distribute.")
                .color(tokens.color.on_surface_muted)
                .italics(),
        );
        return;
    }

    let mut shape_bounds = Vec::with_capacity(sel_ids.len());
    let mut collective_bounds: Option<Rect> = None;

    for &id in &sel_ids {
        if let Some(shape) = doc.find_shape(id) {
            if let Some(b) = shape.world_bounds() {
                collective_bounds = Some(match collective_bounds {
                    Some(cb) => cb.union(&b),
                    None => b,
                });
                shape_bounds.push((id, b));
            }
        }
    }

    let Some(cb) = collective_bounds else {
        return;
    };

    ui.label(RichText::new("Align:").strong());
    ui.horizontal(|ui| {
        if ui
            .button("⇤ Left")
            .on_hover_text("Align left edges")
            .clicked()
        {
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dx = cb.min_x - b.min_x;
                changes.push((id, Point2::new(dx, 0.0)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Left");
        }

        if ui
            .button("⇋ Center H")
            .on_hover_text("Align horizontal centers")
            .clicked()
        {
            let center_x = (cb.min_x + cb.max_x) * 0.5;
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dx = center_x - (b.min_x + b.max_x) * 0.5;
                changes.push((id, Point2::new(dx, 0.0)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Center Horizontal");
        }

        if ui
            .button("⇥ Right")
            .on_hover_text("Align right edges")
            .clicked()
        {
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dx = cb.max_x - b.max_x;
                changes.push((id, Point2::new(dx, 0.0)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Right");
        }
    });

    ui.horizontal(|ui| {
        if ui
            .button("⤒ Top")
            .on_hover_text("Align top edges")
            .clicked()
        {
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dy = cb.min_y - b.min_y;
                changes.push((id, Point2::new(0.0, dy)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Top");
        }

        if ui
            .button("⇅ Center V")
            .on_hover_text("Align vertical centers")
            .clicked()
        {
            let center_y = (cb.min_y + cb.max_y) * 0.5;
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dy = center_y - (b.min_y + b.max_y) * 0.5;
                changes.push((id, Point2::new(0.0, dy)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Center Vertical");
        }

        if ui
            .button("⤓ Bottom")
            .on_hover_text("Align bottom edges")
            .clicked()
        {
            let mut changes = Vec::new();
            for &(id, b) in &shape_bounds {
                let dy = cb.max_y - b.max_y;
                changes.push((id, Point2::new(0.0, dy)));
            }
            dispatch_transform_shapes(doc, undo, changes);
            toast.show("Aligned Bottom");
        }
    });

    ui.separator();
    ui.label(RichText::new("Distribute Spacing:").strong());

    ui.horizontal(|ui| {
        if ui
            .button("↔ Horizontal")
            .on_hover_text("Distribute equal horizontal spacing")
            .clicked()
            && shape_bounds.len() >= 3
        {
            let mut sorted = shape_bounds.clone();
            sorted.sort_by(|a, b| a.1.min_x.total_cmp(&b.1.min_x));

            if let (Some(first), Some(last)) = (sorted.first(), sorted.last()) {
                let first_x = first.1.min_x;
                let last_x = last.1.min_x;
                let step = (last_x - first_x) / (sorted.len() - 1) as f64;

                let mut changes = Vec::new();
                for (i, &(id, b)) in sorted.iter().enumerate() {
                    let target_x = first_x + (i as f64) * step;
                    let dx = target_x - b.min_x;
                    changes.push((id, Point2::new(dx, 0.0)));
                }
                dispatch_transform_shapes(doc, undo, changes);
                toast.show("Distributed Horizontally");
            }
        }

        if ui
            .button("↕ Vertical")
            .on_hover_text("Distribute equal vertical spacing")
            .clicked()
            && shape_bounds.len() >= 3
        {
            let mut sorted = shape_bounds.clone();
            sorted.sort_by(|a, b| a.1.min_y.total_cmp(&b.1.min_y));

            if let (Some(first), Some(last)) = (sorted.first(), sorted.last()) {
                let first_y = first.1.min_y;
                let last_y = last.1.min_y;
                let step = (last_y - first_y) / (sorted.len() - 1) as f64;

                let mut changes = Vec::new();
                for (i, &(id, b)) in sorted.iter().enumerate() {
                    let target_y = first_y + (i as f64) * step;
                    let dy = target_y - b.min_y;
                    changes.push((id, Point2::new(0.0, dy)));
                }
                dispatch_transform_shapes(doc, undo, changes);
                toast.show("Distributed Vertically");
            }
        }
    });
}
