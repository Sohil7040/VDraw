//! Laser CAM Setup & Toolpath Inspector Panel.

use super::toast::ToastManager;
use crate::tokens::{SemanticColors, ThemeTokens};
use core_cam::*;
use core_doc::{Document, UndoStack};
use core_tools::AppCommand;
use egui::{Color32, Stroke, Ui, Vec2};

/// Show the dedicated Laser CAM panel.
pub fn show_laser_panel(
    ui: &mut Ui,
    doc: &mut Document,
    _undo: &mut UndoStack,
    config: &mut LaserProductionConfig,
    toast: &mut ToastManager,
) -> Vec<AppCommand> {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let c = &tokens.color;
    let mut commands = Vec::new();
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 8.0);

    // ─── 1. Header & Live Mode Switch ─────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("Laser Production");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let preview_btn = if config.preview {
                egui::Button::new(egui::RichText::new("◉ Preview Active").color(c.on_accent))
                    .fill(c.laser_active)
            } else {
                egui::Button::new("▷ Preview Job")
            };
            if ui.add(preview_btn).clicked() {
                if config.preview {
                    commands.push(AppCommand::SwitchWorkflowLaserCam);
                } else {
                    commands.push(AppCommand::SwitchWorkflowPreview);
                }
            }
        });
    });

    ui.separator();

    // ─── 2. Material Preset Selector ──────────────────────────────────────────
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Material Preset:").strong());
            egui::ComboBox::from_id_salt("material_preset_combo")
                .selected_text("Select preset…")
                .show_ui(ui, |ui| {
                    for preset in BUILTIN_MATERIAL_PRESETS {
                        if ui
                            .selectable_label(
                                false,
                                format!("{} ({})", preset.name, preset.category),
                            )
                            .clicked()
                        {
                            if let Some(page) = doc.active_page() {
                                for layer in &page.layers {
                                    let mut s = LayerLaserSettings::new_cut(
                                        preset.speed_mm_s,
                                        preset.power_pct,
                                        0,
                                    );
                                    s.mode = preset.mode;
                                    s.air_assist = preset.air_assist;
                                    s.passes = preset.passes;
                                    s.kerf_mm = preset.kerf_mm;
                                    config.layer_settings.insert(layer.name.clone(), s);
                                }
                            }
                            config.kerf = preset.kerf_mm;
                            toast.show(format!("Applied material preset: {}", preset.name));
                        }
                    }
                });
        });
    });

    // ─── 3. Layer Operations Table ────────────────────────────────────────────
    ui.label(
        egui::RichText::new("Layer Laser Operations:")
            .small()
            .strong()
            .color(c.on_surface_muted),
    );

    let layers_info: Vec<(String, Color32)> = if let Some(page) = doc.active_page() {
        page.layers
            .iter()
            .enumerate()
            .map(|(idx, l)| {
                let ruida = RUIDA_PALETTE[idx % 16];
                (l.name.clone(), SemanticColors::from_rgb_bytes(ruida))
            })
            .collect()
    } else {
        Vec::new()
    };

    if layers_info.is_empty() {
        ui.label(
            egui::RichText::new("No layers in document")
                .italics()
                .color(c.on_surface_disabled),
        );
    } else {
        for (idx, (layer_name, layer_color)) in layers_info.iter().enumerate() {
            let settings = config
                .layer_settings
                .entry(layer_name.clone())
                .or_insert_with(|| {
                    let name_lower = layer_name.to_lowercase();
                    if name_lower.contains("engrave") || name_lower.contains("scan") {
                        LayerLaserSettings::new_engrave(180.0, 40.0, (idx % 16) as u8)
                    } else if name_lower.contains("score") || name_lower.contains("mark") {
                        LayerLaserSettings::new_score(60.0, 30.0, (idx % 16) as u8)
                    } else if name_lower.contains("guide") || name_lower.contains("ignore") {
                        LayerLaserSettings {
                            mode: LaserMode::Ignore,
                            color_index: (idx % 16) as u8,
                            ..Default::default()
                        }
                    } else {
                        LayerLaserSettings::new_cut(25.0, 80.0, (idx % 16) as u8)
                    }
                });

            let mode_color32 = SemanticColors::from_rgb_bytes(settings.mode.color_rgb());

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, mode_color32);
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.0, *layer_color),
                        egui::StrokeKind::Outside,
                    );

                    ui.strong(layer_name);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt(format!("mode_{layer_name}"))
                            .selected_text(settings.mode.label())
                            .show_ui(ui, |ui| {
                                for m in [
                                    LaserMode::Cut,
                                    LaserMode::Score,
                                    LaserMode::Engrave,
                                    LaserMode::Dot,
                                    LaserMode::Pen,
                                    LaserMode::Ignore,
                                ] {
                                    let text = egui::RichText::new(m.label())
                                        .color(SemanticColors::from_rgb_bytes(m.color_rgb()));
                                    if ui.selectable_label(settings.mode == m, text).clicked() {
                                        settings.mode = m;
                                    }
                                }
                            });
                    });
                });

                if settings.mode != LaserMode::Ignore {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(
                            egui::DragValue::new(&mut settings.speed_mm_s)
                                .range(1.0..=1000.0)
                                .suffix(" mm/s"),
                        );
                        ui.separator();
                        ui.label("Power:");
                        ui.add(
                            egui::DragValue::new(&mut settings.power_pct)
                                .range(1.0..=100.0)
                                .suffix(" %"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Passes:");
                        ui.add(egui::DragValue::new(&mut settings.passes).range(1..=20));
                        ui.separator();
                        let air_label = if settings.air_assist {
                            "Air Assist: ON"
                        } else {
                            "Air Assist: OFF"
                        };
                        if ui
                            .selectable_label(settings.air_assist, air_label)
                            .clicked()
                        {
                            settings.air_assist = !settings.air_assist;
                        }
                    });
                }
            });
        }
    }

    // ─── 4. Toolpath Optimization & Machine Rules ─────────────────────────────
    ui.collapsing("Toolpath Optimization & Rules", |ui| {
        ui.checkbox(
            &mut config.inside_out,
            "Inside-to-Outside (Cut inner holes first)",
        );
        ui.checkbox(
            &mut config.optimize_order,
            "Optimize Rapid Travel (Greedy TSP)",
        );
        ui.checkbox(
            &mut config.show_sequence_badges,
            "Show Order Badges (1, 2, 3...)",
        );

        ui.horizontal(|ui| {
            ui.label("Kerf Offset:");
            ui.add(
                egui::DragValue::new(&mut config.kerf)
                    .range(0.0..=2.0)
                    .speed(0.01)
                    .suffix(" mm"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Rapid Speed:");
            ui.add(
                egui::DragValue::new(&mut config.rapid_speed_mm_s)
                    .range(50.0..=2000.0)
                    .suffix(" mm/s"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Lead-In:");
            ui.add(
                egui::DragValue::new(&mut config.lead_in)
                    .range(0.0..=10.0)
                    .speed(0.1)
                    .suffix(" mm"),
            );
            ui.separator();
            ui.label("Lead-Out:");
            ui.add(
                egui::DragValue::new(&mut config.lead_out)
                    .range(0.0..=10.0)
                    .speed(0.1)
                    .suffix(" mm"),
            );
        });
    });

    // ─── 5. Job Metrics & Execution Summary ───────────────────────────────────
    let plan = generate_job_plan(doc, config);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Job Estimation:").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} Toolpaths", plan.toolpaths.len()));
            });
        });

        ui.horizontal(|ui| {
            ui.label("Cut Length:");
            ui.strong(format!("{:.1} mm", plan.total_cut_length_mm));
        });

        ui.horizontal(|ui| {
            ui.label("Rapid Travel:");
            ui.strong(format!("{:.1} mm", plan.total_rapid_length_mm));
        });

        ui.horizontal(|ui| {
            ui.label("Est. Duration:");
            let mins = (plan.estimated_duration_s / 60.0).floor() as u32;
            let secs = (plan.estimated_duration_s % 60.0).round() as u32;
            ui.strong(format!("{mins:02}:{secs:02} min"));
        });
    });

    // ─── 6. Action Export Buttons ─────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(egui::RichText::new("💾 Export G-Code").color(c.on_accent))
                    .fill(c.laser_active),
            )
            .clicked()
        {
            commands.push(AppCommand::ExportGcode);
        }

        if ui.button("💾 Export PLT / HPGL").clicked() {
            let plt = plan.export_plt();
            let path = "output_laser_job.plt";
            if std::fs::write(path, &plt).is_ok() {
                toast.show(format!("Exported HPGL/PLT to {path}"));
            } else {
                toast.show_warning("Failed to write PLT file");
            }
        }
    });

    commands
}
