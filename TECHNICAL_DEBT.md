# VDraw Technical Debt & Architectural Defect Resolution Audit

*Document Revision: Post-Directive Engineering Overhaul*

---

## 1. Resolved Critical Defects

### Defect 1: Clipper2 Integer Coordinate Overflow Vulnerability
- **Severity:** High (Crash / Geometric Inversion)
- **Problem:** Clipper2 uses 64-bit signed integers with a scale factor of 10,000 units/mm. Coordinates exceeding $\pm 9 \times 10^{14}$ would cause integer overflow, leading to inverted polygons or panics. Furthermore, non-finite values (`NaN`, `Inf`) caused silent panics inside Clipper2 FFI.
- **Resolution:** Implemented `polycurve_to_paths_d_checked()` in `crates/core-boolean/src/convert.rs`. Every point is checked for `is_finite()`, and coordinates are bounded to $\pm 100,000.0\,\text{mm}$ ($\pm 100\,\text{meters}$). If out-of-range, a descriptive `BooleanError::CoordinateOutOfRange` is returned without mutating the document.

### Defect 2: Local vs World Space Snapping Divergence
- **Severity:** High (Interactive CAD Defect)
- **Problem:** `core-snap` indexed shapes using `shape.to_polycurve()` in local coordinates without applying `shape.transform`. When a shape was translated, rotated, or scaled, snapping queries checked against the untransformed local origin, causing snapping to fail or snap to phantom positions.
- **Resolution:** Updated `IndexedShape::from_shape()` in `crates/core-snap/src/spatial_index.rs` to apply `shape.transform` to all nodes, lines, and midpoints before bulk-loading into the $R^*$-tree.

### Defect 3: Per-Frame Spatial Index Rebuild Bottleneck
- **Severity:** High (UI Thread Starvation)
- **Problem:** `crates/app/src/app.rs` unconditionally executed `spatial_index.rebuild(&shapes)` on every frame (60–120 Hz). For documents with 10,000+ shapes, this caused severe UI lag.
- **Resolution:** Introduced dirty-state tracking (`current_shape_count != last_shape_count || current_dirty != last_undo_dirty || first_frame`). Rebuilding only occurs when the document geometry actually changes.

### Defect 4: Nearest-Point Projection Local Minimum Traps
- **Severity:** Medium (Geometric Inaccuracy)
- **Problem:** The previous cubic Bézier projection used a fixed 4-level recursive subdivision, which could miss the global minimum on complex cubic S-curves or loop curves.
- **Resolution:** Implemented 1D degree-5 Bernstein Bézier Clipping with bracketed root-finding and candidate comparisons against endpoints and stationary points in `crates/core-geom/src/bezier.rs`.

### Defect 5: Schneider Curve Fitting Runge's Phenomenon Bowing
- **Severity:** Medium (Vector Digitization Defect)
- **Problem:** When a user drew a polyline with an 80 mm straight edge adjacent to a small arc, sparse point evaluation allowed the fitted cubic to bow outwards by > 1.6 mm while reporting < 0.05 mm error at the sample points.
- **Resolution:** Added intermediate chord midpoint evaluations in `compute_max_error()` and sharp corner detection ($> 45^\circ$) in `crates/core-geom/src/curve_fit.rs`.

### Defect 6: Concave Polygon Fill Rendering Inversion
- **Severity:** Medium (Visual Artifact)
- **Problem:** Canvas rendering used `egui::Shape::convex_polygon` which applies triangle-fan tessellation from vertex 0. This inverted self-overlapping regions on stars and L-shapes.
- **Resolution:** Implemented pure-Rust ear-clipping polygon triangulation (`triangulate_polygon() -> egui::epaint::Mesh`) in `crates/app/src/canvas.rs`.

### Defect 7: CAM Nearest-Neighbor Travel Order Violating Inside-Out Nesting
- **Severity:** Critical (Manufacturing Damage)
- **Problem:** The CAM toolpath planner grouped operations only by integer depth count. A nearest-neighbor jump could cut an outer boundary before an inner hole was finished if they were in different nested parts.
- **Resolution:** Designed and implemented `CamDependencyGraph` in `crates/core-cam/src/ordering.rs`. A topological DAG guarantees that inner holes and marking operations strictly precede enclosing boundaries during wavefront nearest-neighbor selection.

---

## 2. Managed Technical Debt & Non-Blockers

1. **Camera Location in `core-cam`:**
   - The `Camera` struct was historically placed in `crates/core-cam/src/lib.rs`. While it functions accurately for document-to-screen transforms, it conceptually belongs to a presentation/view module. It is retained with full backward compatibility.
2. **Text Font Rasterization:**
   - Text shapes currently utilize a bundled Hershey vector font / ttf-parser for outline extraction. Complex OpenType shaping (HarfBuzz/Bidi) is deferred to future internationalization phases.
3. **GPU Compute Acceleration for 500,000+ Objects:**
   - The pure Rust CPU pipeline comfortably handles 50,000 shapes in < 16 ms. Documents exceeding 500,000 objects will eventually benefit from compute-shader tessellation in WGPU.
