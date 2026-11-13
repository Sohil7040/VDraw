//! Left tool palette derived from the [`AppCommand`] catalog (`CommandKind::Tool`).

use crate::command_palette::build_command_catalog;
use crate::tokens::{RadiusTokens, SpacingTokens, ThemeTokens};
use core_tools::{AppCommand, CommandKind, ToolId};

/// Persistent state for the left tool palette.
#[derive(Debug, Clone)]
pub struct ToolbarState {
    pub active_tool: ToolId,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            active_tool: ToolId::Pick,
        }
    }
}

/// Renders the left tool palette by iterating `AppCommand` catalog entries with
/// `CommandKind::Tool`. Returns any clicked [`AppCommand`] for dispatch.
pub fn show_toolbar(ui: &mut egui::Ui, active_tool: ToolId) -> Option<AppCommand> {
    ui.set_width(SpacingTokens::TOOLBAR_WIDTH);
    let tokens = ThemeTokens::from_ctx(ui.ctx());
    let mut triggered: Option<AppCommand> = None;

    let tool_entries: Vec<_> = build_command_catalog()
        .into_iter()
        .filter(|e| e.kind == CommandKind::Tool)
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("toolbar_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, SpacingTokens::XS);
            ui.add_space(SpacingTokens::XS);

            for entry in tool_entries {
                let AppCommand::ActivateTool(tool_id) = entry.command else {
                    continue;
                };
                let is_active = active_tool == tool_id;
                let btn_size = egui::vec2(SpacingTokens::TOOLBAR_WIDTH - 12.0, 28.0);

                let fill = if is_active {
                    tokens.color.surface_active
                } else {
                    tokens.color.surface_elevated
                };
                let stroke = if is_active {
                    egui::Stroke::new(1.5, tokens.color.accent)
                } else {
                    egui::Stroke::new(1.0, tokens.color.transparent)
                };
                let text_col = if is_active {
                    tokens.color.accent_hover
                } else {
                    tokens.color.on_surface
                };

                let label = format!("{}  {}", entry.icon, tool_id.label());
                let btn = egui::Button::new(egui::RichText::new(label).small().color(text_col))
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(egui::CornerRadius::same(RadiusTokens::MD))
                    .min_size(btn_size);

                let tooltip = format!("{} [{}]", tool_id.label(), tool_id.shortcut());
                let resp = ui.add(btn).on_hover_text(&tooltip);
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("{} Tool", tool_id.label()),
                    )
                });

                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                if resp.clicked() {
                    triggered = Some(AppCommand::ActivateTool(tool_id));
                }
            }
        });

    triggered
}
