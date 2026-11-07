use core_doc::{Shape, ShapeId};
use core_geom::{Point2, Polycurve, Rect};
use rstar::{PointDistance, RTree, RTreeObject, AABB};

/// Pre-computed geometric features of a shape stored in the spatial R*-tree.
#[derive(Debug, Clone)]
pub struct IndexedShape {
    pub shape_id: ShapeId,
    pub bounds: Rect,
    pub nodes: Vec<Point2>,
    pub midpoints: Vec<Point2>,
    pub center: Point2,
    pub quadrants: Vec<Point2>,
    pub bbox_points: [Point2; 9],
    pub polycurve: Polycurve,
}

impl PartialEq for IndexedShape {
    fn eq(&self, other: &Self) -> bool {
        self.shape_id == other.shape_id
    }
}

impl IndexedShape {
    /// Extract snap candidates and bounding envelope from a document shape.
    pub fn from_shape(shape: &Shape) -> Self {
        let polycurve = shape.to_polycurve();
        let bounds = polycurve.bounds().unwrap_or(Rect::ZERO);
        let center = bounds.center();
        let bbox_points = bounds.nine_points();

        let mut nodes = Vec::new();
        let mut midpoints = Vec::new();

        for subpath in &polycurve.subpaths {
            for node in &subpath.nodes {
                nodes.push(node.point);
            }

            // Extract segment midpoints at t = 0.5
            let count = subpath.nodes.len();
            if count >= 2 {
                let segments = if subpath.closed { count } else { count - 1 };
                for i in 0..segments {
                    let next = (i + 1) % count;
                    let n0 = &subpath.nodes[i];
                    let n1 = &subpath.nodes[next];
                    let h1 = n0.handle_out.unwrap_or(n0.point);
                    let h2 = n1.handle_in.unwrap_or(n1.point);
                    let b = core_geom::CubicBezier::new(n0.point, h1, h2, n1.point);
                    let (left, _) = b.split(0.5);
                    midpoints.push(left.p3);
                }
            }
        }

        // Compute quadrant extrema
        let quadrants = vec![
            Point2::new(center.x, bounds.min_y), // Top
            Point2::new(bounds.max_x, center.y), // Right
            Point2::new(center.x, bounds.max_y), // Bottom
            Point2::new(bounds.min_x, center.y), // Left
        ];

        Self {
            shape_id: shape.id,
            bounds,
            nodes,
            midpoints,
            center,
            quadrants,
            bbox_points,
            polycurve,
        }
    }
}

impl RTreeObject for IndexedShape {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bounds.min_x, self.bounds.min_y],
            [self.bounds.max_x, self.bounds.max_y],
        )
    }
}

impl PointDistance for IndexedShape {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let p = Point2::new(point[0], point[1]);
        let d = self.bounds.distance_to_point(p);
        d * d
    }
}

/// R*-Tree spatial index caching shapes and geometric snap features for ultra-fast queries.
#[derive(Debug, Clone, Default)]
pub struct SpatialIndex {
    tree: RTree<IndexedShape>,
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self { tree: RTree::new() }
    }

    /// Bulk load spatial items using O(N log N) STR (Sort-Tile-Recursive) packing.
    pub fn bulk_load(shapes: &[Shape]) -> Self {
        let items: Vec<IndexedShape> = shapes.iter().map(IndexedShape::from_shape).collect();
        Self {
            tree: RTree::bulk_load(items),
        }
    }

    /// Insert or update an individual shape.
    pub fn insert(&mut self, shape: &Shape) {
        self.remove(shape.id);
        self.tree.insert(IndexedShape::from_shape(shape));
    }

    /// Remove a shape by ID.
    pub fn remove(&mut self, shape_id: ShapeId) -> bool {
        let to_remove = self
            .tree
            .iter()
            .find(|item| item.shape_id == shape_id)
            .cloned();

        if let Some(item) = to_remove {
            self.tree.remove(&item).is_some()
        } else {
            false
        }
    }

    /// Rebuild index from a collection of shapes.
    pub fn rebuild(&mut self, shapes: &[Shape]) {
        *self = Self::bulk_load(shapes);
    }

    /// Return total indexed shapes.
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    /// Query shapes intersecting an axis-aligned envelope.
    pub fn locate_in_envelope<'a>(
        &'a self,
        bounds: &Rect,
    ) -> impl Iterator<Item = &'a IndexedShape> {
        let aabb = AABB::from_corners([bounds.min_x, bounds.min_y], [bounds.max_x, bounds.max_y]);
        self.tree.locate_in_envelope_intersecting(&aabb)
    }

    /// Find nearest shape to a point.
    pub fn nearest_neighbor(&self, p: Point2) -> Option<&IndexedShape> {
        self.tree.nearest_neighbor(&[p.x, p.y])
    }

    /// Iterate all indexed shapes.
    pub fn iter(&self) -> impl Iterator<Item = &IndexedShape> {
        self.tree.iter()
    }
}
