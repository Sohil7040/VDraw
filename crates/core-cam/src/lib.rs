//! Camera / viewport state for VDraw — Phase 0 stub.

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

// ─── Laser CAM Domain Types ──────────────────────────────────────────────────

/// Laser processing operation role assigned to geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LaserRole {
    #[default]
    Cut,
    Score,
    RasterEngrave,
}

/// Material profile configuring speeds, powers, passes, and kerf compensation.
#[derive(Debug, Clone, PartialEq)]
pub struct LaserProfile {
    pub name: String,
    pub cut_speed_mm_s: f64,
    pub cut_power_pct: f64,
    pub cut_passes: u32,
    pub score_speed_mm_s: f64,
    pub score_power_pct: f64,
    pub engrave_speed_mm_s: f64,
    pub engrave_power_pct: f64,
    pub kerf_width_mm: f64,
}

impl Default for LaserProfile {
    fn default() -> Self {
        Self {
            name: "Default 3mm Acrylic".to_string(),
            cut_speed_mm_s: 15.0,
            cut_power_pct: 85.0,
            cut_passes: 1,
            score_speed_mm_s: 40.0,
            score_power_pct: 20.0,
            engrave_speed_mm_s: 300.0,
            engrave_power_pct: 35.0,
            kerf_width_mm: 0.15,
        }
    }
}

/// Kerf offset compensation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerfCompensation {
    None,
    Outside, // Outer contours: +kerf/2
    Inside,  // Inner contours / holes: -kerf/2
}

/// Supported CAM export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamExportFormat {
    GCode,
    Hpgl,
    Svg,
}

/// Stub node for the CAM containment tree (inside-out topological cut ordering).
#[derive(Debug, Clone, Default)]
pub struct ContainmentNode {
    pub shape_id: u64,
    pub children: Vec<ContainmentNode>,
}
