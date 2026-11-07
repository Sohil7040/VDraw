# Phase 3 Changelog — R*-Tree Spatial Index, Snapping Engine, Dynamic & Alignment Guides, Ruler Guidelines

**Release / Milestone:** Phase 3  
**Status:** Completed  
**Date:** 2026-11-07  

---

## 1. Summary of Changes

Phase 3 implements an industrial-grade geometric snapping engine and dynamic guide system powered by an $R^*$-tree spatial index (`rstar`). It delivers sub-millisecond query performance over tens of thousands of vector primitives, an interactive ruler guideline subsystem, dynamic polar angle raycasting with distance ticks, smart edge-to-edge/center-to-center alignment guides with equal spacing detection, priority-weighted target resolution, and floating HUD badges.

---

## 2. New Modules & Capabilities

### A. $R^*$-Tree Spatial Index (`crates/core-snap/src/spatial_index.rs`)
- **IndexedShape Geometry Envelope:**
  - Implements `rstar::RTreeObject` and `rstar::PointDistance` for vector shapes.
  - Generates tight AABB envelopes for sub-millisecond spatial queries.
  - Supports both bulk-loading via Sort-Tile-Recursive (STR) algorithm and incremental insert/remove operations.
  - Uses `locate_in_envelope_intersecting` to accurately match large shapes that intersect cursor gravity radii without being strictly contained inside them.
- **50,000 Shape Snapping Benchmark:**
  - SPEC verification test `test_benchmark_50k_shapes_latency` loads 50,000 parametric vector shapes into the spatial index.
  - Bulk loading completes in **~390 ms**.
  - Query latency achieves **0.0055 ms** (5.5 µs) per point search, beating the $< 2.0\text{ ms}$ requirement by **~360×**.

### B. Geometric Snapping Solver (`crates/core-snap/src/solver.rs`)
- **Weighted Distance and Priority Resolution:**
  - Gravity radius in screen pixels (default 8 px, converted to document mm via zoom level).
  - Evaluates geometric candidates and selects winners by lowest composite score: $S = \text{dist} \times \text{priority\_weight}$.
  - Supports 11 snap target kinds with calibrated priority weights:
    - `Node` (Priority 100, Diamond marker)
    - `Intersection` (Priority 95, Diagonal X marker)
    - `Midpoint` (Priority 90, Triangle marker)
    - `Center` / Centroid (Priority 85, Circle with crosshair)
    - `Quadrant` (Priority 80, Quarter-circle arc marker)
    - `Perpendicular` (Priority 75, Right-angle marker)
    - `Tangent` (Priority 70, Circle with tangent line marker)
    - `Edge` / Curve nearest point (Priority 60, Hourglass marker)
    - `BboxPoint` (9-point cardinal corners/centers) (Priority 50, Square marker)
    - `Guideline` (Priority 40, Guideline dot marker)
    - `Grid` (Priority 20, Grid dot marker)
- **Alt Key Modifier:**
  - Holding `Alt` temporarily suppresses all snapping for fine manual positioning.

### C. Dynamic Polar Angle Guides (`crates/core-snap/src/dynamic_guide.rs`)
- **Raycasting at 15° Increments:**
  - Angular snaps at 0°, 15°, 30°, 45°, 60°, 75°, 90°, 105°, 120°, 135°, 150°, 165°, 180° relative to drag origin.
  - Distance tick snapping along active ray (e.g. 10 mm intervals).
  - Violet dashed ray rendering across canvas with distance tick markers.
  - Global toggle via status bar pill or `Alt+Shift+D` shortcut.

### D. Smart Alignment Guides (`crates/core-snap/src/alignment.rs`)
- **Edge-to-Edge & Center-to-Center Detection:**
  - Left-to-Left, Left-to-Right, Right-to-Left, Right-to-Right edge alignments.
  - Top-to-Top, Top-to-Bottom, Bottom-to-Top, Bottom-to-Bottom edge alignments.
  - Center-X and Center-Y alignments across multiple objects.
  - Equal spacing detection between consecutive collinear shapes.
  - Magenta dashed alignment guidelines drawn across canvas with tick markers.

### E. Interactive Ruler Guidelines (`crates/core-doc/src/guideline.rs`)
- **Interactive Drag-to-Spawn:**
  - Clicking and dragging from top ruler spawns a horizontal guideline.
  - Clicking and dragging from left ruler spawns a vertical guideline.
  - Visual coordinate badge pill displayed on the ruler header.
  - Persistent document storage with undo/redo support.

### F. Canvas Overlays & Status Bar (`crates/app`)
- **Visual Feedback:**
  - Specialized glyph per snap target kind drawn in teal (`#20D3B5`).
  - Floating dark HUD pill badge adjacent to cursor showing target kind and coordinates: `Node (X, Y) mm`.
  - Status bar toggles for master Snap (`⊕ Snap`), Grid (`⊞ Grid`), Guidelines (`⊢ Guides`), and Dynamic Guides (`∠ Dynamic (15°)`).

---

## 3. Verification & Metrics

- **Unit & Property Tests:** 33 tests passing across all crates.
- **Benchmark:**
  - 50,000 shapes bulk loaded in 390 ms.
  - Spatial index query latency: **0.0055 ms** (target $< 2.0\text{ ms}$).
- **Compiler & Linter:** Clean compilation with 0 warnings on `cargo clippy --all-targets --all-features -- -D warnings`.
- **Formatting:** `cargo fmt --all -- --check` passes cleanly.
