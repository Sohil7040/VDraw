use crate::bezier::CubicBezier;
use crate::node::{BezierNode, NodeType};
use crate::point::Point2;
use crate::rect::Rect;
use serde::{Deserialize, Serialize};

/// Standard cubic Bézier circular arc approximation factor:
/// 4/3 * (sqrt(2) - 1) ≈ 0.5522847498307936
pub const KAPPA: f64 = 0.5522847498307936;

/// Corner style for rectangles and polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CornerStyle {
    #[default]
    Round, // Fillet
    Scallop, // Inverted round
    Chamfer, // Flat cut
}

/// A continuous subpath consisting of connected `BezierNode`s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Subpath {
    pub nodes: Vec<BezierNode>,
    pub closed: bool,
}

impl Subpath {
    pub fn new(closed: bool) -> Self {
        Self {
            nodes: Vec::new(),
            closed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn push_node(&mut self, node: BezierNode) {
        self.nodes.push(node);
    }

    /// Convert the subpath into a sequence of `CubicBezier` segments.
    pub fn to_cubics(&self) -> Vec<CubicBezier> {
        let n = self.nodes.len();
        if n < 2 {
            return Vec::new();
        }

        let segment_count = if self.closed { n } else { n - 1 };
        let mut cubics = Vec::with_capacity(segment_count);

        for i in 0..segment_count {
            let next_i = (i + 1) % n;
            let n0 = &self.nodes[i];
            let n1 = &self.nodes[next_i];

            let p0 = n0.point;
            let p3 = n1.point;

            let p1 = n0.handle_out.unwrap_or_else(|| p0.lerp(p3, 1.0 / 3.0));
            let p2 = n1.handle_in.unwrap_or_else(|| p0.lerp(p3, 2.0 / 3.0));

            cubics.push(CubicBezier::new(p0, p1, p2, p3));
        }

        cubics
    }

    /// Total arc length of this subpath.
    pub fn length(&self) -> f64 {
        self.to_cubics().iter().map(|c| c.length()).sum()
    }

    /// Flatten the subpath into a polyline within given tolerance.
    pub fn flatten(&self, tolerance: f64) -> Vec<Point2> {
        let cubics = self.to_cubics();
        if cubics.is_empty() {
            return Vec::new();
        }

        let mut pts = Vec::new();
        for (idx, c) in cubics.iter().enumerate() {
            let seg_pts = c.flatten(tolerance);
            if idx == 0 {
                pts.extend(seg_pts);
            } else {
                // Skip the duplicate first point
                pts.extend(seg_pts.into_iter().skip(1));
            }
        }
        pts
    }

    /// Bounding box of the subpath.
    pub fn bounds(&self) -> Option<Rect> {
        let cubics = self.to_cubics();
        if cubics.is_empty() {
            if let Some(first) = self.nodes.first() {
                return Some(Rect::from_point(first.point));
            }
            return None;
        }

        let mut r = cubics[0].bounds();
        for c in &cubics[1..] {
            r = r.union(&c.bounds());
        }
        Some(r)
    }

    /// Split segment at index `seg_idx` at parameter `t` (0.0..1.0).
    /// Inserts a new node and updates handles. Returns the index of the newly inserted node.
    pub fn split_segment(&mut self, seg_idx: usize, t: f64) -> Option<usize> {
        let n = self.nodes.len();
        if n < 2 {
            return None;
        }
        let max_seg = if self.closed { n } else { n - 1 };
        if seg_idx >= max_seg {
            return None;
        }

        let next_idx = (seg_idx + 1) % n;
        let p0 = self.nodes[seg_idx].point;
        let p3 = self.nodes[next_idx].point;
        let p1 = self.nodes[seg_idx]
            .handle_out
            .unwrap_or_else(|| p0.lerp(p3, 1.0 / 3.0));
        let p2 = self.nodes[next_idx]
            .handle_in
            .unwrap_or_else(|| p0.lerp(p3, 2.0 / 3.0));

        let cubic = CubicBezier::new(p0, p1, p2, p3);
        let (left, right) = cubic.split(t.clamp(1e-4, 1.0 - 1e-4));

        self.nodes[seg_idx].handle_out = Some(left.p1);

        let new_node =
            BezierNode::with_handles(left.p3, Some(left.p2), Some(right.p1), NodeType::Smooth);

        let insert_idx = seg_idx + 1;
        self.nodes.insert(insert_idx, new_node);

        let updated_next = (insert_idx + 1) % self.nodes.len();
        self.nodes[updated_next].handle_in = Some(right.p2);

        Some(insert_idx)
    }

    /// Remove the node at `node_idx`.
    pub fn remove_node(&mut self, node_idx: usize) -> bool {
        if node_idx < self.nodes.len() {
            self.nodes.remove(node_idx);
            true
        } else {
            false
        }
    }

    /// Update node type and re-enforce handle constraints.
    pub fn set_node_type(&mut self, node_idx: usize, node_type: NodeType) -> bool {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.set_node_type(node_type);
            true
        } else {
            false
        }
    }
}

/// A 2D vector path composed of one or more subpaths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Polycurve {
    pub subpaths: Vec<Subpath>,
}

impl Polycurve {
    pub fn new() -> Self {
        Self {
            subpaths: Vec::new(),
        }
    }

    pub fn from_subpath(subpath: Subpath) -> Self {
        Self {
            subpaths: vec![subpath],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.subpaths.is_empty() || self.subpaths.iter().all(|s| s.is_empty())
    }

    pub fn push_subpath(&mut self, subpath: Subpath) {
        self.subpaths.push(subpath);
    }

    /// Construct a straight 2-point line as a Polycurve.
    pub fn line(start: Point2, end: Point2) -> Self {
        let mut sub = Subpath::new(false);
        sub.push_node(BezierNode::new(start, NodeType::Cusp));
        sub.push_node(BezierNode::new(end, NodeType::Cusp));
        Self::from_subpath(sub)
    }

    /// Construct a rectangle normalized to cubic Béziers with optional corner radius.
    pub fn rectangle(rect: Rect, corner_radius: f64, _corner_style: CornerStyle) -> Self {
        let r = corner_radius
            .min(rect.width() * 0.5)
            .min(rect.height() * 0.5)
            .max(0.0);
        let mut sub = Subpath::new(true);

        if r <= 1e-6 {
            // Sharp corners
            sub.push_node(BezierNode::new(
                Point2::new(rect.min_x, rect.min_y),
                NodeType::Cusp,
            ));
            sub.push_node(BezierNode::new(
                Point2::new(rect.max_x, rect.min_y),
                NodeType::Cusp,
            ));
            sub.push_node(BezierNode::new(
                Point2::new(rect.max_x, rect.max_y),
                NodeType::Cusp,
            ));
            sub.push_node(BezierNode::new(
                Point2::new(rect.min_x, rect.max_y),
                NodeType::Cusp,
            ));
        } else {
            // Fillet (rounded corners) using KAPPA * r
            let k = KAPPA * r;

            // Top-left corner
            let tl_start = Point2::new(rect.min_x, rect.min_y + r);
            let tl_end = Point2::new(rect.min_x + r, rect.min_y);
            sub.push_node(BezierNode::with_handles(
                tl_start,
                None,
                Some(Point2::new(rect.min_x, rect.min_y + r - k)),
                NodeType::Smooth,
            ));
            sub.push_node(BezierNode::with_handles(
                tl_end,
                Some(Point2::new(rect.min_x + r - k, rect.min_y)),
                None,
                NodeType::Smooth,
            ));

            // Top-right corner
            let tr_start = Point2::new(rect.max_x - r, rect.min_y);
            let tr_end = Point2::new(rect.max_x, rect.min_y + r);
            sub.push_node(BezierNode::with_handles(
                tr_start,
                None,
                Some(Point2::new(rect.max_x - r + k, rect.min_y)),
                NodeType::Smooth,
            ));
            sub.push_node(BezierNode::with_handles(
                tr_end,
                Some(Point2::new(rect.max_x, rect.min_y + r - k)),
                None,
                NodeType::Smooth,
            ));

            // Bottom-right corner
            let br_start = Point2::new(rect.max_x, rect.max_y - r);
            let br_end = Point2::new(rect.max_x - r, rect.max_y);
            sub.push_node(BezierNode::with_handles(
                br_start,
                None,
                Some(Point2::new(rect.max_x, rect.max_y - r + k)),
                NodeType::Smooth,
            ));
            sub.push_node(BezierNode::with_handles(
                br_end,
                Some(Point2::new(rect.max_x - r + k, rect.max_y)),
                None,
                NodeType::Smooth,
            ));

            // Bottom-left corner
            let bl_start = Point2::new(rect.min_x + r, rect.max_y);
            let bl_end = Point2::new(rect.min_x, rect.max_y - r);
            sub.push_node(BezierNode::with_handles(
                bl_start,
                None,
                Some(Point2::new(rect.min_x + r - k, rect.max_y)),
                NodeType::Smooth,
            ));
            sub.push_node(BezierNode::with_handles(
                bl_end,
                Some(Point2::new(rect.min_x, rect.max_y - r + k)),
                None,
                NodeType::Smooth,
            ));
        }

        Self::from_subpath(sub)
    }

    /// Construct an ellipse normalized to 4 cubic Bézier curves using KAPPA.
    pub fn ellipse(center: Point2, rx: f64, ry: f64) -> Self {
        let kx = KAPPA * rx;
        let ky = KAPPA * ry;

        let mut sub = Subpath::new(true);

        // Top node: (cx, cy - ry)
        sub.push_node(BezierNode::with_handles(
            Point2::new(center.x, center.y - ry),
            Some(Point2::new(center.x - kx, center.y - ry)),
            Some(Point2::new(center.x + kx, center.y - ry)),
            NodeType::Symmetrical,
        ));

        // Right node: (cx + rx, cy)
        sub.push_node(BezierNode::with_handles(
            Point2::new(center.x + rx, center.y),
            Some(Point2::new(center.x + rx, center.y - ky)),
            Some(Point2::new(center.x + rx, center.y + ky)),
            NodeType::Symmetrical,
        ));

        // Bottom node: (cx, cy + ry)
        sub.push_node(BezierNode::with_handles(
            Point2::new(center.x, center.y + ry),
            Some(Point2::new(center.x + kx, center.y + ry)),
            Some(Point2::new(center.x - kx, center.y + ry)),
            NodeType::Symmetrical,
        ));

        // Left node: (cx - rx, cy)
        sub.push_node(BezierNode::with_handles(
            Point2::new(center.x - rx, center.y),
            Some(Point2::new(center.x - rx, center.y + ky)),
            Some(Point2::new(center.x - rx, center.y - ky)),
            NodeType::Symmetrical,
        ));

        Self::from_subpath(sub)
    }

    /// Regular polygon with `sides` (3..=50) centered at `center`.
    pub fn polygon(center: Point2, radius: f64, sides: u32) -> Self {
        let n = sides.clamp(3, 50);
        let mut sub = Subpath::new(true);
        for i in 0..n {
            let angle =
                (i as f64) * std::f64::consts::TAU / (n as f64) - std::f64::consts::FRAC_PI_2;
            let pt = Point2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            sub.push_node(BezierNode::new(pt, NodeType::Cusp));
        }
        Self::from_subpath(sub)
    }

    /// Star with `points` (3..=50), outer radius, and inner radius centered at `center`.
    pub fn star(center: Point2, outer_radius: f64, inner_radius: f64, points: u32) -> Self {
        let n = points.clamp(3, 50) * 2;
        let mut sub = Subpath::new(true);
        for i in 0..n {
            let r = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
            let angle =
                (i as f64) * std::f64::consts::TAU / (n as f64) - std::f64::consts::FRAC_PI_2;
            let pt = Point2::new(center.x + r * angle.cos(), center.y + r * angle.sin());
            sub.push_node(BezierNode::new(pt, NodeType::Cusp));
        }
        Self::from_subpath(sub)
    }

    /// Archimedean spiral with `turns` (0.5..=20.0) expanding to `expansion` radius.
    pub fn spiral(center: Point2, turns: f64, expansion: f64) -> Self {
        let turns = turns.clamp(0.5, 20.0);
        let expansion = expansion.max(0.1);
        let total_angle = turns * std::f64::consts::TAU;
        let b = expansion / total_angle; // r(theta) = b * theta

        let segments = ((turns * 4.0).ceil() as usize).max(4);
        let delta_theta = total_angle / segments as f64;

        let mut sub = Subpath::new(false);
        let mut prev_pt = center;
        let mut prev_tangent = Point2::new(b, 0.0);

        for seg in 0..segments {
            let theta1 = (seg + 1) as f64 * delta_theta;

            let r1 = b * theta1;
            let pt1 = Point2::new(center.x + r1 * theta1.cos(), center.y + r1 * theta1.sin());

            let tan1 = Point2::new(
                b * theta1.cos() - r1 * theta1.sin(),
                b * theta1.sin() + r1 * theta1.cos(),
            );

            let dt_third = delta_theta / 3.0;
            let cp1 = prev_pt + prev_tangent * dt_third;
            let cp2 = pt1 - tan1 * dt_third;

            if seg == 0 {
                sub.push_node(BezierNode::with_handles(
                    prev_pt,
                    None,
                    Some(cp1),
                    NodeType::Smooth,
                ));
            } else {
                let last_idx = sub.nodes.len() - 1;
                sub.nodes[last_idx].handle_out = Some(cp1);
            }

            sub.push_node(BezierNode::with_handles(
                pt1,
                Some(cp2),
                None,
                NodeType::Smooth,
            ));

            prev_pt = pt1;
            prev_tangent = tan1;
        }

        Self::from_subpath(sub)
    }

    /// Bounding box enclosing all subpaths.
    pub fn bounds(&self) -> Option<Rect> {
        let mut r: Option<Rect> = None;
        for sub in &self.subpaths {
            if let Some(sub_b) = sub.bounds() {
                r = match r {
                    Some(cur) => Some(cur.union(&sub_b)),
                    None => Some(sub_b),
                };
            }
        }
        r
    }

    /// Total arc length of all subpaths.
    pub fn length(&self) -> f64 {
        self.subpaths.iter().map(|s| s.length()).sum()
    }

    /// Flatten all subpaths into polylines within tolerance (mm).
    pub fn flatten(&self, tolerance: f64) -> Vec<Point2> {
        let mut pts = Vec::new();
        for sub in &self.subpaths {
            pts.extend(sub.flatten(tolerance));
        }
        pts
    }

    /// Compute the closest point on any subpath to query point `p`.
    /// Returns `(closest_point, distance)`.
    pub fn closest_point(&self, p: Point2) -> (Point2, f64) {
        let mut best_pt = p;
        let mut min_dist = f64::MAX;

        for sub in &self.subpaths {
            for c in sub.to_cubics() {
                let (_, pt, dist) = c.closest_point(p);
                if dist < min_dist {
                    min_dist = dist;
                    best_pt = pt;
                }
            }
        }

        if min_dist == f64::MAX {
            (p, 0.0)
        } else {
            (best_pt, min_dist)
        }
    }
}
