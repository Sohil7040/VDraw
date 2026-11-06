# Phase 1 Changelog — Core Geometry, Document Model, Tools & Selection

**Release / Milestone:** Phase 1  
**Status:** Completed  
**Date:** 2026-11-06  

---

## 1. Summary of Changes

Phase 1 establishes the mathematical and architectural core of VDraw. It introduces clean-room implementations of 2D geometry primitives and Bézier algorithms in `core-geom`, the hierarchical document and undo/redo model in `core-doc`, the interactive tool state machine in `core-tools`, and interactive rendering/manipulation in `app`.

---

## 2. New Modules & Capabilities

### A. Geometry Engine (`crates/core-geom`)
- **Primitives:**
  - `Point2`: Vector arithmetic, dot/cross products, Euclidean distance, normalization, angle calculation, and lossless kurbo conversion.
  - `Rect`: Min/max representation, intersection, union, point containment, inflation, coordinate translation, and kurbo conversion.
  - `Affine2`: 2D affine transformation matrices supporting translation, non-uniform scaling, rotation around an arbitrary origin or pivot, skew/shear, matrix multiplication, inversion, and point/rect transformations.
- **Node Mechanics:**
  - `NodeType`: `Cusp` (independent handle angles and lengths), `Smooth` (collinear $G^1$ continuity, independent lengths), and `Symmetrical` (collinear and equal lengths).
  - `BezierNode`: Position and incoming/outgoing handles with automatic constraint propagation whenever a handle is updated.
- **Cubic Bézier Algorithms:**
  - **de Casteljau Split:** Exact split at parameter $t \in [0, 1]$ and midpoint $t = 0.5$.
  - **Evaluation & Derivatives:** Exact computation of $B(t)$, velocity $B'(t)$, acceleration $B''(t)$, unit tangents, and unit normals.
  - **Arc Length:** High-precision adaptive 5-point Gauss-Legendre quadrature.
  - **Closest-Point Solver:** Fast bounding box rejection, 4-level bisection for global minimum localization, followed by Newton-Raphson refinement and $[0, 1]$ parameter clamping.
  - **Adaptive Flattening:** Tolerance-based recursive subdivision ($\le 0.02\text{ mm}$ default) generating polygonal approximations.
  - **Exact Bounding Box:** Tight extrema computation via quadratic roots of $B'(t) = 0$.
- **Polycurve & Primitives Normalization:**
  - `Polycurve` and `Subpath` representing composite multi-subpath paths.
  - Primitive generators normalizing Lines, Rectangles (with optional fillet radius), and Ellipses/Circles into cubic Bézier segments (using $\kappa = 4/3 \times (\sqrt{2} - 1) \approx 0.5522847498307936$).

### B. Document Model & Undo/Redo (`crates/core-doc`)
- **Document Hierarchy:**
  - `Document` containing multiple `Page`s (A4 default: $210 \times 297\text{ mm}$).
  - `Page` containing multiple `Layer`s with visibility, locking, and opacity.
  - `Layer` containing ordered `Shape` elements.
- **Shape Representation:**
  - `ShapeKind`: `Path`, `Rect`, `Ellipse`, `Line` (with placeholders for Polygon, Star, Text, Group).
  - Normalization: Every shape provides `to_polycurve()` for uniform geometry and CAM processing.
  - Local and world-space tight bounding box computation.
  - Layered hit-testing against shapes and outlines.
- **Styling & Units:**
  - `Color`: RGBA $u8$ representation with hex formatting/parsing.
  - `Style`: Fill, stroke width, stroke color, line cap (`Butt`, `Round`, `Square`), line join (`Miter`, `Round`, `Bevel`), and hairline classification ($\le 0.1\text{ mm}$ for laser cutting).
  - `Unit`: Support for Millimeters, Inches, and Points with conversion utilities.
- **Selection & Handles:**
  - `Selection`: Set of selected shape IDs with bounding box computation.
  - `TransformMode`: Toggle between `Scale` mode (8 handles) and `RotateSkew` mode (4 corner rotation handles, 4 skew handles, center pivot point).
  - Handle hit-testing with customizable hit radius.
- **Undo / Redo Command Pattern:**
  - `Command` trait: `execute(&mut Document)` and `undo(&mut Document)`.
  - `UndoStack`: Configurable history depth with `can_undo()`, `can_redo()`, undo/redo descriptions, and push.
  - Core commands: `AddShapeCommand`, `RemoveShapesCommand`, `TransformShapesCommand`, `CompoundCommand`.

### C. Tools & State Machine (`crates/core-tools`)
- **Tool Architecture:**
  - `Tool` trait: `on_pointer_down`, `on_pointer_move`, `on_pointer_up`, `on_key`, `render_overlay`, `cursor`, and `tool_id`.
  - `ToolManager`: Active tool lifecycle, event routing, and overlay drawing.
- **Implemented Tools:**
  1. **Pick Tool (`ToolId::Pick`, shortcut `V`):**
     - Single shape selection and multi-selection with Shift.
     - Canvas click-drag marquee (rubber band) selection.
     - Click selected shape to toggle between Scale and Rotate/Skew modes.
     - 8-handle interactive scaling with opposite anchor pinning; Ctrl restricts to 1:1 aspect ratio.
     - Corner rotation around center pivot; Ctrl snaps to 15° increments.
     - Interactive shape translation / dragging.
     - Contextual mouse cursors (`ResizeNorthSouth`, `ResizeEastWest`, etc.).
  2. **Rectangle Tool (`ToolId::Rectangle`, shortcut `F6`):**
     - Interactive drag creation with live preview.
     - Shift creates rectangle centered at start point.
     - Ctrl constrains to 1:1 square.
     - Commits via `AddShapeCommand` on release.
  3. **Ellipse Tool (`ToolId::Ellipse`, shortcut `F7`):**
     - Interactive drag creation with live preview.
     - Shift creates ellipse centered at start point.
     - Ctrl constrains to 1:1 circle.
     - Commits via `AddShapeCommand` on release.
  4. **Line Tool (`ToolId::Line`, shortcut `L`):**
     - Interactive 2-point line creation.
     - Ctrl snaps angle to 15° increments.
  5. **Pen Tool (`ToolId::Pen`, shortcut `B`):**
     - Click to place Cusp nodes.
     - Click-drag to pull out smooth cubic Bézier handles.
     - Live rubber-band curve preview to the cursor.
     - Click near the start node to close the path.
     - Double-click or Enter to finish open path.
     - Commits via `AddShapeCommand`.

### D. UI & Canvas Integration (`crates/app`)
- **Canvas Rendering:**
  - Document page rendering with drop shadow and border.
  - Adaptive metric millimeter rulers with major/minor tick marks and cursor tracking indicators.
  - Coordinate transformations between document world space (mm) and screen space (px).
  - Shape rendering with fills, strokes, and hairline laser styling.
  - Active tool overlays: selection bounds, 8 scale handles, rotation/skew handles, center pivot, marquee box, and pen rubber-band curves.
- **Application Shell & Menus:**
  - Dynamic Undo/Redo menu items showing action descriptions (e.g. `Undo Create Rectangle`).
  - Keyboard shortcuts wired: `Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Delete`, `Ctrl+A`, `Escape`, `V`, `F6`, `F7`, `L`, `B`.
  - Property bar updating in real time with selection geometry ($X, Y, W, H$).
  - Status bar displaying cursor coordinates in mm, zoom level, and selected count.
- **Headless Screenshot Generator:**
  - Built-in `screenshot` module utilizing `tiny-skia` for pixel-perfect UI capture under headless / CI / Wayland environments.

---

## 3. Verification & Quality Metrics

- **Unit & Property Tests:**
  - `core-geom`: 11 tests including `proptest` verifying split-join continuity (`prop_split_join_continuity`) and local minimum convergence of the closest-point solver (`prop_closest_point_is_local_minimum`).
  - `core-doc`: 4 tests verifying shape creation, removal, transformation, undo/redo state restoration, and click-to-toggle transform mode.
  - All tests passing with 0 failures.
- **Linters & Formatters:**
  - `cargo fmt --all -- --check` passing clean.
  - `cargo clippy --all-targets --all-features -- -D warnings` passing with 0 warnings.
- **Release Binary:**
  - `cargo build --release` compiles cleanly without warnings or errors.

---

## 4. Known Issues & Non-blocking Items

1. **Vello Integration:** Currently, vector shapes are rendered into egui's painter pipeline. GPU-accelerated Vello rendering via an `egui_wgpu` callback will be wired as a dedicated backend option in subsequent iterations.
2. **Text / Glyph Outlines:** Reserved for Phase 4 / Phase 6 per the milestone roadmap.
3. **Advanced Node Editing (Shape/Node Tool):** Phase 1 establishes the node data structures (`Cusp`, `Smooth`, `Symmetrical`) and Pen creation. Direct node manipulation and handle dragging will be implemented in Phase 2.
