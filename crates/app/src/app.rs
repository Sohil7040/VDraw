//! VDrawApp — the top-level eframe application.

use core_doc::{Document, RemoveShapesCommand, UndoStack};
use core_snap::SnapSolver;
use core_tools::{ToolId, ToolManager};
use egui::ViewportCommand;

use crate::{
    canvas::{self, CanvasState},
    command_palette::CommandPalette,
    panels::{
        dock::{self, DockState},
        statusbar::StatusBarState,
        toolbar::ToolbarState,
    },
    theme,
    workspace::{WorkspacePreset, WorkspaceState},
};

/// The root application state.
pub struct VDrawApp {
    doc: Document,
    undo_stack: UndoStack,
    tool_manager: ToolManager,
    snap_solver: SnapSolver,
    canvas: CanvasState,
    toolbar: ToolbarState,
    dock: DockState,
    workspace: WorkspaceState,
    statusbar: StatusBarState,
    palette: CommandPalette,
    /// Whether this is the first frame (used to fit the page on startup).
    first_frame: bool,
}

impl VDrawApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply theme
        cc.egui_ctx.set_visuals(theme::build_visuals(true));
        let mut style = (*cc.egui_ctx.style()).clone();
        theme::apply_style(&mut style);
        cc.egui_ctx.set_style(style);

        Self {
            doc: Document::new("Untitled 1"),
            undo_stack: UndoStack::new(),
            tool_manager: ToolManager::new(ToolId::Pick),
            snap_solver: SnapSolver::default(),
            canvas: CanvasState::default(),
            toolbar: ToolbarState::default(),
            dock: DockState::default(),
            workspace: WorkspaceState::default(),
            statusbar: StatusBarState::default(),
            palette: CommandPalette::new(),
            first_frame: true,
        }
    }
}

impl eframe::App for VDrawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Synchronize tool selection ─────────────────────────────────────
        if self.toolbar.active_tool != self.tool_manager.active_id() {
            self.tool_manager.set_tool(self.toolbar.active_tool);
        }

        // ── Global keyboard shortcuts ──────────────────────────────────────
        let mut undo_action = false;
        let mut redo_action = false;
        let mut delete_action = false;
        let mut select_all_action = false;

        ctx.input(|i| {
            // Ctrl+K → command palette
            if i.key_pressed(egui::Key::K) && i.modifiers.ctrl {
                self.palette.open = !self.palette.open;
            }
            // Zoom shortcuts
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.canvas.zoom = (self.canvas.zoom * 1.25).min(CanvasState::MAX_ZOOM);
            }
            if i.key_pressed(egui::Key::Minus) {
                self.canvas.zoom = (self.canvas.zoom / 1.25).max(CanvasState::MIN_ZOOM);
            }
            if i.key_pressed(egui::Key::Num0) || i.key_pressed(egui::Key::Insert) {
                let rect = ctx.screen_rect();
                self.canvas.fit_page(rect);
            }

            // Undo / Redo
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z) {
                undo_action = true;
            }
            if (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
                || (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
            {
                redo_action = true;
            }

            // Delete / Backspace
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                delete_action = true;
            }

            // Select All
            if i.modifiers.ctrl && i.key_pressed(egui::Key::A) {
                select_all_action = true;
            }

            // Phase 3: Toggle Dynamic Guides (Alt+Shift+D)
            if i.modifiers.alt && i.modifiers.shift && i.key_pressed(egui::Key::D) {
                self.snap_solver.config.dynamic_guides.enabled =
                    !self.snap_solver.config.dynamic_guides.enabled;
                self.statusbar.dynamic_snap = self.snap_solver.config.dynamic_guides.enabled;
            }

            // Tool shortcuts (when no text input active)
            if !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) {
                    self.toolbar.active_tool = ToolId::Pick;
                    self.tool_manager.set_tool(ToolId::Pick);
                } else if i.key_pressed(egui::Key::F10) {
                    self.toolbar.active_tool = ToolId::Shape;
                    self.tool_manager.set_tool(ToolId::Shape);
                } else if i.key_pressed(egui::Key::F5) {
                    self.toolbar.active_tool = ToolId::Freehand;
                    self.tool_manager.set_tool(ToolId::Freehand);
                } else if i.key_pressed(egui::Key::F6) {
                    self.toolbar.active_tool = ToolId::Rectangle;
                    self.tool_manager.set_tool(ToolId::Rectangle);
                } else if i.key_pressed(egui::Key::F7) {
                    self.toolbar.active_tool = ToolId::Ellipse;
                    self.tool_manager.set_tool(ToolId::Ellipse);
                } else if i.key_pressed(egui::Key::Y) {
                    self.toolbar.active_tool = ToolId::Polygon;
                    self.tool_manager.set_tool(ToolId::Polygon);
                } else if i.key_pressed(egui::Key::A) {
                    self.toolbar.active_tool = ToolId::Spiral;
                    self.tool_manager.set_tool(ToolId::Spiral);
                } else if i.key_pressed(egui::Key::L) {
                    self.toolbar.active_tool = ToolId::Line;
                    self.tool_manager.set_tool(ToolId::Line);
                } else if i.key_pressed(egui::Key::B) {
                    self.toolbar.active_tool = ToolId::Pen;
                    self.tool_manager.set_tool(ToolId::Pen);
                } else if i.key_pressed(egui::Key::Escape) {
                    self.tool_manager.on_cancel(&mut self.doc);
                    self.doc.selection.clear();
                }
            }
        });

        if undo_action {
            self.undo_stack.undo(&mut self.doc);
        }
        if redo_action {
            self.undo_stack.redo(&mut self.doc);
        }
        if delete_action && !self.doc.selection.is_empty() {
            let ids: Vec<_> = self.doc.selection.ids.iter().copied().collect();
            self.undo_stack
                .apply(Box::new(RemoveShapesCommand::new(ids)), &mut self.doc);
            self.doc.selection.clear();
        }
        if select_all_action {
            let ids: Vec<_> = self
                .doc
                .active_page()
                .map(|p| p.all_shapes().map(|s| s.id).collect())
                .unwrap_or_default();
            for id in ids {
                self.doc.selection.add(id);
            }
        }

        // ── Command palette overlay ───────────────────────────────────────
        self.palette.show(ctx);

        // ── Menu bar ─────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(4, 2)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    // File menu
                    ui.menu_button("File", |ui| {
                        if ui.button("New         Ctrl+N").clicked() {
                            self.doc = Document::new("Untitled");
                            self.undo_stack.clear();
                            ui.close_menu();
                        }
                        if ui.button("Open…       Ctrl+O").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Save        Ctrl+S").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("Save As…    Ctrl+Shift+S").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export PNG…").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("Export SVG…").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit        Ctrl+Q").clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });

                    // Edit menu
                    ui.menu_button("Edit", |ui| {
                        let undo_label = match self.undo_stack.undo_name() {
                            Some(name) => format!("Undo {}  Ctrl+Z", name),
                            None => "Undo  Ctrl+Z".to_string(),
                        };
                        let redo_label = match self.undo_stack.redo_name() {
                            Some(name) => format!("Redo {}  Ctrl+Y", name),
                            None => "Redo  Ctrl+Y".to_string(),
                        };

                        if ui
                            .add_enabled(self.undo_stack.can_undo(), egui::Button::new(undo_label))
                            .clicked()
                        {
                            self.undo_stack.undo(&mut self.doc);
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(self.undo_stack.can_redo(), egui::Button::new(redo_label))
                            .clicked()
                        {
                            self.undo_stack.redo(&mut self.doc);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(
                                !self.doc.selection.is_empty(),
                                egui::Button::new("Delete  Del"),
                            )
                            .clicked()
                        {
                            let ids: Vec<_> = self.doc.selection.ids.iter().copied().collect();
                            self.undo_stack
                                .apply(Box::new(RemoveShapesCommand::new(ids)), &mut self.doc);
                            self.doc.selection.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select All  Ctrl+A").clicked() {
                            let ids: Vec<_> = self
                                .doc
                                .active_page()
                                .map(|p| p.all_shapes().map(|s| s.id).collect())
                                .unwrap_or_default();
                            for id in ids {
                                self.doc.selection.add(id);
                            }
                            ui.close_menu();
                        }
                        if ui.button("Deselect All  Esc").clicked() {
                            self.doc.selection.clear();
                            ui.close_menu();
                        }
                    });

                    // View menu
                    ui.menu_button("View", |ui| {
                        if ui.button("Zoom In    +").clicked() {
                            self.canvas.zoom = (self.canvas.zoom * 1.25).min(CanvasState::MAX_ZOOM);
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out   -").clicked() {
                            self.canvas.zoom = (self.canvas.zoom / 1.25).max(CanvasState::MIN_ZOOM);
                            ui.close_menu();
                        }
                        if ui.button("Fit Page   0").clicked() {
                            let rect = ctx.screen_rect();
                            self.canvas.fit_page(rect);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .checkbox(&mut self.workspace.show_grid, "Show Grid")
                            .clicked()
                        {
                            self.canvas.show_grid = self.workspace.show_grid;
                        }
                        if ui
                            .checkbox(&mut self.workspace.show_rulers, "Show Rulers")
                            .clicked()
                        {
                            self.canvas.show_rulers = self.workspace.show_rulers;
                        }
                        ui.separator();
                        ui.menu_button("Workspace Preset", |ui| {
                            if ui
                                .radio_value(
                                    &mut self.workspace.preset,
                                    WorkspacePreset::Illustration,
                                    "Illustration",
                                )
                                .clicked()
                            {
                                self.workspace.apply_preset(WorkspacePreset::Illustration);
                                ui.close_menu();
                            }
                            if ui
                                .radio_value(
                                    &mut self.workspace.preset,
                                    WorkspacePreset::LaserCutting,
                                    "Laser Cutting",
                                )
                                .clicked()
                            {
                                self.workspace.apply_preset(WorkspacePreset::LaserCutting);
                                ui.close_menu();
                            }
                            if ui
                                .radio_value(
                                    &mut self.workspace.preset,
                                    WorkspacePreset::TechnicalDrawing,
                                    "Technical Drawing",
                                )
                                .clicked()
                            {
                                self.workspace
                                    .apply_preset(WorkspacePreset::TechnicalDrawing);
                                ui.close_menu();
                            }
                        });
                    });

                    // Tools menu
                    ui.menu_button("Tools", |ui| {
                        if ui.button("Pick Tool (V)").clicked() {
                            self.toolbar.active_tool = ToolId::Pick;
                            self.tool_manager.set_tool(ToolId::Pick);
                            ui.close_menu();
                        }
                        if ui.button("Shape / Node Tool (F10)").clicked() {
                            self.toolbar.active_tool = ToolId::Shape;
                            self.tool_manager.set_tool(ToolId::Shape);
                            ui.close_menu();
                        }
                        if ui.button("Freehand Tool (F5)").clicked() {
                            self.toolbar.active_tool = ToolId::Freehand;
                            self.tool_manager.set_tool(ToolId::Freehand);
                            ui.close_menu();
                        }
                        if ui.button("Rectangle (F6)").clicked() {
                            self.toolbar.active_tool = ToolId::Rectangle;
                            self.tool_manager.set_tool(ToolId::Rectangle);
                            ui.close_menu();
                        }
                        if ui.button("Ellipse (F7)").clicked() {
                            self.toolbar.active_tool = ToolId::Ellipse;
                            self.tool_manager.set_tool(ToolId::Ellipse);
                            ui.close_menu();
                        }
                        if ui.button("Polygon (Y)").clicked() {
                            self.toolbar.active_tool = ToolId::Polygon;
                            self.tool_manager.set_tool(ToolId::Polygon);
                            ui.close_menu();
                        }
                        if ui.button("Star").clicked() {
                            self.toolbar.active_tool = ToolId::Star;
                            self.tool_manager.set_tool(ToolId::Star);
                            ui.close_menu();
                        }
                        if ui.button("Spiral (A)").clicked() {
                            self.toolbar.active_tool = ToolId::Spiral;
                            self.tool_manager.set_tool(ToolId::Spiral);
                            ui.close_menu();
                        }
                        if ui.button("2-Point Line (L)").clicked() {
                            self.toolbar.active_tool = ToolId::Line;
                            self.tool_manager.set_tool(ToolId::Line);
                            ui.close_menu();
                        }
                        if ui.button("Pen Tool (B)").clicked() {
                            self.toolbar.active_tool = ToolId::Pen;
                            self.tool_manager.set_tool(ToolId::Pen);
                            ui.close_menu();
                        }
                    });

                    // Help menu
                    ui.menu_button("Help", |ui| {
                        if ui.button("Command Palette (Ctrl+K)").clicked() {
                            self.palette.open = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("About VDraw").clicked() {
                            ui.close_menu();
                        }
                    });
                });
            });

        // ── Property bar ─────────────────────────────────────────────────
        if self.workspace.show_prop_bar {
            egui::TopBottomPanel::top("prop_bar")
                .exact_height(theme::PROPBAR_HEIGHT)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::BG_PANEL)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                )
                .show(ctx, |ui| {
                    crate::panels::properties::show_prop_bar(
                        ui,
                        &mut self.doc,
                        &mut self.undo_stack,
                        &mut self.tool_manager,
                    );
                });
        }

        // ── Status bar ───────────────────────────────────────────────────
        if self.workspace.show_status_bar {
            egui::TopBottomPanel::bottom("status_bar")
                .exact_height(theme::STATUSBAR_HEIGHT)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::BG_PANEL)
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                )
                .show(ctx, |ui| {
                    crate::panels::statusbar::show_statusbar(
                        ui,
                        &mut self.canvas,
                        &mut self.statusbar,
                        &self.doc,
                        &mut self.snap_solver,
                    );
                });
        }

        // ── Left toolbar ─────────────────────────────────────────────────
        if self.workspace.show_toolbar {
            egui::SidePanel::left("toolbar")
                .exact_width(theme::TOOLBAR_WIDTH)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                )
                .show(ctx, |ui| {
                    crate::panels::toolbar::show_toolbar(ui, &mut self.toolbar);
                });
        }

        // ── Right dock panel ─────────────────────────────────────────────
        if self.workspace.show_right_panel {
            egui::SidePanel::right("panels")
                .min_width(220.0)
                .default_width(260.0)
                .frame(
                    egui::Frame::NONE
                        .fill(theme::BG_PANEL)
                        .inner_margin(egui::Margin::same(6))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER)),
                )
                .show(ctx, |ui| {
                    dock::show_dock_tabs(ui, &mut self.dock);
                    egui::ScrollArea::vertical()
                        .id_salt("right_dock_scroll")
                        .show(ui, |ui| {
                            dock::show_dock_content(
                                ui,
                                &self.dock,
                                &mut self.doc,
                                &mut self.undo_stack,
                            );
                        });
                });
        }

        // ── Central canvas ───────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::CANVAS_BG))
            .show(ctx, |ui| {
                // Sync workspace settings → canvas state
                self.canvas.show_grid = self.workspace.show_grid;
                self.canvas.show_rulers = self.workspace.show_rulers;

                // Fit page on first frame
                if self.first_frame {
                    self.first_frame = false;
                    let rect = ui.available_rect_before_wrap();
                    self.canvas.fit_page(rect);
                }

                // Synchronize spatial index and guidelines
                let shapes: Vec<_> = self
                    .doc
                    .active_page()
                    .map(|p| p.all_shapes().cloned().collect())
                    .unwrap_or_default();
                self.snap_solver.spatial_index.rebuild(&shapes);
                self.snap_solver.guidelines = self.doc.guidelines.clone();

                canvas::show_canvas(
                    ui,
                    &mut self.canvas,
                    &mut self.doc,
                    &mut self.undo_stack,
                    &mut self.tool_manager,
                    &mut self.snap_solver,
                );
            });
    }
}
