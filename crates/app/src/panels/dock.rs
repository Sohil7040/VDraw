//! Docking system for the right panel area.

/// Which tab is active in the right dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockTab {
    #[default]
    Objects,
    Properties,
    Colors,
    Align,
    Laser,
}

impl DockTab {
    pub fn label(self) -> &'static str {
        match self {
            DockTab::Objects => "Objects",
            DockTab::Properties => "Properties",
            DockTab::Colors => "Colors",
            DockTab::Align => "Align",
            DockTab::Laser => "Laser",
        }
    }

    pub const ALL: &'static [DockTab] = &[
        DockTab::Objects,
        DockTab::Properties,
        DockTab::Colors,
        DockTab::Align,
        DockTab::Laser,
    ];
}

/// The state of the right-side dock panel system.
#[derive(Debug, Default, Clone)]
pub struct DockState {
    pub active_tab: DockTab,
}

/// Show the dock tab bar and return the active tab.
pub fn show_dock_tabs(ui: &mut egui::Ui, state: &mut DockState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for &tab in DockTab::ALL {
            let is_active = state.active_tab == tab;
            let text = egui::RichText::new(tab.label()).small();
            let text = if is_active {
                text.color(crate::theme::ACCENT)
            } else {
                text.color(crate::theme::TEXT_SECONDARY)
            };

            if ui.selectable_label(is_active, text).clicked() {
                state.active_tab = tab;
            }
        }
    });
    ui.separator();
}

/// Show the content area for the currently active dock tab.
pub fn show_dock_content(ui: &mut egui::Ui, state: &DockState) {
    match state.active_tab {
        DockTab::Objects => {
            super::object_manager::show_object_manager(ui);
        }
        DockTab::Properties => {
            show_properties_panel(ui);
        }
        DockTab::Colors => {
            show_colors_panel(ui);
        }
        DockTab::Align => {
            show_align_panel(ui);
        }
        DockTab::Laser => {
            show_laser_panel(ui);
        }
    }
}

fn show_properties_panel(ui: &mut egui::Ui) {
    ui.heading("Properties");
    ui.separator();
    ui.label(
        egui::RichText::new("No selection")
            .color(egui::Color32::from_gray(110))
            .italics(),
    );
}

fn show_colors_panel(ui: &mut egui::Ui) {
    ui.heading("Colors");
    ui.separator();
    ui.label("Fill:");
    ui.color_edit_button_srgba(&mut egui::Color32::from_rgb(0, 0, 0));
    ui.label("Stroke:");
    ui.color_edit_button_srgba(&mut egui::Color32::from_rgb(0, 0, 0));
}

fn show_align_panel(ui: &mut egui::Ui) {
    ui.heading("Align & Distribute");
    ui.separator();
    ui.label(egui::RichText::new("Phase 0 stub").color(egui::Color32::from_gray(110)));
}

fn show_laser_panel(ui: &mut egui::Ui) {
    ui.heading("Laser Settings");
    ui.separator();
    ui.label(egui::RichText::new("Phase 0 stub").color(egui::Color32::from_gray(110)));
}
