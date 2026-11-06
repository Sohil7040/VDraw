//! Left toolbar — tool selection palette.

use core_tools::ToolId;

use crate::theme;

/// Persistent state for the toolbar.
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

/// A tool entry shown in the toolbar.
struct ToolEntry {
    id: ToolId,
    icon: &'static str,
}

static SELECTION_GROUP: &[ToolEntry] = &[
    ToolEntry {
        id: ToolId::Pick,
        icon: "↖",
    },
    ToolEntry {
        id: ToolId::FreeTransform,
        icon: "⤡",
    },
];

static DRAWING_GROUP: &[ToolEntry] = &[
    ToolEntry {
        id: ToolId::Pen,
        icon: "✒",
    },
    ToolEntry {
        id: ToolId::Line,
        icon: "╱",
    },
    ToolEntry {
        id: ToolId::Freehand,
        icon: "〜",
    },
    ToolEntry {
        id: ToolId::Rectangle,
        icon: "▭",
    },
    ToolEntry {
        id: ToolId::Ellipse,
        icon: "⬭",
    },
    ToolEntry {
        id: ToolId::Polygon,
        icon: "⬡",
    },
    ToolEntry {
        id: ToolId::Spiral,
        icon: "🌀",
    },
];

static EDITING_GROUP: &[ToolEntry] = &[
    ToolEntry {
        id: ToolId::Eraser,
        icon: "⌫",
    },
    ToolEntry {
        id: ToolId::Knife,
        icon: "⚔",
    },
];

static TEXT_GROUP: &[ToolEntry] = &[ToolEntry {
    id: ToolId::Text,
    icon: "T",
}];

static VIEW_GROUP: &[ToolEntry] = &[
    ToolEntry {
        id: ToolId::Zoom,
        icon: "🔍",
    },
    ToolEntry {
        id: ToolId::Pan,
        icon: "✋",
    },
];

pub fn show_toolbar(ui: &mut egui::Ui, state: &mut ToolbarState) {
    ui.set_width(theme::TOOLBAR_WIDTH);

    egui::ScrollArea::vertical()
        .id_salt("toolbar_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);

            show_group(ui, state, "Selection", SELECTION_GROUP);
            ui.separator();
            show_group(ui, state, "Drawing", DRAWING_GROUP);
            ui.separator();
            show_group(ui, state, "Editing", EDITING_GROUP);
            ui.separator();
            show_group(ui, state, "Text", TEXT_GROUP);
            ui.separator();
            show_group(ui, state, "View", VIEW_GROUP);
        });
}

fn show_group(ui: &mut egui::Ui, state: &mut ToolbarState, _group_name: &str, tools: &[ToolEntry]) {
    for entry in tools {
        let is_active = state.active_tool == entry.id;
        let btn_size = egui::vec2(theme::TOOLBAR_WIDTH - 4.0, theme::TOOLBAR_WIDTH - 4.0);

        let btn_text = egui::RichText::new(entry.icon)
            .size(18.0)
            .color(if is_active {
                theme::ACCENT
            } else {
                theme::TEXT_PRIMARY
            });

        let btn_fill = if is_active {
            theme::BG_ACTIVE
        } else {
            theme::BG_PANEL
        };

        let frame = egui::Frame::NONE
            .fill(btn_fill)
            .corner_radius(egui::CornerRadius::same(theme::CORNER_RADIUS))
            .inner_margin(egui::Margin::same(2));

        let r = frame.show(ui, |ui| {
            ui.set_min_size(btn_size);
            ui.centered_and_justified(|ui| ui.label(btn_text)).inner
        });

        let r = ui.interact(
            r.response.rect,
            ui.id().with(entry.id),
            egui::Sense::click(),
        );

        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let tooltip = format!("{} [{}]", entry.id.label(), entry.id.shortcut());
        let r = r.on_hover_text(tooltip);

        if r.clicked() {
            state.active_tool = entry.id;
        }
    }
}
