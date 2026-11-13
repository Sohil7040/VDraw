use crate::style::Style;
use core_geom::{Affine2, CornerStyle, Point2, Polycurve, Rect};
use serde::{Deserialize, Serialize};

/// Unique identifier for shapes in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShapeId(pub u64);

impl ShapeId {
    /// Sentinel identifier representing no concrete shape owner (e.g. grid/guidelines).
    pub const NONE: Self = Self(0);
}

impl std::fmt::Display for ShapeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Geometric shape types supported by VDraw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeKind {
    /// Arbitrary Bézier polycurve (open or closed subpaths).
    Path(Polycurve),

    /// Rectangle with optional corner radius.
    Rect {
        rect: Rect,
        corner_radius: f64,
        corner_style: CornerStyle,
    },

    /// Ellipse defined by center and radii.
    Ellipse {
        center: Point2,
        rx: f64,
        ry: f64,
    },

    /// Straight 2-point line segment.
    Line {
        start: Point2,
        end: Point2,
    },

    // ── Phase 2+ shape placeholders ──
    Polygon {
        center: Point2,
        radius: f64,
        sides: u32,
    },
    Star {
        center: Point2,
        outer_radius: f64,
        inner_radius: f64,
        points: u32,
    },
    Spiral {
        center: Point2,
        turns: f64,
        expansion: f64,
    },
    Text {
        text: String,
        font_size: f64,
        position: Point2,
    },
    Group {
        shapes: Vec<Shape>,
    },
}

/// A scene object element containing geometry, transformation, style, and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub name: String,
    pub kind: ShapeKind,
    pub transform: Affine2,
    pub style: Style,
    pub visible: bool,
    pub locked: bool,
}

impl Shape {
    pub fn new(id: ShapeId, name: impl Into<String>, kind: ShapeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            transform: Affine2::IDENTITY,
            style: Style::default_outline(),
            visible: true,
            locked: false,
        }
    }

    pub fn rectangle(id: ShapeId, rect: Rect, corner_radius: f64) -> Self {
        Self::new(
            id,
            "Rectangle",
            ShapeKind::Rect {
                rect,
                corner_radius,
                corner_style: CornerStyle::Round,
            },
        )
    }

    pub fn ellipse(id: ShapeId, center: Point2, rx: f64, ry: f64) -> Self {
        Self::new(id, "Ellipse", ShapeKind::Ellipse { center, rx, ry })
    }

    pub fn line(id: ShapeId, start: Point2, end: Point2) -> Self {
        Self::new(id, "Line", ShapeKind::Line { start, end })
    }

    pub fn path(id: ShapeId, polycurve: Polycurve) -> Self {
        Self::new(id, "Path", ShapeKind::Path(polycurve))
    }

    pub fn polygon(id: ShapeId, center: Point2, radius: f64, sides: u32) -> Self {
        Self::new(
            id,
            "Polygon",
            ShapeKind::Polygon {
                center,
                radius,
                sides,
            },
        )
    }

    pub fn star(
        id: ShapeId,
        center: Point2,
        outer_radius: f64,
        inner_radius: f64,
        points: u32,
    ) -> Self {
        Self::new(
            id,
            "Star",
            ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                points,
            },
        )
    }

    pub fn spiral(id: ShapeId, center: Point2, turns: f64, expansion: f64) -> Self {
        Self::new(
            id,
            "Spiral",
            ShapeKind::Spiral {
                center,
                turns,
                expansion,
            },
        )
    }

    pub fn group(id: ShapeId, shapes: Vec<Shape>) -> Self {
        Self::new(id, "Group", ShapeKind::Group { shapes })
    }

    pub fn is_group(&self) -> bool {
        matches!(self.kind, ShapeKind::Group { .. })
    }

    /// Recursively visits all leaf shapes in this shape or group, accumulating affine transforms.
    pub fn for_each_primitive<F>(&self, parent_tr: &Affine2, f: &mut F)
    where
        F: FnMut(&Shape, &Affine2),
    {
        let acc = *parent_tr * self.transform;
        match &self.kind {
            ShapeKind::Group { shapes } => {
                for child in shapes {
                    child.for_each_primitive(&acc, f);
                }
            }
            _ => {
                f(self, &acc);
            }
        }
    }

    /// Converts parametric primitive shape kinds (Rect, Ellipse, Polygon, etc.)
    /// into an editable cubic Bézier `Path`.
    pub fn convert_to_path(&mut self) {
        if !matches!(self.kind, ShapeKind::Path(_)) {
            let poly = self.to_polycurve();
            self.kind = ShapeKind::Path(poly);
        }
    }

    /// Normalise the shape geometry to a cubic Bézier `Polycurve`.
    pub fn to_polycurve(&self) -> Polycurve {
        match &self.kind {
            ShapeKind::Path(poly) => poly.clone(),
            ShapeKind::Rect {
                rect,
                corner_radius,
                corner_style,
            } => Polycurve::rectangle(*rect, *corner_radius, *corner_style),
            ShapeKind::Ellipse { center, rx, ry } => Polycurve::ellipse(*center, *rx, *ry),
            ShapeKind::Line { start, end } => Polycurve::line(*start, *end),
            ShapeKind::Polygon {
                center,
                radius,
                sides,
            } => Polycurve::polygon(*center, *radius, *sides),
            ShapeKind::Star {
                center,
                outer_radius,
                inner_radius,
                points,
            } => Polycurve::star(*center, *outer_radius, *inner_radius, *points),
            ShapeKind::Spiral {
                center,
                turns,
                expansion,
            } => Polycurve::spiral(*center, *turns, *expansion),
            ShapeKind::Group { shapes } => {
                let mut combined = Polycurve::new();
                for child in shapes {
                    let child_poly = child.to_polycurve();
                    let transformed = child_poly.transform(&child.transform);
                    combined.subpaths.extend(transformed.subpaths);
                }
                combined
            }
            ShapeKind::Text { .. } => Polycurve::new(),
        }
    }

    /// Local axis-aligned bounding box (untransformed).
    pub fn local_bounds(&self) -> Option<Rect> {
        match &self.kind {
            ShapeKind::Path(poly) => poly.bounds(),
            ShapeKind::Rect { rect, .. } => Some(*rect),
            ShapeKind::Ellipse { center, rx, ry } => Some(Rect::new(
                center.x - rx,
                center.y - ry,
                center.x + rx,
                center.y + ry,
            )),
            ShapeKind::Line { start, end } => Some(Rect::from_points(*start, *end)),
            ShapeKind::Group { shapes } => {
                let mut total: Option<Rect> = None;
                for child in shapes {
                    if let Some(cb) = child.world_bounds() {
                        total = Some(total.map_or(cb, |t| t.union(&cb)));
                    }
                }
                total
            }
            _ => self.to_polycurve().bounds(),
        }
    }

    /// World-space bounding box including the shape's affine transform.
    pub fn world_bounds(&self) -> Option<Rect> {
        let local = self.local_bounds()?;
        Some(self.transform.transform_rect(&local))
    }

    /// Test if a query point in world space hits this shape (within `tolerance` mm).
    pub fn hit_test(&self, world_pt: Point2, tolerance: f64) -> bool {
        // Quick rejection via world bounding box inflated by tolerance
        if let Some(b) = self.world_bounds() {
            if !b.inflate(tolerance, tolerance).contains(world_pt) {
                return false;
            }
        } else {
            return false;
        }

        // Map point into local space via inverse transform
        let local_pt = match self.transform.inverse() {
            Some(inv) => inv.transform_point(world_pt),
            None => return false,
        };

        match &self.kind {
            ShapeKind::Group { shapes } => {
                for child in shapes {
                    if child.hit_test(local_pt, tolerance) {
                        return true;
                    }
                }
                false
            }
            ShapeKind::Rect { rect, .. } => {
                let inflated = rect.inflate(tolerance, tolerance);
                if self.style.fill.is_some() {
                    inflated.contains(local_pt)
                } else {
                    // Stroke-only hit test
                    let inner = rect.inflate(-tolerance, -tolerance);
                    inflated.contains(local_pt) && !inner.contains(local_pt)
                }
            }
            ShapeKind::Ellipse { center, rx, ry } => {
                let dx = (local_pt.x - center.x) / (rx + tolerance);
                let dy = (local_pt.y - center.y) / (ry + tolerance);
                let d_sq = dx * dx + dy * dy;
                if self.style.fill.is_some() {
                    d_sq <= 1.0
                } else {
                    let in_dx = (local_pt.x - center.x) / (rx - tolerance).max(0.1);
                    let in_dy = (local_pt.y - center.y) / (ry - tolerance).max(0.1);
                    let in_d_sq = in_dx * in_dx + in_dy * in_dy;
                    d_sq <= 1.0 && in_d_sq >= 1.0
                }
            }
            ShapeKind::Line { start, end } => {
                let seg = core_geom::CubicBezier::line(*start, *end);
                let (_, _, dist) = seg.closest_point(local_pt);
                dist <= tolerance
            }
            _ => {
                // Polycurve distance check
                let poly = self.to_polycurve();
                for sub in &poly.subpaths {
                    for cubic in sub.to_cubics() {
                        let (_, _, dist) = cubic.closest_point(local_pt);
                        if dist <= tolerance {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}
