//! Zero-allocation [`SnapProvider`] implementations for geometry, grid, polar, and ortho snapping.

use crate::contracts::{SnapCtx, SnapKind, SnapMask, SnapPoint, SnapProvider, POLAR_STEP_DEG};
use crate::grid::GridConfig;
use crate::intersection::{cubic_control_bounds, for_each_polycurve_intersection, segment_cubic};
use crate::spatial_index::{IndexedShape, SpatialIndex};
use core_doc::ShapeId;
use core_geom::{CubicBezier, Point2, Rect};
use std::sync::Arc;

/// Maximum de Casteljau subdivision depth for zero-allocation perpendicular/tangent root finding.
const MAX_ROOT_DEPTH: u8 = 14;

/// Pre-computed world-space query window derived from a [`SnapCtx`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct QueryWindow {
    pub cursor_world: Point2,
    pub threshold_world: f64,
    pub threshold_sq: f64,
    pub search_box: Rect,
}

impl QueryWindow {
    /// Converts screen-space cursor & threshold into a world-space query window.
    #[inline]
    pub(crate) fn from_ctx(ctx: &SnapCtx<'_>) -> Option<Self> {
        if ctx.threshold_px <= 0.0 || !ctx.view.scale.is_finite() || ctx.view.scale.abs() <= 1e-12 {
            return None;
        }
        let cursor_world = ctx.view.screen_to_world(ctx.cursor_screen);
        if !cursor_world.is_finite() {
            return None;
        }
        let threshold_world = (ctx.threshold_px as f64) / ctx.view.scale.abs().max(1e-12);
        let threshold_sq = threshold_world * threshold_world;
        let search_box = Rect::new(
            cursor_world.x - threshold_world,
            cursor_world.y - threshold_world,
            cursor_world.x + threshold_world,
            cursor_world.y + threshold_world,
        );
        Some(Self {
            cursor_world,
            threshold_world,
            threshold_sq,
            search_box,
        })
    }
}

/// Emits [`SnapKind::Endpoint`] for open-subpath terminal vertices and [`SnapKind::Node`]
/// for interior and closed-subpath vertices within `q.threshold_sq`.
#[inline]
pub(crate) fn collect_shape_nodes_and_endpoints(
    shape: &IndexedShape,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    if shape.polycurve.subpaths.is_empty() {
        for &pt in &shape.nodes {
            if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                out.push(SnapPoint::new(pt, SnapKind::Node, shape.shape_id));
            }
        }
        return;
    }

    for subpath in &shape.polycurve.subpaths {
        let count = subpath.nodes.len();
        if count == 0 {
            continue;
        }

        if subpath.closed {
            for node in &subpath.nodes {
                if node.point.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                    out.push(SnapPoint::new(node.point, SnapKind::Node, shape.shape_id));
                }
            }
        } else {
            let first = subpath.nodes[0].point;
            if first.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                out.push(SnapPoint::new(first, SnapKind::Endpoint, shape.shape_id));
            }

            if count > 1 {
                for node in &subpath.nodes[1..count - 1] {
                    if node.point.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                        out.push(SnapPoint::new(node.point, SnapKind::Node, shape.shape_id));
                    }
                }
                let last = subpath.nodes[count - 1].point;
                if last.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                    out.push(SnapPoint::new(last, SnapKind::Endpoint, shape.shape_id));
                }
            }
        }
    }
}

/// Emits [`SnapKind::Midpoint`] candidates from `shape.midpoints` within `q.threshold_sq`.
#[inline]
pub(crate) fn collect_shape_midpoints(
    shape: &IndexedShape,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    for &mid in &shape.midpoints {
        if mid.distance_squared_to(q.cursor_world) <= q.threshold_sq {
            out.push(SnapPoint::new(mid, SnapKind::Midpoint, shape.shape_id));
        }
    }
}

/// Emits [`SnapKind::Center`] from `shape.center` if within `q.threshold_sq`.
#[inline]
pub(crate) fn collect_shape_center(
    shape: &IndexedShape,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    if shape.center.distance_squared_to(q.cursor_world) <= q.threshold_sq {
        out.push(SnapPoint::new(
            shape.center,
            SnapKind::Center,
            shape.shape_id,
        ));
    }
}

/// Emits [`SnapKind::Quadrant`] candidates from `shape.quadrants` within `q.threshold_sq`.
#[inline]
pub(crate) fn collect_shape_quadrants(
    shape: &IndexedShape,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    for &quad in &shape.quadrants {
        if quad.distance_squared_to(q.cursor_world) <= q.threshold_sq {
            out.push(SnapPoint::new(quad, SnapKind::Quadrant, shape.shape_id));
        }
    }
}

/// Appends a [`SnapPoint`] only if no candidate of the same kind was already appended
/// since `start_idx` within `dedup_tol_sq`.
#[inline]
fn push_dedup(
    out: &mut Vec<SnapPoint>,
    start_idx: usize,
    point: Point2,
    kind: SnapKind,
    source: ShapeId,
    dedup_tol_sq: f64,
) {
    let duplicate = out[start_idx..].iter().any(|existing| {
        existing.kind == kind && existing.position.distance_squared_to(point) <= dedup_tol_sq
    });
    if !duplicate {
        out.push(SnapPoint::new(point, kind, source));
    }
}

/// Emits [`SnapKind::Intersection`] candidates for all intersecting shape pairs inside `q.search_box`
/// without allocating temporary vectors.
pub(crate) fn collect_index_intersections(
    index: &SpatialIndex,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    let start_idx = out.len();
    for shape_a in index.locate_in_envelope(&q.search_box) {
        for shape_b in index.locate_in_envelope(&q.search_box) {
            if shape_a.shape_id >= shape_b.shape_id {
                continue;
            }
            if !shape_a.bounds.intersects(&shape_b.bounds) {
                continue;
            }
            for_each_polycurve_intersection(
                &shape_a.polycurve,
                &shape_b.polycurve,
                Some(&q.search_box),
                |pt| {
                    if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                        push_dedup(
                            out,
                            start_idx,
                            pt,
                            SnapKind::Intersection,
                            shape_a.shape_id,
                            1e-4,
                        );
                    }
                },
            );
        }
    }
}

#[inline]
fn all_same_strict_sign(coeffs: &[f64; 6]) -> bool {
    let all_pos = coeffs.iter().all(|&c| c > 1e-9);
    let all_neg = coeffs.iter().all(|&c| c < -1e-9);
    all_pos || all_neg
}

fn refine_root<F: Fn(f64) -> f64>(eval: F) -> f64 {
    let mut lo = 0.0;
    let mut hi = 1.0;
    let mut f_lo = eval(lo);
    let f_hi = eval(hi);
    if f_lo.abs() <= 1e-12 {
        return lo;
    }
    if f_hi.abs() <= 1e-12 {
        return hi;
    }
    if f_lo * f_hi <= 0.0 {
        for _ in 0..10 {
            let mid = 0.5 * (lo + hi);
            let f_mid = eval(mid);
            if f_mid.abs() <= 1e-12 {
                return mid;
            }
            if f_lo * f_mid <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
                f_lo = f_mid;
            }
        }
        0.5 * (lo + hi)
    } else {
        let f_mid = eval(0.5);
        if f_mid.abs() <= f_lo.abs() && f_mid.abs() <= f_hi.abs() {
            0.5
        } else if f_lo.abs() <= f_hi.abs() {
            0.0
        } else {
            1.0
        }
    }
}

fn find_perp_cubic(
    b: &CubicBezier,
    ref_origin: Point2,
    q: &QueryWindow,
    shape_id: ShapeId,
    start_idx: usize,
    depth: u8,
    out: &mut Vec<SnapPoint>,
) {
    if !cubic_control_bounds(b).intersects(&q.search_box) {
        return;
    }

    let q0 = b.p0 - ref_origin;
    let q1 = b.p1 - ref_origin;
    let q2 = b.p2 - ref_origin;
    let q3 = b.p3 - ref_origin;

    let v0 = (b.p1 - b.p0) * 3.0;
    let v1 = (b.p2 - b.p1) * 3.0;
    let v2 = (b.p3 - b.p2) * 3.0;

    let coeffs = [
        q0.dot(v0),
        0.6 * q1.dot(v0) + 0.4 * q0.dot(v1),
        0.3 * q2.dot(v0) + 0.6 * q1.dot(v1) + 0.1 * q0.dot(v2),
        0.1 * q3.dot(v0) + 0.6 * q2.dot(v1) + 0.3 * q1.dot(v2),
        0.4 * q3.dot(v1) + 0.6 * q2.dot(v2),
        q3.dot(v2),
    ];

    if all_same_strict_sign(&coeffs) {
        return;
    }

    if depth >= MAX_ROOT_DEPTH || (b.p3 - b.p0).length_squared() < 1e-6 {
        let t = refine_root(|s| (b.evaluate(s) - ref_origin).dot(b.derivative(s)));
        let pt = b.evaluate(t);
        if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
            let dir = pt - ref_origin;
            let orthogonal = if dir.length_squared() <= 1e-12 {
                true
            } else {
                dir.normalize().dot(b.tangent(t)).abs() < 1e-2
            };
            if orthogonal {
                push_dedup(out, start_idx, pt, SnapKind::Perpendicular, shape_id, 1e-4);
            }
        }
        return;
    }

    let (left, right) = b.split_half();
    find_perp_cubic(&left, ref_origin, q, shape_id, start_idx, depth + 1, out);
    find_perp_cubic(&right, ref_origin, q, shape_id, start_idx, depth + 1, out);
}

fn find_tangent_cubic(
    b: &CubicBezier,
    ref_origin: Point2,
    q: &QueryWindow,
    shape_id: ShapeId,
    start_idx: usize,
    depth: u8,
    out: &mut Vec<SnapPoint>,
) {
    if !cubic_control_bounds(b).intersects(&q.search_box) {
        return;
    }

    let q0 = b.p0 - ref_origin;
    let q1 = b.p1 - ref_origin;
    let q2 = b.p2 - ref_origin;
    let q3 = b.p3 - ref_origin;

    let v0 = (b.p1 - b.p0) * 3.0;
    let v1 = (b.p2 - b.p1) * 3.0;
    let v2 = (b.p3 - b.p2) * 3.0;

    let coeffs = [
        q0.cross(v0),
        0.6 * q1.cross(v0) + 0.4 * q0.cross(v1),
        0.3 * q2.cross(v0) + 0.6 * q1.cross(v1) + 0.1 * q0.cross(v2),
        0.1 * q3.cross(v0) + 0.6 * q2.cross(v1) + 0.3 * q1.cross(v2),
        0.4 * q3.cross(v1) + 0.6 * q2.cross(v2),
        q3.cross(v2),
    ];

    if all_same_strict_sign(&coeffs) {
        return;
    }

    if depth >= MAX_ROOT_DEPTH || (b.p3 - b.p0).length_squared() < 1e-6 {
        let t = refine_root(|s| (b.evaluate(s) - ref_origin).cross(b.derivative(s)));
        let pt = b.evaluate(t);
        let dir = pt - ref_origin;
        if dir.length_squared() > 1e-8
            && pt.distance_squared_to(q.cursor_world) <= q.threshold_sq
            && dir.normalize().cross(b.tangent(t)).abs() < 1e-2
        {
            push_dedup(out, start_idx, pt, SnapKind::Tangent, shape_id, 1e-4);
        }
        return;
    }

    let (left, right) = b.split_half();
    find_tangent_cubic(&left, ref_origin, q, shape_id, start_idx, depth + 1, out);
    find_tangent_cubic(&right, ref_origin, q, shape_id, start_idx, depth + 1, out);
}

/// Emits [`SnapKind::Perpendicular`] foot points on `shape` relative to `ref_origin`
/// within `q.threshold_sq`, using zero heap allocations.
pub(crate) fn collect_shape_perpendicular(
    shape: &IndexedShape,
    ref_origin: Point2,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    if !ref_origin.is_finite() {
        return;
    }
    let start_idx = out.len();
    for subpath in &shape.polycurve.subpaths {
        let count = subpath.nodes.len();
        if count < 2 {
            continue;
        }
        let segments = if subpath.closed { count } else { count - 1 };
        for i in 0..segments {
            let n0 = &subpath.nodes[i];
            let n1 = &subpath.nodes[(i + 1) % count];
            if n0.handle_out.is_none() && n1.handle_in.is_none() {
                let seg_box = Rect::from_points(n0.point, n1.point);
                if !seg_box.intersects(&q.search_box) {
                    continue;
                }
                let v = n1.point - n0.point;
                let len_sq = v.length_squared();
                if len_sq > 1e-12 {
                    let t = (ref_origin - n0.point).dot(v) / len_sq;
                    if (0.0..=1.0).contains(&t) {
                        let pt = n0.point + v * t;
                        if pt.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                            push_dedup(
                                out,
                                start_idx,
                                pt,
                                SnapKind::Perpendicular,
                                shape.shape_id,
                                1e-4,
                            );
                        }
                    }
                }
            } else {
                let b = segment_cubic(n0, n1);
                find_perp_cubic(&b, ref_origin, q, shape.shape_id, start_idx, 0, out);
            }
        }
    }
}

/// Emits [`SnapKind::Tangent`] points on curved segments of `shape` relative to `ref_origin`
/// within `q.threshold_sq`, using zero heap allocations.
pub(crate) fn collect_shape_tangent(
    shape: &IndexedShape,
    ref_origin: Point2,
    q: &QueryWindow,
    out: &mut Vec<SnapPoint>,
) {
    if !ref_origin.is_finite() {
        return;
    }
    let start_idx = out.len();
    for subpath in &shape.polycurve.subpaths {
        let count = subpath.nodes.len();
        if count < 2 {
            continue;
        }
        let segments = if subpath.closed { count } else { count - 1 };
        for i in 0..segments {
            let n0 = &subpath.nodes[i];
            let n1 = &subpath.nodes[(i + 1) % count];
            if n0.handle_out.is_none() && n1.handle_in.is_none() {
                continue;
            }
            let b = segment_cubic(n0, n1);
            find_tangent_cubic(&b, ref_origin, q, shape.shape_id, start_idx, 0, out);
        }
    }
}

/// Composite geometric snap provider querying an R*-tree [`SpatialIndex`] for
/// nodes, endpoints, midpoints, centers, quadrants, intersections, perpendiculars, and tangents.
#[derive(Debug, Clone)]
pub struct GeometrySnapProvider {
    pub index: Arc<SpatialIndex>,
    pub reference_origin: Option<Point2>,
    pub modes: SnapMask,
}

impl GeometrySnapProvider {
    /// Creates a [`GeometrySnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self {
            index,
            reference_origin: None,
            modes: SnapMask::all(),
        }
    }

    /// Creates a [`GeometrySnapProvider`] by wrapping an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }

    /// Sets an optional reference origin used for [`SnapKind::Perpendicular`] and [`SnapKind::Tangent`].
    pub fn with_reference_origin(mut self, origin: Option<Point2>) -> Self {
        self.reference_origin = origin;
        self
    }

    /// Restricts which geometric candidate kinds are emitted by this provider.
    pub fn with_modes(mut self, modes: SnapMask) -> Self {
        self.modes = modes;
        self
    }
}

impl SnapProvider for GeometrySnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };

        let want_node_or_endpoint =
            self.modes.contains(SnapMask::NODE) || self.modes.contains(SnapMask::ENDPOINT);
        let want_midpoint = self.modes.contains(SnapMask::MIDPOINT);
        let want_center = self.modes.contains(SnapMask::CENTER);
        let want_quadrant = self.modes.contains(SnapMask::QUADRANT);
        let want_perp = self.modes.contains(SnapMask::PERPENDICULAR);
        let want_tan = self.modes.contains(SnapMask::TANGENT);

        for shape in self.index.locate_in_envelope(&q.search_box) {
            if want_node_or_endpoint {
                collect_shape_nodes_and_endpoints(shape, &q, out);
            }
            if want_midpoint {
                collect_shape_midpoints(shape, &q, out);
            }
            if want_center {
                collect_shape_center(shape, &q, out);
            }
            if want_quadrant {
                collect_shape_quadrants(shape, &q, out);
            }
            if let Some(ref_origin) = self.reference_origin {
                if want_perp {
                    collect_shape_perpendicular(shape, ref_origin, &q, out);
                }
                if want_tan {
                    collect_shape_tangent(shape, ref_origin, &q, out);
                }
            }
        }

        if self.modes.contains(SnapMask::INTERSECTION) {
            collect_index_intersections(&self.index, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Node`] and [`SnapKind::Endpoint`]
/// candidates from an R*-tree [`SpatialIndex`].
#[derive(Debug, Clone)]
pub struct NodeSnapProvider {
    pub index: Arc<SpatialIndex>,
}

impl NodeSnapProvider {
    /// Creates a [`NodeSnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self { index }
    }

    /// Creates a [`NodeSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }
}

impl SnapProvider for NodeSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_nodes_and_endpoints(shape, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Midpoint`] candidates from an R*-tree [`SpatialIndex`].
#[derive(Debug, Clone)]
pub struct MidpointSnapProvider {
    pub index: Arc<SpatialIndex>,
}

impl MidpointSnapProvider {
    /// Creates a [`MidpointSnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self { index }
    }

    /// Creates a [`MidpointSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }
}

impl SnapProvider for MidpointSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_midpoints(shape, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Center`] candidates from an R*-tree [`SpatialIndex`].
#[derive(Debug, Clone)]
pub struct CenterSnapProvider {
    pub index: Arc<SpatialIndex>,
}

impl CenterSnapProvider {
    /// Creates a [`CenterSnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self { index }
    }

    /// Creates a [`CenterSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }
}

impl SnapProvider for CenterSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_center(shape, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Quadrant`] cardinal extrema candidates
/// from an R*-tree [`SpatialIndex`].
#[derive(Debug, Clone)]
pub struct QuadrantSnapProvider {
    pub index: Arc<SpatialIndex>,
}

impl QuadrantSnapProvider {
    /// Creates a [`QuadrantSnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self { index }
    }

    /// Creates a [`QuadrantSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }
}

impl SnapProvider for QuadrantSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_quadrants(shape, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Intersection`] candidates between
/// neighboring indexed shapes inside the cursor's R*-tree search envelope.
#[derive(Debug, Clone)]
pub struct IntersectionSnapProvider {
    pub index: Arc<SpatialIndex>,
}

impl IntersectionSnapProvider {
    /// Creates an [`IntersectionSnapProvider`] backed by a shared [`SpatialIndex`].
    pub fn new(index: Arc<SpatialIndex>) -> Self {
        Self { index }
    }

    /// Creates an [`IntersectionSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex) -> Self {
        Self::new(Arc::new(index))
    }
}

impl SnapProvider for IntersectionSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        collect_index_intersections(&self.index, &q, out);
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Perpendicular`] candidates relative to an
/// optional reference origin.
#[derive(Debug, Clone)]
pub struct PerpendicularSnapProvider {
    pub index: Arc<SpatialIndex>,
    pub reference_origin: Option<Point2>,
}

impl PerpendicularSnapProvider {
    /// Creates a [`PerpendicularSnapProvider`] with a shared [`SpatialIndex`] and reference origin.
    pub fn new(index: Arc<SpatialIndex>, reference_origin: Option<Point2>) -> Self {
        Self {
            index,
            reference_origin,
        }
    }

    /// Creates a [`PerpendicularSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex, reference_origin: Option<Point2>) -> Self {
        Self::new(Arc::new(index), reference_origin)
    }
}

impl SnapProvider for PerpendicularSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(ref_origin) = self.reference_origin else {
            return;
        };
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_perpendicular(shape, ref_origin, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Tangent`] candidates relative to an
/// optional reference origin.
#[derive(Debug, Clone)]
pub struct TangentSnapProvider {
    pub index: Arc<SpatialIndex>,
    pub reference_origin: Option<Point2>,
}

impl TangentSnapProvider {
    /// Creates a [`TangentSnapProvider`] with a shared [`SpatialIndex`] and reference origin.
    pub fn new(index: Arc<SpatialIndex>, reference_origin: Option<Point2>) -> Self {
        Self {
            index,
            reference_origin,
        }
    }

    /// Creates a [`TangentSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex, reference_origin: Option<Point2>) -> Self {
        Self::new(Arc::new(index), reference_origin)
    }
}

impl SnapProvider for TangentSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(ref_origin) = self.reference_origin else {
            return;
        };
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_tangent(shape, ref_origin, &q, out);
        }
    }
}

/// Combined [`SnapProvider`] emitting both [`SnapKind::Perpendicular`] and [`SnapKind::Tangent`]
/// candidates relative to an optional reference origin.
#[derive(Debug, Clone)]
pub struct PerpTangentSnapProvider {
    pub index: Arc<SpatialIndex>,
    pub reference_origin: Option<Point2>,
}

impl PerpTangentSnapProvider {
    /// Creates a [`PerpTangentSnapProvider`] with a shared [`SpatialIndex`] and reference origin.
    pub fn new(index: Arc<SpatialIndex>, reference_origin: Option<Point2>) -> Self {
        Self {
            index,
            reference_origin,
        }
    }

    /// Creates a [`PerpTangentSnapProvider`] from an owned [`SpatialIndex`].
    pub fn from_index(index: SpatialIndex, reference_origin: Option<Point2>) -> Self {
        Self::new(Arc::new(index), reference_origin)
    }
}

impl SnapProvider for PerpTangentSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(ref_origin) = self.reference_origin else {
            return;
        };
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        for shape in self.index.locate_in_envelope(&q.search_box) {
            collect_shape_perpendicular(shape, ref_origin, &q, out);
            collect_shape_tangent(shape, ref_origin, &q, out);
        }
    }
}

/// Dedicated [`SnapProvider`] emitting [`SnapKind::Grid`] candidates from [`GridConfig`].
#[derive(Debug, Clone, Copy, Default)]
pub struct GridSnapProvider {
    pub config: GridConfig,
}

impl GridSnapProvider {
    /// Creates a [`GridSnapProvider`] with the supplied [`GridConfig`].
    pub const fn new(config: GridConfig) -> Self {
        Self { config }
    }

    /// Creates an enabled [`GridSnapProvider`] with explicit major spacing and subdivisions.
    pub fn with_spacing(major_spacing_mm: f64, subdivisions: usize) -> Self {
        Self {
            config: GridConfig {
                enabled: true,
                major_spacing_mm,
                subdivisions: subdivisions.max(1),
                snap_to_minor: subdivisions > 1,
                ..GridConfig::default()
            },
        }
    }
}

impl SnapProvider for GridSnapProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        if let Some((pt, _)) = self.config.snap_point(q.cursor_world, q.threshold_world) {
            out.push(SnapPoint::new(pt, SnapKind::Grid, ShapeId::NONE));
        }
    }
}

/// Polar angle tracking [`SnapProvider`] projecting cursor coordinates onto rays at multiples
/// of `step_deg` (defaulting to [`POLAR_STEP_DEG`] = 15°) radiating from a reference origin.
#[derive(Debug, Clone, Copy)]
pub struct PolarConstraintProvider {
    pub origin: Option<Point2>,
    pub step_deg: f64,
    pub enabled: bool,
    pub kind: SnapKind,
}

impl Default for PolarConstraintProvider {
    fn default() -> Self {
        Self {
            origin: None,
            step_deg: POLAR_STEP_DEG,
            enabled: true,
            kind: SnapKind::Grid,
        }
    }
}

impl PolarConstraintProvider {
    /// Creates an active [`PolarConstraintProvider`] centered at `origin` with [`POLAR_STEP_DEG`].
    pub fn new(origin: Point2) -> Self {
        Self {
            origin: Some(origin),
            ..Self::default()
        }
    }

    /// Creates a [`PolarConstraintProvider`] with an optional reference origin.
    pub fn with_optional_origin(origin: Option<Point2>) -> Self {
        Self {
            origin,
            ..Self::default()
        }
    }

    /// Configures the angular increment in degrees.
    pub fn with_step_deg(mut self, step_deg: f64) -> Self {
        self.step_deg = step_deg;
        self
    }

    /// Enables or disables polar constraint emission.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Synchronizes `enabled` with [`SnapMask::POLAR`].
    pub fn with_modes(mut self, modes: SnapMask) -> Self {
        self.enabled = modes.contains(SnapMask::POLAR);
        self
    }

    /// Overrides the emitted [`SnapKind`] (defaults to [`SnapKind::Grid`]).
    pub fn with_kind(mut self, kind: SnapKind) -> Self {
        self.kind = kind;
        self
    }

    /// Projects `cursor_world` onto the nearest polar ray through `origin`.
    pub fn project(&self, cursor_world: Point2) -> Option<Point2> {
        if !self.enabled || !self.step_deg.is_finite() || self.step_deg <= 1e-6 {
            return None;
        }
        let origin = self.origin?;
        if !origin.is_finite() || !cursor_world.is_finite() {
            return None;
        }

        let delta = cursor_world - origin;
        if delta.length_squared() <= 1e-12 {
            return None;
        }

        let mut angle_deg = delta.y.atan2(delta.x).to_degrees();
        if angle_deg < 0.0 {
            angle_deg += 360.0;
        }
        let snapped_deg = ((angle_deg / self.step_deg).round() * self.step_deg) % 360.0;
        let rad = snapped_deg.to_radians();
        let unit = Point2::new(rad.cos(), rad.sin());
        let proj_len = delta.dot(unit);
        if proj_len <= 0.0 {
            return None;
        }
        Some(origin + unit * proj_len)
    }
}

impl SnapProvider for PolarConstraintProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        if let Some(projected) = self.project(q.cursor_world) {
            if projected.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                out.push(SnapPoint::new(projected, self.kind, ShapeId::NONE));
            }
        }
    }
}

/// Orthogonal constraint [`SnapProvider`] projecting cursor coordinates onto the horizontal or
/// vertical axis passing through a reference origin.
#[derive(Debug, Clone, Copy)]
pub struct OrthoConstraintProvider {
    pub origin: Option<Point2>,
    pub enabled: bool,
    pub kind: SnapKind,
}

impl Default for OrthoConstraintProvider {
    fn default() -> Self {
        Self {
            origin: None,
            enabled: true,
            kind: SnapKind::Grid,
        }
    }
}

impl OrthoConstraintProvider {
    /// Creates an active [`OrthoConstraintProvider`] anchored at `origin`.
    pub fn new(origin: Point2) -> Self {
        Self {
            origin: Some(origin),
            ..Self::default()
        }
    }

    /// Creates an [`OrthoConstraintProvider`] with an optional reference origin.
    pub fn with_optional_origin(origin: Option<Point2>) -> Self {
        Self {
            origin,
            ..Self::default()
        }
    }

    /// Enables or disables ortho constraint emission.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Synchronizes `enabled` with [`SnapMask::ORTHO`].
    pub fn with_modes(mut self, modes: SnapMask) -> Self {
        self.enabled = modes.contains(SnapMask::ORTHO);
        self
    }

    /// Overrides the emitted [`SnapKind`] (defaults to [`SnapKind::Grid`]).
    pub fn with_kind(mut self, kind: SnapKind) -> Self {
        self.kind = kind;
        self
    }

    /// Projects `cursor_world` onto whichever of the horizontal or vertical axes through `origin`
    /// is closer.
    pub fn project(&self, cursor_world: Point2) -> Option<Point2> {
        if !self.enabled {
            return None;
        }
        let origin = self.origin?;
        if !origin.is_finite() || !cursor_world.is_finite() {
            return None;
        }

        let dx = (cursor_world.x - origin.x).abs();
        let dy = (cursor_world.y - origin.y).abs();
        if dy <= dx {
            Some(Point2::new(cursor_world.x, origin.y))
        } else {
            Some(Point2::new(origin.x, cursor_world.y))
        }
    }
}

impl SnapProvider for OrthoConstraintProvider {
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
        let mut out = Vec::new();
        self.collect_candidates(ctx, &mut out);
        out
    }

    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        let Some(q) = QueryWindow::from_ctx(ctx) else {
            return;
        };
        if let Some(projected) = self.project(q.cursor_world) {
            if projected.distance_squared_to(q.cursor_world) <= q.threshold_sq {
                out.push(SnapPoint::new(projected, self.kind, ShapeId::NONE));
            }
        }
    }
}
