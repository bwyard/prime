use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_dynamics::*;

fn bench_rk4_step(c: &mut Criterion) {
    c.bench_function("rk4_step_1000", |b| {
        b.iter(|| {
            (0..1000).fold(1.0_f32, |state, i| {
                rk4_step(black_box(state), i as f32 * 0.01, 0.01, |_t, s| -s)
            })
        })
    });
}

fn bench_lorenz_1000(c: &mut Criterion) {
    c.bench_function("lorenz_1000_steps", |b| {
        b.iter(|| {
            (0..1000).fold((1.0_f32, 1.0, 1.0), |state, _| {
                lorenz_step(black_box(state), 10.0, 28.0, 8.0/3.0, 0.01)
            })
        })
    });
}

fn bench_lsystem_generations(c: &mut Criterion) {
    let rules = vec![
        LRule { symbol: 'F', replacement: "F+F-F-F+F" },
    ];
    let mut group = c.benchmark_group("lsystem_koch");
    for gen in [3, 5, 7] {
        group.bench_function(format!("gen={gen}"), |b| {
            b.iter(|| lsystem_generate(black_box("F"), &rules, gen))
        });
    }
    group.finish();
}

fn bench_van_der_pol(c: &mut Criterion) {
    c.bench_function("van_der_pol_1000", |b| {
        b.iter(|| {
            (0..1000).fold((1.0_f32, 0.0_f32), |(x, v), _| {
                van_der_pol_step(black_box(x), v, 1.0, 0.01)
            })
        })
    });
}

fn bench_integrate_simpson(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrate_simpson");
    for n in [100, 1000, 10000] {
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| integrate_simpson(black_box(|x: f32| x.sin()), 0.0, std::f32::consts::PI, n))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_rk4_step,
    bench_lorenz_1000,
    bench_lsystem_generations,
    bench_van_der_pol,
    bench_integrate_simpson,
);
criterion_main!(benches);
