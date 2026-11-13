//! Context-sensitive tool property bar (all Boolean/Contour actions route via [`AppCommand`]).

use crate::panels::toast::ToastManager;
use crate::tokens::{SpacingTokens, ThemeTokens};
use core_boolean::{BooleanOp, ContourJoin};
use core_doc::{Document, ModifyShapeKindCommand, ShapeKind, UndoStack};
use core_geom::NodeType;
use core_tools::{AppCommand, KnifeMode, ToolId, ToolManager};

/// Show the context property bar beneath the top command bar, returning any [`AppCommand`]s triggered.
pub fn show_prop_bar(
    ui: &mut egui::Ui,
    doc: &mut Document,
    undo: &mut UndoStack,
    tool_mgr: &mut ToolManager,
    toast: &mut ToastManager,
) -> Vec<AppCommand> {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let c = &tokens.color;
    let mut commands = Vec::new();

    ui.set_height(SpacingTokens::COMMANDBAR_HEIGHT - 4.0);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        match tool_mgr.active_id() {
            ToolId::Shape => {
                ui.label(egui::RichText::new("☩ Node Tool").color(c.accent).strong());
                ui.separator();

                if let Some(shape_tool) = tool_mgr.shape_tool_mut() {
                    let sel_count = shape_tool.selected_nodes.len();
                    if sel_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{sel_count} node(s)"))
                                .color(c.on_surface)
                                .small(),
                        );
                        if ui
                            .button("Cusp")
                            .on_hover_text("Independent handle angles and lengths")
                            .clicked()
                        {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Cusp);
                        }
                        if ui
                            .button("Smooth")
                            .on_hover_text("Collinear G1 continuous handles")
                            .clicked()
                        {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Smooth);
                        }
                        if ui
                            .button("Symmetrical")
                            .on_hover_text("Collinear and equal length handles")
                            .clicked()
                        {
                            shape_tool.set_selected_nodes_type(doc, undo, NodeType::Symmetrical);
                        }
                        ui.separator();
                        if ui.button("Delete Node").clicked() {
                            shape_tool.delete_selected_nodes(doc, undo);
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "Click anchor to edit handles  ·  Double-click curve to insert node",
                            )
                            .color(c.on_surface_muted)
                            .small()
                            .italics(),
                        );
                    }
                }
            }
            ToolId::Knife => {
                ui.label(egui::RichText::new("⚔ Knife Tool").color(c.accent).strong());
                ui.separator();
                if let Some(knife) = tool_mgr.knife_tool_mut() {
                    ui.selectable_value(&mut knife.mode, KnifeMode::Straight, "Straight Cut");
                    ui.selectable_value(&mut knife.mode, KnifeMode::Freehand, "Freehand Cut");
                    ui.separator();
                    ui.checkbox(&mut knife.auto_close, "Auto-close Slices");
                }
            }
            ToolId::Eraser => {
                ui.label(egui::RichText::new("⌫ Eraser Tool").color(c.accent).strong());
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
                        .color(c.accent)
                        .strong(),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "Click any path segment between intersections to auto-trim",
                    )
                    .color(c.on_surface_muted)
                    .small(),
                );
            }
            ToolId::Text => {
                ui.label(
                    egui::RichText::new("T Text Outlines")
                        .color(c.accent)
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
                ui.label(egui::RichText::new("⬡ Polygon").color(c.accent).strong());
                ui.separator();
                if let Some(poly_tool) = tool_mgr.polygon_tool_mut() {
                    ui.label("Sides:");
                    ui.add(egui::DragValue::new(&mut poly_tool.sides).range(3..=50));
                }
            }
            ToolId::Star => {
                ui.label(egui::RichText::new("★ Star").color(c.accent).strong());
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
                ui.label(egui::RichText::new("🌀 Spiral").color(c.accent).strong());
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
                ui.label(egui::RichText::new("〜 Freehand").color(c.accent).strong());
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
                        .color(c.on_surface)
                        .strong(),
                );
            }
        }

        ui.separator();

        let sel_count = doc.selection.len();
        if sel_count == 0 {
            ui.label(
                egui::RichText::new(format!(
                    "Page: ({:.1} × {:.1} mm)  ·  Units: {}",
                    doc.active_page().map(|p| p.width_mm).unwrap_or(210.0),
                    doc.active_page().map(|p| p.height_mm).unwrap_or(297.0),
                    doc.unit.label()
                ))
                .color(c.on_surface_muted)
                .small(),
            );
        } else {
            let label = if sel_count == 1 {
                "1 selected".to_string()
            } else {
                format!("{sel_count} selected")
            };
            ui.label(
                egui::RichText::new(label)
                    .color(c.on_surface_muted)
                    .small(),
            );

            if let Some(bounds) = doc.selection_bounds() {
                ui.separator();
                ui.label(
                    egui::RichText::new("⊕ Center:")
                        .color(c.snap_center)
                        .small(),
                );
                ui.monospace(format!("X: {:>6.1}", bounds.center().x));
                ui.monospace(format!("Y: {:>6.1}", bounds.center().y));
                ui.separator();
                ui.monospace(format!("W: {:>6.1}", bounds.width()));
                ui.monospace(format!("H: {:>6.1}", bounds.height()));
            }

            if sel_count >= 2 {
                ui.separator();
                if ui
                    .button("⊞ Weld")
                    .on_hover_text("Weld overlapping shapes into one unified path")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(BooleanOp::Weld));
                }
                if ui
                    .button("⊟ Trim")
                    .on_hover_text("Subtract front shapes from the back shape")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(BooleanOp::Trim));
                }
                if ui
                    .button("⊠ Intersect")
                    .on_hover_text("Keep only overlapping intersection region")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(BooleanOp::Intersect));
                }
                if ui
                    .button("⊞̷ Exclude")
                    .on_hover_text("XOR: Exclude overlapping regions")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(BooleanOp::Exclude));
                }
                if ui
                    .button("⬚ Boundary")
                    .on_hover_text("Create outer perimeter silhouette")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(BooleanOp::CreateBoundary));
                }
            }

            if sel_count >= 1 {
                ui.separator();
                if ui
                    .button("⤢ Contour +3mm")
                    .on_hover_text("Generate a +3.0 mm parallel laser kerf contour")
                    .clicked()
                {
                    commands.push(AppCommand::OffsetSelection {
                        distance_mm: 3.0,
                        join: ContourJoin::Round,
                    });
                }
            }

            let can_convert_curves = doc.selection.ids.iter().any(|&id| {
                doc.find_shape(id)
                    .is_some_and(|s| !matches!(s.kind, ShapeKind::Path(_)))
            });

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
                    toast.show("Converted primitive(s) to curves");
                }
            }
        }
    });

    commands
}
