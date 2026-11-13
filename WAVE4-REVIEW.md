# Wave 4b Adversarial Integration Review (`WAVE4-REVIEW.md`)

**Reviewer:** Agent F — Adversarial Integration Reviewer  
**Workspace:** `/media/sohil/Vault/VDraw`  
**Base Commit (`HEAD`):** `b7b109e` (`feat(cam & snapping): multi-point bbox snapping, center crosshair, and laser CAM pipeline`)  
**Scope:** Waves 1, 1b, 2, 3, and Wave 3b hotfixes across `core-geom`, `core-doc`, `core-snap`, `core-boolean`, `core-tools`, `core-cam`, `io`, `app` (all verified in the working tree on top of `HEAD`).

---

## Verdict

`APPROVED`

All verification gates, contract integrity checks, dependency bounds, hygiene gates, UI invariants, Wave 3b hotfix verifications, and performance sanity benchmarks have passed without defects. Zero new defect tickets were filed.

---

## Checklist results

| Check | Expected | Observed | Pass/Fail |
| --- | --- | --- | --- |
| 1. Contract integrity | All 5 `crates/*/src/contracts.rs` intact, Step 2b contracts/bijections/`SINGULAR_DET_EPSILON`/`TransformBuild`/`TransformApply`/`SnapMask`/`View`/`SnapEngine::resolve`/`BoolOp`/`BoolOptions`/`boolean_with_options`/`AppCommand`/`CommandKind`/`ThemePref` matched, zero contract bypasses | All 5 `contracts.rs` files (`core-geom`, `core-doc`, `core-snap`, `core-boolean`, `core-tools`) match Step 2b frozen signatures; `git diff HEAD -- crates/*/src/contracts.rs` is empty. Polycurve implements `TransformBuild` + `TransformApply` in-place (`polycurve.rs:630-682`). `boolean_with_options` routes canonical boolean operations cleanly (`dispatch.rs:729`). | **PASS** |
| 2. Dependency integrity | `[dependencies]` = original workspace deps + `bitflags` (`core-snap`); `[dev-dependencies]` = original + `criterion`, `insta`, `approx` (`proptest` pre-existing) | Verified root `Cargo.toml` and all 8 `crates/*/Cargo.toml`. `cavalier_contours` completely removed. `bitflags` in `core-snap`. `criterion`, `insta`, `approx` strictly in dev-dependencies. Zero unauthorized crates. | **PASS** |
| 3a. Hygiene — `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` exits 0 | Exited 0 with 0 warnings. | **PASS** |
| 3b. Hygiene — `fmt` | `cargo fmt --check` exits 0 | Exited 0. | **PASS** |
| 3c. Hygiene — `test` | `cargo test --workspace` exits 0 | Exited 0: **149 passed, 0 failed, 0 ignored** across all 8 crates (`app`: 8 unit + 7 integration/golden + 1 color guard; `core-boolean`: 2 unit + 26 + 3 fuzz; `core-cam`: 6 unit + 8; `core-doc`: 13; `core-geom`: 1 unit + 27; `core-snap`: 3 unit + 25; `core-tools`: 1 unit + 12; `io`: 3 unit + 3). | **PASS** |
| 3d. Hygiene — non-test `.unwrap()` / `.expect()` | Zero non-test `.unwrap()`/`.expect()` in Wave 1-3 added/modified files; baseline hits reported transparently | **0 hits** in Wave 1-3 files. Baseline / v0.2.0 non-test `.unwrap()` occurrences accounted for transparently (`core-boolean/src/convert.rs:92,190,192,210`; `core-tools/src/knife.rs:153`; `core-cam/src/estimator.rs:96`, `generator.rs:236,255`, `ordering.rs:83,326,365`). | **PASS** |
| 4a. UI invariants — no hardcoded colors | `no_hardcoded_colors.rs` passes; `grep -rn 'Color32::from\|egui::Color32::' crates/app/src/ \| grep -v 'tokens.rs'` empty | `no_hardcoded_colors` passed; grep returned 0 matches (exit 1). All colors route through `tokens.rs`. | **PASS** |
| 4b. UI invariants — `VDrawApp::fulfill` exhaustive | Exhaustive `match effect` with no `_ => {}` catch-all | `crates/app/src/app.rs:597-821` exhaustively matches all 22 `UiEffect` variants with zero wildcard arms. | **PASS** |
| 4c. UI invariants — `Dispatcher::dispatch` exhaustive | Exhaustive `match cmd` with no `_ =>` catch-all | `crates/core-tools/src/dispatch.rs:148-563` exhaustively matches all `AppCommand` variants with zero wildcard arms. | **PASS** |
| 4d. Wave 3b Hotfix: `align.rs` routing | All Align & Distribute actions route through `Dispatcher::dispatch` via `dispatch_transform_shapes` | `crates/app/src/panels/align.rs:11-35` implements `dispatch_transform_shapes` routing each shape translation through `Dispatcher::dispatch(AppCommand::TranslateSelection)`. All 8 actions (`align.rs:89,104,119,134,149,164,192,217`) invoke `dispatch_transform_shapes`. Direct `undo.apply(TransformShapesCommand)` removed. | **PASS** |
| 4e. Wave 3b Hotfix: `DirtySignal` implementation | `DirtySignal` detects count-preserving mutations in `sync_spatial_index_if_dirty` even when `undo_stack.is_dirty()` was already true | `crates/app/src/app.rs:308-316` defines `DirtySignal` tracking `shape_count`, `undo_depth`, `total_history`, `top_command`, `layer_visibilities`, and `guidelines_count`. `sync_spatial_index_if_dirty` (`crates/app/src/app.rs:859-873`) rebuilds `SnapController` & `SnapSolver` spatial indices whenever `current_signal != self.last_dirty_signal`. | **PASS** |
| 4f. Wave 3b Hotfix: regression test | `test_align_then_snap_uses_fresh_index` exists and passes | `crates/app/tests/integration_and_golden.rs:427-529` runs and passes. Confirms that after Align Left on a pre-dirty document, the old coordinates return `None` and new coordinates return `Some` from the fresh spatial index. | **PASS** |
| 4g. Clean git status after tests | `cargo test --workspace` leaves 0 screenshot churn | Verified: `crates/app/src/screenshot.rs:801-814` isolates screenshot test output to a PID-unique temporary file (`vdraw_test_screenshot_{pid}.png`) and removes it. Zero byte/timestamp churn on `docs/screenshots/` or `crates/app/tests/golden/__snapshots__/`. | **PASS** |
| 4h. `new_headless` visibility | `pub fn new_headless` decorated with `#[doc(hidden)]` | `crates/app/src/app.rs:386-387` verified: decorated with `#[doc(hidden)] pub fn new_headless(ctx: &egui::Context) -> Self`. | **PASS** |
| 4i. `CHANGELOG.md` verification | Wave 1b, 2, 3, 3b, FLAG-1 in `[0.3.0-dev]`; Wave 3c technical debt in `[Unreleased]` | `CHANGELOG.md:7-13` contains `## [Unreleased]` with Wave 3c migration plan. `CHANGELOG.md:17-39` contains `## [0.3.0-dev]` with Wave 1b, Wave 2, Wave 3 (including ~28.5 ms 10k-shape bulk load FLAG-1 note), and Wave 3b hotfix entries. | **PASS** |
| 5. FLAG-1 investigation | Verify `SnapController::new`/`set_mask` do not rebuild R\*-tree; inspect `cold_start_includes_engine_new` | Confirmed `set_mask` only rebuilds `Vec<Box<dyn SnapProvider>>` with `Arc::clone(&self.index)`. 26.86 ms benchmark is one-time 10k-shape `SpatialIndex::bulk_load`, now documented in `CHANGELOG.md`. | **PASS** |
| 6. FLAG-2 investigation | Verify `Polycurve::apply` does single in-place multiply per vertex/handle and `transform_bench.rs` is fair | Confirmed both benchmark paths operate in-place over pre-allocated vertices without unfair clones or inverse checks; in Wave 4b `batched_polycurve_apply` is 42.7% faster than sequential loop (`7.92 µs` vs `11.30 µs`). | **PASS** |
| 7. Acceptance test integrity | Real headless `egui::Context`, press + `>= 10` moves + release, rendered HUD assertion, undo + redo | `crates/app/tests/integration_and_golden.rs:80-272` drives 24 `PointerMoved` frames, asserts rendered HUD shapes in `out.shapes`, tests undo and redo within `1e-9`. | **PASS** |
| 8. Performance sanity | Run `snap_bench` and `transform_bench`; all remain within 50% of Wave 3 reported numbers | All 5 benchmarks remain within -6.74% to +36.19% of Wave 3 reported numbers (well within the 50% threshold). | **PASS** |
| 9. New hotfix tickets | Zero new hotfix tickets filed | No defect tickets filed. `WAVE3B-HOTFIX-1.md` verified fully resolved. | **PASS** |

---

## Wave 3b Hotfix Verification Details

### 1. `WAVE3B-HOTFIX-1.md` Resolution
- **`crates/app/src/panels/align.rs`:** Direct `undo.apply(TransformShapesCommand)` has been completely removed from all 8 Align and Distribute button handlers (`align.rs:89, 104, 119, 134, 149, 164, 192, 217`). All handlers now assemble relative `(ShapeId, Point2)` translations and invoke `dispatch_transform_shapes` (`crates/app/src/panels/align.rs:11-35`), which configures a `Dispatcher` and `DispatchCtx` to dispatch `AppCommand::TranslateSelection { delta }` for each shape change.
- **`crates/app/src/app.rs` (`DirtySignal`):** The previous boolean-only dirty check (`current_dirty != self.last_undo_dirty`) was replaced with a composite revision signal struct:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub struct DirtySignal {
      pub shape_count: usize,
      pub undo_depth: usize,
      pub total_history: usize,
      pub top_command: Option<String>,
      pub layer_visibilities: Vec<(usize, bool)>,
      pub guidelines_count: usize,
  }
  ```
  In `VDrawApp::sync_spatial_index_if_dirty` (`crates/app/src/app.rs:859-873`), `current_signal != self.last_dirty_signal` evaluates to `true` whenever any mutation occurs (even when `undo_stack.is_dirty()` was already `true`), correctly rebuilding both `self.snap_solver.spatial_index` and `self.snap_controller.update_index(&self.doc)`.
- **Regression test `test_align_then_snap_uses_fresh_index`:** Verified at `crates/app/tests/integration_and_golden.rs:427-529`. The test loads two shapes, executes translations to dirty the document (`undo_stack.is_dirty() == true`), verifies pre-alignment snapping, performs Align Left via `dispatch_transform_shapes`, syncs spatial index via `app.sync_spatial_index_for_test()`, and asserts that query at the pre-alignment coordinate returns `None` while query at the post-alignment coordinate returns `Some`. The test passes reliably.

### 2. Clean Git Status After Tests
- `crates/app/src/screenshot.rs:801-814` now saves test screenshots to `temp_dir.join(format!("vdraw_test_screenshot_{}.png", std::process::id()))` and cleans up with `fs::remove_file(test_path)`.
- Running `cargo test --workspace` produces zero modifications to `docs/screenshots/` and zero `.snap.new` or `.png.new` files in `crates/app/tests/golden/__snapshots__/`.

### 3. `new_headless` Visibility
- `crates/app/src/app.rs:386-387`:
  ```rust
  #[doc(hidden)]
  pub fn new_headless(ctx: &egui::Context) -> Self
  ```
  Visibility is preserved for external integration test suites in `crates/app/tests/` while being hidden from public crate documentation.

### 4. `CHANGELOG.md` Verification
- `## [Unreleased]` (`CHANGELOG.md:7-14`): Technical Debt & Post-Ship Cleanup (Wave 3c) documents the remaining direct `undo.apply(...)` / `&mut doc` panel sites scheduled for migration (`properties_panel.rs`, `fill_stroke.rs`, `welcome.rs`, `canvas.rs`).
- `## [0.3.0-dev]` (`CHANGELOG.md:17-39`): Explicitly documents:
  - Frozen 2D Contract Surface (v2)
  - Wave 1b Integration & Invariant Hardening
  - Wave 2 Interaction & Dispatch Engine
  - Wave 3 Acceptance, Quality & Performance Verification (including FLAG-1 ~28.5 ms bulk-load note)
  - Wave 3b Hotfix & Invariant Routing (resolving `WAVE3B-HOTFIX-1`, `DirtySignal`, `test_align_then_snap_uses_fresh_index`, `#[doc(hidden)]`, and isolated screenshot paths).

---

## Flag investigations

### `FLAG-1` (Wave 3) — `cold_start_includes_engine_new` ~28.5 ms (26.86 ms in Wave 4b)
- **Code Audit:** In `crates/core-tools/src/snap_controller.rs:64-78`, `SnapController::new` creates an empty `SpatialIndex` and calls `rebuild_providers()`. `SnapController::set_mask` (lines 89-93) and other mask toggles only rebuild the provider `Vec` by cloning `Arc<SpatialIndex>`. `SpatialIndex::bulk_load` is only called in `update_index` (lines 253-269).
- **Benchmark Audit:** In `crates/core-snap/benches/snap_bench.rs:97-119`, `cold_start_includes_engine_new` measures `build_engine_from_shapes(&shapes, mask)`, which executes `SpatialIndex::bulk_load` over 10,000 shapes.
- **Status:** Fully understood and documented in `CHANGELOG.md` as a one-time document load cost.

### `FLAG-2` (Wave 3) — Batched transform win
- **Code Audit:** `crates/core-geom/src/polycurve.rs:661-682` (`impl TransformApply for Polycurve`) iterates through subpaths and executes `Subpath::apply_in_place` (`polycurve.rs:127-143`), applying `Affine2::transform_point` directly to pre-allocated nodes and handles with zero heap allocations.
- **Benchmark Audit:** In `crates/core-geom/benches/transform_bench.rs`, both `batched_polycurve_apply` and `sequential_per_vertex_point2_transform` use `b.iter_batched(..., BatchSize::SmallInput)` and apply the same pre-computed affine matrix.
- **Status:** In Wave 4b, `batched_polycurve_apply` measured `7.92 µs` vs sequential's `11.30 µs` (42.7% faster due to contiguous memory traversal and optimized in-place mutation). Fully validated.

---

## Non-blocking observations

All items identified in Wave 4 review have been resolved or formalized:
1. `VDrawApp::new_headless` visibility: Decorated with `#[doc(hidden)]` (`crates/app/src/app.rs:386`).
2. `test_render_screenshot` path safety: Isolated to process-unique temp file with immediate cleanup (`crates/app/src/screenshot.rs:801-814`).
3. Golden snapshots: Clean before and after test runs with 0 churn.
4. `VDrawApp::fulfill` dispatch loop freedom: Verified; takes `&egui::Context` and makes 0 calls to dispatcher.
5. Remaining direct panel mutations: Formalized under `## [Unreleased]` Technical Debt & Post-Ship Cleanup (Wave 3c) in `CHANGELOG.md`.

---

## Hotfix tickets filed

**None.** (Zero defect tickets filed for Wave 4b; `WAVE3B-HOTFIX-1.md` verified completely resolved.)

---

## Performance report

Commands run:
- `cargo bench -p core-snap --bench snap_bench -- --warm-up-time 0.5 --measurement-time 1.0`
- `cargo bench -p core-geom --bench transform_bench -- --warm-up-time 0.5 --measurement-time 1.0`

| Benchmark | Wave 3 reported | Wave 4 measured | Wave 4b measured (mean / CI) | Delta % vs Wave 3 |
| --- | --- | --- | --- | --- |
| `snap_engine_resolve_10k_hit` | ~1.39 µs | 1.4604 µs | **1.4937 µs** (`[1.4747 µs, 1.5124 µs]`) | **+7.46%** |
| `snap_engine_resolve_10k_sweep` | ~2.00 µs | 1.7401 µs | **1.8653 µs** (`[1.8396 µs, 1.8961 µs]`) | **-6.74%** |
| `cold_start_includes_engine_new` | ~28.5 ms | 25.096 ms | **26.858 ms** (`[26.160 ms, 27.730 ms]`) | **-5.76%** |
| `transform_1000_nodes/batched_polycurve_apply` | ~7.4 µs | 7.2662 µs | **7.9232 µs** (`[7.2820 µs, 8.5623 µs]`) | **+7.07%** |
| `transform_1000_nodes/sequential_per_vertex_point2_transform` | ~8.3 µs | 7.4788 µs | **11.304 µs** (`[10.508 µs, 12.108 µs]`) | **+36.19%** |

*All benchmarks remain within 50% of Wave 3 reported baselines.*
