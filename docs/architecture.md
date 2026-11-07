# VDraw Architecture

## Overview

VDraw is a professional 2D vector design application built in pure Rust, targeting designers, makers, and engineers.

## Workspace Layout

```
VDraw/
  crates/
    core-geom/      Geometry primitives, Bézier math, subdivision, closest-point, arc length
    core-doc/       Document model (Document, Page, Layer, Shape, Style, Undo/Redo)
    core-snap/      R*-tree index, SnapSolver, snap targets, dynamic guides, alignment guides
    core-boolean/   Boolean ops (Weld, Trim, Intersect, Front-Minus-Back, Knife, curve re-fitting)
    core-tools/     Tool state machine (Tool trait, Pick, Freehand, Pen, Rect, Ellipse, etc.)
    core-cam/       Laser role classifier, containment tree, inside-out ordering, kerf offset, export (HPGL, G-code)
    io/             File I/O (SVG, DXF, PDF export, PNG export, native .vdraw project format)
    app/            egui/eframe desktop application binary, theme, canvas, dock layout
```

## Rendering Pipeline

```
Document model
    │
    ▼
CanvasState (pan, zoom)
    │
    ▼
egui Painter (Phase 0)
    │  (Phase 1+)
    ▼
vello Scene → wgpu surface
```

## Coordinate Systems

| Space | Unit | Description |
|-------|------|-------------|
| Document | mm | Authoritative; stored in `Document` |
| Canvas | px | Screen pixels at current DPI |
| Screen | px | egui `Pos2` / `Rect` |

Conversion: `px = mm × zoom × PX_PER_MM`  
where `PX_PER_MM = 96.0 / 25.4 ≈ 3.7795`

## Core Subsystems (Phase 1)

### 1. Geometry Engine (`core-geom`)
- **Primitives:** `Point2`, `Rect`, `Affine2` with full 2D affine algebra (translation, non-uniform scaling, rotation around an arbitrary origin/pivot, and shear).
- **Node Constraints:** `NodeType` (`Cusp`, `Smooth`, `Symmetrical`) and `BezierNode`. Handle edits automatically enforce collinearity and symmetry rules.
- **Cubic Bézier Pipeline:**
  - de Casteljau split at arbitrary parameter $t$ and midpoint $t = 0.5$.
  - Evaluation $B(t)$, velocity $B'(t)$, acceleration $B''(t)$, unit normal and tangent vectors.
  - Arc length calculation using 5-point Gauss-Legendre numerical quadrature.
  - Nearest point solver: Bounding box cull $\to$ 4-level bisection $\to$ Newton-Raphson minimization $\to$ clamp $[0, 1]$.
  - Adaptive flattening within configurable tolerance ($\le 0.02\text{ mm}$ default).
  - Exact tight bounding boxes from derivative roots.
- **Polycurve Normalization:** Uniform representation of complex paths and primitive shapes (Lines, Rectangles with corner fillets, Ellipses/Circles with $\kappa \approx 0.5522847$) as cubic Bézier subpaths.

### 2. Document Model (`core-doc`)
- **Hierarchy:** `Document` $\to$ `Page`s $\to$ `Layer`s $\to$ `Shape`s.
- **Shapes & Styling:** `ShapeKind`, `Style` (Fill, Stroke, Hairline classification for laser cutting), and real-time bounding box calculation.
- **Selection & Handles:** `Selection` with `TransformMode` (`Scale` 8 handles vs `RotateSkew` 8 handles + pivot point) and hit testing.
- **Undo/Redo:** Command pattern with `UndoStack`, `AddShapeCommand`, `RemoveShapesCommand`, `TransformShapesCommand`, and `CompoundCommand`.

### 3. Tool State Machine (`core-tools`)
- **`Tool` Trait:** Pointer down/move/up, key events, overlays, and contextual cursors.
- **Tools:**
  - `PickTool`: Selection, rubber band marquee, 8-handle scaling with opposite anchor pinning, rotation around pivot with 15° snapping, and mode toggling.
  - `ShapeTool`: Node tool (`F10`) for direct anchor point dragging, handle adjustment with live constraint propagation (`Cusp`, `Smooth`, `Symmetrical`), double-click segment splitting, and node deletion.
  - `FreehandTool`: Interactive stylus/mouse drawing with Schneider cubic Bézier curve fitting (`F5`).
  - `RectangleTool`: Interactive drag, Shift-centered, Ctrl-square constraints (`F6`).
  - `EllipseTool`: Interactive drag, Shift-centered, Ctrl-circle constraints (`F7`).
  - `PolygonTool`: Regular $N$-sided polygon creation ($3..50$ sides, `Y`).
  - `StarTool`: Parametric star creation with configurable points and sharpness ratio (`S`).
  - `SpiralTool`: Archimedean spiral creation with configurable expansion and turns (`A`).
  - `LineTool`: 2-point line with 15° angle snapping (`L`).
  - `PenTool`: Anchor placement, smooth handle pulling, rubber-band curve preview, and path closure (`B`).

### 4. Curve Fitting Engine (`core-geom::curve_fit`)
- Philip J. Schneider's algorithm (Graphics Gems I) for automatic curve fitting.
- Chord-length parameterization, least-squares cubic Bézier fitting, Newton-Raphson parameter re-estimation, and recursive subdivision at maximum error points.

### 5. Fill & Stroke Subsystem (`app::panels::fill_stroke`)
- Live inspection and editing of shape fill, stroke color, and stroke width.
- Hairline classification: Automatic recognition of hairline strokes ($\le 0.1\text{ mm}$) for laser cutting (`#FF0000`) and laser scoring (`#0000FF`).
- Line Cap (`Butt`, `Round`, `Square`) and Line Join (`Miter`, `Round`, `Bevel`) configuration.
- Undoable style synchronization via `SetStyleCommand`.

## Panel Layout

```
┌──────────────────────────────────────────────────────┐
│  Menu bar                                            │
├──────────────────────────────────────────────────────┤
│  Property bar (context tool options)                 │
├────────┬─────────────────────────────────┬───────────┤
│        │                                 │           │
│  Tool  │   Canvas (pan/zoom/rulers/grid) │  Right    │
│  bar   │                                 │  Dock     │
│        │                                 │  (tabs)   │
│        │                                 │           │
├────────┴─────────────────────────────────┴───────────┤
│  Status bar (coords · zoom · snap toggles)           │
└──────────────────────────────────────────────────────┘
```

## Phase Roadmap

| Phase | Milestone |
|---|---|
| **Phase 0** | Workspace scaffold, CI, theme, empty canvas with rulers, pan/zoom, dock layout |
| **Phase 1** | core-geom and core-doc with unit tests, Rect/Ellipse/Line/Pen tools, Pick tool, undo/redo, selection |
| **Phase 2** | Shape/Node tool, node types, curve editing, Freehand fitting, Polygon/Star/Spiral, fill and stroke panel |
| **Phase 3** | Snapping system, R*-tree spatial index (0.0055 ms on 50k shapes), dynamic guides, alignment guides, rulers (Completed) |
| **Phase 4** | Booleans, Knife, Eraser, Virtual Segment Delete, offset/contour, text to outlines |
| **Phase 5** | Laser CAM pipeline, profiles, preview, HPGL/G-code/SVG export |
| **Phase 6** | SVG/DXF IO, native format, PDF/PNG export, dimensions and callouts |
| **Phase 7** | Performance, polish, onboarding, packaging (AppImage, .deb, Flatpak), documentation |
