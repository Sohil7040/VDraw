use crate::target::AlignmentLine;
use core_geom::{Point2, Rect};
use serde::{Deserialize, Serialize};

/// Smart alignment detection for moving and transforming shapes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AlignmentConfig {
    pub enabled: bool,
    pub align_edges: bool,
    pub align_centers: bool,
    pub equal_spacing: bool,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            align_edges: true,
            align_centers: true,
            equal_spacing: true,
        }
    }
}

/// Result of evaluating smart alignment against neighboring bounding boxes.
#[derive(Debug, Clone, Default)]
pub struct AlignmentResult {
    /// Suggested adjustment to `delta.x` (if aligned horizontally).
    pub snap_dx: Option<f64>,
    /// Suggested adjustment to `delta.y` (if aligned vertically).
    pub snap_dy: Option<f64>,
    /// Visual alignment guideline lines spanning the aligned shapes.
    pub lines: Vec<AlignmentLine>,
}

impl AlignmentConfig {
    /// Evaluate alignment of `dragged_bbox` translated by `current_delta` against `other_boxes`.
    pub fn evaluate(
        &self,
        dragged_bbox: Rect,
        current_delta: Point2,
        other_boxes: &[Rect],
        tolerance_world: f64,
    ) -> AlignmentResult {
        if !self.enabled || other_boxes.is_empty() {
            return AlignmentResult::default();
        }

        let test_box = dragged_bbox.translate(current_delta);
        let mut result = AlignmentResult::default();

        let mut best_x_diff = tolerance_world;
        let mut best_snap_dx = None;
        let mut best_x_line = None;

        let mut best_y_diff = tolerance_world;
        let mut best_snap_dy = None;
        let mut best_y_line = None;

        let test_cx = (test_box.min_x + test_box.max_x) * 0.5;
        let test_cy = (test_box.min_y + test_box.max_y) * 0.5;

        for other in other_boxes {
            let other_cx = (other.min_x + other.max_x) * 0.5;
            let other_cy = (other.min_y + other.max_y) * 0.5;

            // ─── Horizontal (X) Alignments ───────────────────────────────────────
            let x_candidates = [
                // (candidate_x, label, delta_adjustment)
                (other.min_x, "Left to Left", other.min_x - test_box.min_x),
                (other.max_x, "Right to Left", other.max_x - test_box.min_x),
                (other.min_x, "Left to Right", other.min_x - test_box.max_x),
                (other.max_x, "Right to Right", other.max_x - test_box.max_x),
            ];

            if self.align_edges {
                for (target_x, label, adj) in x_candidates {
                    let diff = adj.abs();
                    if diff <= best_x_diff {
                        best_x_diff = diff;
                        best_snap_dx = Some(current_delta.x + adj);

                        let span_min_y = test_box.min_y.min(other.min_y) - 5.0;
                        let span_max_y = test_box.max_y.max(other.max_y) + 5.0;
                        best_x_line = Some(AlignmentLine {
                            start: Point2::new(target_x, span_min_y),
                            end: Point2::new(target_x, span_max_y),
                            label: label.to_string(),
                            is_vertical: true,
                        });
                    }
                }
            }

            if self.align_centers {
                let center_adj = other_cx - test_cx;
                let diff = center_adj.abs();
                if diff <= best_x_diff {
                    best_x_diff = diff;
                    best_snap_dx = Some(current_delta.x + center_adj);

                    let span_min_y = test_box.min_y.min(other.min_y) - 5.0;
                    let span_max_y = test_box.max_y.max(other.max_y) + 5.0;
                    best_x_line = Some(AlignmentLine {
                        start: Point2::new(other_cx, span_min_y),
                        end: Point2::new(other_cx, span_max_y),
                        label: "Center X".to_string(),
                        is_vertical: true,
                    });
                }
            }

            // ─── Vertical (Y) Alignments ─────────────────────────────────────────
            let y_candidates = [
                (other.min_y, "Top to Top", other.min_y - test_box.min_y),
                (other.max_y, "Bottom to Top", other.max_y - test_box.min_y),
                (other.min_y, "Top to Bottom", other.min_y - test_box.max_y),
                (
                    other.max_y,
                    "Bottom to Bottom",
                    other.max_y - test_box.max_y,
                ),
            ];

            if self.align_edges {
                for (target_y, label, adj) in y_candidates {
                    let diff = adj.abs();
                    if diff <= best_y_diff {
                        best_y_diff = diff;
                        best_snap_dy = Some(current_delta.y + adj);

                        let span_min_x = test_box.min_x.min(other.min_x) - 5.0;
                        let span_max_x = test_box.max_x.max(other.max_x) + 5.0;
                        best_y_line = Some(AlignmentLine {
                            start: Point2::new(span_min_x, target_y),
                            end: Point2::new(span_max_x, target_y),
                            label: label.to_string(),
                            is_vertical: false,
                        });
                    }
                }
            }

            if self.align_centers {
                let center_adj = other_cy - test_cy;
                let diff = center_adj.abs();
                if diff <= best_y_diff {
                    best_y_diff = diff;
                    best_snap_dy = Some(current_delta.y + center_adj);

                    let span_min_x = test_box.min_x.min(other.min_x) - 5.0;
                    let span_max_x = test_box.max_x.max(other.max_x) + 5.0;
                    best_y_line = Some(AlignmentLine {
                        start: Point2::new(span_min_x, other_cy),
                        end: Point2::new(span_max_x, other_cy),
                        label: "Center Y".to_string(),
                        is_vertical: false,
                    });
                }
            }
        }

        // ─── Equal Spacing Detection ─────────────────────────────────────────
        if self.equal_spacing && other_boxes.len() >= 2 {
            // Check adjacent pairs in X
            for i in 0..other_boxes.len() {
                for j in (i + 1)..other_boxes.len() {
                    let b1 = &other_boxes[i];
                    let b2 = &other_boxes[j];

                    // Check if b1 and b2 have horizontal gap
                    let gap_x = if b1.max_x < b2.min_x {
                        b2.min_x - b1.max_x
                    } else if b2.max_x < b1.min_x {
                        b1.min_x - b2.max_x
                    } else {
                        -1.0
                    };

                    if gap_x > 0.0 {
                        // Check if test_box can be placed adjacent to b2 with gap_x
                        let target_right = b2.max_x + gap_x;
                        let adj_right = target_right - test_box.min_x;
                        if adj_right.abs() <= best_x_diff {
                            best_x_diff = adj_right.abs();
                            best_snap_dx = Some(current_delta.x + adj_right);
                            best_x_line = Some(AlignmentLine {
                                start: Point2::new(target_right, test_box.min_y),
                                end: Point2::new(target_right, test_box.max_y),
                                label: format!("Equal Spacing ({:.1} mm)", gap_x),
                                is_vertical: true,
                            });
                        }

                        let target_left = b1.min_x - gap_x - test_box.width();
                        let adj_left = target_left - test_box.min_x;
                        if adj_left.abs() <= best_x_diff {
                            best_x_diff = adj_left.abs();
                            best_snap_dx = Some(current_delta.x + adj_left);
                            best_x_line = Some(AlignmentLine {
                                start: Point2::new(target_left, test_box.min_y),
                                end: Point2::new(target_left, test_box.max_y),
                                label: format!("Equal Spacing ({:.1} mm)", gap_x),
                                is_vertical: true,
                            });
                        }
                    }
                }
            }
        }

        result.snap_dx = best_snap_dx;
        result.snap_dy = best_snap_dy;

        if let Some(line) = best_x_line {
            result.lines.push(line);
        }
        if let Some(line) = best_y_line {
            result.lines.push(line);
        }

        result
    }
}
