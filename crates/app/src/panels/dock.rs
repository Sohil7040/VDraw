//! Docking system for the right panel area.

use super::boolean_panel::BooleanPanelState;
use super::toast::ToastManager;

/// Which tab is active in the right dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockTab {
    #[default]
    Objects,
    Properties,
    Colors,
    Boolean,
    Align,
    Laser,
}

impl DockTab {
    pub fn label(self) -> &'static str {
        match self {
            DockTab::Objects => "Objects",
            DockTab::Properties => "Properties",
            DockTab::Colors => "Colors",
            DockTab::Boolean => "Shaping",
            DockTab::Align => "Align",
            DockTab::Laser => "Laser",
        }
    }

    pub const ALL: &'static [DockTab] = &[
        DockTab::Objects,
        DockTab::Properties,
        DockTab::Colors,
        DockTab::Boolean,
        DockTab::Align,
        DockTab::Laser,
    ];
}

/// The state of the right-side dock panel system.
#[derive(Debug, Default)]
pub struct DockState {
    pub active_tab: DockTab,
    pub boolean_state: BooleanPanelState,
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
pub fn show_dock_content(
    ui: &mut egui::Ui,
    state: &mut DockState,
    doc: &mut core_doc::Document,
    undo: &mut core_doc::UndoStack,
    toast: &mut ToastManager,
) {
    match state.active_tab {
        DockTab::Objects => {
            super::object_manager::show_object_manager(ui);
        }
        DockTab::Properties => {
            show_properties_panel(ui);
        }
        DockTab::Colors => {
            super::fill_stroke::show_fill_stroke_panel(ui, doc, undo);
        }
        DockTab::Boolean => {
            super::boolean_panel::show_boolean_panel(
                ui,
                &mut state.boolean_state,
                doc,
                undo,
                toast,
            );
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
