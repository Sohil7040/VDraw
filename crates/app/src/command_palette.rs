//! Command palette overlay (Ctrl+K).

/// State for the command palette modal.
#[derive(Debug, Default)]
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
}

/// A single command entry shown in the palette.
struct Command {
    label: &'static str,
    shortcut: &'static str,
}

static COMMANDS: &[Command] = &[
    Command {
        label: "New Document",
        shortcut: "Ctrl+N",
    },
    Command {
        label: "Open…",
        shortcut: "Ctrl+O",
    },
    Command {
        label: "Save",
        shortcut: "Ctrl+S",
    },
    Command {
        label: "Save As…",
        shortcut: "Ctrl+Shift+S",
    },
    Command {
        label: "Export PNG…",
        shortcut: "Ctrl+Shift+E",
    },
    Command {
        label: "Zoom In",
        shortcut: "+",
    },
    Command {
        label: "Zoom Out",
        shortcut: "-",
    },
    Command {
        label: "Fit Page",
        shortcut: "0",
    },
    Command {
        label: "Toggle Grid",
        shortcut: "#",
    },
    Command {
        label: "Toggle Rulers",
        shortcut: "R",
    },
    Command {
        label: "Toggle Snap",
        shortcut: "%",
    },
    Command {
        label: "Pick Tool",
        shortcut: "V",
    },
    Command {
        label: "Pen Tool",
        shortcut: "B",
    },
    Command {
        label: "Rectangle Tool",
        shortcut: "F6",
    },
    Command {
        label: "Ellipse Tool",
        shortcut: "F7",
    },
    Command {
        label: "Text Tool",
        shortcut: "T",
    },
];

impl CommandPalette {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the palette if it is open. Returns `true` if a command was selected.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.open {
            return false;
        }

        // Check for Escape key to close.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
            return false;
        }

        let mut selected = false;
        let mut keep_open = true;

        // Modal area: dim background.
        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("command_palette_backdrop"))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                let r = ui.allocate_rect(screen_rect, egui::Sense::click());
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(140));
                if r.clicked() {
                    keep_open = false;
                }
            });

        // Palette window centred on screen.
        let palette_w = 480.0_f32;
        let palette_h = 360.0_f32;
        let pos = egui::Pos2::new(
            screen_rect.center().x - palette_w * 0.5,
            screen_rect.center().y - palette_h * 0.5,
        );

        egui::Window::new("Command Palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size([palette_w, palette_h])
            .fixed_pos(pos)
            .frame(
                egui::Frame::window(&ctx.style())
                    .inner_margin(egui::Margin::same(12))
                    .corner_radius(egui::CornerRadius::same(10)),
            )
            .show(ctx, |ui| {
                // Search input
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("🔍  Type a command…")
                        .desired_width(f32::INFINITY)
                        .font(egui::FontId::proportional(16.0)),
                )
                .request_focus();

                ui.separator();

                // Filtered command list
                egui::ScrollArea::vertical()
                    .max_height(palette_h - 80.0)
                    .show(ui, |ui| {
                        let q = self.query.to_lowercase();
                        for cmd in COMMANDS {
                            if !q.is_empty() && !cmd.label.to_lowercase().contains(&q) {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                let r = ui.selectable_label(false, cmd.label);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(cmd.shortcut)
                                                .small()
                                                .color(egui::Color32::from_gray(130)),
                                        );
                                    },
                                );
                                if r.clicked() {
                                    // Phase 0: just close on selection.
                                    selected = true;
                                    keep_open = false;
                                }
                            });
                        }
                    });
            });

        if !keep_open {
            self.open = false;
            self.query.clear();
        }

        selected
    }
}
