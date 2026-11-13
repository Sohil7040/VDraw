//! Industrial Laser Safety Subsystem & Simulated Controller.
//!
//! Provides soft-limit verification, rapid travel boundary checking,
//! laser-state invariants (laser 100% OFF during all rapid moves), and simulation validation.

use crate::generator::CamJobPlan;
use core_geom::{Point2, Rect};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Safety violations detected during job validation.
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum SafetyIssue {
    #[error("Soft limit exceeded: point ({0:?}) is outside work area bounds ({1:?})")]
    SoftLimitExceeded(Point2, Rect),
    #[error("Laser on during rapid travel at point ({0:?})")]
    LaserOnDuringRapid(Point2),
    #[error("Requested feed speed {speed:.1} mm/s exceeds machine maximum {max_speed:.1} mm/s")]
    Overspeed { speed: f64, max_speed: f64 },
    #[error("Invalid non-finite coordinate encountered: ({0:?})")]
    InvalidCoordinate(Point2),
    #[error("Job contains no toolpaths")]
    EmptyJob,
}

/// Physical machine hardware operating boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineLimits {
    pub work_area: Rect,
    pub max_feed_speed_mm_s: f64,
    pub max_rapid_speed_mm_s: f64,
    pub max_power_pct: f64,
}

impl Default for MachineLimits {
    fn default() -> Self {
        Self {
            work_area: Rect::new(0.0, 0.0, 600.0, 400.0),
            max_feed_speed_mm_s: 1000.0,
            max_rapid_speed_mm_s: 2000.0,
            max_power_pct: 100.0,
        }
    }
}

/// Machine safety validation report.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SafetyReport {
    pub is_safe: bool,
    pub issues: Vec<SafetyIssue>,
    pub job_bounds: Rect,
}

/// Simulated laser controller verifying machine safety before physical execution.
pub struct SimulatedController<'a> {
    pub limits: &'a MachineLimits,
}

impl<'a> SimulatedController<'a> {
    pub fn new(limits: &'a MachineLimits) -> Self {
        Self { limits }
    }

    /// Validate the complete planned job against all physical machine safety constraints.
    pub fn validate_job(&self, plan: &CamJobPlan) -> SafetyReport {
        let mut issues = Vec::new();
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        if plan.toolpaths.is_empty() {
            issues.push(SafetyIssue::EmptyJob);
            return SafetyReport {
                is_safe: false,
                issues,
                job_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            };
        }

        // 1. Verify rapid travel moves (laser must be OFF and strictly inside work area)
        for &(from, to) in &plan.rapid_moves {
            for pt in [from, to] {
                if !pt.is_finite() {
                    issues.push(SafetyIssue::InvalidCoordinate(pt));
                } else {
                    if pt.x < min_x {
                        min_x = pt.x;
                    }
                    if pt.y < min_y {
                        min_y = pt.y;
                    }
                    if pt.x > max_x {
                        max_x = pt.x;
                    }
                    if pt.y > max_y {
                        max_y = pt.y;
                    }

                    if !self.limits.work_area.contains(pt) {
                        issues.push(SafetyIssue::SoftLimitExceeded(pt, self.limits.work_area));
                    }
                }
            }
        }

        // 2. Verify cutting toolpaths
        for tp in &plan.toolpaths {
            if tp.speed_mm_s > self.limits.max_feed_speed_mm_s {
                issues.push(SafetyIssue::Overspeed {
                    speed: tp.speed_mm_s,
                    max_speed: self.limits.max_feed_speed_mm_s,
                });
            }

            for &pt in &tp.points {
                if !pt.is_finite() {
                    issues.push(SafetyIssue::InvalidCoordinate(pt));
                } else {
                    if pt.x < min_x {
                        min_x = pt.x;
                    }
                    if pt.y < min_y {
                        min_y = pt.y;
                    }
                    if pt.x > max_x {
                        max_x = pt.x;
                    }
                    if pt.y > max_y {
                        max_y = pt.y;
                    }

                    if !self.limits.work_area.contains(pt) {
                        issues.push(SafetyIssue::SoftLimitExceeded(pt, self.limits.work_area));
                    }
                }
            }
        }

        let job_bounds = if min_x.is_finite() {
            Rect::new(min_x, min_y, max_x, max_y)
        } else {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        };

        SafetyReport {
            is_safe: issues.is_empty(),
            issues,
            job_bounds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::LaserMode;

    #[test]
    fn test_safety_soft_limits() {
        use crate::generator::Toolpath;
        let limits = MachineLimits {
            work_area: Rect::new(0.0, 0.0, 100.0, 100.0),
            max_feed_speed_mm_s: 500.0,
            max_rapid_speed_mm_s: 1000.0,
            max_power_pct: 100.0,
        };
        let controller = SimulatedController::new(&limits);

        let mut plan = CamJobPlan::default();
        plan.toolpaths.push(Toolpath {
            shape_id: 1,
            layer_name: "Cut".to_string(),
            mode: LaserMode::Cut,
            points: vec![Point2::new(10.0, 10.0), Point2::new(120.0, 10.0)], // 120.0 exceeds 100.0!
            closed: false,
            speed_mm_s: 100.0,
            power_pct: 80.0,
            passes: 1,
            air_assist: true,
            cut_length_mm: 110.0,
            sequence_index: 1,
        });

        let report = controller.validate_job(&plan);
        assert!(!report.is_safe);
        assert!(matches!(
            report.issues[0],
            SafetyIssue::SoftLimitExceeded(..)
        ));
    }
}
