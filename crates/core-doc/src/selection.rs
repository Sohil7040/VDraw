use crate::shape::ShapeId;
use core_geom::{Point2, Rect};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Active transform interaction mode for the selection box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TransformMode {
    #[default]
    Scale, // 8 resize handles
    RotateSkew, // Corner rotation handles, edge skew handles, center pivot
}

/// Handle identifiers on the selection bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandleId {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    CenterPivot,
    RotateTopLeft,
    RotateTopRight,
    RotateBottomRight,
    RotateBottomLeft,
    SkewTop,
    SkewBottom,
    SkewLeft,
    SkewRight,
}

/// Position and type of an interactive handle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleInfo {
    pub id: HandleId,
    pub point: Point2,
}

/// Selection state for the active document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    pub ids: HashSet<ShapeId>,
    pub transform_mode: TransformMode,
    pub custom_pivot: Option<Point2>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn contains(&self, id: ShapeId) -> bool {
        self.ids.contains(&id)
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.transform_mode = TransformMode::Scale;
        self.custom_pivot = None;
    }

    pub fn select_single(&mut self, id: ShapeId) {
        if self.ids.len() == 1 && self.ids.contains(&id) {
            // Second click on already selected single shape toggles Rotate/Skew mode!
            self.transform_mode = match self.transform_mode {
                TransformMode::Scale => TransformMode::RotateSkew,
                TransformMode::RotateSkew => TransformMode::Scale,
            };
        } else {
            self.ids.clear();
            self.ids.insert(id);
            self.transform_mode = TransformMode::Scale;
        }
    }

    pub fn toggle(&mut self, id: ShapeId) {
        if self.ids.contains(&id) {
            self.ids.remove(&id);
        } else {
            self.ids.insert(id);
        }
        self.transform_mode = TransformMode::Scale;
    }

    pub fn add(&mut self, id: ShapeId) {
        self.ids.insert(id);
    }

    /// Calculate the pivot point (center of bounding box or custom user pivot).
    pub fn pivot(&self, bounds: Rect) -> Point2 {
        self.custom_pivot.unwrap_or_else(|| bounds.center())
    }

    /// Compute handle positions for the given selection bounding box according to current mode.
    pub fn handles(&self, bounds: Rect) -> Vec<HandleInfo> {
        let min_x = bounds.min_x;
        let min_y = bounds.min_y;
        let max_x = bounds.max_x;
        let max_y = bounds.max_y;
        let mid_x = (min_x + max_x) * 0.5;
        let mid_y = (min_y + max_y) * 0.5;

        match self.transform_mode {
            TransformMode::Scale => vec![
                HandleInfo {
                    id: HandleId::TopLeft,
                    point: Point2::new(min_x, min_y),
                },
                HandleInfo {
                    id: HandleId::Top,
                    point: Point2::new(mid_x, min_y),
                },
                HandleInfo {
                    id: HandleId::TopRight,
                    point: Point2::new(max_x, min_y),
                },
                HandleInfo {
                    id: HandleId::Right,
                    point: Point2::new(max_x, mid_y),
                },
                HandleInfo {
                    id: HandleId::BottomRight,
                    point: Point2::new(max_x, max_y),
                },
                HandleInfo {
                    id: HandleId::Bottom,
                    point: Point2::new(mid_x, max_y),
                },
                HandleInfo {
                    id: HandleId::BottomLeft,
                    point: Point2::new(min_x, max_y),
                },
                HandleInfo {
                    id: HandleId::Left,
                    point: Point2::new(min_x, mid_y),
                },
            ],
            TransformMode::RotateSkew => vec![
                HandleInfo {
                    id: HandleId::RotateTopLeft,
                    point: Point2::new(min_x, min_y),
                },
                HandleInfo {
                    id: HandleId::RotateTopRight,
                    point: Point2::new(max_x, min_y),
                },
                HandleInfo {
                    id: HandleId::RotateBottomRight,
                    point: Point2::new(max_x, max_y),
                },
                HandleInfo {
                    id: HandleId::RotateBottomLeft,
                    point: Point2::new(min_x, max_y),
                },
                HandleInfo {
                    id: HandleId::SkewTop,
                    point: Point2::new(mid_x, min_y),
                },
                HandleInfo {
                    id: HandleId::SkewBottom,
                    point: Point2::new(mid_x, max_y),
                },
                HandleInfo {
                    id: HandleId::SkewLeft,
                    point: Point2::new(min_x, mid_y),
                },
                HandleInfo {
                    id: HandleId::SkewRight,
                    point: Point2::new(max_x, mid_y),
                },
                HandleInfo {
                    id: HandleId::CenterPivot,
                    point: self.pivot(bounds),
                },
            ],
        }
    }

    /// Hit-test selection handles at `world_pt` within `radius_mm`.
    pub fn hit_test_handle(
        &self,
        bounds: Rect,
        world_pt: Point2,
        radius_mm: f64,
    ) -> Option<HandleId> {
        let r_sq = radius_mm * radius_mm;
        for h in self.handles(bounds) {
            if h.point.distance_squared_to(world_pt) <= r_sq {
                return Some(h.id);
            }
        }
        None
    }
}
