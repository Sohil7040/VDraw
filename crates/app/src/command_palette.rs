//! Command palette overlay (`Ctrl+K`) and canonical [`AppCommand`] catalog.

use crate::tokens::ThemeTokens;
use core_boolean::{BooleanOp, ContourJoin};
use core_geom::Point2;
use core_snap::contracts::{HandleMode, SnapKind};
use core_tools::{AppCommand, CommandKind, ThemePref, ToolId};

/// Canonical group ordering for the command palette.
pub const COMMAND_KIND_ORDER: [CommandKind; 7] = [
    CommandKind::Tool,
    CommandKind::Transform,
    CommandKind::Boolean,
    CommandKind::Edit,
    CommandKind::SnapToggle,
    CommandKind::View,
    CommandKind::File,
];

/// Human-readable section heading for each [`CommandKind`].
pub const fn command_kind_label(kind: CommandKind) -> &'static str {
    match kind {
        CommandKind::Tool => "Tools",
        CommandKind::Transform => "Transform",
        CommandKind::Boolean => "Boolean & Path",
        CommandKind::Edit => "Edit & History",
        CommandKind::SnapToggle => "Snapping & Guides",
        CommandKind::View => "View & Appearance",
        CommandKind::File => "File & Export",
    }
}

/// Icon glyph for a [`ToolId`].
pub const fn tool_icon(id: ToolId) -> &'static str {
    match id {
        ToolId::Pick => "↖",
        ToolId::Shape => "☩",
        ToolId::FreeTransform => "⤡",
        ToolId::Pen => "✒",
        ToolId::Line => "╱",
        ToolId::Freehand => "〜",
        ToolId::Rectangle => "▭",
        ToolId::Ellipse => "⬭",
        ToolId::Polygon => "⬡",
        ToolId::Star => "★",
        ToolId::Spiral => "🌀",
        ToolId::Eraser => "⌫",
        ToolId::Knife => "⚔",
        ToolId::VirtualSegmentDelete => "✂",
        ToolId::Text => "T",
        ToolId::Zoom => "🔍",
        ToolId::Pan => "✋",
    }
}

/// A single command entry derived from [`AppCommand`].
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteEntry {
    pub label: String,
    pub shortcut: &'static str,
    pub icon: &'static str,
    pub kind: CommandKind,
    pub command: AppCommand,
}

fn entry(
    label: impl Into<String>,
    shortcut: &'static str,
    icon: &'static str,
    command: AppCommand,
) -> PaletteEntry {
    let kind = command.kind();
    PaletteEntry {
        label: label.into(),
        shortcut,
        icon,
        kind,
        command,
    }
}

/// Builds the complete catalog of user-invocable [`AppCommand`] entries,
/// grouped by [`CommandKind`] and sorted alphabetically within each group.
pub fn build_command_catalog() -> Vec<PaletteEntry> {
    let mut raw = vec![
        // ── File & Export ────────────────────────────────────────────────────
        entry("Export DXF…", "", "📐", AppCommand::ExportDxf),
        entry("Export G-Code…", "", "⚡", AppCommand::ExportGcode),
        entry("Export PNG…", "Ctrl+Shift+E", "🖼", AppCommand::ExportPng),
        entry("Export SVG…", "", "📤", AppCommand::ExportSvg),
        entry("Import SVG…", "", "📥", AppCommand::ImportSvg),
        entry("New Document", "Ctrl+N", "📄", AppCommand::NewDocument),
        entry("Open Document…", "Ctrl+O", "📂", AppCommand::OpenDocument),
        entry("Save Document", "Ctrl+S", "💾", AppCommand::SaveDocument),
        entry(
            "Save Document As…",
            "Ctrl+Shift+S",
            "💾",
            AppCommand::SaveDocumentAs,
        ),
        // ── Edit & Undo History ──────────────────────────────────────────────
        entry("Break Apart Paths", "", "✂", AppCommand::BreakApartPaths),
        entry("Combine Paths", "", "🔗", AppCommand::CombinePaths),
        entry(
            "Convert Text to Path",
            "",
            "T",
            AppCommand::ConvertTextToPath {
                text: "VDraw".to_string(),
                size_mm: 16.0,
            },
        ),
        entry("Delete Selection", "Del", "🗑", AppCommand::DeleteSelection),
        entry("Deselect All", "Esc", "⊘", AppCommand::DeselectAll),
        entry(
            "Duplicate Selection",
            "Ctrl+D",
            "⧉",
            AppCommand::DuplicateSelection,
        ),
        entry("Group Selection", "Ctrl+G", "🗂", AppCommand::GroupSelection),
        entry(
            "History: Jump to Start",
            "",
            "⏮",
            AppCommand::JumpToHistoryIndex(0),
        ),
        entry("Redo", "Ctrl+Y", "↷", AppCommand::Redo),
        entry("Select All", "Ctrl+A", "☑", AppCommand::SelectAll),
        entry("Undo", "Ctrl+Z", "↶", AppCommand::Undo),
        entry(
            "Ungroup Selection",
            "Ctrl+Shift+G",
            "📂",
            AppCommand::UngroupSelection,
        ),
        // ── Tools (every ToolId variant) ─────────────────────────────────────
        entry(
            format!("Tool: {}", ToolId::Pick.label()),
            ToolId::Pick.shortcut(),
            tool_icon(ToolId::Pick),
            AppCommand::ActivateTool(ToolId::Pick),
        ),
        entry(
            format!("Tool: {}", ToolId::Shape.label()),
            ToolId::Shape.shortcut(),
            tool_icon(ToolId::Shape),
            AppCommand::ActivateTool(ToolId::Shape),
        ),
        entry(
            format!("Tool: {}", ToolId::FreeTransform.label()),
            ToolId::FreeTransform.shortcut(),
            tool_icon(ToolId::FreeTransform),
            AppCommand::ActivateTool(ToolId::FreeTransform),
        ),
        entry(
            format!("Tool: {}", ToolId::Pen.label()),
            ToolId::Pen.shortcut(),
            tool_icon(ToolId::Pen),
            AppCommand::ActivateTool(ToolId::Pen),
        ),
        entry(
            format!("Tool: {}", ToolId::Line.label()),
            ToolId::Line.shortcut(),
            tool_icon(ToolId::Line),
            AppCommand::ActivateTool(ToolId::Line),
        ),
        entry(
            format!("Tool: {}", ToolId::Freehand.label()),
            ToolId::Freehand.shortcut(),
            tool_icon(ToolId::Freehand),
            AppCommand::ActivateTool(ToolId::Freehand),
        ),
        entry(
            format!("Tool: {}", ToolId::Rectangle.label()),
            ToolId::Rectangle.shortcut(),
            tool_icon(ToolId::Rectangle),
            AppCommand::ActivateTool(ToolId::Rectangle),
        ),
        entry(
            format!("Tool: {}", ToolId::Ellipse.label()),
            ToolId::Ellipse.shortcut(),
            tool_icon(ToolId::Ellipse),
            AppCommand::ActivateTool(ToolId::Ellipse),
        ),
        entry(
            format!("Tool: {}", ToolId::Polygon.label()),
            ToolId::Polygon.shortcut(),
            tool_icon(ToolId::Polygon),
            AppCommand::ActivateTool(ToolId::Polygon),
        ),
        entry(
            format!("Tool: {}", ToolId::Star.label()),
            ToolId::Star.shortcut(),
            tool_icon(ToolId::Star),
            AppCommand::ActivateTool(ToolId::Star),
        ),
        entry(
            format!("Tool: {}", ToolId::Spiral.label()),
            ToolId::Spiral.shortcut(),
            tool_icon(ToolId::Spiral),
            AppCommand::ActivateTool(ToolId::Spiral),
        ),
        entry(
            format!("Tool: {}", ToolId::Eraser.label()),
            ToolId::Eraser.shortcut(),
            tool_icon(ToolId::Eraser),
            AppCommand::ActivateTool(ToolId::Eraser),
        ),
        entry(
            format!("Tool: {}", ToolId::Knife.label()),
            ToolId::Knife.shortcut(),
            tool_icon(ToolId::Knife),
            AppCommand::ActivateTool(ToolId::Knife),
        ),
        entry(
            format!("Tool: {}", ToolId::VirtualSegmentDelete.label()),
            ToolId::VirtualSegmentDelete.shortcut(),
            tool_icon(ToolId::VirtualSegmentDelete),
            AppCommand::ActivateTool(ToolId::VirtualSegmentDelete),
        ),
        entry(
            format!("Tool: {}", ToolId::Text.label()),
            ToolId::Text.shortcut(),
            tool_icon(ToolId::Text),
            AppCommand::ActivateTool(ToolId::Text),
        ),
        entry(
            format!("Tool: {}", ToolId::Zoom.label()),
            ToolId::Zoom.shortcut(),
            tool_icon(ToolId::Zoom),
            AppCommand::ActivateTool(ToolId::Zoom),
        ),
        entry(
            format!("Tool: {}", ToolId::Pan.label()),
            ToolId::Pan.shortcut(),
            tool_icon(ToolId::Pan),
            AppCommand::ActivateTool(ToolId::Pan),
        ),
        // ── Transforms ───────────────────────────────────────────────────────
        entry(
            "Mirror Selection Across Line",
            "",
            "⧎",
            AppCommand::MirrorSelectionLine {
                point: Point2::ZERO,
                dir: Point2::new(1.0, 1.0),
            },
        ),
        entry(
            "Mirror Selection Horizontal",
            "",
            "⇄",
            AppCommand::MirrorSelectionHorizontal,
        ),
        entry(
            "Mirror Selection Vertical",
            "",
            "⇅",
            AppCommand::MirrorSelectionVertical,
        ),
        entry(
            "Rotate Selection 90°",
            "",
            "↻",
            AppCommand::RotateSelection {
                pivot: None,
                angle_deg: 90.0,
            },
        ),
        entry(
            "Scale Selection 150%",
            "",
            "⤢",
            AppCommand::ScaleSelection {
                pivot: None,
                sx: 1.5,
                sy: 1.5,
            },
        ),
        entry(
            "Translate Selection (+10, 0 mm)",
            "",
            "↦",
            AppCommand::TranslateSelection {
                delta: Point2::new(10.0, 0.0),
            },
        ),
        // ── Boolean & Path Operations ────────────────────────────────────────
        entry(
            "Boolean: Back Minus Front",
            "",
            "⬓",
            AppCommand::BooleanOp(BooleanOp::BackMinusFront),
        ),
        entry(
            "Boolean: Create Boundary",
            "",
            "⬚",
            AppCommand::BooleanOp(BooleanOp::CreateBoundary),
        ),
        entry(
            "Boolean: Exclude (XOR)",
            "",
            "⊞",
            AppCommand::BooleanOp(BooleanOp::Exclude),
        ),
        entry(
            "Boolean: Front Minus Back",
            "",
            "⬒",
            AppCommand::BooleanOp(BooleanOp::FrontMinusBack),
        ),
        entry(
            "Boolean: Intersect",
            "",
            "⊠",
            AppCommand::BooleanOp(BooleanOp::Intersect),
        ),
        entry(
            "Boolean: Simplify",
            "",
            "◌",
            AppCommand::BooleanOp(BooleanOp::Simplify),
        ),
        entry(
            "Boolean: Trim (Difference)",
            "",
            "⊟",
            AppCommand::BooleanOp(BooleanOp::Trim),
        ),
        entry(
            "Boolean: Weld (Union)",
            "",
            "⊞",
            AppCommand::BooleanOp(BooleanOp::Weld),
        ),
        entry(
            "Offset Contour (+3.0 mm)",
            "",
            "⤢",
            AppCommand::OffsetSelection {
                distance_mm: 3.0,
                join: ContourJoin::Round,
            },
        ),
        // ── Snap System & Constraints ────────────────────────────────────────
        entry(
            "Clear All Guidelines",
            "",
            "🧹",
            AppCommand::ClearGuidelines,
        ),
        entry(
            "Handle Mode: Center",
            "",
            "⊕",
            AppCommand::SetHandleMode(HandleMode::Center),
        ),
        entry(
            "Handle Mode: Edge",
            "",
            "∥",
            AppCommand::SetHandleMode(HandleMode::Edge),
        ),
        entry(
            "Handle Mode: Insertion",
            "",
            "⌖",
            AppCommand::SetHandleMode(HandleMode::Insertion),
        ),
        entry(
            "Handle Mode: Node",
            "",
            "■",
            AppCommand::SetHandleMode(HandleMode::Node),
        ),
        entry(
            "Set Polar Step 15°",
            "",
            "∠",
            AppCommand::SetPolarStepDeg(15.0),
        ),
        entry(
            "Toggle Alignment Guides",
            "Alt+Shift+D",
            "⊢",
            AppCommand::ToggleAlignmentGuides,
        ),
        entry(
            "Toggle Guideline Snap",
            "",
            "📏",
            AppCommand::ToggleGuidelineSnap,
        ),
        entry(
            "Toggle Snap Master",
            "F9",
            "🧲",
            AppCommand::ToggleSnapMaster,
        ),
        entry("Toggle Snap Ortho", "F8", "⊥", AppCommand::ToggleSnapOrtho),
        entry("Toggle Snap Polar", "F11", "∠", AppCommand::ToggleSnapPolar),
        entry(
            "Toggle Snap: Center",
            "",
            "⊕",
            AppCommand::ToggleSnapKind(SnapKind::Center),
        ),
        entry(
            "Toggle Snap: Endpoint",
            "",
            "▫",
            AppCommand::ToggleSnapKind(SnapKind::Endpoint),
        ),
        entry(
            "Toggle Snap: Grid",
            "",
            "#",
            AppCommand::ToggleSnapKind(SnapKind::Grid),
        ),
        entry(
            "Toggle Snap: Intersection",
            "",
            "✕",
            AppCommand::ToggleSnapKind(SnapKind::Intersection),
        ),
        entry(
            "Toggle Snap: Midpoint",
            "",
            "◆",
            AppCommand::ToggleSnapKind(SnapKind::Midpoint),
        ),
        entry(
            "Toggle Snap: Node",
            "",
            "■",
            AppCommand::ToggleSnapKind(SnapKind::Node),
        ),
        entry(
            "Toggle Snap: Perpendicular",
            "",
            "⊥",
            AppCommand::ToggleSnapKind(SnapKind::Perpendicular),
        ),
        entry(
            "Toggle Snap: Quadrant",
            "",
            "◇",
            AppCommand::ToggleSnapKind(SnapKind::Quadrant),
        ),
        entry(
            "Toggle Snap: Tangent",
            "",
            "⊙",
            AppCommand::ToggleSnapKind(SnapKind::Tangent),
        ),
        // ── Viewport, Theme & Help ───────────────────────────────────────────
        entry("Fit Page", "0", "⊡", AppCommand::FitPage),
        entry(
            "Open Command Palette",
            "Ctrl+K",
            "⌘",
            AppCommand::OpenCommandPalette,
        ),
        entry("Open Help", "F1", "❓", AppCommand::OpenHelp),
        entry("Start Guided Tour", "", "✦", AppCommand::StartCoachMarks),
        entry(
            "Theme: Dark",
            "",
            "🌙",
            AppCommand::SetTheme(ThemePref(egui::ThemePreference::Dark)),
        ),
        entry(
            "Theme: Light",
            "",
            "☀",
            AppCommand::SetTheme(ThemePref(egui::ThemePreference::Light)),
        ),
        entry(
            "Theme: System",
            "",
            "🖥",
            AppCommand::SetTheme(ThemePref(egui::ThemePreference::System)),
        ),
        entry("Toggle Grid", "#", "⊞", AppCommand::ToggleGrid),
        entry("Toggle Rulers", "R", "📏", AppCommand::ToggleRulers),
        entry(
            "Workflow: Design",
            "",
            "✳",
            AppCommand::SwitchWorkflowDesign,
        ),
        entry(
            "Workflow: Laser CAM",
            "",
            "▤",
            AppCommand::SwitchWorkflowLaserCam,
        ),
        entry(
            "Workflow: Toolpath Preview",
            "",
            "▷",
            AppCommand::SwitchWorkflowPreview,
        ),
        entry("Zoom In", "+", "➕", AppCommand::ZoomIn),
        entry("Zoom Out", "-", "➖", AppCommand::ZoomOut),
        entry("Zoom to Selection", "", "🔍", AppCommand::ZoomToSelection),
    ];

    let mut grouped = Vec::with_capacity(raw.len());
    for kind in COMMAND_KIND_ORDER {
        let mut group: Vec<PaletteEntry> = raw.iter().filter(|e| e.kind == kind).cloned().collect();
        group.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        grouped.extend(group);
    }
    // Keep unused mutable warning clean while guaranteeing deterministic ordering.
    raw.clear();
    grouped
}

/// State for the command palette modal (`Ctrl/Cmd+K`).
#[derive(Debug, Default)]
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    just_opened: bool,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the command palette and requests focus for the search input on next frame.
    pub fn open_palette(&mut self) {
        self.open = true;
        self.selected_index = 0;
        self.just_opened = true;
    }

    /// Closes the command palette and clears search query.
    pub fn close_palette(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
        self.just_opened = false;
    }

    /// Returns all entries in canonical grouped + alphabetically sorted order.
    pub fn all_entries() -> Vec<PaletteEntry> {
        build_command_catalog()
    }

    /// Filters catalog entries by the current search query, preserving `CommandKind` grouping
    /// and alphabetical sorting within each group.
    pub fn filtered_entries(&self) -> Vec<PaletteEntry> {
        let catalog = build_command_catalog();
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            return catalog;
        }
        catalog
            .into_iter()
            .filter(|e| {
                e.label.to_lowercase().contains(&q)
                    || command_kind_label(e.kind).to_lowercase().contains(&q)
                    || e.shortcut.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Renders the palette overlay when open. Returns `Some(AppCommand)` if the user
    /// activates a command via `Enter` or click.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<AppCommand> {
        if !self.open {
            return None;
        }

        let tokens = ThemeTokens::from_ctx(ctx);
        let filtered = self.filtered_entries();
        if filtered.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= filtered.len() {
            self.selected_index = filtered.len() - 1;
        }

        let (esc_pressed, down_pressed, up_pressed, enter_pressed) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
            )
        });

        if esc_pressed {
            self.close_palette();
            return None;
        }

        if !filtered.is_empty() {
            if down_pressed {
                self.selected_index = (self.selected_index + 1) % filtered.len();
            }
            if up_pressed {
                self.selected_index = if self.selected_index == 0 {
                    filtered.len() - 1
                } else {
                    self.selected_index - 1
                };
            }
            if enter_pressed {
                let cmd = filtered.get(self.selected_index).map(|e| e.command.clone());
                self.close_palette();
                return cmd;
            }
        }

        let mut chosen: Option<AppCommand> = None;
        let mut keep_open = true;

        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("command_palette_backdrop"))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let r = ui.allocate_rect(screen_rect, egui::Sense::click());
                ui.painter()
                    .rect_filled(screen_rect, 0.0, tokens.color.backdrop_scrim);
                if r.clicked() {
                    keep_open = false;
                }
            });

        let palette_w = 520.0_f32.min(screen_rect.width() - 32.0).max(240.0);
        let palette_h = 400.0_f32.min(screen_rect.height() - 64.0).max(200.0);
        let pos = egui::Pos2::new(
            screen_rect.center().x - palette_w * 0.5,
            screen_rect.center().y - palette_h * 0.5,
        );

        egui::Window::new("Command Palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .order(egui::Order::Foreground)
            .fixed_size([palette_w, palette_h])
            .fixed_pos(pos)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(tokens.color.surface_elevated)
                    .stroke(egui::Stroke::new(1.0, tokens.color.border_focus))
                    .inner_margin(egui::Margin::same(12))
                    .corner_radius(egui::CornerRadius::same(crate::tokens::RadiusTokens::LG)),
            )
            .show(ctx, |ui| {
                let search_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type a command (tool, transform, boolean, snap, theme)…")
                        .desired_width(f32::INFINITY)
                        .font(egui::FontId::proportional(15.0)),
                );
                search_resp.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::TextEdit,
                        true,
                        "Command Palette Search",
                    )
                });
                if self.just_opened {
                    search_resp.request_focus();
                    self.just_opened = false;
                }
                if search_resp.changed() {
                    self.selected_index = 0;
                }

                ui.add_space(4.0);
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("command_palette_results")
                    .max_height(palette_h - 76.0)
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("No matching commands")
                                    .color(tokens.color.on_surface_muted)
                                    .italics(),
                            );
                            return;
                        }

                        let mut flat_idx = 0usize;
                        for kind in COMMAND_KIND_ORDER {
                            let group: Vec<(usize, &PaletteEntry)> = filtered
                                .iter()
                                .enumerate()
                                .filter(|(_, e)| e.kind == kind)
                                .collect();
                            if group.is_empty() {
                                continue;
                            }

                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(command_kind_label(kind).to_uppercase())
                                    .small()
                                    .strong()
                                    .color(tokens.color.on_surface_muted),
                            );

                            for (idx, item) in group {
                                let is_selected = idx == self.selected_index;
                                let label_text = format!("{}  {}", item.icon, item.label);
                                let rich = if is_selected {
                                    egui::RichText::new(&label_text)
                                        .color(tokens.color.accent_hover)
                                        .strong()
                                } else {
                                    egui::RichText::new(&label_text).color(tokens.color.on_surface)
                                };

                                ui.horizontal(|ui| {
                                    let r = ui
                                        .selectable_label(is_selected, rich)
                                        .on_hover_text(command_kind_label(item.kind));
                                    r.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            &item.label,
                                        )
                                    });
                                    if is_selected && (down_pressed || up_pressed) {
                                        r.scroll_to_me(Some(egui::Align::Center));
                                    }

                                    if !item.shortcut.is_empty() {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(item.shortcut)
                                                        .small()
                                                        .monospace()
                                                        .color(tokens.color.on_surface_muted),
                                                );
                                            },
                                        );
                                    }

                                    if r.clicked() {
                                        chosen = Some(item.command.clone());
                                        keep_open = false;
                                    }
                                });
                                flat_idx += 1;
                            }
                        }
                        let _ = flat_idx;
                    });
            });

        if !keep_open {
            self.close_palette();
        }

        chosen
    }
}
