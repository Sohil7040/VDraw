//! Bottom status bar with pure [`StatusBarModel`] render model and dispatch-only snap chips.

use crate::canvas::{hud_style, CanvasState};
use crate::tokens::{SpacingTokens, ThemeTokens};
use core_doc::Document;
use core_geom::Point2;
use core_snap::contracts::{HandleMode, SnapFeedback, SnapKind, SnapMask};
use core_tools::{AppCommand, SnapController, ToolId};

/// State tracked by the bottom status bar.
#[derive(Debug, Default, Clone)]
pub struct StatusBarState {
    pub snap_enabled: bool,
    pub grid_snap: bool,
    pub guide_snap: bool,
    pub dynamic_snap: bool,
    pub active_snap_feedback: Option<SnapFeedback>,
    pub handle_mode: HandleMode,
    pub frame_time_ms: f32,
    pub shape_count: usize,
}

/// Pure view-model entry for a snap-mask or constraint toggle chip.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapChipModel {
    pub label: &'static str,
    pub tooltip: &'static str,
    pub active: bool,
    pub kind: Option<SnapKind>,
    pub is_contextual_hint: bool,
    pub command: AppCommand,
}

/// Pure render model for the bottom status bar.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBarModel {
    pub units_label: &'static str,
    pub active_tool_label: &'static str,
    pub snap_kind_label: Option<&'static str>,
    pub handle_mode_label: &'static str,
    pub snap_master_enabled: bool,
    pub ortho_enabled: bool,
    pub polar_enabled: bool,
    pub cursor_text: String,
    pub selection_text: String,
    pub page_text: String,
    pub zoom_text: String,
    pub stats_text: String,
    pub snap_chips: Vec<SnapChipModel>,
}

impl StatusBarModel {
    /// Derives a pure [`StatusBarModel`] from live document, snap controller, and status state.
    pub fn from_state(
        status: &StatusBarState,
        doc: &Document,
        snap_controller: &SnapController,
        active_tool: ToolId,
        zoom: f32,
        cursor_mm: Option<Point2>,
    ) -> Self {
        let units_label = doc.unit.label();
        let cursor_text = match cursor_mm {
            Some(p) => format!("{:>7.2}, {:>7.2} {units_label}", p.x, p.y),
            None => format!("   —    ,    —    {units_label}"),
        };

        let selection_text = match doc.selection.len() {
            0 => "No selection".to_string(),
            1 => doc
                .selection_bounds()
                .map(|b| {
                    format!(
                        "1 selected ({:.1} × {:.1} {units_label})",
                        b.width(),
                        b.height()
                    )
                })
                .unwrap_or_else(|| "1 selected".to_string()),
            n => format!("{n} selected"),
        };

        let page_text = doc
            .active_page()
            .map(|page| {
                format!(
                    "{} {:.0}×{:.0} {units_label}",
                    page.name, page.width_mm, page.height_mm
                )
            })
            .unwrap_or_else(|| "No page".to_string());

        let effective_handle = status
            .active_snap_feedback
            .as_ref()
            .map(|fb| fb.handle_mode)
            .unwrap_or(status.handle_mode);

        let snap_kind_label = status.active_snap_feedback.as_ref().map(|fb| fb.label);

        let shape_count = doc
            .active_page()
            .map(|p| p.all_shapes().count())
            .unwrap_or(status.shape_count);

        let stats_text = format!("{shape_count} shapes · {:.1} ms", status.frame_time_ms);
        let zoom_text = format!("{:.0}%", zoom * 100.0);

        let mut snap_chips = Vec::with_capacity(SnapKind::ALL.len() + 3);
        snap_chips.push(SnapChipModel {
            label: if snap_controller.is_master_enabled() {
                "Snap ON"
            } else {
                "Snap OFF"
            },
            tooltip: "Toggle Master Snapping (F9)",
            active: snap_controller.is_master_enabled(),
            kind: None,
            is_contextual_hint: false,
            command: AppCommand::ToggleSnapMaster,
        });

        for kind in SnapKind::ALL {
            let style = hud_style(kind);
            snap_chips.push(SnapChipModel {
                label: kind.label(),
                tooltip: kind.label(),
                active: snap_controller.mask.allows(kind),
                kind: Some(kind),
                is_contextual_hint: style.is_contextual_hint,
                command: AppCommand::ToggleSnapKind(kind),
            });
        }

        let ortho_enabled = snap_controller.mask.contains(SnapMask::ORTHO);
        let polar_enabled = snap_controller.mask.contains(SnapMask::POLAR);

        snap_chips.push(SnapChipModel {
            label: "Ortho",
            tooltip: "Toggle Orthogonal Constraint (F8)",
            active: ortho_enabled,
            kind: None,
            is_contextual_hint: false,
            command: AppCommand::ToggleSnapOrtho,
        });
        snap_chips.push(SnapChipModel {
            label: "Polar",
            tooltip: "Toggle Polar Constraint (F11)",
            active: polar_enabled,
            kind: None,
            is_contextual_hint: false,
            command: AppCommand::ToggleSnapPolar,
        });

        Self {
            units_label,
            active_tool_label: active_tool.label(),
            snap_kind_label,
            handle_mode_label: effective_handle.label(),
            snap_master_enabled: snap_controller.is_master_enabled(),
            ortho_enabled,
            polar_enabled,
            cursor_text,
            selection_text,
            page_text,
            zoom_text,
            stats_text,
            snap_chips,
        }
    }

    /// Formats a concise plain-text summary including active snap and handle mode labels.
    pub fn summary_text(&self) -> String {
        match self.snap_kind_label {
            Some(snap) => format!(
                "{} | Tool: {} | Snap: {} | Handle: {} | {}",
                self.units_label,
                self.active_tool_label,
                snap,
                self.handle_mode_label,
                self.stats_text
            ),
            None => format!(
                "{} | Tool: {} | Handle: {} | {}",
                self.units_label, self.active_tool_label, self.handle_mode_label, self.stats_text
            ),
        }
    }
}

/// Renders the bottom status bar and returns any [`AppCommand`] actions triggered by user clicks.
pub fn show_statusbar(
    ui: &mut egui::Ui,
    canvas: &mut CanvasState,
    status: &StatusBarState,
    doc: &Document,
    snap_controller: &SnapController,
    active_tool: ToolId,
) -> Vec<AppCommand> {
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let model = StatusBarModel::from_state(
        status,
        doc,
        snap_controller,
        active_tool,
        canvas.zoom,
        canvas.cursor_mm,
    );
    let mut commands = Vec::new();

    ui.set_height(SpacingTokens::STATUSBAR_HEIGHT);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        ui.monospace(
            egui::RichText::new(&model.cursor_text)
                .small()
                .color(tokens.color.on_surface_muted),
        );
        ui.separator();

        ui.label(
            egui::RichText::new(format!("Units: {}", model.units_label))
                .small()
                .color(tokens.color.on_surface_muted),
        );
        ui.separator();

        ui.label(
            egui::RichText::new(format!("Tool: {}", model.active_tool_label))
                .small()
                .color(tokens.color.on_surface),
        );
        ui.separator();

        ui.label(
            egui::RichText::new(&model.selection_text)
                .small()
                .color(tokens.color.on_surface_muted),
        );
        ui.separator();

        // Live Snap + Handle Mode indicator
        let snap_badge = match model.snap_kind_label {
            Some(kind_lbl) => format!("Snap: {kind_lbl} [Handle: {}]", model.handle_mode_label),
            None => format!("Handle: {}", model.handle_mode_label),
        };
        let badge_color = status
            .active_snap_feedback
            .as_ref()
            .map(|fb| tokens.color.snap_color(fb.target.kind))
            .unwrap_or(tokens.color.on_surface_muted);
        let badge_resp = ui
            .label(
                egui::RichText::new(&snap_badge)
                    .small()
                    .strong()
                    .color(badge_color),
            )
            .on_hover_text("Active snap target category and drag handle mode");
        badge_resp
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &snap_badge));

        // Handle mode cycle button
        let next_handle = match snap_controller.handle_mode() {
            HandleMode::Node => HandleMode::Center,
            HandleMode::Center => HandleMode::Insertion,
            HandleMode::Insertion => HandleMode::Edge,
            HandleMode::Edge => HandleMode::Node,
        };
        let handle_btn = ui
            .small_button(
                egui::RichText::new("⇄ Handle")
                    .small()
                    .color(tokens.color.on_surface_muted),
            )
            .on_hover_text("Cycle source HandleMode (Node / Center / Insertion / Edge)");
        if handle_btn.clicked() {
            commands.push(AppCommand::SetHandleMode(next_handle));
        }

        ui.separator();

        // Snap-mask toggle chips
        for chip in &model.snap_chips {
            let fg = if !chip.active {
                tokens.color.on_surface_disabled
            } else if let Some(kind) = chip.kind {
                tokens.color.snap_color(kind)
            } else {
                tokens.color.accent_hover
            };
            let text = if chip.is_contextual_hint {
                egui::RichText::new(chip.label).small().italics().color(fg)
            } else {
                egui::RichText::new(chip.label).small().color(fg)
            };
            let resp = ui
                .selectable_label(chip.active, text)
                .on_hover_text(chip.tooltip);
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Toggle {} snap", chip.label),
                )
            });
            if resp.clicked() {
                commands.push(chip.command.clone());
            }
        }

        // Right-aligned stats, page, zoom
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(&model.zoom_text)
                    .small()
                    .color(tokens.color.on_surface),
            );
            let mut zoom_val = canvas.zoom;
            let slider = egui::Slider::new(&mut zoom_val, 0.01_f32..=16.0)
                .logarithmic(true)
                .show_value(false);
            if ui.add_sized([68.0, 16.0], slider).changed() {
                canvas.zoom = zoom_val;
            }
            ui.separator();
            ui.label(
                egui::RichText::new(&model.stats_text)
                    .small()
                    .color(tokens.color.on_surface_muted),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(&model.page_text)
                    .small()
                    .color(tokens.color.on_surface_muted),
            );
        });
    });

    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_doc::ShapeId;
    use core_geom::Point2;
    use core_snap::contracts::{SnapFeedback, SnapKind, SnapPoint, DEFAULT_SNAP_MASK};

    #[test]
    fn test_statusbar_reflects_live_snap_and_handle_mode() {
        let doc = Document::new("StatusTest");
        let snap_controller = SnapController::new(DEFAULT_SNAP_MASK);
        let state = StatusBarState {
            active_snap_feedback: Some(SnapFeedback {
                target: SnapPoint::new(Point2::new(10.0, 20.0), SnapKind::Endpoint, ShapeId(1)),
                screen_pos: egui::pos2(100.0, 120.0),
                glow_radius_px: 10.0,
                label: SnapKind::Endpoint.label(),
                handle_mode: HandleMode::Center,
            }),
            handle_mode: HandleMode::Center,
            ..StatusBarState::default()
        };

        let model = StatusBarModel::from_state(
            &state,
            &doc,
            &snap_controller,
            ToolId::Pick,
            1.0,
            Some(Point2::new(10.0, 20.0)),
        );

        assert_eq!(model.snap_kind_label, Some("Endpoint"));
        assert_eq!(model.handle_mode_label, "Center");
        assert!(model.summary_text().contains("Endpoint"));
        assert!(model.summary_text().contains("Center"));
    }
}
