//! Status bar — bottom bar with coords, zoom, and snap toggles.

use crate::canvas::CanvasState;
use crate::theme;
use core_doc::Document;

/// State tracked by the status bar.
#[derive(Debug, Default, Clone)]
pub struct StatusBarState {
    pub snap_enabled: bool,
    pub grid_snap: bool,
    pub guide_snap: bool,
}

/// Show the bottom status bar.
pub fn show_statusbar(
    ui: &mut egui::Ui,
    canvas: &mut CanvasState,
    status: &mut StatusBarState,
    doc: &Document,
) {
    ui.set_height(theme::STATUSBAR_HEIGHT);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        // Cursor position
        let pos_text = if let Some(p) = canvas.cursor_mm {
            format!("{:>7.2},  {:>7.2} mm", p.x, p.y)
        } else {
            "   —     ,     —     mm".to_string()
        };
        ui.monospace(
            egui::RichText::new(pos_text)
                .small()
                .color(theme::TEXT_SECONDARY),
        );

        ui.separator();

        // Selection info
        let sel_text = match doc.selection.len() {
            0 => "No objects selected".to_string(),
            1 => {
                if let Some(b) = doc.selection_bounds() {
                    format!("1 object ({:.1} × {:.1} mm)", b.width(), b.height())
                } else {
                    "1 object selected".to_string()
                }
            }
            n => format!("{} objects selected", n),
        };
        ui.label(
            egui::RichText::new(sel_text)
                .small()
                .color(theme::TEXT_SECONDARY),
        );

        // Push remaining to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Page info
            if let Some(page) = doc.active_page() {
                ui.label(
                    egui::RichText::new(format!(
                        "{}  {:.0} × {:.0} mm",
                        page.name, page.width_mm, page.height_mm
                    ))
                    .small()
                    .color(theme::TEXT_SECONDARY),
                );
            }

            ui.separator();

            // Snap toggles
            let guide_label = if status.guide_snap {
                egui::RichText::new("⊢ Guide").color(theme::ACCENT)
            } else {
                egui::RichText::new("⊢ Guide").color(theme::TEXT_DISABLED)
            };
            if ui.small_button(guide_label).clicked() {
                status.guide_snap = !status.guide_snap;
            }

            let grid_label = if status.grid_snap {
                egui::RichText::new("⊞ Grid").color(theme::ACCENT)
            } else {
                egui::RichText::new("⊞ Grid").color(theme::TEXT_DISABLED)
            };
            if ui.small_button(grid_label).clicked() {
                status.grid_snap = !status.grid_snap;
                canvas.show_grid = status.grid_snap;
            }

            let snap_label = if status.snap_enabled {
                egui::RichText::new("⊕ Snap").color(theme::ACCENT)
            } else {
                egui::RichText::new("⊕ Snap").color(theme::TEXT_DISABLED)
            };
            if ui.small_button(snap_label).clicked() {
                status.snap_enabled = !status.snap_enabled;
            }

            ui.separator();

            // Zoom control
            let zoom_pct = canvas.zoom * 100.0;
            ui.label(
                egui::RichText::new(format!("{:.0}%", zoom_pct))
                    .small()
                    .color(theme::TEXT_PRIMARY),
            );

            let mut zoom_val = canvas.zoom;
            let slider = egui::Slider::new(&mut zoom_val, 0.01_f32..=16.0)
                .logarithmic(true)
                .show_value(false);
            if ui.add_sized([80.0, 16.0], slider).changed() {
                canvas.zoom = zoom_val;
            }
        });
    });
}
