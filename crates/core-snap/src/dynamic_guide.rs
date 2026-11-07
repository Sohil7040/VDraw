use crate::target::DynamicRay;
use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Settings for dynamic polar angle guides radiating from an origin point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicGuideConfig {
    pub enabled: bool,
    /// Preset angles in degrees to snap against.
    /// Default: [0°, 15°, 30°, 45°, 60°, 75°, 90°, 105°, 120°, 135°, 150°, 165°, 180°, etc.]
    pub angles_deg: Vec<f64>,
    /// Angular tolerance in degrees (default: 2.5°).
    pub angular_tolerance_deg: f64,
    /// Distance tick interval in mm along the ray (e.g. every 5.0 mm).
    pub tick_interval_mm: f64,
    /// Minimum cursor distance from origin in mm before dynamic guide engages.
    pub min_distance_mm: f64,
}

impl Default for DynamicGuideConfig {
    fn default() -> Self {
        // 15-degree steps around the circle: 0, 15, 30, 45, 60, 75, 90, ...
        let angles_deg = (0..24).map(|i| (i * 15) as f64).collect();
        Self {
            enabled: true,
            angles_deg,
            angular_tolerance_deg: 2.5,
            tick_interval_mm: 5.0,
            min_distance_mm: 1.5,
        }
    }
}

impl DynamicGuideConfig {
    /// Attempt to snap `cursor` to a polar ray radiating from `origin`.
    /// Returns the snapped point, distance to ray, and ray metadata if within tolerance.
    pub fn snap(
        &self,
        origin: Point2,
        cursor: Point2,
        tolerance_world: f64,
    ) -> Option<(Point2, f64, DynamicRay)> {
        if !self.enabled {
            return None;
        }

        let dx = cursor.x - origin.x;
        let dy = cursor.y - origin.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < self.min_distance_mm {
            return None;
        }

        let angle_rad = dy.atan2(dx);
        let mut angle_deg = angle_rad.to_degrees();
        if angle_deg < 0.0 {
            angle_deg += 360.0;
        }

        // Find the closest preset angle
        let mut best_angle_deg = None;
        let mut min_deg_diff = f64::MAX;

        for &target_deg in &self.angles_deg {
            let diff = (angle_deg - target_deg).abs();
            let wrap_diff = (360.0 - diff).abs();
            let actual_diff = diff.min(wrap_diff);

            if actual_diff <= self.angular_tolerance_deg && actual_diff < min_deg_diff {
                min_deg_diff = actual_diff;
                best_angle_deg = Some(target_deg);
            }
        }

        let chosen_deg = best_angle_deg?;
        let chosen_rad = chosen_deg.to_radians();

        let unit_x = chosen_rad.cos();
        let unit_y = chosen_rad.sin();

        // Project cursor onto the ray: s = (cursor - origin) . unit
        let projected_dist = (dx * unit_x + dy * unit_y).max(0.0);
        let mut snapped_point = Point2::new(
            origin.x + projected_dist * unit_x,
            origin.y + projected_dist * unit_y,
        );

        // Check perpendicular distance from raw cursor to projected point
        let perp_dist = cursor.distance(snapped_point);
        if perp_dist > tolerance_world {
            return None;
        }

        // Check for distance tick snap along the ray
        let mut tick_point = None;
        if self.tick_interval_mm > 0.0 {
            let nearest_tick_dist =
                (projected_dist / self.tick_interval_mm).round() * self.tick_interval_mm;
            let along_ray_error = (projected_dist - nearest_tick_dist).abs();

            if along_ray_error <= tolerance_world && nearest_tick_dist >= self.min_distance_mm {
                snapped_point = Point2::new(
                    origin.x + nearest_tick_dist * unit_x,
                    origin.y + nearest_tick_dist * unit_y,
                );
                tick_point = Some(snapped_point);
            }
        }

        let total_dist = cursor.distance(snapped_point);

        let dynamic_ray = DynamicRay {
            origin,
            angle_rad: chosen_rad,
            angle_deg: chosen_deg,
            distance_mm: snapped_point.distance(origin),
            tick_point,
        };

        Some((snapped_point, total_dist, dynamic_ray))
    }
}
