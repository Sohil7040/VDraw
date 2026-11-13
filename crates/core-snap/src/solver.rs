use crate::alignment::AlignmentConfig;
use crate::dynamic_guide::DynamicGuideConfig;
use crate::grid::GridConfig;
use crate::guideline::Guideline;
use crate::intersection::{polycurve_guideline_intersections, polycurve_intersections};
use crate::spatial_index::SpatialIndex;
use crate::target::{AlignmentLine, SnapResult, SnapTarget, SnapTargetKind};
use core_doc::ShapeId;
use core_geom::{Point2, Rect};
use serde::{Deserialize, Serialize};

/// Comprehensive snapping configuration controlling all targets and gravity radius.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapConfig {
    pub enabled: bool,
    /// Gravity radius in screen pixels (default: 8.0 px).
    pub gravity_radius_px: f64,
    /// Snap to anchor points on paths, polygons, stars.
    pub snap_to_nodes: bool,
    /// Snap to midpoints of segments and bbox edges.
    pub snap_to_midpoints: bool,
    /// Snap to shape centers and centroids.
    pub snap_to_centers: bool,
    /// Snap to cardinal quadrant points on ellipses and curves.
    pub snap_to_quadrants: bool,
    /// Snap to curve-curve, curve-line, and line-line intersections.
    pub snap_to_intersections: bool,
    /// Snap to perpendicular and tangent points relative to reference origin.
    pub snap_to_tangent_perpendicular: bool,
    /// Snap to closest point on shape outlines.
    pub snap_to_edges: bool,
    /// Snap to 9-point reference box.
    pub snap_to_bbox_points: bool,
    /// Snap to user guidelines.
    pub snap_to_guidelines: bool,
    /// Snap to background coordinate grid.
    pub snap_to_grid: bool,
    /// Polar angle dynamic guides (0°, 15°, 30°, 45°, etc.).
    pub dynamic_guides: DynamicGuideConfig,
    /// Smart alignment guides (edge-to-edge, center-to-center, equal spacing).
    pub alignment: AlignmentConfig,
    /// Background grid settings.
    pub grid: GridConfig,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gravity_radius_px: 8.0,
            snap_to_nodes: true,
            snap_to_midpoints: true,
            snap_to_centers: true,
            snap_to_quadrants: true,
            snap_to_intersections: true,
            snap_to_tangent_perpendicular: true,
            snap_to_edges: true,
            snap_to_bbox_points: true,
            snap_to_guidelines: true,
            snap_to_grid: true,
            dynamic_guides: DynamicGuideConfig::default(),
            alignment: AlignmentConfig::default(),
            grid: GridConfig::default(),
        }
    }
}

/// The core snapping engine resolving cursor and shape positions against all geometric targets.
#[derive(Debug, Clone, Default)]
pub struct SnapSolver {
    pub config: SnapConfig,
    pub spatial_index: SpatialIndex,
    pub guidelines: Vec<Guideline>,
}

impl SnapSolver {
    pub fn new(config: SnapConfig) -> Self {
        Self {
            config,
            spatial_index: SpatialIndex::new(),
            guidelines: Vec::new(),
        }
    }

    /// Add a guideline.
    pub fn add_guideline(&mut self, guideline: Guideline) {
        self.guidelines.push(guideline);
    }

    /// Clear all guidelines.
    pub fn clear_guidelines(&mut self) {
        self.guidelines.clear();
    }

    /// Evaluate snap candidates for a single point query.
    ///
    /// - `cursor`: Raw query point in world coordinates (mm).
    /// - `zoom`: Active viewport scale (pixels per mm).
    /// - `reference_origin`: Optional origin point (e.g. drag start or previous node in Pen tool).
    /// - `exclude_shape_ids`: Shape IDs to exclude from snapping (e.g. shape currently being created/moved).
    pub fn snap_point(
        &self,
        cursor: Point2,
        zoom: f64,
        reference_origin: Option<Point2>,
        exclude_shape_ids: &[ShapeId],
    ) -> SnapResult {
        if !self.config.enabled {
            return SnapResult::no_snap(cursor);
        }

        let effective_zoom = zoom.max(0.0001);
        let tolerance_world = self.config.gravity_radius_px / effective_zoom;

        let search_box = Rect::new(
            cursor.x - tolerance_world,
            cursor.y - tolerance_world,
            cursor.x + tolerance_world,
            cursor.y + tolerance_world,
        );

        let mut candidates: Vec<SnapTarget> = Vec::new();
        let mut active_dynamic_ray = None;

        // ─── 1. Query Spatial Index for Nearby Shapes ─────────────────────────
        let nearby_shapes: Vec<_> = self
            .spatial_index
            .locate_in_envelope(&search_box)
            .filter(|s| !exclude_shape_ids.contains(&s.shape_id))
            .collect();

        for shape in &nearby_shapes {
            // A. Anchor Nodes
            if self.config.snap_to_nodes {
                for &node in &shape.nodes {
                    let d = cursor.distance(node);
                    if d <= tolerance_world {
                        candidates.push(SnapTarget::new(
                            SnapTargetKind::Node,
                            node,
                            d,
                            Some(shape.shape_id),
                        ));
                    }
                }
            }

            // B. Segment Midpoints
            if self.config.snap_to_midpoints {
                for &mid in &shape.midpoints {
                    let d = cursor.distance(mid);
                    if d <= tolerance_world {
                        candidates.push(SnapTarget::new(
                            SnapTargetKind::Midpoint,
                            mid,
                            d,
                            Some(shape.shape_id),
                        ));
                    }
                }
            }

            // C. Shape Center / Centroid
            if self.config.snap_to_centers {
                let d = cursor.distance(shape.center);
                if d <= tolerance_world {
                    candidates.push(SnapTarget::new(
                        SnapTargetKind::Center,
                        shape.center,
                        d,
                        Some(shape.shape_id),
                    ));
                }
            }

            // D. Cardinal Quadrants
            if self.config.snap_to_quadrants {
                for &quad in &shape.quadrants {
                    let d = cursor.distance(quad);
                    if d <= tolerance_world {
                        candidates.push(SnapTarget::new(
                            SnapTargetKind::Quadrant,
                            quad,
                            d,
                            Some(shape.shape_id),
                        ));
                    }
                }
            }

            // E. Bounding Box 9 Reference Points
            if self.config.snap_to_bbox_points {
                for &bp in &shape.bbox_points {
                    let d = cursor.distance(bp);
                    if d <= tolerance_world {
                        candidates.push(SnapTarget::new(
                            SnapTargetKind::BboxPoint,
                            bp,
                            d,
                            Some(shape.shape_id),
                        ));
                    }
                }
            }

            // F. Perpendicular / Tangent (if reference origin provided)
            if self.config.snap_to_tangent_perpendicular {
                if let Some(ref_pt) = reference_origin {
                    // Test perpendicular drop to straight segments
                    let lines = shape.polycurve.flatten(0.1);
                    for i in 0..(lines.len().saturating_sub(1)) {
                        let p1 = lines[i];
                        let p2 = lines[i + 1];
                        let v = p2 - p1;
                        let len_sq = v.x * v.x + v.y * v.y;
                        if len_sq > 1e-8 {
                            let t = ((ref_pt.x - p1.x) * v.x + (ref_pt.y - p1.y) * v.y) / len_sq;
                            if (0.0..=1.0).contains(&t) {
                                let perp_pt = Point2::new(p1.x + t * v.x, p1.y + t * v.y);
                                let d = cursor.distance(perp_pt);
                                if d <= tolerance_world {
                                    candidates.push(SnapTarget::new(
                                        SnapTargetKind::Perpendicular,
                                        perp_pt,
                                        d,
                                        Some(shape.shape_id),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // G. Closest Point on Outline / Edge
            if self.config.snap_to_edges {
                let (closest_pt, d) = shape.polycurve.closest_point(cursor);
                if d <= tolerance_world {
                    candidates.push(SnapTarget::new(
                        SnapTargetKind::Edge,
                        closest_pt,
                        d,
                        Some(shape.shape_id),
                    ));
                }
            }
        }

        // ─── 2. Intersections between Nearby Shapes ───────────────────────────
        if self.config.snap_to_intersections && nearby_shapes.len() >= 2 {
            for i in 0..nearby_shapes.len() {
                for j in (i + 1)..nearby_shapes.len() {
                    let pts = polycurve_intersections(
                        &nearby_shapes[i].polycurve,
                        &nearby_shapes[j].polycurve,
                        Some(&search_box),
                    );
                    for pt in pts {
                        let d = cursor.distance(pt);
                        if d <= tolerance_world {
                            candidates.push(SnapTarget::new(
                                SnapTargetKind::Intersection,
                                pt,
                                d,
                                Some(nearby_shapes[i].shape_id),
                            ));
                        }
                    }
                }
            }
        }

        // ─── 3. User Guidelines ───────────────────────────────────────────────
        if self.config.snap_to_guidelines {
            for guideline in &self.guidelines {
                let d = guideline.distance_to_point(cursor);
                if d <= tolerance_world {
                    let proj = guideline.project_point(cursor);
                    candidates.push(SnapTarget::new(SnapTargetKind::Guideline, proj, d, None));
                }

                // Guideline x Shape Intersections
                if self.config.snap_to_intersections {
                    for shape in &nearby_shapes {
                        let pts = polycurve_guideline_intersections(
                            &shape.polycurve,
                            guideline,
                            &search_box,
                        );
                        for pt in pts {
                            let d = cursor.distance(pt);
                            if d <= tolerance_world {
                                candidates.push(SnapTarget::new(
                                    SnapTargetKind::Intersection,
                                    pt,
                                    d,
                                    Some(shape.shape_id),
                                ));
                            }
                        }
                    }
                }
            }

            // Guideline x Guideline Intersections
            if self.config.snap_to_intersections && self.guidelines.len() >= 2 {
                for i in 0..self.guidelines.len() {
                    for j in (i + 1)..self.guidelines.len() {
                        if let Some(pt) = self.guidelines[i].intersect(&self.guidelines[j]) {
                            let d = cursor.distance(pt);
                            if d <= tolerance_world {
                                candidates.push(SnapTarget::new(
                                    SnapTargetKind::Intersection,
                                    pt,
                                    d,
                                    None,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // ─── 4. Dynamic Polar Guides ──────────────────────────────────────────
        if let Some(origin) = reference_origin {
            if let Some((pt, d, ray)) =
                self.config
                    .dynamic_guides
                    .snap(origin, cursor, tolerance_world)
            {
                let label = format!("{:.0}° ({:.1} mm)", ray.angle_deg, ray.distance_mm);
                let target =
                    SnapTarget::new(SnapTargetKind::DynamicGuide, pt, d, None).with_label(label);
                candidates.push(target);
                active_dynamic_ray = Some(ray);
            }
        }

        // ─── 5. Grid Snapping ─────────────────────────────────────────────────
        if self.config.snap_to_grid {
            if let Some((pt, d)) = self.config.grid.snap_point(cursor, tolerance_world) {
                candidates.push(SnapTarget::new(SnapTargetKind::Grid, pt, d, None));
            }
        }

        // ─── 6. Winner Resolution ─────────────────────────────────────────────
        if candidates.is_empty() {
            return SnapResult::no_snap(cursor);
        }

        // Weighted score balancing Euclidean distance against target priority.
        // Higher priority targets win unless another candidate is substantially closer.
        candidates.sort_by(|a, b| {
            let score_a = a.distance - (a.priority as f64) * 0.005 * tolerance_world;
            let score_b = b.distance - (b.priority as f64) * 0.005 * tolerance_world;
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let winner = candidates.remove(0);

        SnapResult {
            snapped: true,
            point: winner.point,
            target: Some(winner),
            alignment_lines: Vec::new(),
            dynamic_ray: active_dynamic_ray,
        }
    }

    /// Evaluate smart alignment when moving a shape bounding box (backward-compatible signature).
    /// Returns the adjusted translation delta and visual alignment lines.
    pub fn snap_transform(
        &self,
        dragged_bbox: Rect,
        delta: Point2,
        zoom: f64,
        exclude_shape_ids: &[ShapeId],
    ) -> (Point2, Vec<AlignmentLine>) {
        let res = self.snap_transform_advanced(dragged_bbox, delta, zoom, None, exclude_shape_ids);
        (res.delta, res.alignment_lines)
    }

    /// Comprehensive multi-point drag snapping engine (Corelith Studio / CorelDRAW algorithm).
    ///
    /// Evaluates 9 cardinal reference points on the dragged bounding box (center, 4 corners,
    /// 4 edge midpoints) against:
    /// 1. Dynamic alignment rays (center-to-center, edge-to-edge, bed center/edges)
    /// 2. Geometric snap targets (nodes, midpoints, centers, quadrants, intersections)
    /// 3. Coordinate grid
    ///
    /// Returns the magnetic snapped delta, winning snap target, and visual guide lines.
    pub fn snap_transform_advanced(
        &self,
        dragged_bbox: Rect,
        raw_delta: Point2,
        zoom: f64,
        page_size: Option<(f64, f64)>,
        exclude_shape_ids: &[ShapeId],
    ) -> crate::target::TransformSnapResult {
        use crate::target::TransformSnapResult;

        if !self.config.enabled {
            return TransformSnapResult {
                delta: raw_delta,
                snap_target: None,
                alignment_lines: Vec::new(),
                label: None,
            };
        }

        let effective_zoom = zoom.max(0.0001);
        let tolerance_world = self.config.gravity_radius_px / effective_zoom;

        // Current candidate box with raw delta applied
        let p = dragged_bbox.translate(raw_delta);
        let p_cx = (p.min_x + p.max_x) * 0.5;
        let p_cy = (p.min_y + p.max_y) * 0.5;

        // 9 candidate test points on the dragged box
        let test_points = [
            Point2::new(p.min_x, p.min_y), // Top-Left
            Point2::new(p_cx, p.min_y),    // Top-Center
            Point2::new(p.max_x, p.min_y), // Top-Right
            Point2::new(p.min_x, p_cy),    // Mid-Left
            Point2::new(p_cx, p_cy),       // Center
            Point2::new(p.max_x, p_cy),    // Mid-Right
            Point2::new(p.min_x, p.max_y), // Bottom-Left
            Point2::new(p_cx, p.max_y),    // Bottom-Center
            Point2::new(p.max_x, p.max_y), // Bottom-Right
        ];

        // ─── 1. Dynamic Alignment Rays (b4 algorithm) ─────────────────────────
        struct AlignMatch {
            adj: f64,
            target_pos: f64,
            label: String,
            color: [u8; 3],
            score: f64,
        }

        let mut best_align_x: Option<AlignMatch> = None;
        let mut best_align_y: Option<AlignMatch> = None;

        if self.config.alignment.enabled {
            let search_box = p.inflate(10000.0, 10000.0);
            let nearby_shapes: Vec<_> = self
                .spatial_index
                .locate_in_envelope(&search_box)
                .filter(|s| !exclude_shape_ids.contains(&s.shape_id))
                .collect();

            let x_features = [
                (p.min_x, "Left", false),
                (p_cx, "Center", true),
                (p.max_x, "Right", false),
            ];

            let y_features = [
                (p.min_y, "Top", false),
                (p_cy, "Center", true),
                (p.max_y, "Bottom", false),
            ];

            for shape in &nearby_shapes {
                let ob = shape.bounds;
                let ob_cx = (ob.min_x + ob.max_x) * 0.5;
                let ob_cy = (ob.min_y + ob.max_y) * 0.5;

                let other_x_features = [
                    (ob.min_x, "Left", false),
                    (ob_cx, "Center", true),
                    (ob.max_x, "Right", false),
                ];

                let other_y_features = [
                    (ob.min_y, "Top", false),
                    (ob_cy, "Center", true),
                    (ob.max_y, "Bottom", false),
                ];

                // Check X alignments
                for (drag_v, drag_name, drag_is_center) in x_features {
                    for (other_v, other_name, other_is_center) in other_x_features {
                        let adj = other_v - drag_v;
                        let dist = adj.abs();
                        if dist <= tolerance_world {
                            let (weight, label, color) = if drag_is_center && other_is_center {
                                (0.55, "Center to Center".to_string(), [6, 182, 212])
                            // Cyan
                            } else if !drag_is_center && !other_is_center {
                                let name = if drag_name == other_name {
                                    format!("Edge to Edge ({})", drag_name)
                                } else {
                                    format!("Edge to Edge ({} → {})", drag_name, other_name)
                                };
                                (0.80, name, [245, 158, 11]) // Amber
                            } else {
                                (
                                    1.00,
                                    format!("Edge to Center ({} → {})", drag_name, other_name),
                                    [139, 92, 246],
                                ) // Purple
                            };

                            let score = dist * weight;
                            if best_align_x.as_ref().is_none_or(|b| score < b.score) {
                                best_align_x = Some(AlignMatch {
                                    adj,
                                    target_pos: other_v,
                                    label,
                                    color,
                                    score,
                                });
                            }
                        }
                    }
                }

                // Check Y alignments
                for (drag_v, drag_name, drag_is_center) in y_features {
                    for (other_v, other_name, other_is_center) in other_y_features {
                        let adj = other_v - drag_v;
                        let dist = adj.abs();
                        if dist <= tolerance_world {
                            let (weight, label, color) = if drag_is_center && other_is_center {
                                (0.55, "Center to Center".to_string(), [6, 182, 212])
                            // Cyan
                            } else if !drag_is_center && !other_is_center {
                                let name = if drag_name == other_name {
                                    format!("Edge to Edge ({})", drag_name)
                                } else {
                                    format!("Edge to Edge ({} → {})", drag_name, other_name)
                                };
                                (0.80, name, [245, 158, 11]) // Amber
                            } else {
                                (
                                    1.00,
                                    format!("Edge to Center ({} → {})", drag_name, other_name),
                                    [139, 92, 246],
                                ) // Purple
                            };

                            let score = dist * weight;
                            if best_align_y.as_ref().is_none_or(|b| score < b.score) {
                                best_align_y = Some(AlignMatch {
                                    adj,
                                    target_pos: other_v,
                                    label,
                                    color,
                                    score,
                                });
                            }
                        }
                    }
                }
            }

            // Check Bed / Page boundary alignments (if page size given)
            if let Some((page_w, page_h)) = page_size {
                let bed_x_features = [
                    (0.0, "Bed Left", false),
                    (page_w * 0.5, "Bed Center X", true),
                    (page_w, "Bed Right", false),
                ];

                let bed_y_features = [
                    (0.0, "Bed Top", false),
                    (page_h * 0.5, "Bed Center Y", true),
                    (page_h, "Bed Bottom", false),
                ];

                for (drag_v, _, drag_is_center) in x_features {
                    for (bed_v, bed_name, bed_is_center) in bed_x_features {
                        let adj = bed_v - drag_v;
                        let dist = adj.abs();
                        if dist <= tolerance_world {
                            let is_both_center = drag_is_center && bed_is_center;
                            let weight = if is_both_center { 0.60 } else { 0.85 };
                            let color = if is_both_center {
                                [16, 185, 129]
                            } else {
                                [100, 116, 139]
                            };
                            let score = dist * weight;
                            if best_align_x.as_ref().is_none_or(|b| score < b.score) {
                                best_align_x = Some(AlignMatch {
                                    adj,
                                    target_pos: bed_v,
                                    label: bed_name.to_string(),
                                    color,
                                    score,
                                });
                            }
                        }
                    }
                }

                for (drag_v, _, drag_is_center) in y_features {
                    for (bed_v, bed_name, bed_is_center) in bed_y_features {
                        let adj = bed_v - drag_v;
                        let dist = adj.abs();
                        if dist <= tolerance_world {
                            let is_both_center = drag_is_center && bed_is_center;
                            let weight = if is_both_center { 0.60 } else { 0.85 };
                            let color = if is_both_center {
                                [16, 185, 129]
                            } else {
                                [100, 116, 139]
                            };
                            let score = dist * weight;
                            if best_align_y.as_ref().is_none_or(|b| score < b.score) {
                                best_align_y = Some(AlignMatch {
                                    adj,
                                    target_pos: bed_v,
                                    label: bed_name.to_string(),
                                    color,
                                    score,
                                });
                            }
                        }
                    }
                }
            }
        }

        // ─── 2. Direct Point Snapping for 9 Test Points (M1 algorithm) ─────────
        struct PointSnapCandidate {
            adj: Point2,
            target: SnapTarget,
            score: f64,
        }

        let mut best_point_snap: Option<PointSnapCandidate> = None;

        for &test_pt in &test_points {
            let snap_res = self.snap_point(test_pt, zoom, None, exclude_shape_ids);
            if snap_res.snapped {
                if let Some(target) = snap_res.target {
                    let adj = target.point - test_pt;
                    let dist = (adj.x * adj.x + adj.y * adj.y).sqrt();
                    if dist <= tolerance_world {
                        let priority_weight = match target.kind {
                            SnapTargetKind::Center => 0.65,
                            SnapTargetKind::Node => 0.75,
                            SnapTargetKind::Midpoint => 0.85,
                            _ => 1.0,
                        };
                        let score = dist * priority_weight;
                        if best_point_snap.as_ref().is_none_or(|b| score < b.score) {
                            best_point_snap = Some(PointSnapCandidate { adj, target, score });
                        }
                    }
                }
            }
        }

        // ─── 3. Grid Snapping for 9 Test Points ─────────────────────────────────
        let mut best_grid_adj_x: Option<f64> = None;
        let mut best_grid_adj_y: Option<f64> = None;

        if self.config.snap_to_grid && self.config.grid.enabled {
            let step =
                self.config.grid.major_spacing_mm / (self.config.grid.subdivisions as f64).max(1.0);
            if step > 0.001 {
                let mut min_gx = tolerance_world;
                let mut min_gy = tolerance_world;

                for &test_pt in &test_points {
                    let nearest_x = (test_pt.x / step).round() * step;
                    let nearest_y = (test_pt.y / step).round() * step;
                    let diff_x = nearest_x - test_pt.x;
                    let diff_y = nearest_y - test_pt.y;

                    if diff_x.abs() < min_gx.abs() && diff_x.abs() <= tolerance_world {
                        min_gx = diff_x;
                        best_grid_adj_x = Some(diff_x);
                    }
                    if diff_y.abs() < min_gy.abs() && diff_y.abs() <= tolerance_world {
                        min_gy = diff_y;
                        best_grid_adj_y = Some(diff_y);
                    }
                }
            }
        }

        // ─── 4. Winner Arbitration (mg algorithm) ──────────────────────────────
        // Choose X adjustment
        let mut final_adj_x = 0.0;
        let mut winning_line_x = None;
        let mut label_parts = Vec::new();

        if let Some(align_x) = best_align_x {
            final_adj_x = align_x.adj;
            label_parts.push(align_x.label.clone());
            let span_y_min = p.min_y - 200.0;
            let span_y_max = p.max_y + 200.0;
            winning_line_x = Some(
                AlignmentLine::new(
                    Point2::new(align_x.target_pos, span_y_min),
                    Point2::new(align_x.target_pos, span_y_max),
                    align_x.label,
                    true,
                )
                .with_color(align_x.color),
            );
        } else if let Some(pt_snap) = &best_point_snap {
            if pt_snap.adj.x.abs() <= tolerance_world {
                final_adj_x = pt_snap.adj.x;
            }
        } else if let Some(grid_x) = best_grid_adj_x {
            final_adj_x = grid_x;
            label_parts.push("Grid X".to_string());
        }

        // Choose Y adjustment
        let mut final_adj_y = 0.0;
        let mut winning_line_y = None;

        if let Some(align_y) = best_align_y {
            final_adj_y = align_y.adj;
            label_parts.push(align_y.label.clone());
            let span_x_min = p.min_x - 200.0;
            let span_x_max = p.max_x + 200.0;
            winning_line_y = Some(
                AlignmentLine::new(
                    Point2::new(span_x_min, align_y.target_pos),
                    Point2::new(span_x_max, align_y.target_pos),
                    align_y.label,
                    false,
                )
                .with_color(align_y.color),
            );
        } else if let Some(pt_snap) = &best_point_snap {
            if pt_snap.adj.y.abs() <= tolerance_world {
                final_adj_y = pt_snap.adj.y;
            }
        } else if let Some(grid_y) = best_grid_adj_y {
            final_adj_y = grid_y;
            label_parts.push("Grid Y".to_string());
        }

        let mut lines = Vec::new();
        if let Some(lx) = winning_line_x {
            lines.push(lx);
        }
        if let Some(ly) = winning_line_y {
            lines.push(ly);
        }

        let winning_target = best_point_snap.map(|b| b.target);
        if let Some(ref t) = winning_target {
            label_parts.push(t.label.clone());
        }

        let summary_label = if label_parts.is_empty() {
            None
        } else {
            Some(label_parts.join(" · "))
        };

        TransformSnapResult {
            delta: Point2::new(raw_delta.x + final_adj_x, raw_delta.y + final_adj_y),
            snap_target: winning_target,
            alignment_lines: lines,
            label: summary_label,
        }
    }
}

impl crate::contracts::SnapProvider for SnapSolver {
    fn candidates(&self, ctx: &crate::contracts::SnapCtx<'_>) -> Vec<crate::contracts::SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(
        &self,
        ctx: &crate::contracts::SnapCtx<'_>,
        out: &mut Vec<crate::contracts::SnapPoint>,
    ) {
        use crate::contracts::{SnapKind, SnapPoint};
        use crate::intersection::for_each_polycurve_guideline_intersection;
        use crate::providers::{
            collect_index_intersections, collect_shape_center, collect_shape_midpoints,
            collect_shape_nodes_and_endpoints, collect_shape_quadrants, QueryWindow,
        };

        if !self.config.enabled {
            return;
        }
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };

        for shape in self.spatial_index.locate_in_envelope(&q.search_box) {
            if self.config.snap_to_nodes {
                collect_shape_nodes_and_endpoints(shape, &q, out);
            }
            if self.config.snap_to_midpoints {
                collect_shape_midpoints(shape, &q, out);
            }
            if self.config.snap_to_centers {
                collect_shape_center(shape, &q, out);
            }
            if self.config.snap_to_quadrants {
                collect_shape_quadrants(shape, &q, out);
            }
        }

        if self.config.snap_to_intersections {
            collect_index_intersections(&self.spatial_index, &q, out);

            if self.config.snap_to_guidelines {
                for guideline in &self.guidelines {
                    for shape in self.spatial_index.locate_in_envelope(&q.search_box) {
                        for_each_polycurve_guideline_intersection(
                            &shape.polycurve,
                            guideline,
                            &q.search_box,
                            |pt| {
                                if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                                    out.push(SnapPoint::new(
                                        pt,
                                        SnapKind::Intersection,
                                        shape.shape_id,
                                    ));
                                }
                            },
                        );
                    }
                }

                if self.guidelines.len() >= 2 {
                    for i in 0..self.guidelines.len() {
                        for j in (i + 1)..self.guidelines.len() {
                            if let Some(pt) = self.guidelines[i].intersect(&self.guidelines[j]) {
                                if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                                    out.push(SnapPoint::new(
                                        pt,
                                        SnapKind::Intersection,
                                        ShapeId::NONE,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.config.snap_to_grid && self.config.grid.enabled {
            if let Some((pt, _)) = self
                .config
                .grid
                .snap_point(q.cursor_world, q.threshold_world)
            {
                out.push(SnapPoint::new(pt, SnapKind::Grid, ShapeId::NONE));
            }
        }
    }
}

pub(crate) fn compute_delta_impl(
    handle: crate::contracts::HandleMode,
    source_anchor: Point2,
    target: &crate::contracts::SnapPoint,
) -> Point2 {
    use crate::contracts::HandleMode;

    if !source_anchor.is_finite() || !target.position.is_finite() {
        return Point2::ZERO;
    }

    let delta = match handle {
        HandleMode::Node | HandleMode::Center | HandleMode::Insertion | HandleMode::Edge => {
            target.position - source_anchor
        }
    };

    if delta.is_finite() {
        delta
    } else {
        Point2::ZERO
    }
}
