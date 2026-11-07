# Phase 4 Changelog — 2D Boolean Engine, Knife, Eraser, Virtual Segment Delete, Contours & Sleek UI/UX

**Release / Milestone:** Phase 4  
**Status:** Completed  
**Date:** 2026-11-07  

---

## 1. Summary of Changes

Phase 4 delivers an industrial-strength 2D vector Boolean engine, interactive CAD shaping tools, parallel contour offset generation, and TrueType font outline conversion in 100% pure Rust. In direct response to user guidance, this milestone features an extensive overhaul of the user experience: a sleek, aesthetic startup Welcome Modal featuring the official VDraw logo mark, document templates for laser cutters, a dedicated Shaping Dock panel, a context-sensitive Property Bar with one-click boolean actions, real-time canvas overlays, and an animated HUD toast notification system.

---

## 2. New Modules & Capabilities

### A. Boolean CAD Engine (`crates/core-boolean/src/boolean.rs`)
- **8 Core Boolean Operations:**
  - **Weld (Union):** Combines 2 or more overlapping closed curves into a unified, seamless boundary.
  - **Trim (Difference):** Subtracts foreground shapes from the background shape.
  - **Intersect:** Computes the common overlapping region among all selected shapes.
  - **Exclude (XOR):** Retains regions where an odd number of shapes overlap.
  - **Front-Minus-Back:** Subtracts the lowest (background) shape from the top shape.
  - **Back-Minus-Front:** Subtracts the top (foreground) shape from the lowest shape.
  - **Create Boundary:** Computes an outer silhouette perimeter enclosing all shapes while preserving originals.
  - **Simplify:** Eliminates redundant collinear nodes and normalizes paths.
- **Sub-Micron Precision & Curve Re-fitting:**
  - Uses pure-Rust `clipper2-rs` with integer/float coordinate conversion at 0.1-micron precision ($10^{-4}\text{ mm}$).
  - Schneider curve fitting algorithm re-fits smooth cubic Bézier segments over clipped polyline vertices with straight-line detection via Ramer-Douglas-Peucker (RDP).
  - Area invariant property tests verify exact surface conservation.

### B. Knife Slicing Tool (`crates/core-boolean/src/knife.rs`, `crates/core-tools/src/knife.rs`)
- **Straight & Freehand Cutting Modes:**
  - Slices closed shapes across cut lines via fine-slit subtraction ($0.001\text{ mm}$ width) with auto-closing toggle.
  - Slices open paths at exact segment intersection points into independent open Polycurves.
- **Visual Canvas Overlay:**
  - Laser-red cutline with circular pin handles at start and end points during drag.

### C. Eraser Subtraction Tool (`crates/core-boolean/src/eraser.rs`, `crates/core-tools/src/eraser.rs`)
- **Dynamic Circular Brush Subtraction:**
  - Single-click circular punch and continuous drag swept-path subtraction using Round joins and Round endcaps.
  - Adjustable brush radius slider from $1.0\text{ mm}$ to $60.0\text{ mm}$.
- **Visual Canvas Overlay:**
  - Glowing circular brush cursor preview and semi-transparent magenta subtraction trail.

### D. Virtual Segment Delete Tool (`crates/core-boolean/src/virtual_segment.rs`, `crates/core-tools/src/virtual_segment_tool.rs`)
- **Intersection-Bounded Segment Detection:**
  - Automatically identifies subpath segments bounded between geometric intersection points across all document shapes.
  - Hovering previews the candidate segment with magenta dashed outline and scissors glyph (`✂`).
  - Clicking cleanly extracts the targeted segment from the parent Polycurve.

### E. Contour & Parallel Curve Offset (`crates/core-boolean/src/offset.rs`)
- **Laser Kerf Compensation & Inset/Outset Expansion:**
  - Positive offset distance expands outward; negative offset insets inward.
  - Corner join mechanics: `Round`, `Miter(miter_limit)`, and `Bevel`.
  - Driven by pure-Rust `clipper2_rs::inflate_paths_d` and `cavalier_contours`.

### F. TrueType Font Outline Conversion (`crates/core-boolean/src/text_to_path.rs`, `crates/core-tools/src/text_tool.rs`)
- **Zero-Approximation Bézier Degree Elevation:**
  - Parses OpenType/TrueType font glyphs using pure-Rust `ttf-parser`.
  - Elevates quadratic Béziers to cubic Béziers with mathematical exactness:
    $$C_1 = P_0 + \frac{2}{3}(Q_1 - P_0), \quad C_2 = P_2 + \frac{2}{3}(Q_2 - P_2)$$
  - Handles advance widths, kerning, and line origins.
  - Bundles native fallback font (`assets/fonts/default.ttf`) with zero external system font dependencies.

---

## 3. Sleek Aesthetic UI/UX Overhaul

### A. Startup Welcome Modal (`crates/app/src/panels/welcome.rs`)
- **Launch Screen & Project Templates:**
  - Opens on startup with the official VDraw brand logo mark (`assets/icons/icon.png`).
  - Pre-configured laser & CAD document presets:
    - 📄 **Laser Sheet A4** ($210 \times 297\text{ mm}$) — Standard material testing & small signage.
    - ⚡ **Laser Bed 600×400** ($600 \times 400\text{ mm}$) — Medium CO2/diode laser bed.
    - 🏭 **Laser Bed 900×600** ($900 \times 600\text{ mm}$) — Industrial flatbed laser cutter.
    - 📐 **CAD Technical Square** ($300 \times 300\text{ mm}$) — Engineering schematics & gears.
  - Custom width/height input field for non-standard laser beds and scrap materials.
  - Interactive keyboard shortcuts reference card.
  - Accessible anytime via `File -> Welcome Screen…` and `Help -> Welcome Screen…`.

### B. Dedicated Shaping Dock Tab (`crates/app/src/panels/boolean_panel.rs`)
- New **Shaping** tab in the right dock panel with dedicated sections:
  - Combine & Shaping grid (`⊞ Weld`, `⊟ Trim`, `⊠ Intersect`, `⊞̷ Exclude`, `⬚ Boundary`, `◌ Simplify`).
  - Parallel Contour / Offset controls (Offset distance, Round/Miter/Bevel joins).
  - Vector Text to Outlines controls (Text string, Font size, insertion).

### C. Contextual Quick Actions on Property Bar (`crates/app/src/panels/properties.rs`)
- When 2 or more shapes are selected, the Property Bar automatically exposes:
  `[⊞ Weld] [⊟ Trim] [⊠ Intersect] [⊞̷ Exclude] [⬚ Boundary] [⤢ Contour +3mm]`
- Live tool property controls when Knife (`K`), Eraser (`X`), Virtual Segment (`D`), or Text (`T`) tool is active.

### D. Animated HUD Toast Notifications (`crates/app/src/panels/toast.rs`)
- Sleek floating pill at the bottom-right corner of the viewport.
- Smooth alpha fade-in and fade-out animation providing immediate feedback:
  - `✦ ✓ Welded into 1 shape(s)`
  - `✦ ✓ Sliced 1 shape(s) into 2 pieces`
  - `✦ ✓ Generated 1 contour offset(s)`
  - `✦ ✓ Created Laser Bed 600×400 (600 × 400 mm)`

---

## 4. Verification & Metrics

- **Test Suite:** **45 passing tests** across workspace (12 new tests in `core-boolean`).
- **Linter & Compiler:** 0 warnings on `cargo clippy --all-targets --all-features -- -D warnings`.
- **Code Formatting:** `cargo fmt --all -- --check` passes cleanly.
- **Screenshot Verification:** Rendered `docs/vdraw_ui_phase4.png` displaying the welded cog silhouette, punched cutout, knife cutline, eraser brush, contour offset, and toast notification.
