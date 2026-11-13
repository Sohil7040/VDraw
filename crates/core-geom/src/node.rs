use crate::point::Point2;
use serde::{Deserialize, Serialize};

/// Continuity type for a Bézier path node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum NodeType {
    /// Cusp ($G^0$ continuity): Position is continuous, but incoming and outgoing handles
    /// are completely independent in direction and length.
    #[default]
    Cusp,
    /// Smooth ($G^1$ geometric continuity): Handles are collinear (opposite directions),
    /// providing continuous unit tangent vectors while allowing independent handle lengths.
    Smooth,
    /// Symmetrical ($C^1$ parametric continuity): Handles are collinear and identical in length,
    /// providing identical first derivative vectors across the node junction.
    Symmetrical,
}

/// A node along a Bézier curve with incoming and outgoing control handles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BezierNode {
    /// Position of the anchor point in world space.
    pub point: Point2,
    /// Incoming control handle point (world coordinates), if any.
    pub handle_in: Option<Point2>,
    /// Outgoing control handle point (world coordinates), if any.
    pub handle_out: Option<Point2>,
    /// Node continuity rule.
    pub node_type: NodeType,
}

impl BezierNode {
    pub fn new(point: Point2, node_type: NodeType) -> Self {
        Self {
            point,
            handle_in: None,
            handle_out: None,
            node_type,
        }
    }

    pub fn with_handles(
        point: Point2,
        handle_in: Option<Point2>,
        handle_out: Option<Point2>,
        node_type: NodeType,
    ) -> Self {
        Self {
            point,
            handle_in,
            handle_out,
            node_type,
        }
    }

    /// Update `handle_out` and update `handle_in` according to `node_type`.
    pub fn set_handle_out(&mut self, new_out: Option<Point2>) {
        self.handle_out = new_out;
        if let Some(out_p) = new_out {
            let out_vec = out_p - self.point;
            let out_len = out_vec.length();

            match self.node_type {
                NodeType::Cusp => {
                    // Untouched
                }
                NodeType::Smooth => {
                    if let Some(in_p) = self.handle_in {
                        let in_len = (in_p - self.point).length();
                        if out_len > 1e-9 {
                            let dir = -out_vec.normalize();
                            self.handle_in = Some(self.point + dir * in_len);
                        }
                    }
                }
                NodeType::Symmetrical => {
                    self.handle_in = Some(self.point - out_vec);
                }
            }
        }
    }

    /// Update `handle_in` and update `handle_out` according to `node_type`.
    pub fn set_handle_in(&mut self, new_in: Option<Point2>) {
        self.handle_in = new_in;
        if let Some(in_p) = new_in {
            let in_vec = in_p - self.point;
            let in_len = in_vec.length();

            match self.node_type {
                NodeType::Cusp => {
                    // Untouched
                }
                NodeType::Smooth => {
                    if let Some(out_p) = self.handle_out {
                        let out_len = (out_p - self.point).length();
                        if in_len > 1e-9 {
                            let dir = -in_vec.normalize();
                            self.handle_out = Some(self.point + dir * out_len);
                        }
                    }
                }
                NodeType::Symmetrical => {
                    self.handle_out = Some(self.point - in_vec);
                }
            }
        }
    }

    /// Move the anchor point, translating both handles by the same delta.
    pub fn translate(&mut self, delta: Point2) {
        self.point += delta;
        if let Some(h_in) = &mut self.handle_in {
            *h_in += delta;
        }
        if let Some(h_out) = &mut self.handle_out {
            *h_out += delta;
        }
    }

    /// Transforms the node and its handles by an affine transformation.
    pub fn transform(&self, affine: &crate::Affine2) -> Self {
        Self {
            point: affine.transform_point(self.point),
            handle_in: self.handle_in.map(|h| affine.transform_point(h)),
            handle_out: self.handle_out.map(|h| affine.transform_point(h)),
            node_type: self.node_type,
        }
    }

    /// Enforce collinearity and length constraints according to `node_type`.
    pub fn enforce_constraints(&mut self) {
        match self.node_type {
            NodeType::Cusp => {}
            NodeType::Smooth => {
                if let (Some(out_p), Some(in_p)) = (self.handle_out, self.handle_in) {
                    let out_vec = out_p - self.point;
                    let out_len = out_vec.length();
                    let in_vec = in_p - self.point;
                    let in_len = in_vec.length();

                    if out_len > 1e-9 {
                        let dir = -out_vec.normalize();
                        self.handle_in = Some(self.point + dir * in_len);
                    } else if in_len > 1e-9 {
                        let dir = -in_vec.normalize();
                        self.handle_out = Some(self.point + dir * out_len);
                    }
                }
            }
            NodeType::Symmetrical => {
                if let Some(out_p) = self.handle_out {
                    let out_vec = out_p - self.point;
                    self.handle_in = Some(self.point - out_vec);
                } else if let Some(in_p) = self.handle_in {
                    let in_vec = in_p - self.point;
                    self.handle_out = Some(self.point - in_vec);
                }
            }
        }
    }

    /// Switch node type and adjust handles accordingly.
    pub fn set_node_type(&mut self, new_type: NodeType) {
        self.node_type = new_type;
        self.enforce_constraints();
    }
}
