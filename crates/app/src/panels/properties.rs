//! Top property bar — context-sensitive tool and selection options.

use crate::theme;
use core_doc::{Document, ModifyShapeKindCommand, ShapeKind, UndoStack};
use core_geom::NodeType;
use core_tools::{ToolId, ToolManager};

/// Show the property bar at the top of the workspace.
pub fn show_prop_bar(
    ui: &mut egui::Ui,
    doc: &mut Document,
    undo: &mut UndoStack,
    tool_mgr: &mut ToolManager,
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

        // ── 2. Selection Geometry Information ────────────────────────────────
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
                ui.monospace(format!("X: {:>6.2}", bounds.min_x));
                ui.monospace(format!("Y: {:>6.2}", bounds.min_y));
                ui.separator();
                ui.monospace(format!("W: {:>6.2}", bounds.width()));
                ui.monospace(format!("H: {:>6.2}", bounds.height()));
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
                if ui.button("Convert to Curves").on_hover_text("Convert primitive into editable Bézier nodes").clicked() {
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
                }
            }
        }
    });
}
