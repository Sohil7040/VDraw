//! Industrial Laser Motion Profiling & Job Estimation Engine.
//!
//! Models trapezoidal acceleration/deceleration kinematics, rapid moves,
//! pierce dwell times, and multi-pass operations to compute highly accurate job estimates.

use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Detailed job estimation metrics distinguishing kinematic simulations from controller telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CamJobEstimation {
    /// Total active laser cut/mark path length in mm.
    pub total_cut_length_mm: f64,
    /// Total laser-off rapid positioning head travel in mm.
    pub total_rapid_length_mm: f64,
    /// Total raster engraving travel in mm.
    pub total_raster_length_mm: f64,
    /// Total travel across all modes in mm.
    pub total_distance_mm: f64,
    /// Time spent actively cutting/marking with laser ON in seconds.
    pub cut_duration_s: f64,
    /// Time spent positioning with laser OFF in seconds.
    pub rapid_duration_s: f64,
    /// Time spent in raster engraving in seconds.
    pub raster_duration_s: f64,
    /// Total estimated job time in seconds.
    pub total_duration_s: f64,
    /// Number of discrete laser pierce penetrations.
    pub pierce_count: usize,
    /// Whether this estimate was verified via hardware controller lookahead telemetry.
    pub is_controller_verified: bool,
}

impl Default for CamJobEstimation {
    fn default() -> Self {
        Self {
            total_cut_length_mm: 0.0,
            total_rapid_length_mm: 0.0,
            total_raster_length_mm: 0.0,
            total_distance_mm: 0.0,
            cut_duration_s: 0.0,
            rapid_duration_s: 0.0,
            raster_duration_s: 0.0,
            total_duration_s: 0.0,
            pierce_count: 0,
            is_controller_verified: false,
        }
    }
}

/// Calculate duration of a straight linear move under trapezoidal acceleration kinematics.
pub fn trapezoidal_move_time(distance_mm: f64, target_speed_mm_s: f64, accel_mm_s2: f64) -> f64 {
    if distance_mm <= 1e-6 || target_speed_mm_s <= 1e-6 || accel_mm_s2 <= 1e-6 {
        return 0.0;
    }

    // Distance required to accelerate from 0 to target_speed and decelerate back to 0
    let ramp_dist = (target_speed_mm_s * target_speed_mm_s) / accel_mm_s2;

    if distance_mm >= ramp_dist {
        // Trapezoid profile: reaches target_speed and cruises
        let t_acc = target_speed_mm_s / accel_mm_s2;
        let t_dec = t_acc;
        let cruise_dist = distance_mm - ramp_dist;
        let t_cruise = cruise_dist / target_speed_mm_s;
        t_acc + t_cruise + t_dec
    } else {
        // Triangle profile: short move, never reaches target_speed
        2.0 * (distance_mm / accel_mm_s2).sqrt()
    }
}

/// Calculate path execution time over a polyline with trapezoidal motion profile and junction deceleration.
pub fn calculate_path_time(
    points: &[Point2],
    closed: bool,
    feed_speed_mm_s: f64,
    accel_mm_s2: f64,
) -> (f64, f64) {
    if points.len() < 2 {
        return (0.0, 0.0);
    }

    let mut total_len = 0.0;
    let mut total_time = 0.0;

    for i in 1..points.len() {
        let seg_len = points[i - 1].distance(points[i]);
        if seg_len > 1e-6 {
            total_len += seg_len;
            total_time += trapezoidal_move_time(seg_len, feed_speed_mm_s, accel_mm_s2);
        }
    }

    if closed && points.len() >= 3 {
        let closing_len = points.last().unwrap().distance(points[0]);
        if closing_len > 1e-6 {
            total_len += closing_len;
            total_time += trapezoidal_move_time(closing_len, feed_speed_mm_s, accel_mm_s2);
        }
    }

    (total_len, total_time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trapezoidal_motion_profile() {
        let speed = 100.0; // 100 mm/s
        let accel = 1000.0; // 1000 mm/s^2
                            // ramp_dist = 100^2 / 1000 = 10 mm.

        // Case 1: Long move (110 mm)
        // t_ramp = 100 / 1000 = 0.1s acc + 0.1s dec = 0.2s
        // cruise = 100 mm / 100 mm/s = 1.0s
        // total = 1.2s
        let t_long = trapezoidal_move_time(110.0, speed, accel);
        assert!((t_long - 1.2).abs() < 1e-3);

        // Case 2: Short move (2.5 mm < 10 mm)
        // peak velocity = sqrt(2.5 * 1000) = 50 mm/s
        // total time = 2 * sqrt(2.5 / 1000) = 2 * 0.05 = 0.1s
        let t_short = trapezoidal_move_time(2.5, speed, accel);
        assert!((t_short - 0.1).abs() < 1e-3);
    }
}
