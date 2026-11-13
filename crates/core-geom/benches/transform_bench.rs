use core_geom::{Affine2, BezierNode, NodeType, Point2, Polycurve, Subpath, TransformApply};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

fn build_1000_node_polycurve() -> Polycurve {
    let mut sub = Subpath::new(true);
    let n = 1000usize;
    for i in 0..n {
        let angle = (i as f64) * std::f64::consts::TAU / (n as f64);
        let r = 100.0 + 10.0 * (angle * 6.0).sin();
        let pt = Point2::new(r * angle.cos(), r * angle.sin());
        let h_in = Some(pt + Point2::new(-angle.sin() * 0.5, angle.cos() * 0.5));
        let h_out = Some(pt + Point2::new(angle.sin() * 0.5, -angle.cos() * 0.5));
        sub.push_node(BezierNode::with_handles(pt, h_in, h_out, NodeType::Smooth));
    }
    Polycurve::from_subpath(sub)
}

fn bench_transform_1000_nodes(c: &mut Criterion) {
    let orig = build_1000_node_polycurve();
    let matrix = Affine2::translate(Point2::new(42.0, -17.5))
        * Affine2::rotate_around(std::f64::consts::FRAC_PI_4, Point2::new(10.0, 20.0))
        * Affine2::scale_around(1.25, 0.85, Point2::ZERO);

    let mut group = c.benchmark_group("transform_1000_nodes");

    group.bench_function("batched_polycurve_apply", |b| {
        b.iter_batched(
            || orig.clone(),
            |mut poly| {
                let rec = poly.apply(black_box(&matrix));
                black_box((poly, rec))
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("sequential_per_vertex_point2_transform", |b| {
        b.iter_batched(
            || orig.clone(),
            |mut poly| {
                let m = black_box(&matrix);
                for sub in &mut poly.subpaths {
                    for node in &mut sub.nodes {
                        node.point = m.transform_point(node.point);
                        if let Some(h_in) = &mut node.handle_in {
                            *h_in = m.transform_point(*h_in);
                        }
                        if let Some(h_out) = &mut node.handle_out {
                            *h_out = m.transform_point(*h_out);
                        }
                    }
                }
                black_box(poly)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_transform_1000_nodes);
criterion_main!(benches);
