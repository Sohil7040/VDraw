//! Object & Layer Manager panel.

use crate::panels::toast::ToastManager;
use crate::tokens::ThemeTokens;
use core_doc::{Document, ShapeKind};
use core_tools::AppCommand;
use egui::{RichText, Ui};

/// Show the live object / layer manager panel content, returning any [`AppCommand`]s triggered.
pub fn show_object_manager(
    ui: &mut Ui,
    doc: &mut Document,
    toast: &mut ToastManager,
) -> Vec<AppCommand> {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let c = &tokens.color;
    let mut commands = Vec::new();

    ui.horizontal(|ui| {
        ui.heading("Objects & Layers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕ Add Layer").clicked() {
                if let Some(page) = doc.active_page_mut() {
                    let new_idx = page.layers.len() + 1;
                    page.layers
                        .push(core_doc::Layer::new(format!("Layer {new_idx}")));
                    toast.show(format!("Created Layer {new_idx}"));
                }
            }
        });
    });
    ui.separator();

    let selected_ids = doc.selection.ids.clone();

    let Some(page) = doc.active_page_mut() else {
        ui.label(
            RichText::new("No active page")
                .italics()
                .color(c.on_surface_disabled),
        );
        return commands;
    };

    let mut shape_to_select = None;
    let mut shape_to_delete = None;

    egui::ScrollArea::vertical()
        .id_salt("obj_mgr_scroll")
        .show(ui, |ui| {
            let page_header = format!(
                "📄 {} ({:.0} × {:.0} mm)",
                page.name, page.width_mm, page.height_mm
            );
            egui::CollapsingHeader::new(page_header)
                .default_open(true)
                .show(ui, |ui| {
                    for (l_idx, layer) in page.layers.iter_mut().enumerate() {
                        let layer_id = format!("layer_{l_idx}");
                        ui.horizontal(|ui| {
                            let vis_icon = if layer.visible { "👁" } else { "⊘" };
                            if ui
                                .button(vis_icon)
                                .on_hover_text("Toggle layer visibility")
                                .clicked()
                            {
                                layer.visible = !layer.visible;
                            }

                            let lock_icon = if layer.locked { "🔒" } else { "🔓" };
                            if ui
                                .button(lock_icon)
                                .on_hover_text("Toggle layer lock")
                                .clicked()
                            {
                                layer.locked = !layer.locked;
                            }

                            let label_text =
                                format!("🔵 {} ({} shapes)", layer.name, layer.shapes.len());
                            ui.label(RichText::new(label_text).strong());
                        });

                        ui.indent(layer_id, |ui| {
                            if layer.shapes.is_empty() {
                                ui.label(
                                    RichText::new("(empty layer)")
                                        .italics()
                                        .color(c.on_surface_disabled),
                                );
                            } else {
                                for shape in &layer.shapes {
                                    let is_selected = selected_ids.contains(&shape.id);
                                    let kind_icon = match &shape.kind {
                                        ShapeKind::Rect { .. } => "▭",
                                        ShapeKind::Ellipse { .. } => "○",
                                        ShapeKind::Line { .. } => "╱",
                                        ShapeKind::Path(_) => "☩",
                                        ShapeKind::Group { .. } => "🗂",
                                        ShapeKind::Polygon { .. } => "⬡",
                                        ShapeKind::Star { .. } => "★",
                                        ShapeKind::Spiral { .. } => "🌀",
                                        ShapeKind::Text { .. } => "🔤",
                                    };

                                    ui.horizontal(|ui| {
                                        let name =
                                            format!("{kind_icon} {} {}", shape.name, shape.id);
                                        let mut label = RichText::new(name).small();
                                        if is_selected {
                                            label = label.color(c.accent_hover).strong();
                                        }

                                        let resp =
                                            ui.add(egui::SelectableLabel::new(is_selected, label));
                                        if resp.clicked() {
                                            shape_to_select = Some(shape.id);
                                        }

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("🗑")
                                                    .on_hover_text("Delete shape")
                                                    .clicked()
                                                {
                                                    shape_to_delete = Some(shape.id);
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
        });

    if let Some(id) = shape_to_select {
        doc.selection.clear();
        doc.selection.add(id);
    }

    if let Some(id) = shape_to_delete {
        doc.selection.select(id);
        commands.push(AppCommand::DeleteSelection);
    }

    commands
}
