use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Visual display style for the canvas grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridStyle {
    Dots,
    Lines,
}

/// Configuration settings for the coordinate background grid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridConfig {
    pub enabled: bool,
    /// Distance between major grid lines (in mm, default 10.0 mm).
    pub major_spacing_mm: f64,
    /// Number of minor subdivisions per major interval (default 5 => 2.0 mm minor).
    pub subdivisions: usize,
    /// Grid origin point in world coordinates.
    pub origin: Point2,
    /// Whether to snap to minor subdivisions as well as major points.
    pub snap_to_minor: bool,
    /// Display style for rendering.
    pub style: GridStyle,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            major_spacing_mm: 10.0,
            subdivisions: 5,
            origin: Point2::ZERO,
            snap_to_minor: true,
            style: GridStyle::Lines,
        }
    }
}

impl GridConfig {
    /// Return the active snap step in millimeters.
    #[inline]
    pub fn snap_step_mm(&self) -> f64 {
        if self.snap_to_minor && self.subdivisions > 0 {
            self.major_spacing_mm / (self.subdivisions as f64)
        } else {
            self.major_spacing_mm
        }
    }

    /// Snap a query point to the nearest grid intersection within `tolerance_mm`.
    pub fn snap_point(&self, p: Point2, tolerance_mm: f64) -> Option<(Point2, f64)> {
        if !self.enabled {
            return None;
        }

        let step = self.snap_step_mm();
        if step <= 1e-6 {
            return None;
        }

        let rel_x = p.x - self.origin.x;
        let rel_y = p.y - self.origin.y;

        let snapped_x = self.origin.x + (rel_x / step).round() * step;
        let snapped_y = self.origin.y + (rel_y / step).round() * step;

        let snapped = Point2::new(snapped_x, snapped_y);
        let dist = p.distance(snapped);

        if dist <= tolerance_mm {
            Some((snapped, dist))
        } else {
            None
        }
    }
}
