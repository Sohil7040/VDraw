//! Frozen 2D snap engine contracts exposed to tools, documents, and UI overlays.

use crate::spatial_index::SpatialIndex;
use crate::target::SnapTargetKind;
use bitflags::bitflags;
use core_doc::EntityId;
use core_geom::Point2;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Default polar guide increment in degrees.
pub const POLAR_STEP_DEG: f64 = 15.0;

/// Canonical geometric snap target category across the [`SnapEngine`] API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SnapKind {
    Node,
    Center,
    Endpoint,
    Midpoint,
    Quadrant,
    Intersection,
    Grid,
    Perpendicular,
    Tangent,
}

impl SnapKind {
    /// All canonical snap kinds in deterministic tie-break priority order:
    /// `Node > Endpoint > Intersection > Midpoint > Quadrant > Center > Perpendicular > Tangent > Grid`.
    pub const ALL: [Self; 9] = [
        Self::Node,
        Self::Endpoint,
        Self::Intersection,
        Self::Midpoint,
        Self::Quadrant,
        Self::Center,
        Self::Perpendicular,
        Self::Tangent,
        Self::Grid,
    ];

    /// Priority rank used to break exact distance ties deterministically.
    #[inline]
    pub const fn priority(self) -> u8 {
        match self {
            Self::Node => 100,
            Self::Endpoint => 95,
            Self::Intersection => 90,
            Self::Midpoint => 85,
            Self::Quadrant => 80,
            Self::Center => 75,
            Self::Perpendicular => 70,
            Self::Tangent => 65,
            Self::Grid => 60,
        }
    }

    /// Short human-readable label displayed in the canvas HUD and status bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Node => "Node",
            Self::Center => "Center",
            Self::Endpoint => "Endpoint",
            Self::Midpoint => "Midpoint",
            Self::Quadrant => "Quadrant",
            Self::Intersection => "Intersection",
            Self::Grid => "Grid",
            Self::Perpendicular => "Perpendicular",
            Self::Tangent => "Tangent",
        }
    }
}

impl From<SnapKind> for SnapTargetKind {
    fn from(kind: SnapKind) -> Self {
        match kind {
            SnapKind::Node => SnapTargetKind::Node,
            SnapKind::Center => SnapTargetKind::Center,
            SnapKind::Endpoint => SnapTargetKind::Endpoint,
            SnapKind::Midpoint => SnapTargetKind::Midpoint,
            SnapKind::Quadrant => SnapTargetKind::Quadrant,
            SnapKind::Intersection => SnapTargetKind::Intersection,
            SnapKind::Grid => SnapTargetKind::Grid,
            SnapKind::Perpendicular => SnapTargetKind::Perpendicular,
            SnapKind::Tangent => SnapTargetKind::Tangent,
        }
    }
}

impl TryFrom<SnapTargetKind> for SnapKind {
    type Error = SnapTargetKind;

    fn try_from(kind: SnapTargetKind) -> Result<Self, Self::Error> {
        match kind {
            SnapTargetKind::Node => Ok(SnapKind::Node),
            SnapTargetKind::Center => Ok(SnapKind::Center),
            SnapTargetKind::Endpoint => Ok(SnapKind::Endpoint),
            SnapTargetKind::Midpoint => Ok(SnapKind::Midpoint),
            SnapTargetKind::Quadrant => Ok(SnapKind::Quadrant),
            SnapTargetKind::Intersection => Ok(SnapKind::Intersection),
            SnapTargetKind::Grid => Ok(SnapKind::Grid),
            SnapTargetKind::Perpendicular => Ok(SnapKind::Perpendicular),
            SnapTargetKind::Tangent => Ok(SnapKind::Tangent),
            other => Err(other),
        }
    }
}

bitflags! {
    /// Bitmask toggling active snap modes, orthogonal constraint, and polar tracking.
    ///
    /// Pure bitflags only — polar angle configuration lives in [`crate::SnapConfig`], not here.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SnapMask: u32 {
        const NODE          = 1 << 0;
        const ENDPOINT      = 1 << 1;
        const MIDPOINT      = 1 << 2;
        const CENTER        = 1 << 3;
        const QUADRANT      = 1 << 4;
        const INTERSECTION  = 1 << 5;
        const GRID          = 1 << 6;
        const ORTHO         = 1 << 7;
        const POLAR         = 1 << 8;
        const PERPENDICULAR = 1 << 9;
        const TANGENT       = 1 << 10;
    }
}

/// Default enabled snap modes (`Node | Endpoint | Center | Intersection`).
pub const DEFAULT_SNAP_MASK: SnapMask = SnapMask::NODE
    .union(SnapMask::ENDPOINT)
    .union(SnapMask::CENTER)
    .union(SnapMask::INTERSECTION);

impl Default for SnapMask {
    fn default() -> Self {
        DEFAULT_SNAP_MASK
    }
}

impl SnapMask {
    /// Returns `true` if `kind` is enabled in this mask.
    pub const fn allows(self, kind: SnapKind) -> bool {
        let bit = match kind {
            SnapKind::Node => Self::NODE,
            SnapKind::Center => Self::CENTER,
            SnapKind::Endpoint => Self::ENDPOINT,
            SnapKind::Midpoint => Self::MIDPOINT,
            SnapKind::Quadrant => Self::QUADRANT,
            SnapKind::Intersection => Self::INTERSECTION,
            SnapKind::Grid => Self::GRID,
            SnapKind::Perpendicular => Self::PERPENDICULAR,
            SnapKind::Tangent => Self::TANGENT,
        };
        self.contains(bit)
    }

    /// Toggles `kind` on or off in this mask.
    pub fn toggle_kind(&mut self, kind: SnapKind) {
        let bit = match kind {
            SnapKind::Node => Self::NODE,
            SnapKind::Center => Self::CENTER,
            SnapKind::Endpoint => Self::ENDPOINT,
            SnapKind::Midpoint => Self::MIDPOINT,
            SnapKind::Quadrant => Self::QUADRANT,
            SnapKind::Intersection => Self::INTERSECTION,
            SnapKind::Grid => Self::GRID,
            SnapKind::Perpendicular => Self::PERPENDICULAR,
            SnapKind::Tangent => Self::TANGENT,
        };
        self.toggle(bit);
    }
}

/// A 2D world-space snap candidate emitted by a [`SnapProvider`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPoint {
    pub position: Point2,
    pub kind: SnapKind,
    pub source: EntityId,
}

impl SnapPoint {
    /// Constructs a new 2D world-space snap candidate.
    pub const fn new(position: Point2, kind: SnapKind, source: EntityId) -> Self {
        Self {
            position,
            kind,
            source,
        }
    }
}

/// Source-side drag handle semantics used to compute snap translation deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HandleMode {
    #[default]
    Node,
    Center,
    Insertion,
    Edge,
}

impl HandleMode {
    /// Short user-facing label displayed in the bottom status bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Node => "Node",
            Self::Center => "Center",
            Self::Insertion => "Insertion",
            Self::Edge => "Edge",
        }
    }
}

/// Visual feedback payload returned by [`SnapEngine::resolve`] for UI rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapFeedback {
    /// Winning snap point in world coordinates.
    pub target: SnapPoint,
    /// Projected screen-space coordinate where the glow & badge are drawn.
    pub screen_pos: egui::Pos2,
    /// Outer glow halo radius in logical pixels.
    pub glow_radius_px: f32,
    /// Static snap category label displayed beside the indicator (`SnapKind::label()`).
    pub label: &'static str,
    /// Active source handle mode.
    pub handle_mode: HandleMode,
}

/// Canonical 2D viewport projection contract shared by `core-snap`, `core-tools`, and `app`.
///
/// UI (`CanvasState`) constructs/borrows this single `View` definition rather than duplicating
/// screen-to-world projection math in separate crates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Canvas viewport bounds in logical screen pixels.
    pub viewport_rect: egui::Rect,
    /// Current pan offset in logical screen pixels.
    pub translation: egui::Vec2,
    /// Screen pixels per world millimetre (`zoom * PX_PER_MM`).
    pub scale: f64,
}

impl View {
    /// Constructs a 2D [`View`].
    pub const fn new(viewport_rect: egui::Rect, translation: egui::Vec2, scale: f64) -> Self {
        Self {
            viewport_rect,
            translation,
            scale,
        }
    }

    /// Projects a world-space point (mm) into logical screen coordinates (`egui::Pos2`).
    #[inline]
    pub fn world_to_screen(&self, world: Point2) -> egui::Pos2 {
        egui::pos2(
            self.viewport_rect.min.x + (world.x * self.scale) as f32 + self.translation.x,
            self.viewport_rect.min.y + (world.y * self.scale) as f32 + self.translation.y,
        )
    }

    /// Unprojects logical screen coordinates back into world-space mm (`Point2`).
    #[inline]
    pub fn screen_to_world(&self, screen: egui::Pos2) -> Point2 {
        if !self.scale.is_finite() || self.scale.abs() <= 1e-12 {
            return Point2::ZERO;
        }
        Point2::new(
            ((screen.x - self.viewport_rect.min.x - self.translation.x) as f64) / self.scale,
            ((screen.y - self.viewport_rect.min.y - self.translation.y) as f64) / self.scale,
        )
    }
}

/// Query context passed to [`SnapProvider`] implementations and [`SnapEngine::resolve`].
#[derive(Debug, Clone, PartialEq)]
pub struct SnapCtx<'a> {
    pub cursor_screen: egui::Pos2,
    pub view: &'a View,
    pub threshold_px: f32,
}

/// Candidate extraction trait implemented by geometry, grid, and guide sources.
pub trait SnapProvider: Send + Sync {
    /// Extracts candidate snap points matching `ctx`.
    fn candidates(&self, ctx: &SnapCtx<'_>) -> Vec<SnapPoint>;

    /// Allocation-reusing overload for hot paths. Default forwards to [`Self::candidates`].
    fn collect_candidates(&self, ctx: &SnapCtx<'_>, out: &mut Vec<SnapPoint>) {
        out.extend(self.candidates(ctx));
    }
}

/// Pure, deterministic 2D snapping engine combining registered [`SnapProvider`]s,
/// [`SnapMask`], and spatial indexing.
pub struct SnapEngine {
    pub providers: Vec<Box<dyn SnapProvider>>,
    pub modes: SnapMask,
    pub handle_mode: HandleMode,
    pub spatial_index: SpatialIndex,
    scratch: std::cell::RefCell<Vec<SnapPoint>>,
}

impl std::fmt::Debug for SnapEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapEngine")
            .field("provider_count", &self.providers.len())
            .field("modes", &self.modes)
            .field("handle_mode", &self.handle_mode)
            .finish()
    }
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self::new(DEFAULT_SNAP_MASK)
    }
}

impl SnapEngine {
    /// Creates an empty [`SnapEngine`] with the given [`SnapMask`].
    pub fn new(modes: SnapMask) -> Self {
        Self {
            providers: Vec::new(),
            modes,
            handle_mode: HandleMode::Node,
            spatial_index: SpatialIndex::new(),
            scratch: std::cell::RefCell::new(Vec::with_capacity(256)),
        }
    }

    /// Resolves the winning candidate deterministically without mutating semantic state
    /// or allocating in the hot path once the internal scratch buffer is warmed.
    ///
    /// Contract guarantees:
    /// - Pure (`&self`) and idempotent across identical queries.
    /// - Converts `ctx.threshold_px` into world units using `ctx.view.scale`.
    /// - Visits candidates in deterministic provider/insertion order.
    /// - Breaks equal-distance ties (`|d1 - d2| <= 1e-6`) by priority:
    ///   `Node > Endpoint > Intersection > Midpoint > Quadrant > Center > Perpendicular > Tangent > Grid`.
    /// - Returns `None` if no candidate falls within `ctx.threshold_px` (never returns far misses).
    #[inline]
    pub fn resolve(&self, ctx: &SnapCtx<'_>) -> Option<(SnapPoint, SnapFeedback)> {
        let mut scratch = self.scratch.borrow_mut();
        self.resolve_with_scratch(ctx, &mut scratch)
    }

    /// Allocation-free variant writing provider candidates into `scratch`.
    #[inline]
    pub fn resolve_with_scratch(
        &self,
        ctx: &SnapCtx<'_>,
        scratch: &mut Vec<SnapPoint>,
    ) -> Option<(SnapPoint, SnapFeedback)> {
        if self.modes.is_empty() || ctx.threshold_px <= 0.0 || !ctx.view.scale.is_finite() {
            return None;
        }
        let cursor_world = ctx.view.screen_to_world(ctx.cursor_screen);
        let threshold_world = (ctx.threshold_px as f64) / ctx.view.scale.abs().max(1e-12);
        let threshold_sq = threshold_world * threshold_world;

        scratch.clear();
        for provider in &self.providers {
            provider.collect_candidates(ctx, scratch);
        }

        let mut best: Option<(usize, f64, egui::Pos2)> = None;
        for (idx, candidate) in scratch.iter().enumerate() {
            if !self.modes.allows(candidate.kind) || !candidate.position.is_finite() {
                continue;
            }
            let dist_sq = candidate.position.distance_squared_to(cursor_world);
            if dist_sq > threshold_sq {
                continue;
            }
            let screen_pos = ctx.view.world_to_screen(candidate.position);
            if screen_pos.distance(ctx.cursor_screen) > ctx.threshold_px {
                continue;
            }

            let replace = match best {
                None => true,
                Some((best_idx, cur_dist_sq, _)) => {
                    let dist_diff = dist_sq - cur_dist_sq;
                    if dist_diff.abs() <= 1e-6 {
                        matches!(
                            candidate
                                .kind
                                .priority()
                                .cmp(&scratch[best_idx].kind.priority()),
                            Ordering::Greater
                        )
                    } else {
                        dist_sq < cur_dist_sq
                    }
                }
            };
            if replace {
                best = Some((idx, dist_sq, screen_pos));
            }
        }

        best.map(|(idx, _, screen_pos)| {
            let target = scratch[idx].clone();
            let feedback = SnapFeedback {
                label: target.kind.label(),
                target,
                screen_pos,
                glow_radius_px: 10.0,
                handle_mode: self.handle_mode,
            };
            (feedback.target.clone(), feedback)
        })
    }

    /// Computes translation delta `target.position - source_anchor` for the active [`HandleMode`].
    pub fn compute_delta(handle: HandleMode, source_anchor: Point2, target: &SnapPoint) -> Point2 {
        crate::solver::compute_delta_impl(handle, source_anchor, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_doc::ShapeId;

    struct FakeProvider(Vec<SnapPoint>);

    impl SnapProvider for FakeProvider {
        fn candidates(&self, _ctx: &SnapCtx<'_>) -> Vec<SnapPoint> {
            self.0.clone()
        }
    }

    #[test]
    fn snap_kind_bijection_roundtrip() {
        for kind in SnapKind::ALL {
            let legacy: SnapTargetKind = kind.into();
            let roundtrip = SnapKind::try_from(legacy);
            assert_eq!(roundtrip, Ok(kind));
        }
    }

    #[test]
    fn legacy_only_snap_target_kinds_err_cleanly() {
        assert_eq!(
            SnapKind::try_from(SnapTargetKind::BboxPoint),
            Err(SnapTargetKind::BboxPoint)
        );
        assert_eq!(
            SnapKind::try_from(SnapTargetKind::Edge),
            Err(SnapTargetKind::Edge)
        );
    }

    #[test]
    fn resolve_is_pure_and_ordered() {
        let view = View::new(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)),
            egui::Vec2::ZERO,
            2.0,
        );
        let mut engine = SnapEngine::new(SnapMask::NODE | SnapMask::CENTER | SnapMask::ENDPOINT);
        engine.providers.push(Box::new(FakeProvider(vec![
            SnapPoint::new(Point2::new(50.0, 50.0), SnapKind::Center, ShapeId(1)),
            SnapPoint::new(Point2::new(50.0, 50.0), SnapKind::Endpoint, ShapeId(2)),
            SnapPoint::new(Point2::new(50.0, 50.0), SnapKind::Node, ShapeId(3)),
            SnapPoint::new(Point2::new(150.0, 150.0), SnapKind::Node, ShapeId(4)),
        ])));

        let ctx = SnapCtx {
            cursor_screen: egui::pos2(100.0, 100.0), // exactly world (50, 50)
            view: &view,
            threshold_px: 8.0,
        };
        let first = engine.resolve(&ctx);
        let second = engine.resolve(&ctx);
        assert_eq!(first, second);
        let (pt, fb) = first.expect("expected snap hit within threshold");
        assert_eq!(pt.kind, SnapKind::Node);
        assert_eq!(pt.source, ShapeId(3));
        assert_eq!(fb.label, "Node");

        // Outside threshold must return None (never a "best of the far ones")
        let far_ctx = SnapCtx {
            cursor_screen: egui::pos2(200.0, 200.0),
            view: &view,
            threshold_px: 8.0,
        };
        assert_eq!(engine.resolve(&far_ctx), None);
    }
}
