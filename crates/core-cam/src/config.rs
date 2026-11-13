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
    /// Machine motion acceleration for cutting in mm/s^2.
    pub acceleration_mm_s2: f64,
    /// Machine motion acceleration for rapid travel in mm/s^2.
    pub rapid_acceleration_mm_s2: f64,
    /// Time allowance for beam penetration pierce at start of each cut in seconds.
    pub pierce_delay_s: f64,
    /// Machine work area bed width in mm.
    pub work_area_width_mm: f64,
    /// Machine work area bed height in mm.
    pub work_area_height_mm: f64,
    /// Whether to auto-classify operations by shape color if not explicitly assigned.
    pub auto_color_classify: bool,
    /// Whether to show numeric sequence badges (1, 2, 3...) on canvas.
    pub show_sequence_badges: bool,
    /// Layer-specific laser parameters keyed by layer name.
    pub layer_settings: HashMap<String, LayerLaserSettings>,
    /// Object-level laser parameter overrides keyed by ShapeId (u64).
    pub shape_overrides: HashMap<u64, LayerLaserSettings>,
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
            acceleration_mm_s2: 3000.0,
            rapid_acceleration_mm_s2: 5000.0,
            pierce_delay_s: 0.05,
            work_area_width_mm: 600.0,
            work_area_height_mm: 400.0,
            auto_color_classify: false,
            show_sequence_badges: true,
            layer_settings: HashMap::new(),
            shape_overrides: HashMap::new(),
        }
    }
}
