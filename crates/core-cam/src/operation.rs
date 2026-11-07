use serde::{Deserialize, Serialize};

/// Standard Ruida 16-color machine controller palette.
pub const RUIDA_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],      // 0: Black
    [0, 0, 255],    // 1: Blue
    [0, 255, 0],    // 2: Green
    [0, 255, 255],  // 3: Cyan
    [255, 0, 0],    // 4: Red
    [255, 0, 255],  // 5: Magenta
    [255, 255, 0],  // 6: Yellow
    [255, 128, 0],  // 7: Orange
    [128, 0, 128],  // 8: Purple
    [128, 255, 0],  // 9: Lime
    [0, 128, 255],  // 10: Sky Blue
    [0, 128, 128],  // 11: Teal
    [139, 69, 19],  // 12: Brown
    [0, 100, 0],    // 13: Dark Green
    [255, 127, 80], // 14: Coral
    [80, 80, 80],   // 15: Gray
];

/// Laser processing operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LaserMode {
    #[default]
    Cut,
    Score,
    Engrave,
    Dot,
    Pen,
    Ignore,
}

impl LaserMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Score => "Score",
            Self::Engrave => "Engrave (Scan)",
            Self::Dot => "Dot (Perforate)",
            Self::Pen => "Pen (Mark)",
            Self::Ignore => "Ignore",
        }
    }

    pub fn color_rgb(&self) -> [u8; 3] {
        match self {
            Self::Cut => [225, 29, 72],      // Vibrant Red #e11d48
            Self::Score => [5, 150, 105],    // Emerald Green #059669
            Self::Engrave => [2, 132, 199],  // Bright Blue #0284c7
            Self::Dot => [217, 119, 6],      // Amber #d97706
            Self::Pen => [124, 58, 237],     // Violet #7c3aed
            Self::Ignore => [148, 163, 184], // Slate #94a3b8
        }
    }

    /// Execution priority: Dot (0) -> Pen (1) -> Engrave (2) -> Score (3) -> Cut (4).
    pub fn execution_priority(&self) -> u8 {
        match self {
            Self::Dot => 0,
            Self::Pen => 1,
            Self::Engrave => 2,
            Self::Score => 3,
            Self::Cut => 4,
            Self::Ignore => 255,
        }
    }
}

/// Kerf offset compensation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KerfMode {
    #[default]
    Center,
    Outside, // Outer contours offset outward (+kerf / 2)
    Inside,  // Inner contours offset inward (-kerf / 2)
}

/// Comprehensive laser settings for a layer or shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerLaserSettings {
    pub mode: LaserMode,
    pub speed_mm_s: f64,
    pub power_pct: f64,
    pub min_power_pct: f64,
    pub max_power_pct: f64,
    pub passes: u32,
    pub air_assist: bool,
    pub color_index: u8,
    pub kerf_mm: f64,
    pub kerf_mode: KerfMode,
    pub overcut_mm: f64,
    pub lead_in_mm: f64,
    pub lead_out_mm: f64,
    pub micro_link: bool,
    pub micro_link_width_mm: f64,
}

impl LayerLaserSettings {
    pub fn new_cut(speed: f64, power: f64, color_index: u8) -> Self {
        Self {
            mode: LaserMode::Cut,
            speed_mm_s: speed,
            power_pct: power,
            min_power_pct: (power * 0.6).round().clamp(10.0, 100.0),
            max_power_pct: power,
            passes: 1,
            air_assist: true,
            color_index,
            kerf_mm: 0.15,
            kerf_mode: KerfMode::Center,
            overcut_mm: 0.0,
            lead_in_mm: 0.0,
            lead_out_mm: 0.0,
            micro_link: false,
            micro_link_width_mm: 0.8,
        }
    }

    pub fn new_score(speed: f64, power: f64, color_index: u8) -> Self {
        Self {
            mode: LaserMode::Score,
            speed_mm_s: speed,
            power_pct: power,
            min_power_pct: (power * 0.5).round().clamp(10.0, 100.0),
            max_power_pct: power,
            passes: 1,
            air_assist: true,
            color_index,
            kerf_mm: 0.0,
            kerf_mode: KerfMode::Center,
            overcut_mm: 0.0,
            lead_in_mm: 0.0,
            lead_out_mm: 0.0,
            micro_link: false,
            micro_link_width_mm: 0.0,
        }
    }

    pub fn new_engrave(speed: f64, power: f64, color_index: u8) -> Self {
        Self {
            mode: LaserMode::Engrave,
            speed_mm_s: speed,
            power_pct: power,
            min_power_pct: (power * 0.5).round().clamp(10.0, 100.0),
            max_power_pct: power,
            passes: 1,
            air_assist: true,
            color_index,
            kerf_mm: 0.0,
            kerf_mode: KerfMode::Center,
            overcut_mm: 0.0,
            lead_in_mm: 0.0,
            lead_out_mm: 0.0,
            micro_link: false,
            micro_link_width_mm: 0.0,
        }
    }
}

impl Default for LayerLaserSettings {
    fn default() -> Self {
        Self::new_cut(25.0, 80.0, 0)
    }
}

/// Material laser parameter preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub mode: LaserMode,
    pub speed_mm_s: f64,
    pub power_pct: f64,
    pub air_assist: bool,
    pub passes: u32,
    pub kerf_mm: f64,
}

pub const BUILTIN_MATERIAL_PRESETS: &[MaterialPreset] = &[
    MaterialPreset {
        id: "m_ply3",
        name: "Plywood 3mm (Cut)",
        category: "Wood",
        mode: LaserMode::Cut,
        speed_mm_s: 22.0,
        power_pct: 75.0,
        air_assist: true,
        passes: 1,
        kerf_mm: 0.15,
    },
    MaterialPreset {
        id: "m_ply6",
        name: "Plywood 6mm (Cut)",
        category: "Wood",
        mode: LaserMode::Cut,
        speed_mm_s: 12.0,
        power_pct: 88.0,
        air_assist: true,
        passes: 1,
        kerf_mm: 0.18,
    },
    MaterialPreset {
        id: "m_hardwood_engrave",
        name: "Hardwood Surface (Engrave)",
        category: "Wood",
        mode: LaserMode::Engrave,
        speed_mm_s: 300.0,
        power_pct: 30.0,
        air_assist: true,
        passes: 1,
        kerf_mm: 0.0,
    },
    MaterialPreset {
        id: "m_acr3",
        name: "Acrylic 3mm Cast (Cut)",
        category: "Acrylic",
        mode: LaserMode::Cut,
        speed_mm_s: 16.0,
        power_pct: 72.0,
        air_assist: false,
        passes: 1,
        kerf_mm: 0.14,
    },
    MaterialPreset {
        id: "m_acr5",
        name: "Acrylic 5mm Cast (Cut)",
        category: "Acrylic",
        mode: LaserMode::Cut,
        speed_mm_s: 9.0,
        power_pct: 85.0,
        air_assist: false,
        passes: 1,
        kerf_mm: 0.16,
    },
    MaterialPreset {
        id: "m_card2",
        name: "Cardboard / Kraft 2mm (Cut)",
        category: "Paper",
        mode: LaserMode::Cut,
        speed_mm_s: 45.0,
        power_pct: 40.0,
        air_assist: true,
        passes: 1,
        kerf_mm: 0.12,
    },
    MaterialPreset {
        id: "m_leather15",
        name: "Leather 1.5mm (Cut & Mark)",
        category: "Leather",
        mode: LaserMode::Cut,
        speed_mm_s: 30.0,
        power_pct: 50.0,
        air_assist: true,
        passes: 1,
        kerf_mm: 0.13,
    },
];
