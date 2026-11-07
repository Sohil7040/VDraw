use crate::alignment::{AlignmentConfig, AlignmentResult};
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

    /// Evaluate smart alignment when moving a shape bounding box.
    /// Returns the adjusted translation delta and visual alignment lines.
    pub fn snap_transform(
        &self,
        dragged_bbox: Rect,
        delta: Point2,
        zoom: f64,
        exclude_shape_ids: &[ShapeId],
    ) -> (Point2, Vec<AlignmentLine>) {
        if !self.config.enabled || !self.config.alignment.enabled {
            return (delta, Vec::new());
        }

        let effective_zoom = zoom.max(0.0001);
        let tolerance_world = self.config.gravity_radius_px / effective_zoom;

        // Query neighboring shapes across a generous envelope for alignment
        let test_box = dragged_bbox.translate(delta).inflate(5000.0, 5000.0);
        let nearby_boxes: Vec<Rect> = self
            .spatial_index
            .locate_in_envelope(&test_box)
            .filter(|s| !exclude_shape_ids.contains(&s.shape_id))
            .map(|s| s.bounds)
            .collect();

        let align_res: AlignmentResult =
            self.config
                .alignment
                .evaluate(dragged_bbox, delta, &nearby_boxes, tolerance_world);

        let final_delta = Point2::new(
            align_res.snap_dx.unwrap_or(delta.x),
            align_res.snap_dy.unwrap_or(delta.y),
        );

        (final_delta, align_res.lines)
    }
}
