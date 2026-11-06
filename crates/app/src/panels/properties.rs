//! Top property bar — context-sensitive tool and selection options.

use crate::theme;
use core_doc::Document;

/// Show the property bar at the top of the workspace.
pub fn show_prop_bar(ui: &mut egui::Ui, doc: &Document) {
    ui.set_height(theme::PROPBAR_HEIGHT);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        let sel_count = doc.selection.len();
        if sel_count == 0 {
            ui.label(
                egui::RichText::new("No selection")
                    .color(theme::TEXT_SECONDARY)
                    .italics(),
            );
            ui.separator();
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
            // Display selection info
            let label = if sel_count == 1 {
                "1 object selected".to_string()
            } else {
                format!("{} objects selected", sel_count)
            };
            ui.label(
                egui::RichText::new(label)
                    .color(theme::TEXT_PRIMARY)
                    .strong(),
            );

            if let Some(bounds) = doc.selection_bounds() {
                ui.separator();
                ui.monospace(format!("X: {:>6.2} mm", bounds.min_x));
                ui.monospace(format!("Y: {:>6.2} mm", bounds.min_y));
                ui.separator();
                ui.monospace(format!("W: {:>6.2} mm", bounds.width()));
                ui.monospace(format!("H: {:>6.2} mm", bounds.height()));
            }

            ui.separator();
            if let Some(&first_id) = doc.selection.ids.iter().next() {
                if let Some(shape) = doc.find_shape(first_id) {
                    if let Some(stroke) = &shape.style.stroke {
                        let stroke_desc = if stroke.hairline {
                            format!("Stroke: {:.2} mm (Hairline)", stroke.width)
                        } else {
                            format!("Stroke: {:.2} mm", stroke.width)
                        };
                        ui.label(
                            egui::RichText::new(stroke_desc)
                                .small()
                                .color(theme::TEXT_SECONDARY),
                        );
                    }
                }
            }
        }
    });
}
