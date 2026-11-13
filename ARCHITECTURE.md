# VDraw Architecture Documentation

**Authoritative System Specification**  
*Version: 0.2.0 (Post-Directive Architecture Overhaul)*  
*Language: 100% Pure Rust (Edition 2021)*

---

## 1. System Overview

VDraw is a professional-grade, 100% pure Rust desktop 2D vector CAD, technical illustration, and industrial laser CAM application. It is engineered with zero C/C++ FFI dependencies, zero web/Electron runtime layers, and zero compromise on geometric and manufacturing correctness.

The system is organized as an eight-crate Cargo workspace with strict layer isolation:

```
┌────────────────────────────────────────────────────────┐
│                        app                             │
│       Desktop UI (eframe/egui), Tool Dispatch, Panels  │
└──────┬────────────┬─────────────┬────────────┬─────────┘
       │            │             │            │
┌──────▼─────┐┌─────▼──────┐┌─────▼──────┐┌────▼─────────┐
│ core-tools ││  core-cam  ││  core-snap ││      io      │
│ Interactive││ Laser CAM, ││ R*-tree,   ││ .vdraw, SVG, │
│ CAD Tools  ││ G-code Gen ││ Snapping   ││ DXF Exporter │
└──────┬─────┘└─────┬──────┘└─────┬──────┘└────┬─────────┘
       │            │             │            │
┌──────▼────────────▼─────────────▼────────────▼─────────┐
│                        core-doc                        │
│   Document Model, Layers, Commands, Undo/Redo (Ring)   │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                      core-boolean                      │
│     Clipper2 Polygon Ops, Offsets, Knife & Virtual     │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                       core-geom                        │
│   Point, Rect, Affine2, Cubic Bézier, Polycurve, Node  │
└────────────────────────────────────────────────────────┘
```

---

## 2. Coordinate Spaces & DPI Conventions

To prevent DPI scaling inaccuracies and physical coordinate drift, VDraw maintains three strictly distinguished coordinate spaces:

1. **Document World Space ($\text{mm}$):**
   - Authoritative floating-point millimeter coordinates ($x, y \in [-100,000.0, 100,000.0]\,\text{mm}$).
   - All geometric entities (`Point2`, `Rect`, `Polycurve`, `Subpath`, `CamContour`) live in document millimeters.
   - Origin $(0, 0)$ is at the top-left of the page bed by default, with positive $X$ right and positive $Y$ down.

2. **Logical Screen Space ($\text{points}$):**
   - Abstract display coordinates managed by `egui`.
   - Transformation:
     $$\mathbf{P}_{\text{screen}} = (\mathbf{P}_{\text{doc}} \cdot \text{zoom} \cdot \text{pixels\_per\_mm}) + \mathbf{T}_{\text{pan}}$$

3. **Physical Framebuffer Space ($\text{pixels}$):**
   - Actual GPU backbuffer pixels on retina/HiDPI screens.
   - Physical pixels = Logical points $\times$ `egui_ctx.pixels_per_point()`.
   - WGPU rendering and tiny-skia raster previews account explicitly for this DPI factor.

---

## 3. Crate Breakdown & Responsibilities

### `core-geom`
The foundational computational geometry crate:
- **`Affine2`**: $2 \times 3$ affine transformation matrix ($[a, b, c, d, t_x, t_y]$). Certified multiplication order, determinant, IEEE 754 NaN handling, and inversion safeguards ($|\det| \ge 10^{-15}$).
- **`Point2` & `Rect`**: Fundamental primitives with Euclidean distance, centroid, union, intersection, and finiteness validation.
- **`CubicBezier`**: Degree-3 parametric curve with de Casteljau split, exact derivative velocity vectors, curvature calculation, Legendre-Gauss arc-length integration, and global 1D Bézier Clipping root-finding for nearest-point projection.
- **`BezierNode`**: Explicit node continuity management ($G^0$ Cusp, $G^1$ Smooth, $C^1$ Symmetrical).
- **`Polycurve` & `Subpath`**: Compound open/closed curves with chamfer, scallop, and round corner fillets.
- **`curve_fit`**: Schneider cubic curve fitting with sharp corner ($> 45^\circ$) detection and chord intermediate error safeguards.

### `core-boolean`
Numerical robustness and 2D constructive solid geometry:
- Integer scaling fixed at $10,000\,\text{units/mm}$ with coordinate bounds clamped to $\pm 100,000\,\text{mm}$ ($\pm 1,000,000,000$ integer units, well within 64-bit integer limits).
- Disambiguated semantics for `Trim`, `FrontMinusBack`, `BackMinusFront`, `Weld`, `Intersect`, `Exclude`.
- Ray-casting nested hole containment for `KnifeTool` slicing.
- Parametric intersection sorting ($u \in [0, 1]$) for `VirtualSegmentDelete`.
- Compound path offset engine (`offset_shape`) for kerf compensation and contour offset.

### `core-doc`
Document model and transactional state management:
- **`Document`**: Versioned document structure with metadata, page presets (A4, A3, Letter, LaserBedSmall/Medium/Large), layers, and shapes.
- **`Shape` & `ShapeKind`**: Strongly-typed vector primitives (Rect, Ellipse, Line, Path, Group, Polygon, Star, Spiral, Text).
- **`UndoStack`**: $O(1)$ ring buffer using `VecDeque` with transactional rollback on failure and document dirty-state tracking (`is_dirty()`, `mark_saved()`).
- **`validation`**: Complete document integrity validator checking for duplicate IDs, non-finite values, out-of-range coordinates, invalid page dimensions, and empty groups.

### `core-snap`
Real-time spatial snapping and guide engine:
- $R^*$-tree spatial index (`rstar::RTree`) indexing transformed world-space features.
- Dynamic caching with dirty-state rebuild gating (rebuilds only when shape count or generation changes).
- 11 snap targets: Node, Midpoint, Center, Quadrant, Grid, Guideline, Intersection, Perpendicular, Tangent, Smart Alignment, Polar Guides.

### `core-tools`
Interactive CAD tool state machines:
- Pick tool (marquee selection, transform gizmo, scaling with negative flipping, rotation around custom pivot).
- Node/Shape tool, Pen tool, Pencil freehand tool with live Schneider curve fitting.
- Knife, Eraser, Virtual Segment Delete, Rectangle, Ellipse, Line, Polygon, Star, Text.

### `core-cam`
Industrial laser manufacturing architecture:
- Dedicated operation model (`LaserMode`: Vector Cut, Vector Score, Vector Engrave, Raster Engrave, Marking, Disabled).
- Topological containment Directed Acyclic Graph (DAG) for guaranteed Inside-Out cutting order.
- Kerf compensation via `apply_kerf_compensation` with feature collapse detection.
- Complete raster engraving pipeline with brightness/contrast/gamma, Bayer and Floyd-Steinberg dithering, overscan ramps, and DPI scanlines.
- Kinematic job estimation with trapezoidal acceleration and pierce dwell time.
- Dialect postprocessors for Generic G-Code, GRBL 1.1f/h ($32=1, M4), Marlin 2.0, and HPGL/PLT.
- Safety validator and simulated machine controller checking soft limits.

### `io`
Cross-platform CAD file interchange:
- Native `.vdraw` JSON format with automated validation on load.
- Pure Rust SVG exporter with CSS styling, inkscape layer grouping, and absolute path data.
- Pure Rust SVG parser/importer supporting paths, rects, circles, lines, polylines, and styling.
- AutoCAD DXF (R12/R14 LWPOLYLINE) exporter for CNC routers, laser cutters, and CAD tools.

### `app`
Desktop user interface built with `eframe` / `egui`:
- Dark/light studio themes with Corelith/LightBurn ergonomic layout.
- Viewport culling and ear-clipping triangulation mesh rendering for concave/star polygons.
- Live Object Manager panel with layer/shape hierarchy, visibility/lock toggles.
- Live Properties inspector for page setup, shape dimensions, position, stroke, and fill.
- Precision Align & Distribute panel.
- Live Laser CAM Setup panel with toolpath simulation preview, time estimation, and file export.
