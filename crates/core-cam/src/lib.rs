//! Pure Rust Laser CAM (Computer-Aided Manufacturing) Engine for VDraw.
//!
//! Provides production-ready laser toolpath generation, topological nesting containment
//! (Inside-Out cut ordering), travel path optimization (Greedy Nearest Neighbor),
//! material presets, kerf compensation, raster engraving pipeline, motion profiling
//! and dialect postprocessors (Generic G-Code, GRBL 1.1f/h, Marlin 2.0, HPGL-PLT).

pub mod config;
pub mod estimator;
pub mod generator;
pub mod kerf;
pub mod operation;
pub mod ordering;
pub mod postprocessor;
pub mod raster;
pub mod safety;

pub use config::{LaserProductionConfig, OriginCorner};
pub use estimator::{trapezoidal_move_time, CamJobEstimation};
pub use generator::{generate_job_plan, CamJobPlan, Toolpath};
pub use kerf::{apply_kerf_compensation, KerfError};
pub use operation::{
    classify_color_to_mode, KerfMode, LaserMode, LayerLaserSettings, MaterialPreset,
    BUILTIN_MATERIAL_PRESETS, RUIDA_PALETTE,
};
pub use ordering::{
    calculate_containment_depths, optimize_cutting_order, CamContour, CamDependencyGraph,
};
pub use postprocessor::{
    post_process, GenericGcodePostProcessor, GrblPostProcessor, HpglPostProcessor, MachineConfig,
    MachineFlavor, MarlinPostProcessor, PostProcessError, PostProcessor,
};
pub use raster::{
    generate_raster_plan, DitherAlgorithm, RasterImage, RasterJobPlan, RasterScanline, RasterSpan,
};
pub use safety::{MachineLimits, SafetyIssue, SafetyReport, SimulatedController};

use core_geom::{Affine2, Point2};

/// The camera describes the view transform from document space to screen space.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Pan offset in screen pixels.
    pub pan: [f64; 2],
    /// Zoom scale factor (1.0 = 100%).
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub const MIN_ZOOM: f64 = 0.01;
    pub const MAX_ZOOM: f64 = 64.0;

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Convert a document-space point (mm) to screen-space pixels.
    pub fn doc_to_screen(&self, p: Point2, pixels_per_mm: f64) -> Point2 {
        Point2::new(
            p.x * self.zoom * pixels_per_mm + self.pan[0],
            p.y * self.zoom * pixels_per_mm + self.pan[1],
        )
    }

    /// Convert a screen-space pixel to a document-space point (mm).
    pub fn screen_to_doc(&self, p: Point2, pixels_per_mm: f64) -> Point2 {
        Point2::new(
            (p.x - self.pan[0]) / (self.zoom * pixels_per_mm),
            (p.y - self.pan[1]) / (self.zoom * pixels_per_mm),
        )
    }

    /// Build an `Affine2` transform for this camera.
    pub fn to_affine(&self, pixels_per_mm: f64) -> Affine2 {
        let scale = self.zoom * pixels_per_mm;
        Affine2 {
            matrix: [scale, 0.0, 0.0, scale],
            translation: self.pan,
        }
    }
}

/// Supported CAM export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamExportFormat {
    GCode,
    Hpgl,
    Svg,
}

/// Node for the CAM containment tree (inside-out topological cut ordering).
#[derive(Debug, Clone, Default)]
pub struct ContainmentNode {
    pub shape_id: u64,
    pub children: Vec<ContainmentNode>,
}
