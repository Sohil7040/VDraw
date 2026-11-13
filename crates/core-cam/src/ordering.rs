//! Dependency-aware topological laser cut ordering and travel path optimization.
//!
//! Enforces non-negotiable industrial laser manufacturing constraints:
//! 1. Inside-Out Cut Ordering: Inner holes and nested features are ALWAYS cut before
//!    their enclosing outer boundary to prevent workpiece shift or drop.
//! 2. Phase Precedence: Marking, engraving, and scoring are ALWAYS executed before
//!    cutting on any given workpiece.
//! 3. Dependency DAG: Travel optimization (Greedy Nearest Neighbor) operates strictly
//!    over the wavefront of dependency-satisfied (ready) operations.

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
    pub is_inner_hole: bool,
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
            is_inner_hole: false,
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
        if p.x < self.bounds.min_x - 1e-6
            || p.x > self.bounds.max_x + 1e-6
            || p.y < self.bounds.min_y - 1e-6
            || p.y > self.bounds.max_y + 1e-6
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
        if other.bounds.min_x < self.bounds.min_x - 1e-6
            || other.bounds.max_x > self.bounds.max_x + 1e-6
            || other.bounds.min_y < self.bounds.min_y - 1e-6
            || other.bounds.max_y > self.bounds.max_y + 1e-6
        {
            return false;
        }

        // Test sample points of `other`
        let sample_step = (other.points.len() / 8).max(1);
        for i in (0..other.points.len()).step_by(sample_step) {
            if !self.contains_point(other.points[i]) {
                return false;
            }
        }

        // Check center of bounds as well
        let center = Point2::new(
            (other.bounds.min_x + other.bounds.max_x) * 0.5,
            (other.bounds.min_y + other.bounds.max_y) * 0.5,
        );
        self.contains_point(center)
    }
}

/// Compute nesting containment depth and classify inner holes for all closed cut contours.
pub fn calculate_containment_depths(contours: &mut [CamContour]) {
    let n = contours.len();
    for i in 0..n {
        if !contours[i].closed {
            continue;
        }
        let mut depth = 0;
        for j in 0..n {
            if i != j && contours[j].closed && contours[j].contains_contour(&contours[i]) {
                depth += 1;
            }
        }
        contours[i].depth = depth;
        // Odd nesting depth means it is an inner hole enclosed by an outer boundary
        contours[i].is_inner_hole = depth % 2 == 1;
    }
}

/// Directed Acyclic Graph (DAG) for manufacturing execution order.
/// Node `A -> B` indicates that contour `A` MUST be executed BEFORE contour `B`.
pub struct CamDependencyGraph {
    /// Number of nodes
    pub node_count: usize,
    /// Adjacency list: `dependents[A]` contains all nodes `B` that depend on `A`
    pub dependents: Vec<Vec<usize>>,
    /// Number of unsatisfied prerequisites for each node
    pub in_degrees: Vec<usize>,
}

impl CamDependencyGraph {
    pub fn build(contours: &[CamContour], inside_out: bool) -> Self {
        let n = contours.len();
        let mut dependents = vec![Vec::new(); n];
        let mut in_degrees = vec![0; n];

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }

                let mut must_precede = false;

                // 1. Inside-Out Dependency: If contour i is contained inside contour j
                if inside_out && contours[j].closed && contours[j].contains_contour(&contours[i]) {
                    // Inner feature i MUST precede outer cut j
                    if contours[j].mode.is_cut() {
                        must_precede = true;
                    }
                }

                // 2. Phase Precedence Dependency:
                // Non-cut operations (Dot, Pen, Engrave, Score) inside a cut shape MUST precede cutting that shape
                if contours[i].mode.execution_priority() < contours[j].mode.execution_priority()
                    && ((contours[j].closed && contours[j].contains_contour(&contours[i]))
                        || (contours[i].shape_id == contours[j].shape_id
                            && contours[j].mode.is_cut()))
                {
                    must_precede = true;
                }

                if must_precede {
                    dependents[i].push(j);
                    in_degrees[j] += 1;
                }
            }
        }

        Self {
            node_count: n,
            dependents,
            in_degrees,
        }
    }
}

/// Optimize laser cutting order with dependency graph, inside-out nesting, and rapid travel reduction.
pub fn optimize_cutting_order(
    mut contours: Vec<CamContour>,
    start_pos: Point2,
    inside_out: bool,
    optimize_rapid: bool,
) -> Vec<CamContour> {
    if contours.is_empty() {
        return contours;
    }

    // 1. Calculate containment depths
    calculate_containment_depths(&mut contours);

    if !optimize_rapid {
        // Fallback: sort primarily by operation priority, then depth (descending)
        contours.sort_by(|a, b| {
            let p_a = a.mode.execution_priority();
            let p_b = b.mode.execution_priority();
            if p_a != p_b {
                p_a.cmp(&p_b)
            } else if inside_out {
                b.depth.cmp(&a.depth) // Inner holes (higher depth) first
            } else {
                std::cmp::Ordering::Equal
            }
        });
        return contours;
    }

    // 2. Build manufacturing dependency graph
    let mut graph = CamDependencyGraph::build(&contours, inside_out);
    let n = contours.len();
    let mut executed = vec![false; n];
    let mut ordered = Vec::with_capacity(n);
    let mut current_pos = start_pos;

    // 3. Dependency-aware wavefront traversal
    for _ in 0..n {
        // Find all candidates with in_degree == 0 (all dependencies met)
        let ready_indices: Vec<usize> = (0..n)
            .filter(|&i| !executed[i] && graph.in_degrees[i] == 0)
            .collect();

        let candidate_indices = if !ready_indices.is_empty() {
            ready_indices
        } else {
            // Cycle-breaking fallback (should not occur in acyclic geometry)
            let min_deg = (0..n)
                .filter(|&i| !executed[i])
                .map(|i| graph.in_degrees[i])
                .min()
                .unwrap_or(0);
            (0..n)
                .filter(|&i| !executed[i] && graph.in_degrees[i] == min_deg)
                .collect()
        };

        // Determine highest execution phase among ready candidates
        let min_priority = candidate_indices
            .iter()
            .map(|&idx| contours[idx].mode.execution_priority())
            .min()
            .unwrap_or(255);

        // Filter to candidates matching the active execution phase
        // (e.g. all available Engrave/Score operations before Cuts)
        let phase_candidates: Vec<usize> = candidate_indices
            .into_iter()
            .filter(|&idx| contours[idx].mode.execution_priority() == min_priority)
            .collect();

        // Among phase candidates, pick the one with minimum rapid travel from current_pos
        let mut best_idx = phase_candidates[0];
        let mut best_dist = f64::INFINITY;
        let mut best_start_idx = 0;
        let mut best_reverse = false;

        for &idx in &phase_candidates {
            let c = &contours[idx];
            if c.closed {
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

        // Mark executed
        executed[best_idx] = true;
        for &dep in &graph.dependents[best_idx] {
            if graph.in_degrees[dep] > 0 {
                graph.in_degrees[dep] -= 1;
            }
        }

        // Rotate/reverse contour points for optimal entry
        let mut chosen = contours[best_idx].clone();
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

    ordered
}
