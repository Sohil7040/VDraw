//! Table-driven controller managing active [`SnapMask`] providers and spatial indexing.

use core_doc::{Document, Shape};
use core_geom::Point2;
use core_snap::contracts::{
    HandleMode, SnapEngine, SnapKind, SnapMask, SnapProvider, DEFAULT_SNAP_MASK, POLAR_STEP_DEG,
};
use core_snap::providers::{
    CenterSnapProvider, GridSnapProvider, IntersectionSnapProvider, MidpointSnapProvider,
    NodeSnapProvider, OrthoConstraintProvider, PerpendicularSnapProvider, PolarConstraintProvider,
    QuadrantSnapProvider, TangentSnapProvider,
};
use core_snap::SpatialIndex;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderKind {
    Node,
    Center,
    Midpoint,
    Quadrant,
    Intersection,
    Grid,
    Ortho,
    Polar,
}

/// Deterministic table mapping toggleable [`SnapMask`] bits to snap provider builders.
const SNAP_PROVIDER_TABLE: &[(SnapMask, ProviderKind)] = &[
    (SnapMask::NODE, ProviderKind::Node),
    (SnapMask::ENDPOINT, ProviderKind::Node),
    (SnapMask::CENTER, ProviderKind::Center),
    (SnapMask::MIDPOINT, ProviderKind::Midpoint),
    (SnapMask::QUADRANT, ProviderKind::Quadrant),
    (SnapMask::INTERSECTION, ProviderKind::Intersection),
    (SnapMask::GRID, ProviderKind::Grid),
    (SnapMask::ORTHO, ProviderKind::Ortho),
    (SnapMask::POLAR, ProviderKind::Polar),
];

/// Coordinates [`SnapMask`] toggles, spatial index updates, and provider rebuilding for [`SnapEngine`].
#[derive(Debug)]
pub struct SnapController {
    /// Currently configured snap mask bits.
    pub mask: SnapMask,
    master_enabled: bool,
    alignment_guides_enabled: bool,
    guideline_snap_enabled: bool,
    engine: SnapEngine,
    index: Arc<SpatialIndex>,
    grid_spacing_mm: f64,
    polar_step_deg: f64,
    reference_origin: Option<Point2>,
}

impl Default for SnapController {
    fn default() -> Self {
        Self::new(DEFAULT_SNAP_MASK)
    }
}

impl SnapController {
    /// Creates a [`SnapController`] initialized with `mask` and builds its initial providers.
    pub fn new(mask: SnapMask) -> Self {
        let mut controller = Self {
            mask,
            master_enabled: true,
            alignment_guides_enabled: true,
            guideline_snap_enabled: true,
            engine: SnapEngine::new(mask),
            index: Arc::new(SpatialIndex::new()),
            grid_spacing_mm: 10.0,
            polar_step_deg: POLAR_STEP_DEG,
            reference_origin: None,
        };
        controller.rebuild_providers();
        controller
    }

    fn sync_engine_modes(&mut self) {
        self.engine.modes = if self.master_enabled {
            self.mask
        } else {
            SnapMask::empty()
        };
    }

    /// Replaces the active [`SnapMask`] and rebuilds the provider list.
    pub fn set_mask(&mut self, mask: SnapMask) {
        self.mask = mask;
        self.sync_engine_modes();
        self.rebuild_providers();
    }

    /// Toggles the specified [`SnapMask`] bit(s) and rebuilds the provider list.
    pub fn toggle_bit(&mut self, bit: SnapMask) {
        self.mask.toggle(bit);
        self.sync_engine_modes();
        self.rebuild_providers();
    }

    /// Toggles the [`SnapMask`] bit corresponding to `kind` and rebuilds the provider list.
    pub fn toggle_kind(&mut self, kind: SnapKind) {
        self.mask.toggle_kind(kind);
        self.sync_engine_modes();
        self.rebuild_providers();
    }

    /// Toggles the master snap enable switch while preserving the configured [`SnapMask`].
    pub fn toggle_master(&mut self) -> bool {
        self.master_enabled = !self.master_enabled;
        self.sync_engine_modes();
        self.master_enabled
    }

    /// Explicitly sets whether master snapping is enabled.
    pub fn set_master_enabled(&mut self, enabled: bool) {
        self.master_enabled = enabled;
        self.sync_engine_modes();
    }

    /// Returns `true` if master snapping is currently enabled.
    pub fn is_master_enabled(&self) -> bool {
        self.master_enabled
    }

    /// Toggles dynamic alignment guide snapping and returns the updated flag.
    pub fn toggle_alignment_guides(&mut self) -> bool {
        self.alignment_guides_enabled = !self.alignment_guides_enabled;
        self.alignment_guides_enabled
    }

    /// Returns `true` if alignment guides are enabled.
    pub fn alignment_guides_enabled(&self) -> bool {
        self.alignment_guides_enabled
    }

    /// Toggles document guideline snapping and returns the updated flag.
    pub fn toggle_guideline_snap(&mut self) -> bool {
        self.guideline_snap_enabled = !self.guideline_snap_enabled;
        self.guideline_snap_enabled
    }

    /// Returns `true` if guideline snapping is enabled.
    pub fn guideline_snap_enabled(&self) -> bool {
        self.guideline_snap_enabled
    }

    /// Updates the polar tracking angle increment in degrees and rebuilds providers.
    pub fn set_polar_step_deg(&mut self, step_deg: f64) {
        if step_deg.is_finite() && step_deg > 0.0 {
            self.polar_step_deg = step_deg;
            self.rebuild_providers();
        }
    }

    /// Returns the configured polar angle step in degrees.
    pub fn polar_step_deg(&self) -> f64 {
        self.polar_step_deg
    }

    /// Updates the grid spacing in millimetres and rebuilds providers.
    pub fn set_grid_spacing_mm(&mut self, spacing_mm: f64) {
        if spacing_mm.is_finite() && spacing_mm > 0.0 {
            self.grid_spacing_mm = spacing_mm;
            self.rebuild_providers();
        }
    }

    /// Rebuilds the [`SnapEngine`] provider list from [`SNAP_PROVIDER_TABLE`], deduplicating
    /// [`NodeSnapProvider`] across `NODE` and `ENDPOINT`, then appending contextual
    /// [`PerpendicularSnapProvider`] and [`TangentSnapProvider`] hints.
    pub fn rebuild_providers(&mut self) {
        self.engine.providers.clear();
        let mut node_added = false;

        for &(bit, kind) in SNAP_PROVIDER_TABLE {
            if !self.mask.contains(bit) {
                continue;
            }
            match kind {
                ProviderKind::Node => {
                    if !node_added {
                        self.engine
                            .providers
                            .push(Box::new(NodeSnapProvider::new(Arc::clone(&self.index))));
                        node_added = true;
                    }
                }
                ProviderKind::Center => {
                    self.engine
                        .providers
                        .push(Box::new(CenterSnapProvider::new(Arc::clone(&self.index))));
                }
                ProviderKind::Midpoint => {
                    self.engine
                        .providers
                        .push(Box::new(MidpointSnapProvider::new(Arc::clone(&self.index))));
                }
                ProviderKind::Quadrant => {
                    self.engine
                        .providers
                        .push(Box::new(QuadrantSnapProvider::new(Arc::clone(&self.index))));
                }
                ProviderKind::Intersection => {
                    self.engine
                        .providers
                        .push(Box::new(IntersectionSnapProvider::new(Arc::clone(
                            &self.index,
                        ))));
                }
                ProviderKind::Grid => {
                    self.engine
                        .providers
                        .push(Box::new(GridSnapProvider::with_spacing(
                            self.grid_spacing_mm,
                            1,
                        )));
                }
                ProviderKind::Ortho => {
                    self.engine.providers.push(Box::new(
                        OrthoConstraintProvider::with_optional_origin(self.reference_origin)
                            .with_enabled(true),
                    ));
                }
                ProviderKind::Polar => {
                    self.engine.providers.push(Box::new(
                        PolarConstraintProvider::with_optional_origin(self.reference_origin)
                            .with_step_deg(self.polar_step_deg)
                            .with_enabled(true),
                    ));
                }
            }
        }

        // Perpendicular and tangent providers are always-on contextual hints, evaluated when
        // a reference origin is active and permitted by the active SnapMask.
        self.engine
            .providers
            .push(Box::new(PerpendicularSnapProvider::new(
                Arc::clone(&self.index),
                self.reference_origin,
            )));
        self.engine
            .providers
            .push(Box::new(TangentSnapProvider::new(
                Arc::clone(&self.index),
                self.reference_origin,
            )));
    }

    /// Rebuilds the spatial R*-tree index from visible shapes in the active page of `doc`.
    pub fn update_index(&mut self, doc: &Document) {
        let shapes: Vec<Shape> = doc
            .active_page()
            .map(|page| {
                page.layers
                    .iter()
                    .filter(|layer| layer.visible)
                    .flat_map(|layer| layer.shapes.iter().filter(|s| s.visible).cloned())
                    .collect()
            })
            .unwrap_or_default();

        let rebuilt = SpatialIndex::bulk_load(&shapes);
        self.engine.spatial_index = rebuilt.clone();
        self.index = Arc::new(rebuilt);
        self.rebuild_providers();
    }

    /// Sets the optional reference origin used by ortho, polar, perpendicular, and tangent providers.
    pub fn set_reference_origin(&mut self, origin: Option<Point2>) {
        self.reference_origin = origin;
        self.rebuild_providers();
    }

    /// Returns a shared reference to the managed [`SnapEngine`].
    pub fn get_engine(&self) -> &SnapEngine {
        &self.engine
    }

    /// Sets the source-side [`HandleMode`] on the managed [`SnapEngine`].
    pub fn set_handle_mode(&mut self, mode: HandleMode) {
        self.engine.handle_mode = mode;
    }

    /// Returns the active [`HandleMode`].
    pub fn handle_mode(&self) -> HandleMode {
        self.engine.handle_mode
    }

    /// Returns the number of currently registered [`SnapProvider`] instances.
    pub fn provider_count(&self) -> usize {
        self.engine.providers.len()
    }

    /// Returns the currently registered [`SnapProvider`] slice for inspection.
    pub fn active_providers(&self) -> &[Box<dyn SnapProvider>] {
        &self.engine.providers
    }
}
