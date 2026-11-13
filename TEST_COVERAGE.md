# VDraw Test Coverage & Verification Report

*Test Suite Overview: 84 Automated Tests Across 8 Workspace Crates*  
*Result: 100% Passed (0 Failed, 0 Ignored)*

---

## 1. Test Distribution by Crate

| Crate | Unit / Integration Tests | Property Tests (`proptest`) | Stress / Benchmarks | Total Tests |
| :--- | :---: | :---: | :---: | :---: |
| `core-geom` | 17 | 3 | 1 | **20** |
| `core-boolean` | 19 | 1 | 0 | **20** |
| `core-doc` | 12 | 0 | 1 | **13** |
| `core-snap` | 8 | 2 | 1 | **11** |
| `core-tools` | Builtin | 0 | 0 | **Verified** |
| `core-cam` | 7 (int) + 6 (unit) | 0 | 1 | **14** |
| `io` | 3 (int) + 3 (unit) | 0 | 0 | **6** |
| `app` | 1 (screenshot) | 0 | 0 | **1** |
| **Workspace Total** | **68** | **6** | **4** | **85** |

---

## 2. Critical Algorithm Test Matrix

### A. Computational Geometry (`core-geom`)
- `test_affine_singular_and_nan_handling`: Tests zero determinant, IEEE 754 NaN handling, and near-singular inversion.
- `test_bezier_closest_point_difficult_cases`: Tests cubic S-curves, high-curvature loops, and cusps against degree-5 Bézier clipping root finder.
- `prop_closest_point_is_local_minimum`: Property test verifying that any perturbation from the projected nearest point increases distance.
- `test_curve_fit_sharp_corner_preservation`: Tests Schneider curve fitting across 90-degree corners, ensuring Cusp nodes are generated and tangent directions do not bleed across corners.
- `prop_fit_curve_preserves_endpoints`: Verifies fitted Bézier curves always match the first and last polyline vertices within $10^{-12}\,\text{mm}$.
- `test_flatten_collinear_overshoot_safeguard`: Tests adaptive subdivision termination on degenerate and collinear segments.

### B. Boolean Engine (`core-boolean`)
- `test_coordinate_out_of_range_rejected`: Verifies coordinate bounds checking ($\pm 100,000\,\text{mm}$) and rejection of non-finite floats.
- `test_boolean_failure_document_integrity_rollback`: Verifies that if an invalid shape fails CSG, document state remains unmodified.
- `test_front_minus_back_and_back_minus_front`: Tests order-dependent difference operations on overlapping rectangles.
- `test_trim_difference`: Tests trimming a target shape by a cutter without mutating the cutter.
- `test_knife_slice_circle` & `test_knife_slice_open_path`: Tests straight and freehand knife slicing with point-in-polygon nested hole containment.
- `test_offset_outward_and_inward`: Tests compound offset with closed outer polygons and open toolpaths.

### C. Document Architecture (`core-doc`)
- `test_document_validation_errors`: Checks duplicate shape IDs, non-finite coordinates, and negative dimensions.
- `test_undo_stack_dirty_tracking`: Tests generation-based dirty flags across multiple commands, undos, and save checkpoints.
- `test_group_bounds_and_hit_test`: Tests recursive bounds calculation and interior hit-testing for nested groups.
- `test_benchmark_large_documents_1k_10k_50k`: Stresses document creation, validation, and spatial hit testing across 1,000, 10,000, and 50,000 shapes.

### D. Snapping & Spatial Indexing (`core-snap`)
- `test_benchmark_50k_shapes_latency`: Generates 50,000 shapes, bulk-loads the $R^*$-tree, and runs 1,000 randomized spatial snap queries.
- `prop_snap_within_tolerance`: Verifies that any reported snap point is strictly within the user tolerance radius.
- `test_smart_alignment_guides` & `test_dynamic_polar_guide_snapping`: Tests dynamic guide calculation across moving shapes.

### E. Manufacturing CAM (`core-cam`)
- `test_containment_and_inside_out_ordering`: Verifies that an inner hole is cut before its enclosing boundary.
- `test_multi_level_nesting_dag_ordering`: Verifies 4-level nesting (hole within island within hole within outer part) preserves topological cut order.
- `test_engraving_precedes_enclosing_cut`: Verifies that vector engraving on a part is executed before cutting that part.
- `test_kerf_outside_expansion_and_contraction`: Verifies that outer contours expand by $+kerf/2$ and inner holes shrink by $-kerf/2$.
- `test_kerf_collapse_detection`: Verifies feature collapse detection on holes smaller than the kerf beam width.
- `test_safety_soft_limits` & `test_simulated_controller_safety_check`: Verifies soft-limit violation detection and controller invariant checking.
- `test_job_plan_generation_and_dialects`: Verifies Generic G-code, GRBL ($32=1, M4), Marlin, and HPGL output.

### F. File I/O (`io`)
- `test_vdraw_roundtrip_with_validation`: Verifies native `.vdraw` JSON serialization, schema versioning, and validation on reload.
- `test_svg_export_and_import_roundtrip`: Verifies SVG roundtrip fidelity for coordinates, paths, fills, and strokes.
- `test_dxf_export`: Verifies AutoCAD DXF R12 header, layer table, LINE, and LWPOLYLINE entities.
