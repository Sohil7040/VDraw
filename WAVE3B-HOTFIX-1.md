# WAVE3B-HOTFIX-1: Direct Transform Bypass in `align.rs` & Stale Spatial Index Tracking in `app.rs`

- **Owner:** Agent D (`app`) (with optional `AppCommand` extension coordination with Agent C `core-tools` if Align/Distribute commands are promoted to `AppCommand`)
- **Severity:** Medium–High (Architecture invariant violation + stale snap R\*-tree bug)
- **Found during:** Wave 4 Adversarial Integration Review — Check 4 (UI invariants / mutation routing audit)

---

## Defect Summary

1. **Direct `undo.apply(TransformShapesCommand)` bypass in `crates/app/src/panels/align.rs`:**
   All 8 Align and Distribute buttons in `show_align_panel` directly mutate `doc` and `undo` via `undo.apply(Box::new(TransformShapesCommand::new(...)), doc)` instead of routing through `Dispatcher::dispatch(...)` (or `TransformBuild`/`TransformApply`/`TransformRecordCommand`), and do not update `SnapController`:
   - `crates/app/src/panels/align.rs:62-65` (`Align Left`)
   - `crates/app/src/panels/align.rs:81-84` (`Align Center H`)
   - `crates/app/src/panels/align.rs:99-102` (`Align Right`)
   - `crates/app/src/panels/align.rs:119-122` (`Align Top`)
   - `crates/app/src/panels/align.rs:138-141` (`Align Center V`)
   - `crates/app/src/panels/align.rs:156-159` (`Align Bottom`)
   - `crates/app/src/panels/align.rs:189-195` (`Distribute Horizontally`)
   - `crates/app/src/panels/align.rs:221-227` (`Distribute Vertically`)

2. **Boolean-only dirty check in `VDrawApp::sync_spatial_index_if_dirty` (`crates/app/src/app.rs:799-822`) fails to detect subsequent mutations:**
   `VDrawApp::sync_spatial_index_if_dirty` checks:
   ```rust
   let current_dirty = self.undo_stack.is_dirty();

   if self.first_frame
       || current_shape_count != self.last_shape_count
       || current_dirty != self.last_undo_dirty
   ```
   Because `UndoStack::is_dirty()` returns a `bool` (`current_generation != saved_generation`), once the document is already dirty (`last_undo_dirty == true`), any subsequent mutation that preserves `shape_count` and bypasses `Dispatcher::dispatch` will **not** trigger `self.snap_controller.update_index(&self.doc)` or `self.snap_solver.spatial_index.rebuild(&shapes)`.

   Affected call sites when the document is already dirty:
   - `crates/app/src/panels/align.rs:62-227` (Align / Distribute shape transforms)
   - `crates/app/src/panels/properties.rs:42, 49, 56, 60` (`ShapeTool` node edits) and `309` (`Convert to Curves` via `ModifyShapeKindCommand`)
   - `crates/app/src/panels/object_manager.rs:67` (toggling `layer.visible`, which `SnapController::update_index` filters on at `crates/core-tools/src/snap_controller.rs:259`)

---

## Reproduction Hypothesis

1. Create or move any shape on canvas so `undo_stack.is_dirty()` becomes `true` (`last_undo_dirty = true`).
2. Select 2 shapes, open the **Align** dock tab, and click **Left** (or **Center H** / **Distribute**).
3. `align.rs` moves the shapes via `TransformShapesCommand`, preserving `shape_count` and keeping `is_dirty() == true`.
4. Hover or drag near the moved shapes: `SnapController` still resolves snap targets at the shapes' **pre-alignment** coordinates because neither `align.rs` nor `sync_spatial_index_if_dirty` updated the R\*-tree.

---

## Required Fix

1. Track a monotonic revision counter (e.g., expose `UndoStack::generation(&self) -> u64` or `history().len()` + top change, plus layer-visibility state) in `VDrawApp::sync_spatial_index_if_dirty` (`crates/app/src/app.rs:799-822`) instead of a `bool` `is_dirty()` diff, OR return an `AppCommand` / spatial-dirty flag from `dock::show_dock_content`, `show_prop_bar`, and `show_object_manager`.
2. Route `align.rs` mutations through `AppCommand` / `Dispatcher::dispatch` (or emit per-shape `TransformRecordCommand`s through a dispatched command) so `SnapController::update_index(doc)` runs deterministically on commit.
3. Audit remaining direct `undo.apply(...)` / `&mut doc` panel sites (`properties_panel.rs:536`, `fill_stroke.rs:266`, `properties.rs:42,49,56,60,309`, `object_manager.rs:25,67,76`) so spatial-index-affecting edits always refresh `SnapController`.
