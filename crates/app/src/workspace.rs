//! Workspace preset enum and state.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Available workspace layout presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkspacePreset {
    #[default]
    Illustration,
    LaserCutting,
    TechnicalDrawing,
}

impl WorkspacePreset {
    pub fn label(self) -> &'static str {
        match self {
            WorkspacePreset::Illustration => "Illustration",
            WorkspacePreset::LaserCutting => "Laser Cutting",
            WorkspacePreset::TechnicalDrawing => "Technical Drawing",
        }
    }

    pub const ALL: &'static [WorkspacePreset] = &[
        WorkspacePreset::Illustration,
        WorkspacePreset::LaserCutting,
        WorkspacePreset::TechnicalDrawing,
    ];
}

/// Mutable workspace UI state derived from a preset.
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub preset: WorkspacePreset,
    pub show_toolbar: bool,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_right_panel: bool,
    pub show_prop_bar: bool,
    pub show_status_bar: bool,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::from_preset(WorkspacePreset::default())
    }
}

impl WorkspaceState {
    pub fn from_preset(preset: WorkspacePreset) -> Self {
        match preset {
            WorkspacePreset::Illustration => Self {
                preset,
                show_toolbar: true,
                show_rulers: true,
                show_grid: false,
                show_right_panel: true,
                show_prop_bar: true,
                show_status_bar: true,
            },
            WorkspacePreset::LaserCutting => Self {
                preset,
                show_toolbar: true,
                show_rulers: true,
                show_grid: true,
                show_right_panel: true,
                show_prop_bar: true,
                show_status_bar: true,
            },
            WorkspacePreset::TechnicalDrawing => Self {
                preset,
                show_toolbar: true,
                show_rulers: true,
                show_grid: true,
                show_right_panel: false,
                show_prop_bar: true,
                show_status_bar: true,
            },
        }
    }

    /// Apply preset without losing current fine-tuned overrides.
    pub fn apply_preset(&mut self, preset: WorkspacePreset) {
        *self = Self::from_preset(preset);
    }
}
