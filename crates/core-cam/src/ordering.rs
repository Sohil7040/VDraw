use crate::operation::LaserMode;
use core_geom::{Point2, Rect};

/// A flattened vector contour ready for laser toolpath planning.
#[derive(Debug, Clone, PartialEq)]
pub struct CamContour {
    pub points: Vec<Point2>,
    pub closed: bool,
    pub bounds: Rect,
    pub shape_id: u64,
    pub layer_name: String,
    pub mode: LaserMode,
    pub depth: usize,
}

impl CamContour {
    pub fn new(
        points: Vec<Point2>,
        closed: bool,
        shape_id: u64,
        layer_name: String,
        mode: LaserMode,
    ) -> Self {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for pt in &points {
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
        }

        let bounds = if min_x.is_finite() {
            Rect::new(min_x, min_y, max_x, max_y)
        } else {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        };

        Self {
            points,
            closed,
            bounds,
            shape_id,
            layer_name,
            mode,
            depth: 0,
        }
    }

    /// Calculate the total cut perimeter / length in millimeters.
    pub fn length_mm(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mut len = 0.0;
        for i in 1..self.points.len() {
            len += self.points[i - 1].distance(self.points[i]);
        }
        if self.closed && self.points.len() >= 3 {
            len += self.points.last().unwrap().distance(self.points[0]);
        }
        len
    }

    /// Check if point `p` is strictly inside this closed polygon contour (Ray Casting).
    pub fn contains_point(&self, p: Point2) -> bool {
        if !self.closed || self.points.len() < 3 {
            return false;
        }
        if p.x < self.bounds.min_x
            || p.x > self.bounds.max_x
            || p.y < self.bounds.min_y
            || p.y > self.bounds.max_y
        {
            return false;
        }

        let mut inside = false;
        let n = self.points.len();
        let mut j = n - 1;

        for i in 0..n {
            let pi = self.points[i];
            let pj = self.points[j];

            if ((pi.y > p.y) != (pj.y > p.y))
                && (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-12) + pi.x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// Check if `other` contour is completely enclosed inside `self`.
    pub fn contains_contour(&self, other: &CamContour) -> bool {
        if !self.closed || other.points.is_empty() {
            return false;
        }
        // Quick AABB rejection
        if other.bounds.min_x < self.bounds.min_x
            || other.bounds.max_x > self.bounds.max_x
            || other.bounds.min_y < self.bounds.min_y
            || other.bounds.max_y > self.bounds.max_y
        {
            return false;
        }

        // Test sample points of `other`
        let sample_step = (other.points.len() / 5).max(1);
        for i in (0..other.points.len()).step_by(sample_step) {
            if !self.contains_point(other.points[i]) {
                return false;
            }
        }

        true
    }
}

/// Compute nesting containment depth for all closed cut contours.
/// Inner holes will have depth > 0, outer parts will have depth = 0.
pub fn calculate_containment_depths(contours: &mut [CamContour]) {
    let n = contours.len();
    for i in 0..n {
        if !contours[i].closed || contours[i].mode != LaserMode::Cut {
            continue;
        }
        let mut depth = 0;
        for j in 0..n {
            if i != j
                && contours[j].closed
                && contours[j].mode == LaserMode::Cut
                && contours[j].contains_contour(&contours[i])
            {
                depth += 1;
            }
        }
        contours[i].depth = depth;
    }
}

/// Optimize laser cutting order with operation priority, Inside-Out nesting, and rapid travel reduction.
pub fn optimize_cutting_order(
    mut contours: Vec<CamContour>,
    start_pos: Point2,
    inside_out: bool,
    optimize_rapid: bool,
) -> Vec<CamContour> {
    if contours.is_empty() {
        return contours;
    }

    // 1. Calculate containment depth for inside-out cutting
    if inside_out {
        calculate_containment_depths(&mut contours);
    }

    if !optimize_rapid {
        // Fallback: simple sorting by operation priority, then depth (descending)
        contours.sort_by(|a, b| {
            let p_a = a.mode.execution_priority();
            let p_b = b.mode.execution_priority();
            if p_a != p_b {
                p_a.cmp(&p_b)
            } else if inside_out && a.mode == LaserMode::Cut {
                b.depth.cmp(&a.depth) // Higher depth (inner holes) first
            } else {
                std::cmp::Ordering::Equal
            }
        });
        return contours;
    }

    // 2. Greedy Nearest-Neighbor optimization per operation phase
    let mut ordered = Vec::with_capacity(contours.len());
    let mut current_pos = start_pos;

    // Process operations in order: Dot -> Pen -> Engrave -> Score -> Cut
    for op in [
        LaserMode::Dot,
        LaserMode::Pen,
        LaserMode::Engrave,
        LaserMode::Score,
        LaserMode::Cut,
    ] {
        let mut pool: Vec<CamContour> = contours.iter().filter(|c| c.mode == op).cloned().collect();

        if inside_out && op == LaserMode::Cut {
            // Sort pool so deepest nesting (inner holes) comes first
            pool.sort_by(|a, b| b.depth.cmp(&a.depth));
        }

        while !pool.is_empty() {
            let active_depth = if inside_out && op == LaserMode::Cut {
                Some(pool[0].depth)
            } else {
                None
            };

            let mut best_idx = 0;
            let mut best_dist = f64::INFINITY;
            let mut best_start_idx = 0;
            let mut best_reverse = false;

            for (idx, c) in pool.iter().enumerate() {
                // Must finish current depth level before proceeding to outer level
                if let Some(target_d) = active_depth {
                    if c.depth != target_d {
                        break;
                    }
                }

                if c.closed {
                    // For closed contour, find closest vertex to start cut
                    let mut closest_pt_idx = 0;
                    let mut min_pt_dist = f64::INFINITY;
                    for (pt_i, pt) in c.points.iter().enumerate() {
                        let d = current_pos.distance(*pt);
                        if d < min_pt_dist {
                            min_pt_dist = d;
                            closest_pt_idx = pt_i;
                        }
                    }

                    if min_pt_dist < best_dist {
                        best_dist = min_pt_dist;
                        best_idx = idx;
                        best_start_idx = closest_pt_idx;
                        best_reverse = false;
                    }
                } else {
                    // For open contour, check head and tail
                    let d_head = current_pos.distance(c.points[0]);
                    let d_tail = current_pos.distance(*c.points.last().unwrap());

                    if d_head < best_dist {
                        best_dist = d_head;
                        best_idx = idx;
                        best_start_idx = 0;
                        best_reverse = false;
                    }
                    if d_tail < best_dist {
                        best_dist = d_tail;
                        best_idx = idx;
                        best_start_idx = 0;
                        best_reverse = true;
                    }
                }
            }

            let mut chosen = pool.remove(best_idx);

            // Rotate closed loop to start at the closest point
            if chosen.closed && best_start_idx > 0 && best_start_idx < chosen.points.len() {
                let mut rotated = Vec::with_capacity(chosen.points.len());
                rotated.extend_from_slice(&chosen.points[best_start_idx..]);
                rotated.extend_from_slice(&chosen.points[..best_start_idx]);
                chosen.points = rotated;
            } else if !chosen.closed && best_reverse {
                chosen.points.reverse();
            }

            current_pos = if chosen.closed {
                chosen.points[0]
            } else {
                *chosen.points.last().unwrap()
            };

            ordered.push(chosen);
        }
    }

    ordered
}
