//! Snap solver for VDraw — Phase 0 stub.

use core_geom::Point2;

/// Snap configuration flags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapConfig {
    pub enabled: bool,
    pub snap_to_grid: bool,
    pub snap_to_guides: bool,
    pub snap_to_objects: bool,
    pub grid_spacing_mm: f64,
    pub tolerance_px: f64,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            snap_to_grid: true,
            snap_to_guides: true,
            snap_to_objects: true,
            grid_spacing_mm: 1.0,
            tolerance_px: 8.0,
        }
    }
}

/// Result of a snap operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapResult {
    /// The snapped point (in document coordinates, mm).
    pub point: Point2,
    /// Whether snap actually occurred.
    pub snapped: bool,
}

/// The snap solver resolves cursor positions against snap targets.
#[derive(Debug, Clone, Default)]
pub struct SnapSolver {
    pub config: SnapConfig,
}

impl SnapSolver {
    pub fn new(config: SnapConfig) -> Self {
        Self { config }
    }

    /// Attempt to snap `cursor_mm` given the current configuration.
    /// Returns the (possibly snapped) result.
    pub fn snap(&self, cursor_mm: Point2) -> SnapResult {
        if !self.config.enabled || !self.config.snap_to_grid {
            return SnapResult {
                point: cursor_mm,
                snapped: false,
            };
        }

        let g = self.config.grid_spacing_mm;
        let snapped = Point2::new((cursor_mm.x / g).round() * g, (cursor_mm.y / g).round() * g);

        SnapResult {
            point: snapped,
            snapped: true,
        }
    }
}
