//! Modular CNC & Laser Machine Postprocessors.
//!
//! Provides machine-specific output generation for:
//! - Generic G-Code (LinuxCNC, Mach3/4, standard RS-274)
//! - GRBL 1.1f/h ($32=1 laser mode, M4 dynamic laser power, $H homing)
//! - Marlin 2.0 (Laser/Spindle mode)
//! - HPGL / PLT (vinyl plotters, laser markers)

use crate::generator::CamJobPlan;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PostProcessError {
    #[error("Empty job plan: no toolpaths to process")]
    EmptyJob,
    #[error("Postprocessor configuration error: {0}")]
    ConfigError(String),
}

/// Supported CNC/Laser controller dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MachineFlavor {
    #[default]
    GenericGcode,
    Grbl,
    Marlin,
    Hpgl,
}

impl MachineFlavor {
    pub fn label(&self) -> &'static str {
        match self {
            Self::GenericGcode => "Generic G-Code (RS-274)",
            Self::Grbl => "GRBL 1.1f/h (Laser Mode $32=1)",
            Self::Marlin => "Marlin 2.0 Laser",
            Self::Hpgl => "HPGL / PLT Plotter",
        }
    }
}

/// Machine configuration parameters for postprocessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfig {
    pub flavor: MachineFlavor,
    pub homing_on_start: bool,
    pub dynamic_power: bool, // GRBL M4 dynamic laser power
    pub pwm_max: u32,        // 1000 for GRBL, 255 for Marlin
    pub air_assist: bool,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            flavor: MachineFlavor::GenericGcode,
            homing_on_start: false,
            dynamic_power: true,
            pwm_max: 1000,
            air_assist: true,
        }
    }
}

/// Trait implemented by all machine-specific dialect postprocessors.
pub trait PostProcessor {
    fn generate(
        &self,
        plan: &CamJobPlan,
        config: &MachineConfig,
    ) -> Result<String, PostProcessError>;
}

/// Generic RS-274 / LinuxCNC / Mach3 postprocessor.
pub struct GenericGcodePostProcessor;

impl PostProcessor for GenericGcodePostProcessor {
    fn generate(
        &self,
        plan: &CamJobPlan,
        _config: &MachineConfig,
    ) -> Result<String, PostProcessError> {
        let mut out = String::with_capacity(8192);
        out.push_str("; VDraw Laser CAM - Generic G-Code (RS-274)\n");
        out.push_str(&format!(
            "; Total Cut: {:.1} mm | Rapid: {:.1} mm | Est: {:.1} s\n",
            plan.total_cut_length_mm, plan.total_rapid_length_mm, plan.estimated_duration_s
        ));
        out.push_str("G21 ; Millimeter units\n");
        out.push_str("G90 ; Absolute positioning\n");
        out.push_str("M5  ; Laser OFF\n\n");

        for (i, tp) in plan.toolpaths.iter().enumerate() {
            if tp.points.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "; --- Sequence #{}: {} [{}] (Feed: {:.0} mm/s, Power: {:.0}%) ---\n",
                i + 1,
                tp.layer_name,
                tp.mode.label(),
                tp.speed_mm_s,
                tp.power_pct
            ));

            if tp.air_assist {
                out.push_str("M8  ; Air assist ON\n");
            } else {
                out.push_str("M9  ; Air assist OFF\n");
            }

            let start = tp.points[0];
            out.push_str(&format!("G0 X{:.3} Y{:.3}\n", start.x, start.y));

            let pwm = (tp.power_pct * 10.0).round().clamp(0.0, 1000.0) as u32;
            let feed_rate = (tp.speed_mm_s * 60.0).round();
            out.push_str(&format!("M3 S{}\n", pwm));

            for pt in tp.points.iter().skip(1) {
                out.push_str(&format!("G1 X{:.3} Y{:.3} F{:.0}\n", pt.x, pt.y, feed_rate));
            }

            if tp.closed && tp.points.len() >= 3 {
                out.push_str(&format!(
                    "G1 X{:.3} Y{:.3} F{:.0}\n",
                    start.x, start.y, feed_rate
                ));
            }

            out.push_str("M5  ; Laser OFF\n\n");
        }

        out.push_str("; Return to home\n");
        out.push_str("G0 X0 Y0\n");
        out.push_str("M9  ; Air assist OFF\n");
        out.push_str("M2  ; Program End\n");
        Ok(out)
    }
}

/// GRBL 1.1f/h postprocessor with $32=1 laser mode and M4 dynamic power.
pub struct GrblPostProcessor;

impl PostProcessor for GrblPostProcessor {
    fn generate(
        &self,
        plan: &CamJobPlan,
        config: &MachineConfig,
    ) -> Result<String, PostProcessError> {
        let mut out = String::with_capacity(8192);
        out.push_str("; VDraw Laser CAM - GRBL 1.1f/h Postprocessor\n");
        if config.homing_on_start {
            out.push_str("$H  ; Run homing cycle\n");
        }
        out.push_str("$32=1 ; Enable Laser Mode\n");
        out.push_str("G21   ; Metric mm\n");
        out.push_str("G90   ; Absolute positioning\n");
        out.push_str("M5    ; Laser OFF\n\n");

        let laser_cmd = if config.dynamic_power { "M4" } else { "M3" };

        for (i, tp) in plan.toolpaths.iter().enumerate() {
            if tp.points.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "; Sequence #{}: {} ({})\n",
                i + 1,
                tp.layer_name,
                tp.mode.label()
            ));

            if tp.air_assist && config.air_assist {
                out.push_str("M8\n");
            }

            let start = tp.points[0];
            out.push_str(&format!("G0 X{:.3} Y{:.3}\n", start.x, start.y));

            let pwm = (tp.power_pct * (config.pwm_max as f64) / 100.0)
                .round()
                .clamp(0.0, config.pwm_max as f64) as u32;
            let feed_rate = (tp.speed_mm_s * 60.0).round();

            out.push_str(&format!("{} S{}\n", laser_cmd, pwm));

            for pt in tp.points.iter().skip(1) {
                out.push_str(&format!("G1 X{:.3} Y{:.3} F{:.0}\n", pt.x, pt.y, feed_rate));
            }

            if tp.closed && tp.points.len() >= 3 {
                out.push_str(&format!(
                    "G1 X{:.3} Y{:.3} F{:.0}\n",
                    start.x, start.y, feed_rate
                ));
            }

            out.push_str("M5\n\n");
        }

        out.push_str("M5 ; Safety laser off\n");
        out.push_str("M9 ; Air off\n");
        out.push_str("G0 X0 Y0\n");
        Ok(out)
    }
}

/// Marlin 2.0 Laser mode postprocessor.
pub struct MarlinPostProcessor;

impl PostProcessor for MarlinPostProcessor {
    fn generate(
        &self,
        plan: &CamJobPlan,
        config: &MachineConfig,
    ) -> Result<String, PostProcessError> {
        let mut out = String::with_capacity(8192);
        out.push_str("; VDraw Laser CAM - Marlin 2.0 Laser Postprocessor\n");
        out.push_str("G21 ; mm\n");
        out.push_str("G90 ; absolute\n");
        out.push_str("M5  ; laser off\n\n");

        for (i, tp) in plan.toolpaths.iter().enumerate() {
            if tp.points.is_empty() {
                continue;
            }
            out.push_str(&format!("; Op #{}: {}\n", i + 1, tp.layer_name));

            let start = tp.points[0];
            out.push_str(&format!("G0 X{:.3} Y{:.3}\n", start.x, start.y));

            let pwm = (tp.power_pct * (config.pwm_max as f64) / 100.0)
                .round()
                .clamp(0.0, config.pwm_max as f64) as u32;
            let feed = (tp.speed_mm_s * 60.0).round();

            out.push_str(&format!("M3 S{}\n", pwm));

            for pt in tp.points.iter().skip(1) {
                out.push_str(&format!("G1 X{:.3} Y{:.3} F{:.0}\n", pt.x, pt.y, feed));
            }

            if tp.closed && tp.points.len() >= 3 {
                out.push_str(&format!(
                    "G1 X{:.3} Y{:.3} F{:.0}\n",
                    start.x, start.y, feed
                ));
            }

            out.push_str("M5\n\n");
        }

        out.push_str("M5\n");
        out.push_str("G0 X0 Y0\n");
        Ok(out)
    }
}

/// HPGL / PLT vinyl and laser marker postprocessor.
pub struct HpglPostProcessor;

impl PostProcessor for HpglPostProcessor {
    fn generate(
        &self,
        plan: &CamJobPlan,
        _config: &MachineConfig,
    ) -> Result<String, PostProcessError> {
        let mut out = String::with_capacity(4096);
        out.push_str("IN;\n");
        out.push_str("SP1;\n");

        // 40 plotter units per mm (standard 1016 DPI)
        let scale = 40.0;

        for tp in &plan.toolpaths {
            if tp.points.is_empty() {
                continue;
            }
            let p0 = tp.points[0];
            out.push_str(&format!("PU{:.0},{:.0};\n", p0.x * scale, p0.y * scale));

            for pt in tp.points.iter().skip(1) {
                out.push_str(&format!("PD{:.0},{:.0};\n", pt.x * scale, pt.y * scale));
            }
            if tp.closed && tp.points.len() >= 3 {
                out.push_str(&format!("PD{:.0},{:.0};\n", p0.x * scale, p0.y * scale));
            }
            out.push_str("PU;\n");
        }

        out.push_str("SP0;\n");
        Ok(out)
    }
}

/// Helper function to dispatch export by configured machine flavor.
pub fn post_process(plan: &CamJobPlan, config: &MachineConfig) -> Result<String, PostProcessError> {
    match config.flavor {
        MachineFlavor::GenericGcode => GenericGcodePostProcessor.generate(plan, config),
        MachineFlavor::Grbl => GrblPostProcessor.generate(plan, config),
        MachineFlavor::Marlin => MarlinPostProcessor.generate(plan, config),
        MachineFlavor::Hpgl => HpglPostProcessor.generate(plan, config),
    }
}
