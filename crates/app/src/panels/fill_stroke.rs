//! Fill and Stroke inspector panel for VDraw.

use crate::tokens::{SemanticColors, ThemeTokens};
use core_doc::{Color, Document, LineCap, LineJoin, SetStyleCommand, Stroke, UndoStack};

/// Palette of curated swatches for vector design and laser cutting.
const SWATCHES: &[(&str, Color)] = &[
    ("Laser Cut Red", Color::rgb(255, 0, 0)),
    ("Laser Score Blue", Color::rgb(0, 0, 255)),
    ("Black", Color::rgb(0, 0, 0)),
    ("Charcoal", Color::rgb(38, 42, 51)),
    ("White", Color::rgb(255, 255, 255)),
    ("Electric Violet", Color::rgb(124, 92, 255)),
    ("Teal Guide", Color::rgb(32, 211, 181)),
    ("Safety Orange", Color::rgb(255, 120, 0)),
    ("Yellow", Color::rgb(255, 210, 0)),
    ("Emerald Green", Color::rgb(0, 200, 100)),
];

pub fn show_fill_stroke_panel(ui: &mut egui::Ui, doc: &mut Document, undo: &mut UndoStack) {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let c = &tokens.color;

    ui.heading("Fill & Stroke");
    ui.separator();

    let selected_ids: Vec<_> = doc.selection.ids.iter().copied().collect();

    if selected_ids.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "No shapes selected.\nSelect shapes on the canvas to inspect and edit fill & stroke.",
            )
            .color(c.on_surface_muted)
            .italics(),
        );
        return;
    }

    let primary_id = selected_ids[0];
    let current_style = match doc.find_shape(primary_id) {
        Some(s) => s.style.clone(),
        None => return,
    };

    let mut new_style = current_style.clone();
    let mut style_modified = false;

    // ── 1. Fill Section ──────────────────────────────────────────────────
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Fill");
            let mut has_fill = new_style.fill.is_some();
            if ui.checkbox(&mut has_fill, "Enabled").changed() {
                if has_fill {
                    new_style.fill = Some(SemanticColors::to_doc_color(c.accent));
                } else {
                    new_style.fill = None;
                }
                style_modified = true;
            }
        });

        if let Some(fill_color) = &mut new_style.fill {
            ui.add_space(4.0);
            let mut egui_col = SemanticColors::from_doc_color(*fill_color);
            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_srgba(&mut egui_col).changed() {
                    *fill_color = SemanticColors::to_doc_color(egui_col);
                    style_modified = true;
                }
                ui.monospace(fill_color.to_hex());
            });

            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                for &(name, swatch_color) in SWATCHES {
                    let mut swatch_egui = SemanticColors::from_doc_color(swatch_color);
                    if ui
                        .color_edit_button_srgba(&mut swatch_egui)
                        .on_hover_text(name)
                        .clicked()
                    {
                        *fill_color = swatch_color;
                        style_modified = true;
                    }
                }
            });
        }
    });

    ui.add_space(8.0);

    // ── 2. Stroke Section ────────────────────────────────────────────────
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Stroke");
            let mut has_stroke = new_style.stroke.is_some();
            if ui.checkbox(&mut has_stroke, "Enabled").changed() {
                if has_stroke {
                    new_style.stroke = Some(Stroke::default());
                } else {
                    new_style.stroke = None;
                }
                style_modified = true;
            }
        });

        if let Some(stroke) = &mut new_style.stroke {
            ui.add_space(4.0);
            let mut egui_col = SemanticColors::from_doc_color(stroke.color);
            ui.horizontal(|ui| {
                ui.label("Color:");
                if ui.color_edit_button_srgba(&mut egui_col).changed() {
                    stroke.color = SemanticColors::to_doc_color(egui_col);
                    style_modified = true;
                }
                ui.monospace(stroke.color.to_hex());
            });

            ui.horizontal_wrapped(|ui| {
                for &(name, swatch_color) in SWATCHES {
                    let mut swatch_egui = SemanticColors::from_doc_color(swatch_color);
                    if ui
                        .color_edit_button_srgba(&mut swatch_egui)
                        .on_hover_text(name)
                        .clicked()
                    {
                        stroke.color = swatch_color;
                        style_modified = true;
                    }
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Width:");
                if ui
                    .add(
                        egui::DragValue::new(&mut stroke.width)
                            .speed(0.02)
                            .range(0.01..=50.0)
                            .suffix(" mm"),
                    )
                    .changed()
                {
                    stroke.hairline = stroke.width <= 0.1;
                    style_modified = true;
                }
            });

            ui.add_space(4.0);
            if stroke.width <= 0.1 || stroke.hairline {
                if stroke.color.r > 200 && stroke.color.b < 80 {
                    ui.label(
                        egui::RichText::new("✂ Laser Cut (Hairline <= 0.1mm)")
                            .color(c.laser_cut)
                            .strong(),
                    );
                } else if stroke.color.b > 200 && stroke.color.r < 80 {
                    ui.label(
                        egui::RichText::new("✏ Laser Score (Hairline <= 0.1mm)")
                            .color(c.laser_score)
                            .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("⚡ Hairline Stroke")
                            .color(c.accent)
                            .small(),
                    );
                }
            }

            ui.horizontal(|ui| {
                if ui.button("✂ Laser Cut (0.05mm Red)").clicked() {
                    stroke.width = 0.05;
                    stroke.hairline = true;
                    stroke.color = Color::rgb(255, 0, 0);
                    style_modified = true;
                }
                if ui.button("✏ Laser Score (0.05mm Blue)").clicked() {
                    stroke.width = 0.05;
                    stroke.hairline = true;
                    stroke.color = Color::rgb(0, 0, 255);
                    style_modified = true;
                }
            });

            ui.horizontal(|ui| {
                for &w in &[0.2, 0.5, 1.0, 2.0] {
                    let text = format!("{w:.1}mm");
                    if ui
                        .selectable_label((stroke.width - w).abs() < 1e-4, text)
                        .clicked()
                    {
                        stroke.width = w;
                        stroke.hairline = w <= 0.1;
                        style_modified = true;
                    }
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Cap:");
                if ui
                    .selectable_label(stroke.cap == LineCap::Butt, "Butt")
                    .clicked()
                {
                    stroke.cap = LineCap::Butt;
                    style_modified = true;
                }
                if ui
                    .selectable_label(stroke.cap == LineCap::Round, "Round")
                    .clicked()
                {
                    stroke.cap = LineCap::Round;
                    style_modified = true;
                }
                if ui
                    .selectable_label(stroke.cap == LineCap::Square, "Square")
                    .clicked()
                {
                    stroke.cap = LineCap::Square;
                    style_modified = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Join:");
                if ui
                    .selectable_label(stroke.join == LineJoin::Miter, "Miter")
                    .clicked()
                {
                    stroke.join = LineJoin::Miter;
                    style_modified = true;
                }
                if ui
                    .selectable_label(stroke.join == LineJoin::Round, "Round")
                    .clicked()
                {
                    stroke.join = LineJoin::Round;
                    style_modified = true;
                }
                if ui
                    .selectable_label(stroke.join == LineJoin::Bevel, "Bevel")
                    .clicked()
                {
                    stroke.join = LineJoin::Bevel;
                    style_modified = true;
                }
            });
        }
    });

    if style_modified {
        for &id in &selected_ids {
            let old_s = match doc.find_shape(id) {
                Some(s) => s.style.clone(),
                None => continue,
            };
            let cmd = Box::new(SetStyleCommand::new(id, old_s, new_style.clone()));
            undo.apply(cmd, doc);
        }
    }
}
