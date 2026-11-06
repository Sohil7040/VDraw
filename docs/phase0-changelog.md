# Phase 0 Changelog

## [0.1.0] — Phase 0 Scaffold

### Added
- Cargo workspace with 8 member crates
- `core-geom`: `Point2`, `Rect`, `Affine2`, `BezPath` / `PathSegment`
- `core-doc`: `Document`, `Page`, `Layer`, `SceneObject`
- `core-snap`: `SnapSolver`, `SnapConfig`, `SnapResult`
- `core-boolean`: `boolean_op` stub with `BooleanOp` enum
- `core-tools`: `ToolId` enum, `Tool` trait
- `core-cam`: `Camera` with `doc_to_screen` / `screen_to_doc`
- `io`: `save_json` / `load_json` via `serde_json`
- `app` binary crate (`vdraw`):
  - Full dark theme via `theme.rs` token constants + `build_visuals()`
  - `CanvasState` with pan / zoom (`Ctrl+scroll`, Space+drag, MMB drag)
  - A4 page rendering with drop shadow
  - Adaptive grid (1 mm / 5 mm / 10 mm based on zoom)
  - Rulers (horizontal + vertical) with mm tick marks and cursor indicator
  - Menu bar: File, Edit, View (workspace presets), Object, Effects, Tools, Help
  - Property bar placeholder
  - Status bar with cursor position (mm), zoom slider, snap toggles
  - Left toolbar with tool groups and emoji icons + tooltips
  - Right dock panel with tabs: Objects, Properties, Colors, Align, Laser
  - Command palette modal (Ctrl+K) with search
  - `WorkspacePreset` enum: Illustration, LaserCutting, TechnicalDrawing
- CI workflow: fmt check, clippy (-D warnings), tests, release build
- `docs/architecture.md`
- Asset placeholder READMEs

### Technical notes
- Renderer: eframe/wgpu
- egui version: 0.31
- All code passes `cargo clippy --all-targets -- -D warnings`
- All code formatted with `cargo fmt --all`
