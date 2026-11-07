use crate::operation::{KerfMode, LayerLaserSettings};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Machine table coordinate origin corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum OriginCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl OriginCorner {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TopLeft => "Top-Left",
            Self::TopRight => "Top-Right",
            Self::BottomLeft => "Bottom-Left",
            Self::BottomRight => "Bottom-Right",
        }
    }
}

/// Global production job preparation and machine setup parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserProductionConfig {
    /// Whether toolpath simulation and preview are active.
    pub preview: bool,
    /// Default kerf width offset in mm.
    pub kerf: f64,
    /// Kerf compensation mode (Center, Outside, Inside).
    pub kerf_mode: KerfMode,
    /// Tangent lead-in cut length in mm.
    pub lead_in: f64,
    /// Tangent lead-out cut length in mm.
    pub lead_out: f64,
    /// Corner closure overcut in mm.
    pub overcut: f64,
    /// Optimize rapid head travel sequence (Greedy Nearest Neighbor).
    pub optimize_order: bool,
    /// Cut inner closed holes before outer contours to avoid part drop.
    pub inside_out: bool,
    /// Laser home / origin corner.
    pub origin_corner: OriginCorner,
    /// Laser head rapid travel speed in mm/s (laser OFF).
    pub rapid_speed_mm_s: f64,
    /// Starting laser position X in mm.
    pub start_x: f64,
    /// Starting laser position Y in mm.
    pub start_y: f64,
    /// Whether to show numeric sequence badges (1, 2, 3...) on canvas.
    pub show_sequence_badges: bool,
    /// Layer-specific laser parameters keyed by layer name or ID.
    pub layer_settings: HashMap<String, LayerLaserSettings>,
}

impl Default for LaserProductionConfig {
    fn default() -> Self {
        Self {
            preview: false,
            kerf: 0.15,
            kerf_mode: KerfMode::Center,
            lead_in: 0.0,
            lead_out: 0.0,
            overcut: 0.0,
            optimize_order: true,
            inside_out: true,
            origin_corner: OriginCorner::TopLeft,
            rapid_speed_mm_s: 300.0,
            start_x: 8.0,
            start_y: 8.0,
            show_sequence_badges: true,
            layer_settings: HashMap::new(),
        }
    }
}
