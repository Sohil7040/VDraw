//! Frozen exhaustive command surface shared by menus, toolbars, context menu, and Ctrl+K palette.

use crate::tool::ToolId;
use core_boolean::{BooleanOp, ContourJoin};
use core_geom::Point2;
use core_snap::{HandleMode, SnapKind};

/// Persisted UI appearance theme preference wrapping `egui::ThemePreference` (`System | Dark | Light`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemePref(pub egui::ThemePreference);

impl Default for ThemePref {
    fn default() -> Self {
        Self(egui::ThemePreference::System)
    }
}

/// High-level command grouping used by the command palette and menu dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    Tool,
    Transform,
    Boolean,
    SnapToggle,
    View,
    File,
    Edit,
}

/// Exhaustive set of user-invocable application commands.
///
/// Every toolbar button, top bar action, context-menu item, snap toggle, transform,
/// boolean op, and command-palette entry maps directly to one of these variants.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AppCommand {
    // ── File & Export ────────────────────────────────────────────────────────
    NewDocument,
    OpenDocument,
    SaveDocument,
    SaveDocumentAs,
    ImportSvg,
    ExportSvg,
    ExportDxf,
    ExportPng,
    ExportGcode,

    // ── Edit & Undo History ──────────────────────────────────────────────────
    Undo,
    Redo,
    JumpToHistoryIndex(usize),
    DuplicateSelection,
    DeleteSelection,
    SelectAll,
    DeselectAll,

    // ── Tools (every ToolId variant) ─────────────────────────────────────────
    ActivateTool(ToolId),

    // ── Transforms (2D Planar) ───────────────────────────────────────────────
    TranslateSelection {
        delta: Point2,
    },
    RotateSelection {
        pivot: Option<Point2>,
        angle_deg: f64,
    },
    MirrorSelectionHorizontal,
    MirrorSelectionVertical,
    MirrorSelectionLine {
        point: Point2,
        dir: Point2,
    },
    ScaleSelection {
        pivot: Option<Point2>,
        sx: f64,
        sy: f64,
    },

    // ── Boolean & Path Operations ────────────────────────────────────────────
    BooleanOp(BooleanOp),
    OffsetSelection {
        distance_mm: f64,
        join: ContourJoin,
    },
    ConvertTextToPath {
        text: String,
        size_mm: f64,
    },
    GroupSelection,
    UngroupSelection,
    CombinePaths,
    BreakApartPaths,

    // ── Snap System & Constraints ────────────────────────────────────────────
    ToggleSnapMaster,
    ToggleSnapKind(SnapKind),
    SetHandleMode(HandleMode),
    ToggleSnapOrtho,
    ToggleSnapPolar,
    SetPolarStepDeg(f64),
    ToggleAlignmentGuides,
    ToggleGuidelineSnap,
    ClearGuidelines,

    // ── Viewport, Theme & Help ───────────────────────────────────────────────
    ZoomIn,
    ZoomOut,
    FitPage,
    ZoomToSelection,
    ToggleGrid,
    ToggleRulers,
    SetTheme(ThemePref),
    SwitchWorkflowDesign,
    SwitchWorkflowLaserCam,
    SwitchWorkflowPreview,
    OpenCommandPalette,
    OpenHelp,
    StartCoachMarks,
}

impl AppCommand {
    /// Stable grouping for palette ranking, telemetry-free routing, and menu categorization.
    pub const fn kind(&self) -> CommandKind {
        match self {
            AppCommand::NewDocument
            | AppCommand::OpenDocument
            | AppCommand::SaveDocument
            | AppCommand::SaveDocumentAs
            | AppCommand::ImportSvg
            | AppCommand::ExportSvg
            | AppCommand::ExportDxf
            | AppCommand::ExportPng
            | AppCommand::ExportGcode => CommandKind::File,

            AppCommand::Undo
            | AppCommand::Redo
            | AppCommand::JumpToHistoryIndex(_)
            | AppCommand::DuplicateSelection
            | AppCommand::DeleteSelection
            | AppCommand::SelectAll
            | AppCommand::DeselectAll
            | AppCommand::GroupSelection
            | AppCommand::UngroupSelection
            | AppCommand::CombinePaths
            | AppCommand::BreakApartPaths
            | AppCommand::ConvertTextToPath { .. } => CommandKind::Edit,

            AppCommand::ActivateTool(_) => CommandKind::Tool,

            AppCommand::TranslateSelection { .. }
            | AppCommand::RotateSelection { .. }
            | AppCommand::MirrorSelectionHorizontal
            | AppCommand::MirrorSelectionVertical
            | AppCommand::MirrorSelectionLine { .. }
            | AppCommand::ScaleSelection { .. } => CommandKind::Transform,

            AppCommand::BooleanOp(_) | AppCommand::OffsetSelection { .. } => CommandKind::Boolean,

            AppCommand::ToggleSnapMaster
            | AppCommand::ToggleSnapKind(_)
            | AppCommand::SetHandleMode(_)
            | AppCommand::ToggleSnapOrtho
            | AppCommand::ToggleSnapPolar
            | AppCommand::SetPolarStepDeg(_)
            | AppCommand::ToggleAlignmentGuides
            | AppCommand::ToggleGuidelineSnap
            | AppCommand::ClearGuidelines => CommandKind::SnapToggle,

            AppCommand::ZoomIn
            | AppCommand::ZoomOut
            | AppCommand::FitPage
            | AppCommand::ZoomToSelection
            | AppCommand::ToggleGrid
            | AppCommand::ToggleRulers
            | AppCommand::SetTheme(_)
            | AppCommand::SwitchWorkflowDesign
            | AppCommand::SwitchWorkflowLaserCam
            | AppCommand::SwitchWorkflowPreview
            | AppCommand::OpenCommandPalette
            | AppCommand::OpenHelp
            | AppCommand::StartCoachMarks => CommandKind::View,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_command_has_kind() {
        let all = [
            AppCommand::NewDocument,
            AppCommand::OpenDocument,
            AppCommand::SaveDocument,
            AppCommand::SaveDocumentAs,
            AppCommand::ImportSvg,
            AppCommand::ExportSvg,
            AppCommand::ExportDxf,
            AppCommand::ExportPng,
            AppCommand::ExportGcode,
            AppCommand::Undo,
            AppCommand::Redo,
            AppCommand::JumpToHistoryIndex(0),
            AppCommand::DuplicateSelection,
            AppCommand::DeleteSelection,
            AppCommand::SelectAll,
            AppCommand::DeselectAll,
            AppCommand::ActivateTool(ToolId::Pick),
            AppCommand::TranslateSelection {
                delta: Point2::ZERO,
            },
            AppCommand::RotateSelection {
                pivot: None,
                angle_deg: 90.0,
            },
            AppCommand::MirrorSelectionHorizontal,
            AppCommand::MirrorSelectionVertical,
            AppCommand::MirrorSelectionLine {
                point: Point2::ZERO,
                dir: Point2::new(1.0, 0.0),
            },
            AppCommand::ScaleSelection {
                pivot: None,
                sx: 1.0,
                sy: 1.0,
            },
            AppCommand::BooleanOp(BooleanOp::Weld),
            AppCommand::OffsetSelection {
                distance_mm: 1.0,
                join: ContourJoin::Round,
            },
            AppCommand::ConvertTextToPath {
                text: String::new(),
                size_mm: 10.0,
            },
            AppCommand::GroupSelection,
            AppCommand::UngroupSelection,
            AppCommand::CombinePaths,
            AppCommand::BreakApartPaths,
            AppCommand::ToggleSnapMaster,
            AppCommand::ToggleSnapKind(SnapKind::Node),
            AppCommand::SetHandleMode(HandleMode::Node),
            AppCommand::ToggleSnapOrtho,
            AppCommand::ToggleSnapPolar,
            AppCommand::SetPolarStepDeg(15.0),
            AppCommand::ToggleAlignmentGuides,
            AppCommand::ToggleGuidelineSnap,
            AppCommand::ClearGuidelines,
            AppCommand::ZoomIn,
            AppCommand::ZoomOut,
            AppCommand::FitPage,
            AppCommand::ZoomToSelection,
            AppCommand::ToggleGrid,
            AppCommand::ToggleRulers,
            AppCommand::SetTheme(ThemePref::default()),
            AppCommand::SwitchWorkflowDesign,
            AppCommand::SwitchWorkflowLaserCam,
            AppCommand::SwitchWorkflowPreview,
            AppCommand::OpenCommandPalette,
            AppCommand::OpenHelp,
            AppCommand::StartCoachMarks,
        ];

        for cmd in &all {
            // Exhaustive structural check guaranteeing every variant belongs to a CommandKind.
            let expected = match cmd {
                AppCommand::NewDocument
                | AppCommand::OpenDocument
                | AppCommand::SaveDocument
                | AppCommand::SaveDocumentAs
                | AppCommand::ImportSvg
                | AppCommand::ExportSvg
                | AppCommand::ExportDxf
                | AppCommand::ExportPng
                | AppCommand::ExportGcode => CommandKind::File,

                AppCommand::Undo
                | AppCommand::Redo
                | AppCommand::JumpToHistoryIndex(_)
                | AppCommand::DuplicateSelection
                | AppCommand::DeleteSelection
                | AppCommand::SelectAll
                | AppCommand::DeselectAll
                | AppCommand::GroupSelection
                | AppCommand::UngroupSelection
                | AppCommand::CombinePaths
                | AppCommand::BreakApartPaths
                | AppCommand::ConvertTextToPath { .. } => CommandKind::Edit,

                AppCommand::ActivateTool(_) => CommandKind::Tool,

                AppCommand::TranslateSelection { .. }
                | AppCommand::RotateSelection { .. }
                | AppCommand::MirrorSelectionHorizontal
                | AppCommand::MirrorSelectionVertical
                | AppCommand::MirrorSelectionLine { .. }
                | AppCommand::ScaleSelection { .. } => CommandKind::Transform,

                AppCommand::BooleanOp(_) | AppCommand::OffsetSelection { .. } => {
                    CommandKind::Boolean
                }

                AppCommand::ToggleSnapMaster
                | AppCommand::ToggleSnapKind(_)
                | AppCommand::SetHandleMode(_)
                | AppCommand::ToggleSnapOrtho
                | AppCommand::ToggleSnapPolar
                | AppCommand::SetPolarStepDeg(_)
                | AppCommand::ToggleAlignmentGuides
                | AppCommand::ToggleGuidelineSnap
                | AppCommand::ClearGuidelines => CommandKind::SnapToggle,

                AppCommand::ZoomIn
                | AppCommand::ZoomOut
                | AppCommand::FitPage
                | AppCommand::ZoomToSelection
                | AppCommand::ToggleGrid
                | AppCommand::ToggleRulers
                | AppCommand::SetTheme(_)
                | AppCommand::SwitchWorkflowDesign
                | AppCommand::SwitchWorkflowLaserCam
                | AppCommand::SwitchWorkflowPreview
                | AppCommand::OpenCommandPalette
                | AppCommand::OpenHelp
                | AppCommand::StartCoachMarks => CommandKind::View,
            };
            assert_eq!(cmd.kind(), expected);
        }
    }
}
