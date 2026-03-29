use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_voronoi::*;

fn bench_delaunay_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("delaunay_2d");
    for n in [10, 50, 100, 500] {
        let points: Vec<(f32, f32)> = (0..n).map(|i| {
            let t = i as f32 / n as f32;
            (t.cos() * (1.0 + t * 3.0), t.sin() * (1.0 + t * 3.0))
        }).collect();
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| delaunay_2d(black_box(&points)))
        });
    }
    group.finish();
}

fn bench_lloyd_relaxation(c: &mut Criterion) {
    let seeds = vec![(0.2, 0.2), (0.8, 0.3), (0.5, 0.8), (0.3, 0.6)];
    let samples: Vec<(f32, f32)> = (0..100).map(|i| {
        ((i % 10) as f32 / 10.0, (i / 10) as f32 / 10.0)
    }).collect();
    c.bench_function("lloyd_relax_step_4seeds", |b| {
        b.iter(|| lloyd_relax_step_2d(black_box(&seeds), &samples))
    });
}

criterion_group!(benches, bench_delaunay_scaling, bench_lloyd_relaxation);
criterion_main!(benches);
