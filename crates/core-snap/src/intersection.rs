use crate::guideline::Guideline;
use core_geom::{Point2, Polycurve, Rect};

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

    if (0.0..=1.0).contains(&t1) && (0.0..=1.0).contains(&t2) {
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

    // Flatten both polycurves with tolerance 0.05 mm
    let lines1 = c1.flatten(0.05);
    let lines2 = c2.flatten(0.05);

    if lines1.len() < 2 || lines2.len() < 2 {
        return intersections;
    }

    for i in 0..(lines1.len() - 1) {
        let p1 = lines1[i];
        let p2 = lines1[i + 1];
        let seg1_box = Rect::from_points(p1, p2);

        if let Some(env) = search_envelope {
            if !seg1_box.intersects(env) {
                continue;
            }
        }

        for j in 0..(lines2.len() - 1) {
            let p3 = lines2[j];
            let p4 = lines2[j + 1];
            let seg2_box = Rect::from_points(p3, p4);

            if !seg1_box.intersects(&seg2_box) {
                continue;
            }

            if let Some(pt) = segment_intersection(p1, p2, p3, p4) {
                if let Some(env) = search_envelope {
                    if !env.contains(pt) {
                        continue;
                    }
                }
                // Avoid near duplicates
                if !intersections
                    .iter()
                    .any(|existing: &Point2| existing.distance(pt) < 0.01)
                {
                    intersections.push(pt);
                }
            }
        }
    }

    intersections
}

/// Find intersections between a polycurve and a guideline within a bounding envelope.
pub fn polycurve_guideline_intersections(
    curve: &Polycurve,
    guideline: &Guideline,
    search_envelope: &Rect,
) -> Vec<Point2> {
    let mut intersections = Vec::new();
    let lines = curve.flatten(0.05);

    if lines.len() < 2 {
        return intersections;
    }

    // Represent the guideline across the search envelope as a long line segment
    let (g1, g2) = match guideline.orientation {
        crate::guideline::GuidelineOrientation::Horizontal(y) => {
            if y < search_envelope.min_y || y > search_envelope.max_y {
                return intersections;
            }
            (
                Point2::new(search_envelope.min_x - 10.0, y),
                Point2::new(search_envelope.max_x + 10.0, y),
            )
        }
        crate::guideline::GuidelineOrientation::Vertical(x) => {
            if x < search_envelope.min_x || x > search_envelope.max_x {
                return intersections;
            }
            (
                Point2::new(x, search_envelope.min_y - 10.0),
                Point2::new(x, search_envelope.max_y + 10.0),
            )
        }
        crate::guideline::GuidelineOrientation::Angled { origin, angle_rad } => {
            let ux = angle_rad.cos();
            let uy = angle_rad.sin();
            let len = 1000.0;
            (
                Point2::new(origin.x - len * ux, origin.y - len * uy),
                Point2::new(origin.x + len * ux, origin.y + len * uy),
            )
        }
    };

    for i in 0..(lines.len() - 1) {
        let p1 = lines[i];
        let p2 = lines[i + 1];
        let seg_box = Rect::from_points(p1, p2);

        if !seg_box.intersects(search_envelope) {
            continue;
        }

        if let Some(pt) = segment_intersection(p1, p2, g1, g2) {
            if search_envelope.contains(pt)
                && !intersections
                    .iter()
                    .any(|existing: &Point2| existing.distance(pt) < 0.01)
            {
                intersections.push(pt);
            }
        }
    }

    intersections
}
