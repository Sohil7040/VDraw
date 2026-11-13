use crate::guideline::Guideline;
use core_geom::{BezierNode, CubicBezier, Point2, Polycurve, Rect};

/// Maximum de Casteljau recursion depth for zero-allocation intersection checks.
const MAX_INTERSECT_DEPTH: u8 = 12;

/// Constructs the [`CubicBezier`] for the segment between `n0` and `n1`.
#[inline]
pub(crate) fn segment_cubic(n0: &BezierNode, n1: &BezierNode) -> CubicBezier {
    let p0 = n0.point;
    let p3 = n1.point;
    let p1 = n0.handle_out.unwrap_or_else(|| p0.lerp(p3, 1.0 / 3.0));
    let p2 = n1.handle_in.unwrap_or_else(|| p0.lerp(p3, 2.0 / 3.0));
    CubicBezier::new(p0, p1, p2, p3)
}

/// Fast control-point convex-hull bounding rectangle for a [`CubicBezier`].
#[inline]
pub(crate) fn cubic_control_bounds(b: &CubicBezier) -> Rect {
    let min_x = b.p0.x.min(b.p1.x).min(b.p2.x).min(b.p3.x);
    let min_y = b.p0.y.min(b.p1.y).min(b.p2.y).min(b.p3.y);
    let max_x = b.p0.x.max(b.p1.x).max(b.p2.x).max(b.p3.x);
    let max_y = b.p0.y.max(b.p1.y).max(b.p2.y).max(b.p3.y);
    Rect {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

/// Returns `true` if `b` is flat within `tolerance` (mm) and its interior handles
/// stay within the chord span.
#[inline]
pub(crate) fn is_cubic_flat(b: &CubicBezier, tolerance: f64) -> bool {
    let chord = b.p3 - b.p0;
    let chord_len = chord.length();
    if chord_len < 1e-9 {
        (b.p1 - b.p0).length().max((b.p2 - b.p0).length()) <= tolerance
    } else {
        let d1 = chord.cross(b.p1 - b.p0).abs() / chord_len;
        let d2 = chord.cross(b.p2 - b.p0).abs() / chord_len;
        let u = chord / chord_len;
        let t1 = (b.p1 - b.p0).dot(u);
        let t2 = (b.p2 - b.p0).dot(u);
        d1 <= tolerance
            && d2 <= tolerance
            && t1 >= -tolerance
            && t1 <= chord_len + tolerance
            && t2 >= -tolerance
            && t2 <= chord_len + tolerance
    }
}

#[inline]
fn rect_perimeter(r: &Rect) -> f64 {
    (r.max_x - r.min_x) + (r.max_y - r.min_y)
}

fn intersect_cubics<F>(
    b1: &CubicBezier,
    b2: &CubicBezier,
    search_envelope: Option<&Rect>,
    depth1: u8,
    depth2: u8,
    on_hit: &mut F,
) where
    F: FnMut(Point2),
{
    let box1 = cubic_control_bounds(b1);
    let box2 = cubic_control_bounds(b2);

    if !box1.intersects(&box2) {
        return;
    }
    if let Some(env) = search_envelope {
        if !box1.intersects(env) || !box2.intersects(env) {
            return;
        }
    }

    let flat1 = depth1 >= MAX_INTERSECT_DEPTH || is_cubic_flat(b1, 0.02);
    let flat2 = depth2 >= MAX_INTERSECT_DEPTH || is_cubic_flat(b2, 0.02);

    if flat1 && flat2 {
        if let Some(pt) = segment_intersection(b1.p0, b1.p3, b2.p0, b2.p3) {
            if search_envelope.is_none_or(|env| env.contains(pt)) {
                on_hit(pt);
            }
        }
        return;
    }

    if !flat1 && (flat2 || rect_perimeter(&box1) >= rect_perimeter(&box2)) {
        let (l1, r1) = b1.split_half();
        intersect_cubics(&l1, b2, search_envelope, depth1 + 1, depth2, on_hit);
        intersect_cubics(&r1, b2, search_envelope, depth1 + 1, depth2, on_hit);
    } else {
        let (l2, r2) = b2.split_half();
        intersect_cubics(b1, &l2, search_envelope, depth1, depth2 + 1, on_hit);
        intersect_cubics(b1, &r2, search_envelope, depth1, depth2 + 1, on_hit);
    }
}

/// Visits every intersection point between `c1` and `c2` inside `search_envelope`
/// without allocating heap memory.
pub fn for_each_polycurve_intersection<F>(
    c1: &Polycurve,
    c2: &Polycurve,
    search_envelope: Option<&Rect>,
    mut on_hit: F,
) where
    F: FnMut(Point2),
{
    for sub1 in &c1.subpaths {
        let n1 = sub1.nodes.len();
        if n1 < 2 {
            continue;
        }
        let segs1 = if sub1.closed { n1 } else { n1 - 1 };

        for i in 0..segs1 {
            let b1 = segment_cubic(&sub1.nodes[i], &sub1.nodes[(i + 1) % n1]);
            let box1 = cubic_control_bounds(&b1);
            if let Some(env) = search_envelope {
                if !box1.intersects(env) {
                    continue;
                }
            }

            for sub2 in &c2.subpaths {
                let n2 = sub2.nodes.len();
                if n2 < 2 {
                    continue;
                }
                let segs2 = if sub2.closed { n2 } else { n2 - 1 };

                for j in 0..segs2 {
                    let b2 = segment_cubic(&sub2.nodes[j], &sub2.nodes[(j + 1) % n2]);
                    let box2 = cubic_control_bounds(&b2);
                    if !box1.intersects(&box2) {
                        continue;
                    }
                    if let Some(env) = search_envelope {
                        if !box2.intersects(env) {
                            continue;
                        }
                    }

                    intersect_cubics(&b1, &b2, search_envelope, 0, 0, &mut on_hit);
                }
            }
        }
    }
}

fn guideline_segment(guideline: &Guideline, search_envelope: &Rect) -> Option<(Point2, Point2)> {
    match guideline.orientation {
        crate::guideline::GuidelineOrientation::Horizontal(y) => {
            if y < search_envelope.min_y || y > search_envelope.max_y {
                return None;
            }
            Some((
                Point2::new(search_envelope.min_x - 10.0, y),
                Point2::new(search_envelope.max_x + 10.0, y),
            ))
        }
        crate::guideline::GuidelineOrientation::Vertical(x) => {
            if x < search_envelope.min_x || x > search_envelope.max_x {
                return None;
            }
            Some((
                Point2::new(x, search_envelope.min_y - 10.0),
                Point2::new(x, search_envelope.max_y + 10.0),
            ))
        }
        crate::guideline::GuidelineOrientation::Angled { origin, angle_rad } => {
            let ux = angle_rad.cos();
            let uy = angle_rad.sin();
            let len = 1000.0;
            Some((
                Point2::new(origin.x - len * ux, origin.y - len * uy),
                Point2::new(origin.x + len * ux, origin.y + len * uy),
            ))
        }
    }
}

fn intersect_cubic_with_segment<F>(
    b: &CubicBezier,
    g1: Point2,
    g2: Point2,
    search_envelope: &Rect,
    depth: u8,
    on_hit: &mut F,
) where
    F: FnMut(Point2),
{
    let box_b = cubic_control_bounds(b);
    if !box_b.intersects(search_envelope) {
        return;
    }

    if depth >= MAX_INTERSECT_DEPTH || is_cubic_flat(b, 0.02) {
        if let Some(pt) = segment_intersection(b.p0, b.p3, g1, g2) {
            if search_envelope.contains(pt) {
                on_hit(pt);
            }
        }
        return;
    }

    let (l, r) = b.split_half();
    intersect_cubic_with_segment(&l, g1, g2, search_envelope, depth + 1, on_hit);
    intersect_cubic_with_segment(&r, g1, g2, search_envelope, depth + 1, on_hit);
}

/// Visits every intersection point between `curve` and `guideline` inside `search_envelope`
/// without allocating heap memory.
pub fn for_each_polycurve_guideline_intersection<F>(
    curve: &Polycurve,
    guideline: &Guideline,
    search_envelope: &Rect,
    mut on_hit: F,
) where
    F: FnMut(Point2),
{
    let Some((g1, g2)) = guideline_segment(guideline, search_envelope) else {
        return;
    };

    for sub in &curve.subpaths {
        let n = sub.nodes.len();
        if n < 2 {
            continue;
        }
        let segs = if sub.closed { n } else { n - 1 };

        for i in 0..segs {
            let b = segment_cubic(&sub.nodes[i], &sub.nodes[(i + 1) % n]);
            intersect_cubic_with_segment(&b, g1, g2, search_envelope, 0, &mut on_hit);
        }
    }
}

/// Fast 2D line segment intersection.
/// Returns intersection point if segments p1-p2 and p3-p4 intersect.
pub fn segment_intersection(p1: Point2, p2: Point2, p3: Point2, p4: Point2) -> Option<Point2> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let det = d1x * d2y - d1y * d2x;
    if det.abs() < 1e-9 {
        return None; // Parallel or collinear
    }

    let dx = p3.x - p1.x;
    let dy = p3.y - p1.y;

    let t1 = (dx * d2y - dy * d2x) / det;
    let t2 = (dx * d1y - dy * d1x) / det;

    if (-1e-9..=1.0 + 1e-9).contains(&t1) && (-1e-9..=1.0 + 1e-9).contains(&t2) {
        let t1 = t1.clamp(0.0, 1.0);
        Some(Point2::new(p1.x + t1 * d1x, p1.y + t1 * d1y))
    } else {
        None
    }
}

/// Find all intersection points between two polycurves within an optional bounding envelope.
pub fn polycurve_intersections(
    c1: &Polycurve,
    c2: &Polycurve,
    search_envelope: Option<&Rect>,
) -> Vec<Point2> {
    let mut intersections = Vec::new();
    for_each_polycurve_intersection(c1, c2, search_envelope, |pt| {
        if !intersections
            .iter()
            .any(|existing: &Point2| existing.distance(pt) < 0.01)
        {
            intersections.push(pt);
        }
    });
    intersections
}

/// Find intersections between a polycurve and a guideline within a bounding envelope.
pub fn polycurve_guideline_intersections(
    curve: &Polycurve,
    guideline: &Guideline,
    search_envelope: &Rect,
) -> Vec<Point2> {
    let mut intersections = Vec::new();
    for_each_polycurve_guideline_intersection(curve, guideline, search_envelope, |pt| {
        if !intersections
            .iter()
            .any(|existing: &Point2| existing.distance(pt) < 0.01)
        {
            intersections.push(pt);
        }
    });
    intersections
}
