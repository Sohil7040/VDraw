use core_doc::ShapeId;
use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Categories of snap targets supported by the VDraw snapping engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SnapTargetKind {
    /// Exact anchor node on a path or polygon vertex (Priority: 100).
    Node = 100,
    /// Terminal vertex of an open path or curve segment (Priority: 95).
    Endpoint = 95,
    /// Intersection of two curves, segments, or guidelines (Priority: 90).
    Intersection = 90,
    /// Midpoint of a segment or bounding box edge (Priority: 80).
    Midpoint = 80,
    /// Center / centroid of a shape or bounding box (Priority: 75).
    Center = 75,
    /// Quadrant point (cardinal extrema) on an ellipse or curve (Priority: 70).
    Quadrant = 70,
    /// Perpendicular projection from a reference point (Priority: 65).
    Perpendicular = 65,
    /// Tangent point from a reference point (Priority: 64).
    Tangent = 64,
    /// 9-point reference box (TopLeft, BottomRight, etc.) (Priority: 60).
    BboxPoint = 60,
    /// Smart alignment guide (edge-to-edge, center-to-center) (Priority: 55).
    AlignmentGuide = 55,
    /// User guideline (horizontal, vertical, angled) (Priority: 50).
    Guideline = 50,
    /// Dynamic polar guide ray (0°, 15°, 30°, 45°, etc.) (Priority: 45).
    DynamicGuide = 45,
    /// Nearest point on a curve outline / edge (Priority: 40).
    Edge = 40,
    /// Background coordinate grid (Priority: 20).
    Grid = 20,
}

impl SnapTargetKind {
    /// Default priority score used for winner resolution when distances are equal.
    pub fn default_priority(&self) -> u8 {
        *self as u8
    }

    /// User-facing short label for the HUD badge.
    pub fn label_name(&self) -> &'static str {
        match self {
            Self::Node => "Node",
            Self::Endpoint => "Endpoint",
            Self::Intersection => "Intersection",
            Self::Midpoint => "Midpoint",
            Self::Center => "Center",
            Self::Quadrant => "Quadrant",
            Self::Perpendicular => "Perpendicular",
            Self::Tangent => "Tangent",
            Self::BboxPoint => "Bbox",
            Self::AlignmentGuide => "Align",
            Self::Guideline => "Guide",
            Self::DynamicGuide => "Angle",
            Self::Edge => "Edge",
            Self::Grid => "Grid",
        }
    }
}

/// A specific candidate snap target resolved in world coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapTarget {
    pub kind: SnapTargetKind,
    pub point: Point2,
    pub distance: f64,
    pub priority: u8,
    pub label: String,
    pub source_id: Option<ShapeId>,
}

impl SnapTarget {
    pub fn new(
        kind: SnapTargetKind,
        point: Point2,
        distance: f64,
        source_id: Option<ShapeId>,
    ) -> Self {
        let label = kind.label_name().to_string();
        Self {
            kind,
            point,
            distance,
            priority: kind.default_priority(),
            label,
            source_id,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// A visual alignment guide line spanning across canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentLine {
    pub start: Point2,
    pub end: Point2,
    pub label: String,
    pub is_vertical: bool,
    pub color_rgb: [u8; 3],
}

impl AlignmentLine {
    pub fn new(start: Point2, end: Point2, label: impl Into<String>, is_vertical: bool) -> Self {
        Self {
            start,
            end,
            label: label.into(),
            is_vertical,
            color_rgb: [124, 92, 255], // Default purple/accent
        }
    }

    pub fn with_color(mut self, rgb: [u8; 3]) -> Self {
        self.color_rgb = rgb;
        self
    }
}

/// The outcome of evaluating snapping during a shape translation / move.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransformSnapResult {
    /// Snapped translation delta to apply to the shapes.
    pub delta: Point2,
    /// Winning snap target (if a point / node / center snap occurred).
    pub snap_target: Option<SnapTarget>,
    /// Active dynamic alignment lines (Center-to-Center, Edge-to-Edge, Bed Center, etc.).
    pub alignment_lines: Vec<AlignmentLine>,
    /// Summary label describing the snap (e.g. "Center to Center", "Node", "Bed Center X").
    pub label: Option<String>,
}

/// Dynamic angle ray originating from a reference point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicRay {
    pub origin: Point2,
    pub angle_rad: f64,
    pub angle_deg: f64,
    pub distance_mm: f64,
    pub tick_point: Option<Point2>,
}

/// The outcome of a snap solver evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapResult {
    /// True if a snap target was acquired.
    pub snapped: bool,
    /// Snapped world coordinate (or original query point if no snap occurred).
    pub point: Point2,
    /// The winning snap target, if snapped.
    pub target: Option<SnapTarget>,
    /// Active smart alignment lines to draw on canvas.
    pub alignment_lines: Vec<AlignmentLine>,
    /// Active dynamic angular guide ray to draw on canvas.
    pub dynamic_ray: Option<DynamicRay>,
}

impl SnapResult {
    pub fn no_snap(point: Point2) -> Self {
        Self {
            snapped: false,
            point,
            target: None,
            alignment_lines: Vec::new(),
            dynamic_ray: None,
        }
    }

    pub fn from_target(target: SnapTarget) -> Self {
        let point = target.point;
        Self {
            snapped: true,
            point,
            target: Some(target),
            alignment_lines: Vec::new(),
            dynamic_ray: None,
        }
    }
}
