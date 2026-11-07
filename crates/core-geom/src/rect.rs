use crate::point::Point2;
use serde::{Deserialize, Serialize};

/// An axis-aligned rectangle defined by min and max coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect {
    pub const ZERO: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
    };

    #[inline]
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    #[inline]
    pub fn from_origin_size(origin: Point2, width: f64, height: f64) -> Self {
        Self::new(origin.x, origin.y, origin.x + width, origin.y + height)
    }

    #[inline]
    pub fn from_points(p0: Point2, p1: Point2) -> Self {
        Self::new(p0.x, p0.y, p1.x, p1.y)
    }

    #[inline]
    pub fn from_point(p: Point2) -> Self {
        Self {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        }
    }

    #[inline]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    #[inline]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    #[inline]
    pub fn min(&self) -> Point2 {
        Point2::new(self.min_x, self.min_y)
    }

    #[inline]
    pub fn max(&self) -> Point2 {
        Point2::new(self.max_x, self.max_y)
    }

    #[inline]
    pub fn center(&self) -> Point2 {
        Point2::new(
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
        )
    }

    #[inline]
    pub fn contains(&self, p: Point2) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.y >= self.min_y && p.y <= self.max_y
    }

    #[inline]
    pub fn contains_rect(&self, other: &Self) -> bool {
        other.min_x >= self.min_x
            && other.max_x <= self.max_x
            && other.min_y >= self.min_y
            && other.max_y <= self.max_y
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    #[inline]
    pub fn include_point(&mut self, p: Point2) {
        self.min_x = self.min_x.min(p.x);
        self.min_y = self.min_y.min(p.y);
        self.max_x = self.max_x.max(p.x);
        self.max_y = self.max_y.max(p.y);
    }

    #[inline]
    pub fn inflate(&self, dx: f64, dy: f64) -> Self {
        Self {
            min_x: self.min_x - dx,
            min_y: self.min_y - dy,
            max_x: self.max_x + dx,
            max_y: self.max_y + dy,
        }
    }

    #[inline]
    pub fn translate(&self, delta: Point2) -> Self {
        Self {
            min_x: self.min_x + delta.x,
            min_y: self.min_y + delta.y,
            max_x: self.max_x + delta.x,
            max_y: self.max_y + delta.y,
        }
    }

    /// Calculate the minimum Euclidean distance from a point to the rectangle.
    /// Returns 0.0 if the point is inside the rectangle.
    #[inline]
    pub fn distance_to_point(&self, p: Point2) -> f64 {
        let dx = (self.min_x - p.x).max(0.0).max(p.x - self.max_x);
        let dy = (self.min_y - p.y).max(0.0).max(p.y - self.max_y);
        (dx * dx + dy * dy).sqrt()
    }

    /// Returns the 9 canonical reference points of the bounding box:
    /// [TopLeft, TopCenter, TopRight, MidLeft, Center, MidRight, BottomLeft, BottomCenter, BottomRight].
    #[inline]
    pub fn nine_points(&self) -> [Point2; 9] {
        let mid_x = (self.min_x + self.max_x) * 0.5;
        let mid_y = (self.min_y + self.max_y) * 0.5;
        [
            Point2::new(self.min_x, self.min_y), // TopLeft
            Point2::new(mid_x, self.min_y),      // TopCenter
            Point2::new(self.max_x, self.min_y), // TopRight
            Point2::new(self.min_x, mid_y),      // MidLeft
            Point2::new(mid_x, mid_y),           // Center
            Point2::new(self.max_x, mid_y),      // MidRight
            Point2::new(self.min_x, self.max_y), // BottomLeft
            Point2::new(mid_x, self.max_y),      // BottomCenter
            Point2::new(self.max_x, self.max_y), // BottomRight
        ]
    }
}

// ─── Kurbo interop ───────────────────────────────────────────────────────────

impl From<kurbo::Rect> for Rect {
    #[inline]
    fn from(r: kurbo::Rect) -> Self {
        Self {
            min_x: r.x0,
            min_y: r.y0,
            max_x: r.x1,
            max_y: r.y1,
        }
    }
}

impl From<Rect> for kurbo::Rect {
    #[inline]
    fn from(r: Rect) -> Self {
        kurbo::Rect::new(r.min_x, r.min_y, r.max_x, r.max_y)
    }
}
