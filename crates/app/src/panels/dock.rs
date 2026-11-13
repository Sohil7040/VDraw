//! Docking system for the right inspector panel area.

use super::boolean_panel::BooleanPanelState;
use super::properties_panel::PropertiesInspectorState;
use super::toast::ToastManager;
use crate::tokens::ThemeTokens;
use core_tools::AppCommand;

/// Which tab is active in the right dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockTab {
    #[default]
    Properties,
    Objects,
    Colors,
    Boolean,
    Align,
    Laser,
}

impl DockTab {
    pub fn label(self) -> &'static str {
        match self {
            DockTab::Properties => "Properties",
            DockTab::Objects => "Objects",
            DockTab::Colors => "Colors",
            DockTab::Boolean => "Shaping",
            DockTab::Align => "Align",
            DockTab::Laser => "Laser",
        }
    }

    pub const ALL: &'static [DockTab] = &[
        DockTab::Properties,
        DockTab::Objects,
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
    pub properties_state: PropertiesInspectorState,
    pub boolean_state: BooleanPanelState,
    pub cam_config: core_cam::LaserProductionConfig,
}

/// Show the dock tab bar.
pub fn show_dock_tabs(ui: &mut egui::Ui, state: &mut DockState) {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for &tab in DockTab::ALL {
            let is_active = state.active_tab == tab;
            let text = egui::RichText::new(tab.label()).small();
            let text = if is_active {
                text.color(tokens.color.accent_hover).strong()
            } else {
                text.color(tokens.color.on_surface_muted)
            };

            let resp = ui.selectable_label(is_active, text);
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("{} Tab", tab.label()),
                )
            });
            if resp.clicked() {
                state.active_tab = tab;
            }
        }
    });
    ui.separator();
}

/// Show the content area for the currently active dock tab, returning any [`AppCommand`]s
/// triggered for dispatch.
pub fn show_dock_content(
    ui: &mut egui::Ui,
    state: &mut DockState,
    doc: &mut core_doc::Document,
    undo: &mut core_doc::UndoStack,
    toast: &mut ToastManager,
) -> Vec<AppCommand> {
    match state.active_tab {
        DockTab::Properties => super::properties_panel::show_properties_panel(
            ui,
            &mut state.properties_state,
            doc,
            undo,
            toast,
        ),
        DockTab::Objects => super::object_manager::show_object_manager(ui, doc, toast),
        DockTab::Colors => {
            super::fill_stroke::show_fill_stroke_panel(ui, doc, undo);
            Vec::new()
        }
        DockTab::Boolean => {
            super::boolean_panel::show_boolean_panel(ui, &mut state.boolean_state, doc)
        }
        DockTab::Align => {
            super::align::show_align_panel(ui, doc, undo, toast);
            Vec::new()
        }
        DockTab::Laser => {
            super::laser_panel::show_laser_panel(ui, doc, undo, &mut state.cam_config, toast)
        }
    }
}
