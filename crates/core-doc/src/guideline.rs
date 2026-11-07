use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Unique identifier for a guideline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GuidelineId(pub u64);

/// Geometric orientation and positioning of a guideline.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GuidelineOrientation {
    /// Horizontal line spanning the whole canvas at constant Y (in mm).
    Horizontal(f64),
    /// Vertical line spanning the whole canvas at constant X (in mm).
    Vertical(f64),
    /// Angled guideline passing through an origin with a specified angle in radians.
    Angled { origin: Point2, angle_rad: f64 },
}

/// A user guideline in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guideline {
    pub id: GuidelineId,
    pub orientation: GuidelineOrientation,
    pub locked: bool,
    pub name: Option<String>,
}

impl Guideline {
    pub fn horizontal(id: u64, y_mm: f64) -> Self {
        Self {
            id: GuidelineId(id),
            orientation: GuidelineOrientation::Horizontal(y_mm),
            locked: false,
            name: None,
        }
    }

    pub fn vertical(id: u64, x_mm: f64) -> Self {
        Self {
            id: GuidelineId(id),
            orientation: GuidelineOrientation::Vertical(x_mm),
            locked: false,
            name: None,
        }
    }

    pub fn angled(id: u64, origin: Point2, angle_rad: f64) -> Self {
        Self {
            id: GuidelineId(id),
            orientation: GuidelineOrientation::Angled { origin, angle_rad },
            locked: false,
            name: None,
        }
    }

    /// Calculate the minimum Euclidean distance from a point to this guideline.
    pub fn distance_to_point(&self, p: Point2) -> f64 {
        match self.orientation {
            GuidelineOrientation::Horizontal(y) => (p.y - y).abs(),
            GuidelineOrientation::Vertical(x) => (p.x - x).abs(),
            GuidelineOrientation::Angled { origin, angle_rad } => {
                let dx = p.x - origin.x;
                let dy = p.y - origin.y;
                let nx = -angle_rad.sin();
                let ny = angle_rad.cos();
                (dx * nx + dy * ny).abs()
            }
        }
    }

    /// Project a point orthogonally onto this guideline.
    pub fn project_point(&self, p: Point2) -> Point2 {
        match self.orientation {
            GuidelineOrientation::Horizontal(y) => Point2::new(p.x, y),
            GuidelineOrientation::Vertical(x) => Point2::new(x, p.y),
            GuidelineOrientation::Angled { origin, angle_rad } => {
                let dx = p.x - origin.x;
                let dy = p.y - origin.y;
                let ux = angle_rad.cos();
                let uy = angle_rad.sin();
                let proj = dx * ux + dy * uy;
                Point2::new(origin.x + proj * ux, origin.y + proj * uy)
            }
        }
    }

    /// Intersect this guideline with another guideline, if they meet at a single point.
    pub fn intersect(&self, other: &Self) -> Option<Point2> {
        match (self.orientation, other.orientation) {
            (GuidelineOrientation::Horizontal(y), GuidelineOrientation::Vertical(x))
            | (GuidelineOrientation::Vertical(x), GuidelineOrientation::Horizontal(y)) => {
                Some(Point2::new(x, y))
            }
            (
                GuidelineOrientation::Horizontal(y),
                GuidelineOrientation::Angled { origin, angle_rad },
            )
            | (
                GuidelineOrientation::Angled { origin, angle_rad },
                GuidelineOrientation::Horizontal(y),
            ) => {
                let sin_a = angle_rad.sin();
                if sin_a.abs() < 1e-9 {
                    None // Parallel
                } else {
                    let cos_a = angle_rad.cos();
                    let t = (y - origin.y) / sin_a;
                    Some(Point2::new(origin.x + t * cos_a, y))
                }
            }
            (
                GuidelineOrientation::Vertical(x),
                GuidelineOrientation::Angled { origin, angle_rad },
            )
            | (
                GuidelineOrientation::Angled { origin, angle_rad },
                GuidelineOrientation::Vertical(x),
            ) => {
                let cos_a = angle_rad.cos();
                if cos_a.abs() < 1e-9 {
                    None // Parallel
                } else {
                    let sin_a = angle_rad.sin();
                    let t = (x - origin.x) / cos_a;
                    Some(Point2::new(x, origin.y + t * sin_a))
                }
            }
            (
                GuidelineOrientation::Angled {
                    origin: o1,
                    angle_rad: a1,
                },
                GuidelineOrientation::Angled {
                    origin: o2,
                    angle_rad: a2,
                },
            ) => {
                let u1 = Point2::new(a1.cos(), a1.sin());
                let u2 = Point2::new(a2.cos(), a2.sin());
                let det = u1.x * u2.y - u1.y * u2.x;
                if det.abs() < 1e-9 {
                    None
                } else {
                    let dx = o2.x - o1.x;
                    let dy = o2.y - o1.y;
                    let t = (dx * u2.y - dy * u2.x) / det;
                    Some(Point2::new(o1.x + t * u1.x, o1.y + t * u1.y))
                }
            }
            _ => None, // Both horizontal or both vertical
        }
    }
}
