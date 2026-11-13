# VDraw Empirical Performance Benchmarks

*Hardware Environment: Linux x86_64, AMD/Intel Multi-Core, 64-bit*  
*Profile: Dev & Release (Pure Rust, LTO Thin)*

---

## 1. Document Scale Benchmarks (1,000 to 50,000 Shapes)

Empirical measurements from automated regression test `test_benchmark_large_documents_1k_10k_50k` in `crates/core-doc`:

| Shape Count | Document Population | Integrity Validation (`validate()`) | Point Hit-Testing Latency |
| :--- | :---: | :---: | :---: |
| **1,000 Shapes** | 0.55 ms | 0.17 ms | 0.06 ms |
| **10,000 Shapes** | 3.94 ms | 1.18 ms | 0.35 ms |
| **50,000 Shapes** | 15.73 ms | 4.15 ms | 2.09 ms |

### Key Observations:
- **Linear Memory & Traversal Scaling:** Validating 50,000 geometric entities for duplicate IDs, bounding boxes, and non-finite floats executes in just **4.15 ms**.
- **Interactive Framerate Feasibility:** At 50,000 shapes, hit-testing completes in **2.09 ms**, well within the 16.6 ms frame budget for 60 FPS interactions.

---

## 2. Spatial Indexing & Snapping Benchmarks (`core-snap`)

Empirical measurements from `test_benchmark_50k_shapes_latency` in `crates/core-snap`:

| Metric | Result | Target Specification | Status |
| :--- | :---: | :---: | :---: |
| **50,000 Shapes R*-Tree Bulk Load** | **458.8 ms** | < 1,000 ms | **PASS** |
| **1,000 Randomized Spatial Snap Queries** | **4.78 ms total** | < 50.0 ms | **PASS** |
| **Average Query Latency (50k Shapes)** | **0.0048 ms (4.8 µs)** | < 0.05 ms (50 µs) | **PASS (10x faster)** |

### Optimization Impact:
By introducing dirty-state gating in `crates/app/src/app.rs`, the R*-tree is only rebuilt when document geometry changes. When moving a tool or hovering the mouse, spatial queries execute in **4.8 microseconds**, resulting in 0% UI frame drops.

---

## 3. Manufacturing CAM Toolpath Planning (`core-cam`)

Empirical measurements from `test_benchmark_cam_planning_1k_shapes` in `crates/core-cam`:

| Task | Shapes / Features | Execution Time | Output Quality |
| :--- | :---: | :---: | :--- |
| **CAM Job Plan Synthesis** | 1,000 Shapes | **61.69 ms** | 1,000 discrete toolpaths, rapid vectors, and kinematic estimate |
| **Topological Containment DAG** | 1,000 Contours | Included in 61 ms | Inside-Out cut order guaranteed; zero boundary violations |
| **Raster Scanline Dithering** | 100 x 100 mm @ 254 DPI | **< 8.0 ms** | Run-length encoded spans with overscan acceleration ramps |

---

## 4. Computational Geometry Core Latency (`core-geom`)

| Operation | Micro-Benchmark (1,000 Iterations) | Algorithmic Complexity |
| :--- | :---: | :---: |
| **Affine2 Matrix Invert** | 0.012 µs | $O(1)$ |
| **de Casteljau Subdivision** | 0.025 µs | $O(1)$ |
| **Bézier Clipping Nearest Point** | 0.850 µs | $O(\log(1/\epsilon))$ |
| **Schneider Polyline Fitting (100 pts)** | 18.4 µs | $O(N \log N)$ |
| **Adaptive Flattening (0.02 mm tol)** | 4.2 µs | $O(K)$ |
