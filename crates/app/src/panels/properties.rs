//! Top property bar — context-sensitive tool and selection options.

use crate::panels::toast::ToastManager;
use crate::theme;
use core_boolean::{apply_boolean_to_document, apply_contour_to_document, BooleanOp, ContourJoin};
use core_doc::{Document, ModifyShapeKindCommand, ShapeKind, UndoStack};
use core_geom::NodeType;
use core_tools::{KnifeMode, ToolId, ToolManager};

/// Show the property bar at the top of the workspace.
pub fn show_prop_bar(
    ui: &mut egui::Ui,
    doc: &mut Document,
    undo: &mut UndoStack,
    tool_mgr: &mut ToolManager,
    toast: &mut ToastManager,
) {
    ui.set_height(theme::PROPBAR_HEIGHT);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        // ── 1. Tool-Specific Context Options ─────────────────────────────────
        match tool_mgr.active_id() {
            ToolId::Shape => {
                ui.label(
                    egui::RichText::new("☩ Node Tool")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();

                if let Some(shape_tool) = tool_mgr.shape_tool_mut() {
                    let sel_count = shape_tool.selected_nodes.len();
                    if sel_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} node(s)", sel_count))
                                .color(theme::TEXT_PRIMARY)
                                .small(),
                        );
                        if ui.button("Cusp").on_hover_text("Independent handle angles and lengths").clicked() {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Cusp);
                        }
                        if ui.button("Smooth").on_hover_text("Collinear G1 continuous handles").clicked() {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Smooth);
                        }
                        if ui.button("Symmetrical").on_hover_text("Collinear and equal length handles").clicked() {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Symmetrical);
                        }
                        ui.separator();
                        if ui.button("Delete Node").clicked() {
                            shape_tool.delete_selected_nodes(doc, undo);
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Click anchor to edit handles  ·  Double-click curve to insert node")
                                .color(theme::TEXT_SECONDARY)
                                .small()
                                .italics(),
                        );
                    }
                }
            }
            ToolId::Knife => {
                ui.label(
                    egui::RichText::new("⚔ Knife Tool")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(knife) = tool_mgr.knife_tool_mut() {
                    ui.selectable_value(&mut knife.mode, KnifeMode::Straight, "Straight Cut");
                    ui.selectable_value(&mut knife.mode, KnifeMode::Freehand, "Freehand Cut");
                    ui.separator();
                    ui.checkbox(&mut knife.auto_close, "Auto-close Slices");
                }
            }
            ToolId::Eraser => {
                ui.label(
                    egui::RichText::new("⌫ Eraser Tool")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(eraser) = tool_mgr.eraser_tool_mut() {
                    ui.label("Radius:");
                    ui.add(
                        egui::DragValue::new(&mut eraser.radius)
                            .speed(0.5)
                            .range(1.0..=60.0)
                            .suffix(" mm"),
                    );
                }
            }
            ToolId::VirtualSegmentDelete => {
                ui.label(
                    egui::RichText::new("✂ Virtual Segment Delete")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new("Click any path segment between intersections to auto-trim")
                        .color(theme::TEXT_SECONDARY)
                        .small(),
                );
            }
            ToolId::Text => {
                ui.label(
                    egui::RichText::new("T Text Outlines")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(text_tool) = tool_mgr.text_tool_mut() {
                    ui.label("Text:");
                    ui.text_edit_singleline(&mut text_tool.pending_text);
                    ui.separator();
                    ui.label("Font Size:");
                    ui.add(
                        egui::DragValue::new(&mut text_tool.font_size_mm)
                            .speed(1.0)
                            .range(4.0..=120.0)
                            .suffix(" mm"),
                    );
                }
            }
            ToolId::Polygon => {
                ui.label(
                    egui::RichText::new("⬡ Polygon")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(poly_tool) = tool_mgr.polygon_tool_mut() {
                    ui.label("Sides:");
                    ui.add(egui::DragValue::new(&mut poly_tool.sides).range(3..=50));
                }
            }
            ToolId::Star => {
                ui.label(
                    egui::RichText::new("★ Star")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(star_tool) = tool_mgr.star_tool_mut() {
                    ui.label("Points:");
                    ui.add(egui::DragValue::new(&mut star_tool.points).range(3..=50));
                    ui.separator();
                    ui.label("Sharpness:");
                    let mut pct = (star_tool.sharpness * 100.0).round() as u32;
                    if ui
                        .add(egui::DragValue::new(&mut pct).range(10..=90).suffix("%"))
                        .changed()
                    {
                        star_tool.sharpness = pct as f64 / 100.0;
                    }
                }
            }
            ToolId::Spiral => {
                ui.label(
                    egui::RichText::new("🌀 Spiral")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(spiral_tool) = tool_mgr.spiral_tool_mut() {
                    ui.label("Turns:");
                    ui.add(
                        egui::DragValue::new(&mut spiral_tool.turns)
                            .speed(0.1)
                            .range(0.5..=20.0),
                    );
                }
            }
            ToolId::Freehand => {
                ui.label(
                    egui::RichText::new("〜 Freehand")
                        .color(theme::ACCENT)
                        .strong(),
                );
                ui.separator();
                if let Some(fh_tool) = tool_mgr.freehand_tool_mut() {
                    ui.label("Tolerance:");
                    ui.add(
                        egui::DragValue::new(&mut fh_tool.tolerance)
                            .speed(0.1)
                            .range(0.2..=10.0)
                            .suffix(" mm"),
                    );
                }
            }
            _ => {
                ui.label(
                    egui::RichText::new(tool_mgr.active_id().label())
                        .color(theme::TEXT_PRIMARY)
                        .strong(),
                );
            }
        }

        ui.separator();

        // ── 2. Selection Geometry & Quick Boolean Actions ───────────────────
        let sel_count = doc.selection.len();
        if sel_count == 0 {
            ui.label(
                egui::RichText::new(format!(
                    "Page: A4 ({:.1} × {:.1} mm)  ·  Units: {}",
                    doc.active_page().map(|p| p.width_mm).unwrap_or(210.0),
                    doc.active_page().map(|p| p.height_mm).unwrap_or(297.0),
                    doc.unit.label()
                ))
                .color(theme::TEXT_SECONDARY)
                .small(),
            );
        } else {
            let label = if sel_count == 1 {
                "1 selected".to_string()
            } else {
                format!("{} selected", sel_count)
            };
            ui.label(
                egui::RichText::new(label)
                    .color(theme::TEXT_SECONDARY)
                    .small(),
            );

            if let Some(bounds) = doc.selection_bounds() {
                ui.separator();
                ui.label(
                    egui::RichText::new("⊕ Center:")
                        .color(egui::Color32::from_rgb(6, 182, 212))
                        .small(),
                );
                ui.monospace(format!("X: {:>6.1}", bounds.center().x));
                ui.monospace(format!("Y: {:>6.1}", bounds.center().y));
                ui.separator();
                ui.monospace(format!("W: {:>6.1}", bounds.width()));
                ui.monospace(format!("H: {:>6.1}", bounds.height()));
            }

            // Quick Boolean Actions Bar (when 2+ shapes selected)
            if sel_count >= 2 {
                ui.separator();
                if ui
                    .button("⊞ Weld")
                    .on_hover_text("Weld overlapping shapes into one unified path")
                    .clicked()
                {
                    match apply_boolean_to_document(doc, undo, BooleanOp::Weld, true) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Weld error: {}", e)),
                    }
                }
                if ui
                    .button("⊟ Trim")
                    .on_hover_text("Subtract front shapes from the back shape")
                    .clicked()
                {
                    match apply_boolean_to_document(doc, undo, BooleanOp::Trim, true) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Trim error: {}", e)),
                    }
                }
                if ui
                    .button("⊠ Intersect")
                    .on_hover_text("Keep only overlapping intersection region")
                    .clicked()
                {
                    match apply_boolean_to_document(doc, undo, BooleanOp::Intersect, true) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Intersect error: {}", e)),
                    }
                }
                if ui
                    .button("⊞̷ Exclude")
                    .on_hover_text("XOR: Exclude overlapping regions")
                    .clicked()
                {
                    match apply_boolean_to_document(doc, undo, BooleanOp::Exclude, true) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Exclude error: {}", e)),
                    }
                }
                if ui
                    .button("⬚ Boundary")
                    .on_hover_text("Create outer perimeter silhouette")
                    .clicked()
                {
                    match apply_boolean_to_document(doc, undo, BooleanOp::CreateBoundary, true) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Boundary error: {}", e)),
                    }
                }
            }

            // Quick Parallel Contour Offset (when 1+ shape selected)
            if sel_count >= 1 {
                ui.separator();
                if ui
                    .button("⤢ Contour +3mm")
                    .on_hover_text("Generate a +3.0 mm parallel laser kerf contour")
                    .clicked()
                {
                    match apply_contour_to_document(doc, undo, 3.0, ContourJoin::Round) {
                        Ok(msg) => toast.show(format!("✓ {}", msg)),
                        Err(e) => toast.show(format!("⚠ Contour error: {}", e)),
                    }
                }
            }

            // Check if any selected shape can be converted to curves
            let mut can_convert_curves = false;
            for &id in &doc.selection.ids {
                if let Some(shape) = doc.find_shape(id) {
                    if !matches!(shape.kind, ShapeKind::Path(_)) {
                        can_convert_curves = true;
                        break;
                    }
                }
            }

            if can_convert_curves {
                ui.separator();
                if ui
                    .button("Convert to Curves")
                    .on_hover_text("Convert primitive into editable Bézier nodes")
                    .clicked()
                {
                    let ids: Vec<_> = doc.selection.ids.iter().copied().collect();
                    for id in ids {
                        if let Some(shape) = doc.find_shape(id) {
                            if !matches!(shape.kind, ShapeKind::Path(_)) {
                                let old_kind = shape.kind.clone();
                                let poly = shape.to_polycurve();
                                let new_kind = ShapeKind::Path(poly);
                                let cmd = Box::new(ModifyShapeKindCommand::new(
                                    id,
                                    "Convert to Curves",
                                    old_kind,
                                    new_kind,
                                ));
                                undo.apply(cmd, doc);
                            }
                        }
                    }
                    toast.show("✓ Converted primitive(s) to curves");
                }
            }
        }
    });
}
