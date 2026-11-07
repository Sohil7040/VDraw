# Phase 2 Changelog — Shape/Node Tool, Curve Editing, Curve Fitting, Parametric Primitives & Fill/Stroke Panel

**Release / Milestone:** Phase 2  
**Status:** Completed  
**Date:** 2026-11-07  

---

## 1. Summary of Changes

Phase 2 introduces comprehensive curve and node editing capabilities, automatic digitized curve fitting using Schneider's algorithm, parametric generator tools for regular polygons, stars, and Archimedean spirals, and an interactive Fill & Stroke inspector panel with laser CAM hairline classification.

---

## 2. New Modules & Capabilities

### A. Automatic Curve Fitting (`crates/core-geom/src/curve_fit.rs`)
- **Schneider's Algorithm (Graphics Gems I):**
  - Robust clean-room implementation of Philip J. Schneider's algorithm for fitting digitized points with cubic Bézier splines within error tolerance.
  - Chord-length parameterization with endpoint tangent estimation.
  - Least-squares fitting of cubic Bézier control points $P_1$ and $P_2$.
  - Euclidean distance maximum error evaluation against digitized points.
  - Newton-Raphson parameter re-estimation iterations.
  - Recursive curve subdivision at points of maximum error, producing continuous $G^1$ `Smooth` internal nodes.
  - Handles edge cases: empty slices, single points, 2-point lines, duplicate/near-coincident points, and non-converging fits.

### B. Parametric Shape Normalization (`crates/core-geom/src/polycurve.rs`)
- **Regular Polygon Generator:**
  - `Polycurve::polygon(center, radius, sides)`: Parametric $N$-sided regular polygon ($3 \le N \le 50$) normalized into closed cubic Bézier subpaths.
- **Star Generator:**
  - `Polycurve::star(center, outer_radius, inner_radius, points)`: Parametric star with $N$ points ($3 \le N \le 50$) and configurable sharpness ratio.
- **Archimedean Spiral Generator:**
  - `Polycurve::spiral(center, turns, expansion)`: Smooth polar spiral $r(\theta) = b \cdot \theta$ ($0.5 \le \text{turns} \le 20$) approximated with cubic Bézier segments with analytical derivative tangent matching.
- **Subpath Node Editing Mechanics:**
  - `Subpath::split_segment(seg_idx, t)`: Exact de Casteljau split inserting a new `BezierNode` at parameter $t$ with smooth handles, preserving exact curve geometry.
  - `Subpath::remove_node(node_idx)`: Deletes anchor point and re-connects neighboring segments.
  - `Subpath::set_node_type(node_idx, node_type)`: Dynamically enforces `Cusp`, `Smooth`, or `Symmetrical` constraints on control handles.

### C. Document Model & Commands (`crates/core-doc`)
- **Shape Constructors & Converters:**
  - Constructors for `Shape::polygon`, `Shape::star`, `Shape::spiral`, and `Shape::path`.
  - `Shape::convert_to_path()`: Converts any parametric primitive (Rectangle, Ellipse, Polygon, Star, Spiral) into editable cubic Bézier nodes.
- **Undoable Node and Style Commands:**
  - `SetStyleCommand`: Reversible changes to shape fill, stroke color, stroke width, caps, and joins.
  - `ModifyShapeKindCommand`: Reversible modifications to a shape's underlying geometry (`ShapeKind`), used for node dragging, handle adjustment, node insertion, node deletion, and "Convert to Curves".

### D. Interactive Tool State Machine (`crates/core-tools`)
- **Shape / Node Tool (`ToolId::Shape`, Shortcut `F10`):**
  - Interactive anchor point selection and dragging.
  - Control handle dragging (`handle_in` / `handle_out`) with live constraint propagation across `Cusp`, `Smooth`, and `Symmetrical` nodes.
  - Shift-click multi-node selection.
  - Double-click on any curve segment splits the curve and inserts an anchor point at the clicked location.
  - Delete key removes selected nodes with undo support.
  - Visual overlays: 7x7 px square anchor points (violet for selected, white for unselected), teal tangent lines, and circular teal handle endpoints.
- **Freehand Tool (`ToolId::Freehand`, Shortcut `F5`):**
  - Smooth pointer tracking with distance filtering.
  - Real-time stroke preview during drag.
  - On mouse release, executes Schneider's algorithm to convert raw stylus/pointer points into smooth cubic Bézier curves within configurable tolerance ($0.2\text{ mm}..10.0\text{ mm}$, default $1.5\text{ mm}$).
- **Polygon Tool (`ToolId::Polygon`, Shortcut `Y`):**
  - Drag from center to set radius; live overlay preview.
  - Configurable number of sides ($3..=50$, default 5).
- **Star Tool (`ToolId::Star`, Shortcut `S`):**
  - Drag from center to set outer radius; live overlay preview.
  - Configurable points ($3..=50$) and sharpness ratio ($10\%..90\%$).
- **Spiral Tool (`ToolId::Spiral`, Shortcut `A`):**
  - Drag from center to set expansion radius; live overlay preview.
  - Configurable number of turns ($0.5..=20.0$, default 3.0).

### E. UI & Dock Panels (`crates/app`)
- **Fill & Stroke Panel (`DockTab::Colors`):**
  - Fill section: Enabled toggle, color picker with RGBA sliders, hex input, and curated swatch palette.
  - Stroke section: Enabled toggle, color picker, stroke width stepper in mm, and swatches.
  - Laser Hairline Classification:
    - Real-time detection of hairline cuts ($\le 0.1\text{ mm}$) and laser color coding (Red `#FF0000` = Laser Cut, Blue `#0000FF` = Laser Score).
    - One-click buttons: "✂ Laser Cut (0.05mm Red)" and "✏ Laser Score (0.05mm Blue)".
    - Preset width buttons ($0.2\text{ mm}, 0.5\text{ mm}, 1.0\text{ mm}, 2.0\text{ mm}$).
    - Line Cap selector (`Butt`, `Round`, `Square`).
    - Line Join selector (`Miter`, `Round`, `Bevel`).
- **Contextual Property Bar:**
  - Displays context-specific controls when `Shape` tool is active: `Cusp`, `Smooth`, `Symmetrical`, `Delete Node`.
  - Displays steppers for `Sides` (Polygon), `Points`/`Sharpness` (Star), `Turns` (Spiral), and `Tolerance` (Freehand).
  - One-click "Convert to Curves" button for any selected primitive.
- **Toolbar & Shortcuts:**
  - Integrated `F10` (Shape Tool), `F5` (Freehand), `Y` (Polygon), `S` (Star), `A` (Spiral).

---

## 3. Verification & Quality Gates

| Verification Gate | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | **PASS** | 100% clean formatting across all crates |
| `cargo clippy --all-targets --all-features -- -D warnings` | **PASS** | 0 warnings, strict linting enforced |
| `cargo test --all` | **PASS** | 22 tests passing, including `proptest` suites: <br>• `prop_split_join_continuity`<br>• `prop_closest_point_is_local_minimum`<br>• `prop_fit_curve_preserves_endpoints` |
| `cargo build --release` | **PASS** | Clean release binary compiled |
| UI & Screenshot Verification | **PASS** | Headless screenshot engine generated `docs/vdraw_ui_phase2.png` |

---

## 4. Known Items & Non-blocking Follow-ups

1. **Laser Containment & Kerf Pipeline:** Slated for Phase 5 (Laser CAM pipeline).
2. **Snapping & Dynamic Alignment Guides:** Next up in Phase 3.
