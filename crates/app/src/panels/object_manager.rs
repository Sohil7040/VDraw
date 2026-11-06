//! Object manager panel (right panel tab).

/// Show the object / layer manager panel content.
pub fn show_object_manager(ui: &mut egui::Ui) {
    ui.heading("Objects");
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("obj_mgr_scroll")
        .show(ui, |ui| {
            // Phase 0: placeholder layer tree
            egui::CollapsingHeader::new("📄 Page 1")
                .default_open(true)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("🔵 Layer 1")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("(empty)")
                                    .italics()
                                    .color(egui::Color32::from_gray(100)),
                            );
                        });
                });
        });
}
