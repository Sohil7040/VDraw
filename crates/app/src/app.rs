//! VDrawApp — the top-level eframe application with Dispatcher-only routing and token theming.

use std::path::PathBuf;
use std::time::Instant;

use core_boolean::BooleanOp;
use core_doc::{Document, UndoStack};
use core_geom::Point2;
use core_snap::contracts::{SnapFeedback, DEFAULT_SNAP_MASK};
use core_snap::SnapSolver;
use core_tools::{
    AppCommand, CommandError, CommandKind, DispatchCtx, Dispatcher, DragSession, SnapController,
    ThemePref, ToolId, ToolManager, UiEffect,
};
use egui::{Key, Modifiers, ViewportCommand};

use crate::{
    canvas::{self, CanvasState},
    command_palette::{build_command_catalog, CommandPalette, PaletteEntry},
    panels::{
        dock::{self, DockState, DockTab},
        properties::show_prop_bar,
        statusbar::{self, StatusBarState},
        toast::ToastManager,
        toolbar::{self, ToolbarState},
        welcome::{self, WelcomeState},
    },
    screenshot, theme,
    tokens::{load_theme_pref, save_theme_pref, SpacingTokens, ThemeTokens},
    workspace::{WorkspacePreset, WorkspaceState},
};

/// Application workflow modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkflowMode {
    #[default]
    Design,
    LaserCam,
    Preview,
}

#[derive(Clone, Copy)]
struct ShortcutBinding {
    modifiers: Modifiers,
    key: Key,
    allow_while_typing: bool,
    command: fn() -> AppCommand,
}

const SHORTCUTS: &[ShortcutBinding] = &[
    // Command palette & help
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::K,
        allow_while_typing: true,
        command: || AppCommand::OpenCommandPalette,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F1,
        allow_while_typing: true,
        command: || AppCommand::OpenHelp,
    },
    // File actions
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::N,
        allow_while_typing: true,
        command: || AppCommand::NewDocument,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::O,
        allow_while_typing: true,
        command: || AppCommand::OpenDocument,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        key: Key::S,
        allow_while_typing: true,
        command: || AppCommand::SaveDocumentAs,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::S,
        allow_while_typing: true,
        command: || AppCommand::SaveDocument,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        key: Key::E,
        allow_while_typing: true,
        command: || AppCommand::ExportPng,
    },
    // Edit & History
    ShortcutBinding {
        modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        key: Key::Z,
        allow_while_typing: true,
        command: || AppCommand::Redo,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::Y,
        allow_while_typing: true,
        command: || AppCommand::Redo,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::Z,
        allow_while_typing: true,
        command: || AppCommand::Undo,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::D,
        allow_while_typing: false,
        command: || AppCommand::DuplicateSelection,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::A,
        allow_while_typing: false,
        command: || AppCommand::SelectAll,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND.plus(Modifiers::SHIFT),
        key: Key::G,
        allow_while_typing: false,
        command: || AppCommand::UngroupSelection,
    },
    ShortcutBinding {
        modifiers: Modifiers::COMMAND,
        key: Key::G,
        allow_while_typing: false,
        command: || AppCommand::GroupSelection,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Delete,
        allow_while_typing: false,
        command: || AppCommand::DeleteSelection,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Backspace,
        allow_while_typing: false,
        command: || AppCommand::DeleteSelection,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Escape,
        allow_while_typing: false,
        command: || AppCommand::DeselectAll,
    },
    // View & Snap toggles
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Plus,
        allow_while_typing: false,
        command: || AppCommand::ZoomIn,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Equals,
        allow_while_typing: false,
        command: || AppCommand::ZoomIn,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Minus,
        allow_while_typing: false,
        command: || AppCommand::ZoomOut,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Num0,
        allow_while_typing: false,
        command: || AppCommand::FitPage,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Insert,
        allow_while_typing: false,
        command: || AppCommand::FitPage,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F8,
        allow_while_typing: false,
        command: || AppCommand::ToggleSnapOrtho,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F9,
        allow_while_typing: false,
        command: || AppCommand::ToggleSnapMaster,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F11,
        allow_while_typing: false,
        command: || AppCommand::ToggleSnapPolar,
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::R,
        allow_while_typing: false,
        command: || AppCommand::ToggleRulers,
    },
    // Tool shortcuts
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::V,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Pick),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F10,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Shape),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F5,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Freehand),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F6,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Rectangle),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::F7,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Ellipse),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Y,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Polygon),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::A,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Spiral),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::L,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Line),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::B,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Pen),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::K,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Knife),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::X,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Eraser),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::D,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::VirtualSegmentDelete),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::T,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Text),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::Z,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Zoom),
    },
    ShortcutBinding {
        modifiers: Modifiers::NONE,
        key: Key::H,
        allow_while_typing: false,
        command: || AppCommand::ActivateTool(ToolId::Pan),
    },
];

/// The root VDraw application state.
/// Monotonic revision signal for detecting shape-count-preserving mutations
/// (such as Align, Distribute, layer visibility toggles, or node edits)
/// so spatial indices stay in lockstep with document changes (`[WAVE3B-HOTFIX-1]`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirtySignal {
    pub shape_count: usize,
    pub undo_depth: usize,
    pub total_history: usize,
    pub top_command: Option<String>,
    pub layer_visibilities: Vec<(usize, bool)>,
    pub guidelines_count: usize,
}

pub struct VDrawApp {
    doc: Document,
    undo_stack: UndoStack,
    dispatcher: Dispatcher,
    tool_manager: ToolManager,
    snap_controller: SnapController,
    drag_session: DragSession,
    snap_solver: SnapSolver,
    canvas: CanvasState,
    toolbar: ToolbarState,
    dock: DockState,
    workspace: WorkspaceState,
    statusbar: StatusBarState,
    palette: CommandPalette,
    pub toast: ToastManager,
    pub welcome: WelcomeState,
    pub workflow_mode: WorkflowMode,
    theme_pref: ThemePref,
    command_search: String,
    help_open: bool,
    first_frame: bool,
    last_dirty_signal: DirtySignal,
    last_ctx: Option<egui::Context>,
    io_dir: Option<PathBuf>,
}

impl VDrawApp {
    /// Creates the live GUI application using `eframe::CreationContext`.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme_pref = load_theme_pref(cc.storage);
        let coach_seen = welcome::load_coach_seen(cc.storage);
        let doc = Document::new("Untitled 1");
        let mut snap_controller = SnapController::new(DEFAULT_SNAP_MASK);
        snap_controller.update_index(&doc);

        let mut welcome = WelcomeState::new();
        welcome.coach_seen = coach_seen;

        let mut app = Self {
            doc,
            undo_stack: UndoStack::new(),
            dispatcher: Dispatcher,
            tool_manager: ToolManager::new(ToolId::Pick),
            snap_controller,
            drag_session: DragSession::new(),
            snap_solver: SnapSolver::default(),
            canvas: CanvasState::default(),
            toolbar: ToolbarState::default(),
            dock: DockState::default(),
            workspace: WorkspaceState::default(),
            statusbar: StatusBarState::default(),
            palette: CommandPalette::new(),
            toast: ToastManager::new(),
            welcome,
            workflow_mode: WorkflowMode::Design,
            theme_pref,
            command_search: String::new(),
            help_open: false,
            first_frame: true,
            last_dirty_signal: DirtySignal::default(),
            last_ctx: Some(cc.egui_ctx.clone()),
            io_dir: None,
        };
        app.apply_theme_pref(&cc.egui_ctx, theme_pref);
        app
    }

    /// Creates a quiet headless application instance suitable for automated tests.
    #[doc(hidden)]
    pub fn new_headless(ctx: &egui::Context) -> Self {
        let doc = Document::new("Headless");
        let mut snap_controller = SnapController::new(DEFAULT_SNAP_MASK);
        snap_controller.update_index(&doc);

        let mut app = Self {
            doc,
            undo_stack: UndoStack::new(),
            dispatcher: Dispatcher,
            tool_manager: ToolManager::new(ToolId::Pick),
            snap_controller,
            drag_session: DragSession::new(),
            snap_solver: SnapSolver::default(),
            canvas: CanvasState::default(),
            toolbar: ToolbarState::default(),
            dock: DockState::default(),
            workspace: WorkspaceState::default(),
            statusbar: StatusBarState::default(),
            palette: CommandPalette::new(),
            toast: ToastManager::new(),
            welcome: WelcomeState::headless(),
            workflow_mode: WorkflowMode::Design,
            theme_pref: ThemePref::default(),
            command_search: String::new(),
            help_open: false,
            first_frame: true,
            last_dirty_signal: DirtySignal::default(),
            last_ctx: Some(ctx.clone()),
            io_dir: None,
        };
        app.apply_theme_pref(ctx, ThemePref::default());
        app
    }

    /// Loads `doc`, clears undo history, and rebuilds the snap spatial index.
    pub fn load_document(&mut self, doc: Document) {
        self.doc = doc;
        self.undo_stack.clear();
        self.drag_session.cancel();
        self.snap_controller.update_index(&self.doc);
        self.canvas.last_snap_feedback = None;
        self.last_dirty_signal = self.compute_dirty_signal();
    }

    /// Returns a mutable reference to the active [`Document`].
    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    /// Returns mutable references to both [`Document`] and [`UndoStack`].
    pub fn doc_and_undo_mut(&mut self) -> (&mut Document, &mut UndoStack) {
        (&mut self.doc, &mut self.undo_stack)
    }

    /// Returns an immutable reference to the active [`Document`].
    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// Returns a mutable reference to the [`UndoStack`].
    pub fn undo_stack_mut(&mut self) -> &mut UndoStack {
        &mut self.undo_stack
    }

    /// Returns an immutable reference to the [`UndoStack`].
    pub fn undo_stack(&self) -> &UndoStack {
        &self.undo_stack
    }

    /// Computes the active [`DirtySignal`] across shapes, undo timeline, layers, and guidelines.
    pub fn compute_dirty_signal(&self) -> DirtySignal {
        let shape_count = self
            .doc
            .active_page()
            .map(|p| p.all_shapes().count())
            .unwrap_or(0);
        let history = self.undo_stack.history();
        let total_history = history.len();
        let undo_depth = history.iter().filter(|e| e.is_applied).count();
        let top_command = self.undo_stack.top_description().map(str::to_string);
        let layer_visibilities = self
            .doc
            .active_page()
            .map(|p| {
                p.layers
                    .iter()
                    .enumerate()
                    .map(|(idx, l)| (idx, l.visible))
                    .collect()
            })
            .unwrap_or_default();
        let guidelines_count = self.doc.guidelines.len();

        DirtySignal {
            shape_count,
            undo_depth,
            total_history,
            top_command,
            layer_visibilities,
            guidelines_count,
        }
    }

    /// Public trigger for test harness to synchronize spatial indexing if dirty.
    pub fn sync_spatial_index_for_test(&mut self) {
        self.sync_spatial_index_if_dirty();
    }

    /// Returns an immutable reference to the [`SnapController`].
    pub fn snap_controller(&self) -> &SnapController {
        &self.snap_controller
    }

    /// Returns an immutable reference to the [`DragSession`].
    pub fn drag_session(&self) -> &DragSession {
        &self.drag_session
    }

    /// Returns the most recent live [`SnapFeedback`] evaluated on the canvas.
    pub fn last_snap_feedback(&self) -> Option<&SnapFeedback> {
        self.canvas.last_snap_feedback.as_ref()
    }

    /// Returns the active [`ThemePref`].
    pub fn theme_pref(&self) -> ThemePref {
        self.theme_pref
    }

    /// Returns the canonical context menu [`AppCommand`] list.
    pub fn context_menu_commands() -> Vec<AppCommand> {
        canvas::context_menu_commands()
    }

    /// Returns every command palette entry derived from [`AppCommand`].
    pub fn command_palette_entries() -> Vec<PaletteEntry> {
        CommandPalette::all_entries()
    }

    /// Enumerates every [`UiEffect`] variant for exhaustive handler verification (`[F-D1]`).
    pub fn all_ui_effects() -> Vec<UiEffect> {
        vec![
            UiEffect::NewDocument,
            UiEffect::OpenDocument,
            UiEffect::SaveDocument,
            UiEffect::SaveDocumentAs,
            UiEffect::ImportSvg,
            UiEffect::ExportSvg,
            UiEffect::ExportDxf,
            UiEffect::ExportPng,
            UiEffect::ExportGcode,
            UiEffect::ZoomIn,
            UiEffect::ZoomOut,
            UiEffect::FitPage,
            UiEffect::ZoomToSelection,
            UiEffect::ToggleGrid,
            UiEffect::ToggleRulers,
            UiEffect::SetTheme(ThemePref::default()),
            UiEffect::SwitchWorkflowDesign,
            UiEffect::SwitchWorkflowLaserCam,
            UiEffect::SwitchWorkflowPreview,
            UiEffect::OpenCommandPalette,
            UiEffect::OpenHelp,
            UiEffect::StartCoachMarks,
        ]
    }

    /// Dispatches an [`AppCommand`] through [`Dispatcher::dispatch`] and fulfills any resulting [`UiEffect`].
    pub fn dispatch_command(&mut self, cmd: AppCommand) -> Result<(), CommandError> {
        let ctx_opt = self.last_ctx.clone();
        self.dispatch_internal(ctx_opt.as_ref(), cmd)
    }

    /// Dispatches an [`AppCommand`] with an explicit [`egui::Context`].
    pub fn dispatch_with_ctx(
        &mut self,
        ctx: &egui::Context,
        cmd: AppCommand,
    ) -> Result<(), CommandError> {
        self.last_ctx = Some(ctx.clone());
        self.dispatch_internal(Some(ctx), cmd)
    }

    fn dispatch_internal(
        &mut self,
        ctx: Option<&egui::Context>,
        cmd: AppCommand,
    ) -> Result<(), CommandError> {
        let effect = {
            let mut dctx = DispatchCtx::new(&mut self.snap_controller, &mut self.tool_manager);
            self.dispatcher
                .dispatch(cmd, &mut self.doc, &mut self.undo_stack, &mut dctx)?;
            dctx.take_pending_action()
        };
        self.toolbar.active_tool = self.tool_manager.active_id();

        if let Some(effect) = effect {
            if let Some(egui_ctx) = ctx {
                self.fulfill(egui_ctx, effect)?;
            } else {
                let fallback_ctx = egui::Context::default();
                self.fulfill(&fallback_ctx, effect)?;
            }
        }
        Ok(())
    }

    fn dispatch_and_toast(&mut self, ctx: &egui::Context, cmd: AppCommand) {
        if let Err(err) = self.dispatch_with_ctx(ctx, cmd) {
            self.surface_command_error(&err);
        }
    }

    fn surface_command_error(&mut self, err: &CommandError) {
        let summary = format!("Action failed: {err}");
        let details = format!("{err:?}");
        self.toast.show_error(summary, details);
    }

    fn resolve_io_path(&self, file_name: &str) -> PathBuf {
        match &self.io_dir {
            Some(dir) => dir.join(file_name),
            None => PathBuf::from(file_name),
        }
    }

    /// Public entry point for exhaustive [`UiEffect`] testing (`[F-D1]`).
    pub fn fulfill_for_test(
        &mut self,
        ctx: &egui::Context,
        effect: UiEffect,
    ) -> Result<(), CommandError> {
        self.fulfill(ctx, effect)
    }

    /// Exhaustively handles every [`UiEffect`] emitted by [`Dispatcher::dispatch`].
    fn fulfill(&mut self, ctx: &egui::Context, effect: UiEffect) -> Result<(), CommandError> {
        match effect {
            UiEffect::NewDocument => {
                self.load_document(Document::new("Untitled"));
                self.toast.show("Created new document");
                Ok(())
            }
            UiEffect::OpenDocument => {
                let open_path = self.resolve_io_path("drawing.vdraw");
                if open_path.exists() {
                    match io::load_vdraw(&open_path) {
                        Ok(loaded) => {
                            self.load_document(loaded);
                            self.toast.show_success("Opened drawing.vdraw");
                        }
                        Err(e) => {
                            self.toast
                                .show_error("Failed to open drawing.vdraw", e.to_string());
                        }
                    }
                } else {
                    self.toast
                        .show_warning("No drawing.vdraw found in working directory");
                }
                Ok(())
            }
            UiEffect::SaveDocument => {
                let save_path = self.resolve_io_path("drawing.vdraw");
                match io::save_vdraw(&self.doc, &save_path) {
                    Ok(()) => {
                        self.undo_stack.mark_saved();
                        self.toast.show_success("Saved to drawing.vdraw");
                    }
                    Err(e) => {
                        self.toast.show_error("Save failed", e.to_string());
                    }
                }
                Ok(())
            }
            UiEffect::SaveDocumentAs => {
                let save_path = self.resolve_io_path("drawing_backup.vdraw");
                match io::save_vdraw(&self.doc, &save_path) {
                    Ok(()) => {
                        self.toast
                            .show_success("Saved copy to drawing_backup.vdraw");
                    }
                    Err(e) => {
                        self.toast.show_error("Save As failed", e.to_string());
                    }
                }
                Ok(())
            }
            UiEffect::ImportSvg => {
                let import_path = self.resolve_io_path("import.svg");
                if import_path.exists() {
                    match io::load_svg(&import_path) {
                        Ok(loaded) => {
                            self.load_document(loaded);
                            self.toast.show_success("Imported import.svg");
                        }
                        Err(e) => {
                            self.toast.show_error("SVG import failed", e.to_string());
                        }
                    }
                } else {
                    self.toast
                        .show_warning("No import.svg found in working directory");
                }
                Ok(())
            }
            UiEffect::ExportSvg => {
                let svg_path = self.resolve_io_path("export_drawing.svg");
                match io::save_svg(&self.doc, &svg_path) {
                    Ok(()) => {
                        self.toast.show_success("Exported to export_drawing.svg");
                    }
                    Err(e) => {
                        self.toast.show_error("SVG export failed", e.to_string());
                    }
                }
                Ok(())
            }
            UiEffect::ExportDxf => {
                let dxf_path = self.resolve_io_path("export_drawing.dxf");
                match io::save_dxf(&self.doc, &dxf_path) {
                    Ok(()) => {
                        self.toast.show_success("Exported to export_drawing.dxf");
                    }
                    Err(e) => {
                        self.toast.show_error("DXF export failed", e.to_string());
                    }
                }
                Ok(())
            }
            UiEffect::ExportPng => {
                let png_path = self.resolve_io_path("vdraw_export.png");
                match screenshot::render_ui_screenshot(&png_path) {
                    Ok(()) => {
                        self.toast.show_success("Exported PNG to vdraw_export.png");
                    }
                    Err(e) => {
                        self.toast.show_error("PNG export failed", e);
                    }
                }
                Ok(())
            }
            UiEffect::ExportGcode => {
                let plan = core_cam::generate_job_plan(&self.doc, &self.dock.cam_config);
                let gcode = plan.export_gcode();
                let gcode_path = self.resolve_io_path("output_laser_job.gcode");
                match std::fs::write(&gcode_path, &gcode) {
                    Ok(()) => {
                        self.toast
                            .show_success(format!("Exported G-Code to {}", gcode_path.display()));
                    }
                    Err(e) => {
                        self.toast.show_error("G-Code export failed", e.to_string());
                    }
                }
                Ok(())
            }
            UiEffect::ZoomIn => {
                self.canvas.zoom = (self.canvas.zoom * 1.25).min(CanvasState::MAX_ZOOM);
                Ok(())
            }
            UiEffect::ZoomOut => {
                self.canvas.zoom = (self.canvas.zoom / 1.25).max(CanvasState::MIN_ZOOM);
                Ok(())
            }
            UiEffect::FitPage => {
                let rect = self
                    .canvas
                    .last_viewport
                    .unwrap_or_else(|| ctx.screen_rect());
                self.canvas.fit_page(rect);
                Ok(())
            }
            UiEffect::ZoomToSelection => {
                if let Some(bounds) = self.doc.selection_bounds() {
                    let rect = self
                        .canvas
                        .last_viewport
                        .unwrap_or_else(|| ctx.screen_rect());
                    self.canvas.zoom_to_rect(bounds, rect);
                }
                Ok(())
            }
            UiEffect::ToggleGrid => {
                self.workspace.show_grid = !self.workspace.show_grid;
                self.canvas.show_grid = self.workspace.show_grid;
                Ok(())
            }
            UiEffect::ToggleRulers => {
                self.workspace.show_rulers = !self.workspace.show_rulers;
                self.canvas.show_rulers = self.workspace.show_rulers;
                Ok(())
            }
            UiEffect::SetTheme(pref) => {
                self.apply_theme_pref(ctx, pref);
                Ok(())
            }
            UiEffect::SwitchWorkflowDesign => {
                self.workflow_mode = WorkflowMode::Design;
                self.dock.cam_config.preview = false;
                self.canvas.cam_config.preview = false;
                self.toast
                    .show("Design mode: vector drawing and arrangement");
                Ok(())
            }
            UiEffect::SwitchWorkflowLaserCam => {
                self.workflow_mode = WorkflowMode::LaserCam;
                self.dock.active_tab = DockTab::Laser;
                self.dock.cam_config.preview = false;
                self.canvas.cam_config.preview = false;
                self.toast
                    .show("Laser CAM mode: configure speeds, powers, and passes");
                Ok(())
            }
            UiEffect::SwitchWorkflowPreview => {
                self.workflow_mode = WorkflowMode::Preview;
                self.dock.cam_config.preview = true;
                self.canvas.cam_config.preview = true;
                self.toast
                    .show("Toolpath preview mode: simulating laser execution sequence");
                Ok(())
            }
            UiEffect::OpenCommandPalette => {
                self.palette.open_palette();
                Ok(())
            }
            UiEffect::OpenHelp => {
                self.help_open = true;
                Ok(())
            }
            UiEffect::StartCoachMarks => {
                self.welcome.start_coach_marks();
                Ok(())
            }
        }
    }

    fn apply_theme_pref(&mut self, ctx: &egui::Context, pref: ThemePref) {
        self.theme_pref = pref;
        let tokens = ThemeTokens::for_pref(pref);
        ctx.set_theme(pref.0);
        ctx.set_visuals_of(egui::Theme::Dark, ThemeTokens::for_dark(true).to_visuals());
        ctx.set_visuals_of(
            egui::Theme::Light,
            ThemeTokens::for_dark(false).to_visuals(),
        );
        ctx.set_visuals(tokens.to_visuals());
        let mut style = (*ctx.style()).clone();
        theme::apply_style(&mut style);
        ctx.set_style(style);
    }

    fn process_shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.wants_keyboard_input();
        let triggered: Vec<AppCommand> = ctx.input(|i| {
            let mut out = Vec::new();
            for binding in SHORTCUTS {
                if !binding.allow_while_typing && (typing || self.palette.open) {
                    continue;
                }
                if i.modifiers.matches_exact(binding.modifiers) && i.key_pressed(binding.key) {
                    out.push((binding.command)());
                    break;
                }
            }
            out
        });

        for cmd in triggered {
            self.dispatch_and_toast(ctx, cmd);
        }
    }

    fn sync_spatial_index_if_dirty(&mut self) {
        let current_signal = self.compute_dirty_signal();

        if self.first_frame || current_signal != self.last_dirty_signal {
            let shapes: Vec<_> = self
                .doc
                .active_page()
                .map(|p| p.all_shapes().cloned().collect())
                .unwrap_or_default();
            self.snap_solver.spatial_index.rebuild(&shapes);
            self.snap_solver.guidelines = self.doc.guidelines.clone();
            self.snap_controller.update_index(&self.doc);
            self.last_dirty_signal = current_signal;
        }
    }

    fn render_top_command_bar(&mut self, ctx: &egui::Context, tokens: &ThemeTokens) {
        let mut pending_cmds: Vec<AppCommand> = Vec::new();

        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::NONE
                    .fill(tokens.color.surface_elevated)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .stroke(egui::Stroke::new(1.0, tokens.color.border)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // File menu
                    ui.menu_button("File", |ui| {
                        if ui.button("Welcome Screen…").clicked() {
                            self.welcome.is_open = true;
                            ui.close_menu();
                        }
                        if ui.button("Guided Tour (3 Steps)…").clicked() {
                            pending_cmds.push(AppCommand::StartCoachMarks);
                            ui.close_menu();
                        }
                        ui.separator();
                        for (label, cmd) in [
                            ("New         Ctrl+N", AppCommand::NewDocument),
                            ("Open…       Ctrl+O", AppCommand::OpenDocument),
                            ("Save        Ctrl+S", AppCommand::SaveDocument),
                            ("Save As…    Ctrl+Shift+S", AppCommand::SaveDocumentAs),
                            ("Import SVG…", AppCommand::ImportSvg),
                            ("Export SVG…", AppCommand::ExportSvg),
                            ("Export DXF…", AppCommand::ExportDxf),
                            ("Export PNG… Ctrl+Shift+E", AppCommand::ExportPng),
                            ("Export G-Code…", AppCommand::ExportGcode),
                        ] {
                            if ui.button(label).clicked() {
                                pending_cmds.push(cmd);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                            ui.close_menu();
                        }
                    });

                    // Edit menu
                    ui.menu_button("Edit", |ui| {
                        let undo_label = match self.undo_stack.undo_name() {
                            Some(name) => format!("Undo {name}  Ctrl+Z"),
                            None => "Undo  Ctrl+Z".to_string(),
                        };
                        let redo_label = match self.undo_stack.redo_name() {
                            Some(name) => format!("Redo {name}  Ctrl+Y"),
                            None => "Redo  Ctrl+Y".to_string(),
                        };
                        if ui
                            .add_enabled(self.undo_stack.can_undo(), egui::Button::new(undo_label))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::Undo);
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(self.undo_stack.can_redo(), egui::Button::new(redo_label))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::Redo);
                            ui.close_menu();
                        }
                        ui.separator();
                        let has_sel = !self.doc.selection.is_empty();
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Duplicate  Ctrl+D"))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::DuplicateSelection);
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(has_sel, egui::Button::new("Delete  Del"))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::DeleteSelection);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select All  Ctrl+A").clicked() {
                            pending_cmds.push(AppCommand::SelectAll);
                            ui.close_menu();
                        }
                        if ui.button("Deselect All  Esc").clicked() {
                            pending_cmds.push(AppCommand::DeselectAll);
                            ui.close_menu();
                        }
                    });

                    // View menu
                    ui.menu_button("View", |ui| {
                        if ui.button("Zoom In    +").clicked() {
                            pending_cmds.push(AppCommand::ZoomIn);
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out   -").clicked() {
                            pending_cmds.push(AppCommand::ZoomOut);
                            ui.close_menu();
                        }
                        if ui.button("Fit Page   0").clicked() {
                            pending_cmds.push(AppCommand::FitPage);
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(
                                !self.doc.selection.is_empty(),
                                egui::Button::new("Zoom to Selection"),
                            )
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::ZoomToSelection);
                            ui.close_menu();
                        }
                        ui.separator();
                        let grid_label = if self.workspace.show_grid {
                            "✓ Show Grid"
                        } else {
                            "Show Grid"
                        };
                        if ui.button(grid_label).clicked() {
                            pending_cmds.push(AppCommand::ToggleGrid);
                            ui.close_menu();
                        }
                        let rulers_label = if self.workspace.show_rulers {
                            "✓ Show Rulers"
                        } else {
                            "Show Rulers"
                        };
                        if ui.button(rulers_label).clicked() {
                            pending_cmds.push(AppCommand::ToggleRulers);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Workspace Preset", |ui| {
                            for (preset, name) in [
                                (WorkspacePreset::Illustration, "Illustration"),
                                (WorkspacePreset::LaserCutting, "Laser Cutting"),
                                (WorkspacePreset::TechnicalDrawing, "Technical Drawing"),
                            ] {
                                if ui
                                    .selectable_label(self.workspace.preset == preset, name)
                                    .clicked()
                                {
                                    self.workspace.apply_preset(preset);
                                    self.canvas.show_grid = self.workspace.show_grid;
                                    self.canvas.show_rulers = self.workspace.show_rulers;
                                    ui.close_menu();
                                }
                            }
                        });
                    });

                    // Object menu
                    ui.menu_button("Object", |ui| {
                        let sel_count = self.doc.selection.len();
                        ui.menu_button("Shaping (Booleans)", |ui| {
                            for (op, label) in [
                                (BooleanOp::Weld, "Weld / Union"),
                                (BooleanOp::Trim, "Trim / Difference"),
                                (BooleanOp::Intersect, "Intersect"),
                                (BooleanOp::Exclude, "Exclude"),
                                (BooleanOp::FrontMinusBack, "Front Minus Back"),
                                (BooleanOp::BackMinusFront, "Back Minus Front"),
                                (BooleanOp::CreateBoundary, "Create Boundary"),
                            ] {
                                if ui
                                    .add_enabled(sel_count >= 2, egui::Button::new(label))
                                    .clicked()
                                {
                                    pending_cmds.push(AppCommand::BooleanOp(op));
                                    ui.close_menu();
                                }
                            }
                            if ui
                                .add_enabled(sel_count >= 1, egui::Button::new("Simplify"))
                                .clicked()
                            {
                                pending_cmds.push(AppCommand::BooleanOp(BooleanOp::Simplify));
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui
                            .add_enabled(sel_count >= 1, egui::Button::new("Group  Ctrl+G"))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::GroupSelection);
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(sel_count >= 1, egui::Button::new("Ungroup  Ctrl+Shift+G"))
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::UngroupSelection);
                            ui.close_menu();
                        }
                    });

                    // Tools menu derived from catalog
                    ui.menu_button("Tools", |ui| {
                        for entry in build_command_catalog()
                            .into_iter()
                            .filter(|e| e.kind == CommandKind::Tool)
                        {
                            let lbl = if entry.shortcut.is_empty() {
                                format!("{}  {}", entry.icon, entry.label)
                            } else {
                                format!("{}  {} ({})", entry.icon, entry.label, entry.shortcut)
                            };
                            if ui.button(lbl).clicked() {
                                pending_cmds.push(entry.command);
                                ui.close_menu();
                            }
                        }
                    });

                    // Settings -> Appearance (Dark / Light / System)
                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Appearance", |ui| {
                            for (pref_val, label) in [
                                (egui::ThemePreference::System, "System"),
                                (egui::ThemePreference::Dark, "Dark"),
                                (egui::ThemePreference::Light, "Light"),
                            ] {
                                let selected = self.theme_pref.0 == pref_val;
                                if ui.selectable_label(selected, label).clicked() {
                                    pending_cmds.push(AppCommand::SetTheme(ThemePref(pref_val)));
                                    ui.close_menu();
                                }
                            }
                        });
                    });

                    ui.separator();

                    // Undo / Redo + History Dropdown
                    let undo_btn = ui
                        .add_enabled(self.undo_stack.can_undo(), egui::Button::new("↶ Undo"))
                        .on_hover_text("Undo last command (Ctrl+Z)");
                    undo_btn.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Undo")
                    });
                    if undo_btn.clicked() {
                        pending_cmds.push(AppCommand::Undo);
                    }

                    let redo_btn = ui
                        .add_enabled(self.undo_stack.can_redo(), egui::Button::new("↷ Redo"))
                        .on_hover_text("Redo command (Ctrl+Y)");
                    redo_btn.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Redo")
                    });
                    if redo_btn.clicked() {
                        pending_cmds.push(AppCommand::Redo);
                    }

                    let history = self.undo_stack.history();
                    ui.menu_button("▾ History", |ui| {
                        if ui
                            .selectable_label(!self.undo_stack.can_undo(), "Initial Document State")
                            .clicked()
                        {
                            pending_cmds.push(AppCommand::JumpToHistoryIndex(0));
                            ui.close_menu();
                        }
                        if !history.is_empty() {
                            ui.separator();
                        }
                        for entry in history {
                            let prefix = if entry.is_applied { "●" } else { "○" };
                            let lbl = format!("{prefix} #{}: {}", entry.index + 1, entry.label);
                            if ui.selectable_label(entry.is_applied, lbl).clicked() {
                                pending_cmds.push(AppCommand::JumpToHistoryIndex(entry.index + 1));
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    // Live command search box opening/filtering CommandPalette
                    let search_edit = egui::TextEdit::singleline(&mut self.command_search)
                        .hint_text("Search commands (Ctrl+K)…")
                        .desired_width(180.0);
                    let search_resp = ui
                        .add(search_edit)
                        .on_hover_text("Type to search commands or press Ctrl+K");
                    search_resp.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::TextEdit,
                            true,
                            "Command Search",
                        )
                    });
                    if search_resp.changed() {
                        if !self.command_search.trim().is_empty() {
                            self.palette.open = true;
                            self.palette.query = self.command_search.clone();
                            self.palette.selected_index = 0;
                        } else {
                            self.palette.query.clear();
                        }
                    }

                    // Right side: F1 Help + Workflow Switcher
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        let help_btn = ui
                            .button("? Help (F1)")
                            .on_hover_text("Keyboard shortcuts & quick help (F1)");
                        help_btn.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Help (F1)")
                        });
                        if help_btn.clicked() {
                            pending_cmds.push(AppCommand::OpenHelp);
                        }

                        for (mode, label, cmd) in [
                            (
                                WorkflowMode::Preview,
                                "▷ Preview",
                                AppCommand::SwitchWorkflowPreview,
                            ),
                            (
                                WorkflowMode::LaserCam,
                                "▤ Laser CAM",
                                AppCommand::SwitchWorkflowLaserCam,
                            ),
                            (
                                WorkflowMode::Design,
                                "✳ Design",
                                AppCommand::SwitchWorkflowDesign,
                            ),
                        ] {
                            let active = self.workflow_mode == mode;
                            let text_col = if active {
                                tokens.color.on_accent
                            } else {
                                tokens.color.on_surface_muted
                            };
                            let fill = if active {
                                tokens.color.laser_active
                            } else {
                                tokens.color.surface_interactive
                            };
                            let btn = egui::Button::new(
                                egui::RichText::new(label).small().strong().color(text_col),
                            )
                            .fill(fill);
                            let resp = ui.add(btn);
                            resp.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                            });
                            if resp.clicked() {
                                pending_cmds.push(cmd);
                            }
                        }
                    });
                });
            });

        for cmd in pending_cmds {
            self.dispatch_and_toast(ctx, cmd);
        }
    }

    fn render_help_modal(&mut self, ctx: &egui::Context, tokens: &ThemeTokens) {
        if !self.help_open {
            return;
        }
        let mut open = self.help_open;
        let mut start_tour = false;
        egui::Window::new("VDraw Shortcuts & Help (F1)")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(420.0, 320.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(tokens.color.surface_elevated)
                    .stroke(egui::Stroke::new(1.0, tokens.color.border)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Essential Shortcuts")
                        .strong()
                        .color(tokens.color.on_surface),
                );
                ui.separator();
                for (keys, desc) in [
                    ("Ctrl/Cmd + K", "Open Command Palette"),
                    ("Ctrl/Cmd + Z / Y", "Undo / Redo"),
                    ("V / F10 / F6 / F7", "Pick / Node / Rectangle / Ellipse"),
                    ("F8 / F9 / F11", "Toggle Ortho / Snap Master / Polar"),
                    ("Space + Drag / Wheel", "Pan / Zoom canvas"),
                    ("F1", "Toggle this Help dialog"),
                ] {
                    ui.horizontal(|ui| {
                        ui.monospace(egui::RichText::new(keys).color(tokens.color.accent_hover));
                        ui.label(egui::RichText::new(desc).color(tokens.color.on_surface_muted));
                    });
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Start 3-Step Guided Tour").clicked() {
                        start_tour = true;
                    }
                    if ui.button("Close").clicked() {
                        self.help_open = false;
                    }
                });
            });
        self.help_open = open;
        if start_tour {
            self.help_open = false;
            self.dispatch_and_toast(ctx, AppCommand::StartCoachMarks);
        }
    }

    /// Renders the entire application UI for a frame (shared by live eframe and headless tests).
    pub fn render_ui(&mut self, ctx: &egui::Context) {
        let frame_start = Instant::now();
        self.last_ctx = Some(ctx.clone());
        let tokens = ThemeTokens::from_ctx(ctx);

        // Keep toolbar active tool synced with ToolManager
        self.toolbar.active_tool = self.tool_manager.active_id();

        // 1. Centralized keyboard shortcuts
        self.process_shortcuts(ctx);

        // 2. Command Palette overlay
        if let Some(cmd) = self.palette.show(ctx) {
            self.command_search.clear();
            self.dispatch_and_toast(ctx, cmd);
        }

        // 3. Top command bar
        self.render_top_command_bar(ctx, &tokens);

        // 4. Context property bar
        if self.workspace.show_prop_bar {
            let mut prop_cmds = Vec::new();
            egui::TopBottomPanel::top("prop_bar")
                .exact_height(SpacingTokens::COMMANDBAR_HEIGHT)
                .frame(
                    egui::Frame::NONE
                        .fill(tokens.color.surface_elevated)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .stroke(egui::Stroke::new(1.0, tokens.color.border)),
                )
                .show(ctx, |ui| {
                    prop_cmds = show_prop_bar(
                        ui,
                        &mut self.doc,
                        &mut self.undo_stack,
                        &mut self.tool_manager,
                        &mut self.toast,
                    );
                });
            for cmd in prop_cmds {
                self.dispatch_and_toast(ctx, cmd);
            }
        }

        // 5. Status bar
        if self.workspace.show_status_bar {
            let mut status_cmds = Vec::new();
            self.statusbar.active_snap_feedback = self.canvas.last_snap_feedback.clone();
            self.statusbar.handle_mode = self.snap_controller.handle_mode();
            self.statusbar.shape_count = self
                .doc
                .active_page()
                .map(|p| p.all_shapes().count())
                .unwrap_or(0);
            egui::TopBottomPanel::bottom("status_bar")
                .exact_height(SpacingTokens::STATUSBAR_HEIGHT)
                .frame(
                    egui::Frame::NONE
                        .fill(tokens.color.surface_elevated)
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .stroke(egui::Stroke::new(1.0, tokens.color.border)),
                )
                .show(ctx, |ui| {
                    status_cmds = statusbar::show_statusbar(
                        ui,
                        &mut self.canvas,
                        &self.statusbar,
                        &self.doc,
                        &self.snap_controller,
                        self.tool_manager.active_id(),
                    );
                });
            for cmd in status_cmds {
                self.dispatch_and_toast(ctx, cmd);
            }
        }

        // 6. Left tool palette
        if self.workspace.show_toolbar {
            let mut tool_cmd = None;
            egui::SidePanel::left("toolbar")
                .exact_width(SpacingTokens::TOOLBAR_WIDTH)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(tokens.color.surface_elevated)
                        .stroke(egui::Stroke::new(1.0, tokens.color.border)),
                )
                .show(ctx, |ui| {
                    tool_cmd = toolbar::show_toolbar(ui, self.tool_manager.active_id());
                });
            if let Some(cmd) = tool_cmd {
                self.dispatch_and_toast(ctx, cmd);
            }
        }

        // 7. Right properties inspector dock
        if self.workspace.show_right_panel {
            let mut dock_cmds = Vec::new();
            egui::SidePanel::right("panels")
                .min_width(220.0)
                .default_width(SpacingTokens::INSPECTOR_WIDTH)
                .frame(
                    egui::Frame::NONE
                        .fill(tokens.color.surface_elevated)
                        .inner_margin(egui::Margin::same(8))
                        .stroke(egui::Stroke::new(1.0, tokens.color.border)),
                )
                .show(ctx, |ui| {
                    dock::show_dock_tabs(ui, &mut self.dock);
                    egui::ScrollArea::vertical()
                        .id_salt("right_dock_scroll")
                        .show(ui, |ui| {
                            dock_cmds = dock::show_dock_content(
                                ui,
                                &mut self.dock,
                                &mut self.doc,
                                &mut self.undo_stack,
                                &mut self.toast,
                            );
                        });
                });
            for cmd in dock_cmds {
                self.dispatch_and_toast(ctx, cmd);
            }
        }

        // 8. Central canvas
        let mut canvas_commands = Vec::new();
        let mut drag_err: Option<CommandError> = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(tokens.color.canvas_bg))
            .show(ctx, |ui| {
                self.canvas.show_grid = self.workspace.show_grid;
                self.canvas.show_rulers = self.workspace.show_rulers;
                self.canvas.cam_config = self.dock.cam_config.clone();

                if self.first_frame {
                    self.first_frame = false;
                    let rect = ui.available_rect_before_wrap();
                    self.canvas.fit_page(rect);
                }

                self.sync_spatial_index_if_dirty();

                let mirror_preview = if self.dock.properties_state.mirror_line_preview {
                    Some((
                        Point2::new(
                            self.dock.properties_state.mirror_line_point_x,
                            self.dock.properties_state.mirror_line_point_y,
                        ),
                        Point2::new(
                            self.dock.properties_state.mirror_line_dir_x,
                            self.dock.properties_state.mirror_line_dir_y,
                        ),
                    ))
                } else {
                    None
                };

                let out = canvas::show_canvas(
                    ui,
                    &mut self.canvas,
                    &mut self.doc,
                    &mut self.undo_stack,
                    &mut self.tool_manager,
                    &mut self.snap_solver,
                    &mut self.snap_controller,
                    &mut self.drag_session,
                    mirror_preview,
                );
                canvas_commands = out.commands;
                drag_err = out.drag_error;

                self.dock.cam_config = self.canvas.cam_config.clone();
            });

        if let Some(err) = drag_err {
            self.surface_command_error(&err);
        }
        for cmd in canvas_commands {
            self.dispatch_and_toast(ctx, cmd);
        }

        // 9. Accessible live SnapFeedback + HandleMode hook for headless/screen-reader inspection
        if let Some(fb) = &self.canvas.last_snap_feedback {
            let hook_text = format!(
                "Snap HUD: {} | Handle: {}",
                fb.target.kind.label(),
                fb.handle_mode.label()
            );
            egui::Area::new(egui::Id::new("snap_feedback_a11y_hook"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .interactable(false)
                .show(ctx, |ui| {
                    let resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&hook_text)
                                .size(1.0)
                                .color(tokens.color.transparent),
                        )
                        .selectable(false),
                    );
                    resp.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &hook_text)
                    });
                });
        }

        // 10. Help modal, Welcome & 3-step Coach Marks, HUD Toasts
        self.render_help_modal(ctx, &tokens);
        self.welcome
            .show(ctx, &mut self.doc, &mut self.canvas, &mut self.toast);
        self.toast.render(ctx);

        self.statusbar.frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
    }
}

impl eframe::App for VDrawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_ui(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        save_theme_pref(storage, self.theme_pref);
        welcome::save_coach_seen(storage, self.welcome.coach_seen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_ui_effect_handling_is_exhaustive() {
        let dir =
            std::env::temp_dir().join(format!("vdraw_test_ui_effects_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let ctx = egui::Context::default();
        let mut app = VDrawApp::new_headless(&ctx);
        app.io_dir = Some(dir.clone());
        let effects = VDrawApp::all_ui_effects();
        assert_eq!(effects.len(), 22);

        for effect in effects {
            let res = app.fulfill_for_test(&ctx, effect);
            assert!(res.is_ok(), "UiEffect handler returned error: {res:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_command_palette_covers_every_app_command() {
        let dir = std::env::temp_dir().join(format!("vdraw_test_palette_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let ctx = egui::Context::default();
        let mut app = VDrawApp::new_headless(&ctx);
        app.io_dir = Some(dir.clone());
        let entries = VDrawApp::command_palette_entries();
        assert!(!entries.is_empty());

        let mut seen_kinds = HashSet::new();
        for entry in &entries {
            seen_kinds.insert(entry.kind);
            // Every palette command dispatches cleanly without panicking (NoSelection is fine when doc is empty)
            let res = app.dispatch_command(entry.command.clone());
            assert!(
                matches!(res, Ok(()) | Err(CommandError::NoSelection)),
                "Unexpected dispatch failure for {:?}: {res:?}",
                entry.command
            );
        }

        for kind in [
            CommandKind::Tool,
            CommandKind::Transform,
            CommandKind::Boolean,
            CommandKind::Edit,
            CommandKind::SnapToggle,
            CommandKind::View,
            CommandKind::File,
        ] {
            assert!(
                seen_kinds.contains(&kind),
                "Missing CommandKind group in palette: {kind:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
