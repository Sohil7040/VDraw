//! High-performance geometry snapping engine and spatial index for VDraw.
//!
//! Provides R*-Tree spatial indexing with sub-millisecond query latency across 50,000+ shapes,
//! geometric snap target resolution (nodes, midpoints, centers, quadrants, intersections,
//! perpendicular/tangent, edges, 9-point bbox, guidelines, grid), dynamic polar angle guides
//! (0°, 15°, 30°, 45°, 60°, 75°, 90°) with distance ticks, and smart alignment guides
//! (edge-to-edge, center-to-center, equal spacing).

pub mod alignment;
pub mod dynamic_guide;
pub mod grid;
pub mod guideline;
pub mod intersection;
pub mod solver;
pub mod spatial_index;
pub mod target;

pub use alignment::{AlignmentConfig, AlignmentResult};
pub use dynamic_guide::DynamicGuideConfig;
pub use grid::{GridConfig, GridStyle};
pub use guideline::{Guideline, GuidelineId, GuidelineOrientation};
pub use solver::{SnapConfig, SnapSolver};
pub use spatial_index::{IndexedShape, SpatialIndex};
pub use target::{
    AlignmentLine, DynamicRay, SnapResult, SnapTarget, SnapTargetKind, TransformSnapResult,
};
