# VDraw Engineering Changelog

All notable changes, architectural improvements, and bug fixes to VDraw are documented in this file.

---

## [Unreleased]
### Technical Debt & Post-Ship Cleanup (Wave 3c)
- Remaining direct `undo.apply(...)` and `&mut doc` panel bypass sites scheduled for migration to formal `Dispatcher::dispatch(AppCommand::...)` pipelines:
  - `crates/app/src/panels/properties_panel.rs` (dimension, position, and property mutations)
  - `crates/app/src/panels/fill_stroke.rs` (color, opacity, stroke width, and dash adjustments)
  - `crates/app/src/panels/welcome.rs` (welcome preset initializations)
  - `crates/app/src/canvas.rs` (direct canvas event bypasses)

---

## [0.3.0-dev] - Multi-Agent CAD & UI Upgrade
- **Frozen 2D Contract Surface (v2):** Transform build/apply split, pure snap resolve, bitflags-only `SnapMask`, exhaustive `AppCommand`, `ThemeTokens` bundle, and frozen boolean signature.
- **Wave 1b Integration & Invariant Hardening:**
  - Guaranteed apply-path inverse invariant for all transform commands.
  - Pinned boolean degenerate-input policy (graceful fallback on empty/overlapping/coincident paths).
  - Snap feedback label bijection test ensuring 1:1 bidirectional mapping between snap targets and HUD labels.
- **Wave 2 Interaction & Dispatch Engine:**
  - Unified interaction command loop routing through `Dispatcher::dispatch` with strict `DispatchCtx`.
  - Deterministic pointer drag session lifecycle (`DragSession`) managing live interactive transforms and commit boundaries.
  - Transactional undo/redo integration with dirty state notifications.
- **Wave 3 Acceptance, Quality & Performance Verification:**
  - Headless full-app end-to-end integration test (`[F-E1]`) verifying pointer events, snapping, drag preview, undo, and redo.
  - Comprehensive boolean CSG fuzz testing (`fuzz_boolean.rs`) covering degenerate, coincident, and high-complexity polygons.
  - Sub-microsecond snap query benchmarks and transform throughput profiling.
  - Golden snapshot regression testing with `insta` and `tiny-skia`.
  - **Performance Note (`FLAG-1`):** Quantified one-time ~28.5 ms cold-start cost for `SpatialIndex::bulk_load` across 10,000 shapes; live spatial index queries remain sub-microsecond (<5 µs) in steady state.
- **Wave 3b Hotfix & Invariant Routing:**
  - Resolved `WAVE3B-HOTFIX-1`: Routed all 8 Align and Distribute actions in `align.rs` through `Dispatcher::dispatch`.
  - Introduced `DirtySignal` tracking `shape_count`, `undo_depth`, `total_history`, `top_command`, layer visibilities, and guidelines count in `VDrawApp::sync_spatial_index_if_dirty`, guaranteeing fresh spatial index rebuilding even when `undo_stack.is_dirty()` was already true.
  - Added regression test `test_align_then_snap_uses_fresh_index` confirming post-alignment snap accuracy.
  - Marked `VDrawApp::new_headless` with `#[doc(hidden)]`.
  - Isolated screenshot test output in `crates/app/src/screenshot.rs` to process-unique temporary paths, ensuring clean test runs without repository file churn.

---

## [0.2.0] - Engineering Improvement Directive Overhaul

### 1. Geometry Engine (`core-geom`)
- **Matrix Robustness:** Added `is_finite()` and `is_identity()` to `Affine2`. Fixed `Affine2::inverse()` IEEE 754 NaN handling and near-singular matrix protection ($|\det| < 10^{-15}$).
- **Global Bézier Root Finder:** Replaced 4-level recursive subdivision in `CubicBezier::closest_point()` with 1D degree-5 Bernstein Bézier Clipping and bracketed root-finding.
- **Adaptive Flattening Safeguards:** Added `MAX_DEPTH = 16` recursion limits and collinear overshoot detection to eliminate infinite loops and false-flatness bugs on degenerate curves.
- **Node Continuity:** Formalized $G^0$ (Cusp), $G^1$ (Smooth), and $C^1$ (Symmetrical) node mechanics. Fixed `BezierNode::with_handles` constructor so it preserves explicit handle orientations.
- **Fillet Geometries:** Implemented `CornerStyle::Chamfer` and `CornerStyle::Scallop` corner styles alongside `Round`. Added full `transform(&self, &Affine2)` support to `Polycurve` and `Subpath`.
- **Schneider Curve Fitting:** Added sharp corner detection ($> 45^\circ$) to segment polylines into Cusp-joined smooth subpaths. Added intermediate chord evaluations to prevent sparse-sample bowing.

### 2. Numerical Robustness & Boolean CSG (`core-boolean`)
- **Dependency Cleanup:** Completely removed unused `cavalier_contours` dependency.
- **Scaled Coordinate Bounds:** Enforced explicit coordinate range validation ($[-100,000.0, 100,000.0]\,\text{mm}$) and non-finite value rejection in `polycurve_to_paths_d_checked`.
- **CSG Semantics:** Disambiguated `Trim`, `FrontMinusBack`, and `BackMinusFront` operations.
- **Knife Tool Nested Holes:** Added ray-casting point-in-polygon assignment for open and closed slice paths.
- **Virtual Segment Trimming:** Replaced vertex-distance truncation with segment-level parametric intersection sorting ($u \in [0, 1]$).
- **Contour Offsetting:** Partitioned compound paths into closed polygons and open toolpaths for offset compensation.

### 3. Document Architecture (`core-doc`)
- **Validation Subsystem:** Created `validation.rs` with `validate_document()` to check duplicate IDs, non-finite values, and invalid dimensions.
- **Page Presets & Metadata:** Added `PagePreset` (A4, A3, A2, Letter, Tabloid, LaserBedSmall/Medium/Large, Custom) and `DocumentMetadata`.
- **Groups & Compound Shapes:** Implemented `ShapeKind::Group` with recursive bounds, hit-testing, and transformation.
- **Bounded Undo Ring Buffer:** Converted `UndoStack` to a `VecDeque` ring buffer for $O(1)$ pruning with document generation dirty tracking.
- **State Decoupling:** Marked `Document.selection` as `#[serde(skip, default)]` to prevent transient UI state from polluting saved documents.

### 4. Snapping & Spatial Indexing (`core-snap`)
- **World-Space Coordinate Fix:** Updated `IndexedShape::from_shape` to transform geometric features into world coordinates prior to indexing.
- **Snap Targets:** Verified all 11 snapping modes (Node, Midpoint, Center, Quadrant, Grid, Guideline, Intersection, Perpendicular, Tangent, Smart Alignment, Polar Guides).
- **Sub-Millisecond Query Latency:** Bulk-loading 50,000 shapes runs in 458 ms; spatial snap queries execute in 4.8 microseconds.

### 5. Manufacturing CAM Subsystem (`core-cam`)
- **Explicit CAM Operations:** Decoupled design styling from manufacturing operations via `LaserMode` (Vector Cut, Vector Score, Vector Engrave, Raster Engrave, Marking, Disabled).
- **Topological Containment DAG:** Replaced flat depth sorting with `CamDependencyGraph`. Guaranteed that inner holes and marking operations strictly precede enclosing cuts.
- **Kerf Compensation:** Integrated parallel contour offsets with feature collapse detection (`apply_kerf_compensation`).
- **Dedicated Raster Pipeline:** Implemented `RasterImage` with grayscale conversion, brightness/contrast/gamma, Bayer 4x4/8x8, Floyd-Steinberg, and Atkinson dithering, scanlines, and overscan ramps.
- **Kinematic Job Estimator:** Implemented trapezoidal motion profiling ($d_{\text{ramp}} = V^2 / A$) and pierce dwell delays.
- **Modular Postprocessors:** Implemented independent postprocessors for Generic G-code, GRBL 1.1f/h ($32=1, M4), Marlin 2.0, and HPGL/PLT.
- **Safety Subsystem:** Implemented `SimulatedController` verifying bed soft limits and ensuring the laser is 100% OFF during all rapid travel moves.

### 6. File I/O Subsystem (`io`)
- **Native `.vdraw` Format:** Implemented pretty-printed JSON serialization with schema validation on load.
- **Universal SVG Exporter:** Pure Rust SVG exporter with Inkscape layer tags, CSS styles, and millimeter units.
- **Pure-Rust SVG Importer:** Custom XML and SVG path tokenizer parsing paths, rects, circles, lines, polylines, and hex/rgb colors.
- **AutoCAD DXF Exporter:** Compatible with R12/R14 LWPOLYLINE entities for CNC routers and laser cutters.

### 7. User Interface & Rendering (`app`)
- **Live Object Manager:** Connected Object Manager to live `doc.active_page().layers` and shapes, with visibility and lock toggles.
- **Live Properties Inspector:** Displays and edits page dimensions, presets, shape coordinates, dimensions, stroke, and fill with transactional undo.
- **Precision Alignment:** Implemented interactive Align Left, Right, Top, Bottom, Center H/V, and Distribute Equal Spacing.
- **Concave Polygon Rendering:** Implemented pure-Rust ear-clipping triangulation for stars and concave polygons.
- **Viewport Culling:** Canvas skips geometry outside the visible screen rectangle.
- **Performance Leak Fix:** Eliminated per-frame spatial index rebuilding via dirty-state tracking.
- **File Menu Integration:** Connected New, Open, Save, Save As, Export SVG, Export DXF, and Export G-Code menu actions.
