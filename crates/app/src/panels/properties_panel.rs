//! Right properties inspector: Transform (Rotate, Mirror, Scale), Boolean shaping, and Style.

use crate::panels::toast::ToastManager;
use crate::tokens::{SemanticColors, ThemeTokens};
use core_boolean::{BooleanOp, ContourJoin};
use core_doc::style::Stroke;
use core_doc::{Document, PagePreset, SetStyleCommand, UndoStack};
use core_geom::{Point2, Rect};
use core_tools::AppCommand;
use egui::{DragValue, RichText, Ui};

/// Persistent UI input state for the Right Properties Inspector.
#[derive(Debug, Clone)]
pub struct PropertiesInspectorState {
    pub use_selection_center_pivot: bool,
    pub pivot_x: f64,
    pub pivot_y: f64,
    pub rotate_angle_deg: f64,
    pub mirror_line_point_x: f64,
    pub mirror_line_point_y: f64,
    pub mirror_line_dir_x: f64,
    pub mirror_line_dir_y: f64,
    pub mirror_line_preview: bool,
    pub scale_uniform: bool,
    pub scale_x: f64,
    pub scale_y: f64,
    pub offset_mm: f64,
    pub offset_join: ContourJoin,
}

impl Default for PropertiesInspectorState {
    fn default() -> Self {
        Self {
            use_selection_center_pivot: true,
            pivot_x: 0.0,
            pivot_y: 0.0,
            rotate_angle_deg: 45.0,
            mirror_line_point_x: 105.0,
            mirror_line_point_y: 148.5,
            mirror_line_dir_x: 1.0,
            mirror_line_dir_y: 1.0,
            mirror_line_preview: false,
            scale_uniform: true,
            scale_x: 1.25,
            scale_y: 1.25,
            offset_mm: 3.0,
            offset_join: ContourJoin::Round,
        }
    }
}

impl PropertiesInspectorState {
    /// Returns the explicit pivot override if custom pivot is selected, or `None` for selection center.
    pub fn effective_pivot(&self) -> Option<Point2> {
        if self.use_selection_center_pivot {
            None
        } else {
            Some(Point2::new(self.pivot_x, self.pivot_y))
        }
    }
}

/// Renders the Right Properties Inspector and returns any dispatched [`AppCommand`]s.
pub fn show_properties_panel(
    ui: &mut Ui,
    inspector: &mut PropertiesInspectorState,
    doc: &mut Document,
    undo: &mut UndoStack,
    toast: &mut ToastManager,
) -> Vec<AppCommand> {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let c = &tokens.color;
    let mut commands = Vec::new();

    ui.heading(RichText::new("Properties").color(c.on_surface));
    ui.separator();

    let sel_ids: Vec<core_doc::ShapeId> = doc.selection.ids.iter().copied().collect();
    let sel_count = sel_ids.len();

    if sel_ids.is_empty() {
        ui.label(RichText::new("Page Setup").color(c.on_surface).strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Title:");
            ui.text_edit_singleline(&mut doc.title);
        });

        if let Some(page) = doc.active_page_mut() {
            ui.horizontal(|ui| {
                ui.label("Page Name:");
                ui.text_edit_singleline(&mut page.name);
            });

            ui.horizontal(|ui| {
                ui.label("Preset:");
                egui::ComboBox::from_id_salt("page_preset_combo")
                    .selected_text("Select Preset…")
                    .show_ui(ui, |ui| {
                        for preset in [
                            PagePreset::A4,
                            PagePreset::A3,
                            PagePreset::Letter,
                            PagePreset::Tabloid,
                            PagePreset::LaserBedSmall,
                            PagePreset::LaserBedMedium,
                            PagePreset::LaserBedLarge,
                        ] {
                            if let Some((w, h)) = preset.dimensions_mm() {
                                if ui
                                    .selectable_label(
                                        false,
                                        format!("{preset:?} ({w:.0}×{h:.0} mm)"),
                                    )
                                    .clicked()
                                {
                                    page.width_mm = w;
                                    page.height_mm = h;
                                    toast.show(format!("Applied preset: {preset:?}"));
                                }
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(
                    DragValue::new(&mut page.width_mm)
                        .speed(1.0)
                        .range(10.0..=5000.0)
                        .suffix(" mm"),
                );
                ui.separator();
                ui.label("Height:");
                ui.add(
                    DragValue::new(&mut page.height_mm)
                        .speed(1.0)
                        .range(10.0..=5000.0)
                        .suffix(" mm"),
                );
            });

            ui.separator();
            ui.label(RichText::new("Statistics").color(c.on_surface).strong());
            let total_shapes: usize = page.layers.iter().map(|l| l.shapes.len()).sum();
            ui.label(
                RichText::new(format!(
                    "Layers: {} · Shapes: {total_shapes}",
                    page.layers.len()
                ))
                .small()
                .color(c.on_surface_muted),
            );
        }
    } else if sel_ids.len() == 1 {
        let shape_id = sel_ids[0];
        if let Some(shape) = doc.find_shape(shape_id).cloned() {
            ui.label(
                RichText::new(format!("Object #{}: {}", shape.id.0, shape.name))
                    .color(c.on_surface)
                    .strong(),
            );
            if let Some(bounds) = shape.world_bounds() {
                let mut x = bounds.min_x;
                let mut y = bounds.min_y;
                let mut w = bounds.width();
                let mut h = bounds.height();

                ui.horizontal(|ui| {
                    ui.label("X:");
                    if ui
                        .add(DragValue::new(&mut x).speed(0.5).suffix(" mm"))
                        .changed()
                    {
                        commands.push(AppCommand::TranslateSelection {
                            delta: Point2::new(x - bounds.min_x, 0.0),
                        });
                    }
                    ui.label("Y:");
                    if ui
                        .add(DragValue::new(&mut y).speed(0.5).suffix(" mm"))
                        .changed()
                    {
                        commands.push(AppCommand::TranslateSelection {
                            delta: Point2::new(0.0, y - bounds.min_y),
                        });
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("W:");
                    if ui
                        .add(
                            DragValue::new(&mut w)
                                .speed(0.5)
                                .range(0.1..=10000.0)
                                .suffix(" mm"),
                        )
                        .changed()
                        && bounds.width() > 1e-4
                    {
                        commands.push(AppCommand::ScaleSelection {
                            pivot: Some(bounds.center()),
                            sx: w / bounds.width(),
                            sy: 1.0,
                        });
                    }
                    ui.label("H:");
                    if ui
                        .add(
                            DragValue::new(&mut h)
                                .speed(0.5)
                                .range(0.1..=10000.0)
                                .suffix(" mm"),
                        )
                        .changed()
                        && bounds.height() > 1e-4
                    {
                        commands.push(AppCommand::ScaleSelection {
                            pivot: Some(bounds.center()),
                            sx: 1.0,
                            sy: h / bounds.height(),
                        });
                    }
                });
            }
        }
    } else {
        ui.label(
            RichText::new(format!("{sel_count} Objects Selected"))
                .color(c.on_surface)
                .strong(),
        );
        let mut cb: Option<Rect> = None;
        for &id in &sel_ids {
            if let Some(shape) = doc.find_shape(id) {
                if let Some(b) = shape.world_bounds() {
                    cb = Some(match cb {
                        Some(cur) => cur.union(&b),
                        None => b,
                    });
                }
            }
        }
        if let Some(b) = cb {
            ui.label(
                RichText::new(format!(
                    "Bounds: {:.1} × {:.1} mm at ({:.1}, {:.1})",
                    b.width(),
                    b.height(),
                    b.min_x,
                    b.min_y
                ))
                .small()
                .color(c.on_surface_muted),
            );
        }
    }

    ui.add_space(6.0);
    let has_selection = sel_count >= 1;

    // ── Transform Section (Rotate, Mirror, Scale) ────────────────────────────
    ui.group(|ui| {
        ui.label(
            RichText::new("TRANSFORM")
                .small()
                .strong()
                .color(c.on_surface_muted),
        );
        ui.add_space(4.0);

        // Pivot selector shared by Rotate & Scale
        ui.checkbox(
            &mut inspector.use_selection_center_pivot,
            "Pivot: Selection Center",
        );
        if !inspector.use_selection_center_pivot {
            ui.horizontal(|ui| {
                ui.label("Pivot X:");
                ui.add(
                    DragValue::new(&mut inspector.pivot_x)
                        .speed(0.5)
                        .suffix(" mm"),
                );
                ui.label("Y:");
                ui.add(
                    DragValue::new(&mut inspector.pivot_y)
                        .speed(0.5)
                        .suffix(" mm"),
                );
            });
        }

        ui.separator();
        // Rotate
        ui.horizontal(|ui| {
            ui.label("Rotate:");
            ui.add(
                DragValue::new(&mut inspector.rotate_angle_deg)
                    .speed(1.0)
                    .range(-360.0..=360.0)
                    .suffix("°"),
            );
            let apply_rot = ui
                .add_enabled(has_selection, egui::Button::new("Apply"))
                .on_hover_text("Rotate selected shape(s) around pivot");
            if apply_rot.clicked() {
                commands.push(AppCommand::RotateSelection {
                    pivot: inspector.effective_pivot(),
                    angle_deg: inspector.rotate_angle_deg,
                });
            }
        });

        ui.separator();
        // Mirror
        ui.label(RichText::new("Mirror:").small().color(c.on_surface_muted));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(has_selection, egui::Button::new("⇄ Horizontal"))
                .on_hover_text("Mirror across vertical axis through selection center")
                .clicked()
            {
                commands.push(AppCommand::MirrorSelectionHorizontal);
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("⇅ Vertical"))
                .on_hover_text("Mirror across horizontal axis through selection center")
                .clicked()
            {
                commands.push(AppCommand::MirrorSelectionVertical);
            }
        });

        ui.collapsing("Mirror Across Custom Line", |ui| {
            ui.horizontal(|ui| {
                ui.label("Pt X:");
                ui.add(DragValue::new(&mut inspector.mirror_line_point_x).speed(0.5));
                ui.label("Y:");
                ui.add(DragValue::new(&mut inspector.mirror_line_point_y).speed(0.5));
            });
            ui.horizontal(|ui| {
                ui.label("Dir X:");
                ui.add(DragValue::new(&mut inspector.mirror_line_dir_x).speed(0.05));
                ui.label("Y:");
                ui.add(DragValue::new(&mut inspector.mirror_line_dir_y).speed(0.05));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut inspector.mirror_line_preview, "Live Preview");
                if ui
                    .add_enabled(has_selection, egui::Button::new("Apply Line Mirror"))
                    .clicked()
                {
                    commands.push(AppCommand::MirrorSelectionLine {
                        point: Point2::new(
                            inspector.mirror_line_point_x,
                            inspector.mirror_line_point_y,
                        ),
                        dir: Point2::new(inspector.mirror_line_dir_x, inspector.mirror_line_dir_y),
                    });
                }
            });
        });

        ui.separator();
        // Scale
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.checkbox(&mut inspector.scale_uniform, "Uniform");
        });
        ui.horizontal(|ui| {
            ui.label("SX:");
            if ui
                .add(
                    DragValue::new(&mut inspector.scale_x)
                        .speed(0.02)
                        .range(0.01..=100.0),
                )
                .changed()
                && inspector.scale_uniform
            {
                inspector.scale_y = inspector.scale_x;
            }
            ui.label("SY:");
            if ui
                .add(
                    DragValue::new(&mut inspector.scale_y)
                        .speed(0.02)
                        .range(0.01..=100.0),
                )
                .changed()
                && inspector.scale_uniform
            {
                inspector.scale_x = inspector.scale_y;
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Apply"))
                .on_hover_text("Scale selected shape(s)")
                .clicked()
            {
                commands.push(AppCommand::ScaleSelection {
                    pivot: inspector.effective_pivot(),
                    sx: inspector.scale_x,
                    sy: inspector.scale_y,
                });
            }
        });
    });

    ui.add_space(6.0);

    // ── Boolean Operations Section ───────────────────────────────────────────
    ui.group(|ui| {
        ui.label(
            RichText::new("BOOLEAN SHAPING")
                .small()
                .strong()
                .color(c.on_surface_muted),
        );
        let can_boolean = sel_count >= 2;
        let ops = [
            (BooleanOp::Weld, "⊞ Weld (Union)"),
            (BooleanOp::Trim, "⊟ Trim (Difference)"),
            (BooleanOp::Intersect, "⊠ Intersect"),
            (BooleanOp::Exclude, "⊞̷ Exclude (XOR)"),
            (BooleanOp::FrontMinusBack, "⬒ Front - Back"),
            (BooleanOp::BackMinusFront, "⬓ Back - Front"),
            (BooleanOp::CreateBoundary, "⬚ Boundary"),
        ];
        ui.columns(2, |cols| {
            for (idx, (op, label)) in ops.iter().enumerate() {
                let col = &mut cols[idx % 2];
                if col
                    .add_enabled(can_boolean, egui::Button::new(*label))
                    .on_hover_text("Requires 2 or more selected shapes")
                    .clicked()
                {
                    commands.push(AppCommand::BooleanOp(*op));
                }
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(has_selection, egui::Button::new("◌ Simplify"))
                .clicked()
            {
                commands.push(AppCommand::BooleanOp(BooleanOp::Simplify));
            }
            ui.add(
                DragValue::new(&mut inspector.offset_mm)
                    .speed(0.25)
                    .range(-100.0..=100.0)
                    .suffix(" mm"),
            );
            if ui
                .add_enabled(has_selection, egui::Button::new("⤢ Offset"))
                .clicked()
            {
                commands.push(AppCommand::OffsetSelection {
                    distance_mm: inspector.offset_mm,
                    join: inspector.offset_join,
                });
            }
        });
    });

    ui.add_space(6.0);

    // ── Style Section (consumes tokens.color for UI; zero hardcoded Color32) ─
    if sel_ids.len() == 1 {
        if let Some(shape) = doc.find_shape(sel_ids[0]).cloned() {
            ui.group(|ui| {
                ui.label(
                    RichText::new("STYLE")
                        .small()
                        .strong()
                        .color(c.on_surface_muted),
                );
                let mut style = shape.style.clone();
                let mut style_changed = false;

                ui.horizontal(|ui| {
                    let mut has_stroke = style.stroke.is_some();
                    if ui.checkbox(&mut has_stroke, "Stroke").changed() {
                        style.stroke = if has_stroke {
                            Some(Stroke::default())
                        } else {
                            None
                        };
                        style_changed = true;
                    }
                    if let Some(stroke) = &mut style.stroke {
                        let mut egui_col = SemanticColors::from_doc_color(stroke.color);
                        if ui.color_edit_button_srgba(&mut egui_col).changed() {
                            stroke.color = SemanticColors::to_doc_color(egui_col);
                            style_changed = true;
                        }
                        if ui
                            .add(
                                DragValue::new(&mut stroke.width)
                                    .speed(0.05)
                                    .range(0.01..=50.0)
                                    .suffix(" mm"),
                            )
                            .changed()
                        {
                            stroke.hairline = stroke.width <= 0.1;
                            style_changed = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let mut has_fill = style.fill.is_some();
                    if ui.checkbox(&mut has_fill, "Fill").changed() {
                        style.fill = if has_fill {
                            Some(SemanticColors::to_doc_color(c.accent))
                        } else {
                            None
                        };
                        style_changed = true;
                    }
                    if let Some(fill) = &mut style.fill {
                        let mut egui_col = SemanticColors::from_doc_color(*fill);
                        if ui.color_edit_button_srgba(&mut egui_col).changed() {
                            *fill = SemanticColors::to_doc_color(egui_col);
                            style_changed = true;
                        }
                    }
                });

                if style_changed {
                    undo.apply(
                        Box::new(SetStyleCommand::new(shape.id, shape.style.clone(), style)),
                        doc,
                    );
                }
            });
        }
    }

    commands
}
