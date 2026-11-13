use core_doc::{Shape, ShapeId};
use core_geom::{Point2, Rect};
use core_snap::{
    GeometrySnapProvider, GridSnapProvider, SnapCtx, SnapEngine, SnapMask, SpatialIndex, View,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn build_10k_shapes() -> Vec<Shape> {
    let mut shapes = Vec::with_capacity(10_000);
    // 100 x 100 grid of alternating rectangles, ellipses, and crossing lines (10,000 shapes)
    for i in 0..100 {
        for j in 0..100 {
            let idx = (i * 100 + j + 1) as u64;
            let id = ShapeId(idx);
            let x = i as f64 * 40.0;
            let y = j as f64 * 40.0;
            let shape = match idx % 3 {
                0 => Shape::rectangle(
                    id,
                    Rect::from_origin_size(Point2::new(x, y), 20.0, 20.0),
                    0.0,
                ),
                1 => Shape::ellipse(id, Point2::new(x + 10.0, y + 10.0), 10.0, 8.0),
                _ => Shape::line(id, Point2::new(x, y), Point2::new(x + 20.0, y + 20.0)),
            };
            shapes.push(shape);
        }
    }
    shapes
}

fn build_engine_from_shapes(shapes: &[Shape], mask: SnapMask) -> SnapEngine {
    let index = Arc::new(SpatialIndex::bulk_load(shapes));
    let mut engine = SnapEngine::new(mask);
    engine.providers.push(Box::new(
        GeometrySnapProvider::new(Arc::clone(&index)).with_reference_origin(Some(Point2::ZERO)),
    ));
    engine.providers.push(Box::new(GridSnapProvider::default()));
    engine.modes = mask;
    engine
}

fn default_bench_view() -> View {
    View::new(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0)),
        egui::Vec2::ZERO,
        2.0,
    )
}

fn build_10k_engine() -> (SnapEngine, View) {
    let shapes = build_10k_shapes();
    let engine = build_engine_from_shapes(&shapes, SnapMask::all());
    let view = default_bench_view();

    // Warm up internal scratch buffer before benchmarking.
    let warmup_ctx = SnapCtx {
        cursor_screen: view.world_to_screen(Point2::new(400.5, 400.5)),
        view: &view,
        threshold_px: 8.0,
    };
    let _ = engine.resolve(&warmup_ctx);

    (engine, view)
}

fn bench_snap_resolve_10k(c: &mut Criterion) {
    let (engine, view) = build_10k_engine();

    c.bench_function("snap_engine_resolve_10k_hit", |b| {
        let ctx = SnapCtx {
            cursor_screen: view.world_to_screen(Point2::new(1200.5, 1200.5)),
            view: &view,
            threshold_px: 8.0,
        };
        b.iter(|| black_box(engine.resolve(black_box(&ctx))));
    });

    c.bench_function("snap_engine_resolve_10k_sweep", |b| {
        let mut step = 0usize;
        b.iter(|| {
            step = step.wrapping_add(1);
            let wx = ((step * 37) % 4000) as f64 + 0.5;
            let wy = ((step * 53) % 4000) as f64 + 0.5;
            let ctx = SnapCtx {
                cursor_screen: view.world_to_screen(Point2::new(wx, wy)),
                view: &view,
                threshold_px: 8.0,
            };
            black_box(engine.resolve(black_box(&ctx)))
        });
    });

    let shapes = build_10k_shapes();
    c.bench_function("cold_start_includes_engine_new", |b| {
        let ctx = SnapCtx {
            cursor_screen: view.world_to_screen(Point2::new(1200.5, 1200.5)),
            view: &view,
            threshold_px: 8.0,
        };
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for i in 0..iters {
                let mask = if i % 2 == 0 {
                    SnapMask::all()
                } else {
                    SnapMask::NODE | SnapMask::ENDPOINT | SnapMask::CENTER | SnapMask::INTERSECTION
                };
                let start = Instant::now();
                let engine = build_engine_from_shapes(black_box(&shapes), black_box(mask));
                let res = engine.resolve(black_box(&ctx));
                total += start.elapsed();
                black_box(res);
            }
            total
        });
    });
}

criterion_group!(benches, bench_snap_resolve_10k);
criterion_main!(benches);
