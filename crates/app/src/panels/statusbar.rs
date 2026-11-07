//! Status bar — bottom bar with coords, selection stats, zoom, and live snap toggles.

use crate::canvas::CanvasState;
use crate::theme;
use core_doc::Document;
use core_snap::SnapSolver;

/// State tracked by the status bar.
#[derive(Debug, Default, Clone)]
pub struct StatusBarState {
    pub snap_enabled: bool,
    pub grid_snap: bool,
    pub guide_snap: bool,
    pub dynamic_snap: bool,
}

/// Show the bottom status bar with real-time geometric coordinate readout and snap switches.
pub fn show_statusbar(
    ui: &mut egui::Ui,
    canvas: &mut CanvasState,
    status: &mut StatusBarState,
    doc: &Document,
    snap_solver: &mut SnapSolver,
) {
    ui.set_height(theme::STATUSBAR_HEIGHT);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        // Cursor position in document coordinates (mm)
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

        // Active snap badge
        if let Some(snap_res) = &canvas.last_snap {
            if snap_res.snapped {
                if let Some(target) = &snap_res.target {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Snap: {}", target.label))
                            .small()
                            .color(theme::SNAP_MARKER),
                    );
                }
            }
        }

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

            // Dynamic Polar Guides toggle (Alt+Shift+D)
            let dyn_label = if snap_solver.config.dynamic_guides.enabled {
                egui::RichText::new("∠ Dynamic (15°)").color(theme::ACCENT)
            } else {
                egui::RichText::new("∠ Dynamic").color(theme::TEXT_DISABLED)
            };
            if ui
                .small_button(dyn_label)
                .on_hover_text("Toggle Polar Angle Dynamic Guides (Alt+Shift+D)")
                .clicked()
            {
                snap_solver.config.dynamic_guides.enabled =
                    !snap_solver.config.dynamic_guides.enabled;
                status.dynamic_snap = snap_solver.config.dynamic_guides.enabled;
            }

            // Guidelines toggle
            let guide_label = if snap_solver.config.snap_to_guidelines {
                egui::RichText::new("⊢ Guides").color(theme::GUIDE_LINE)
            } else {
                egui::RichText::new("⊢ Guides").color(theme::TEXT_DISABLED)
            };
            if ui
                .small_button(guide_label)
                .on_hover_text("Toggle Guideline Snapping")
                .clicked()
            {
                snap_solver.config.snap_to_guidelines = !snap_solver.config.snap_to_guidelines;
                status.guide_snap = snap_solver.config.snap_to_guidelines;
            }

            // Grid toggle
            let grid_label = if snap_solver.config.snap_to_grid {
                egui::RichText::new("⊞ Grid").color(theme::ACCENT)
            } else {
                egui::RichText::new("⊞ Grid").color(theme::TEXT_DISABLED)
            };
            if ui
                .small_button(grid_label)
                .on_hover_text("Toggle Grid & Grid Snapping")
                .clicked()
            {
                snap_solver.config.snap_to_grid = !snap_solver.config.snap_to_grid;
                canvas.show_grid = snap_solver.config.snap_to_grid;
                status.grid_snap = snap_solver.config.snap_to_grid;
            }

            // Master Snap toggle
            let snap_label = if snap_solver.config.enabled {
                egui::RichText::new("⊕ Snapping ON").color(theme::SNAP_MARKER)
            } else {
                egui::RichText::new("⊘ Snapping OFF").color(theme::TEXT_DISABLED)
            };
            if ui
                .small_button(snap_label)
                .on_hover_text("Master Snapping Toggle (%)")
                .clicked()
            {
                snap_solver.config.enabled = !snap_solver.config.enabled;
                status.snap_enabled = snap_solver.config.enabled;
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
