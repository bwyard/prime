use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_random::*;

fn bench_prng_next(c: &mut Criterion) {
    c.bench_function("prng_next", |b| {
        b.iter(|| {
            // Chain 1000 steps
            (0..1000u32).fold(42u32, |s, _| {
                let (_, next) = prng_next(black_box(s));
                next
            })
        })
    });
}

fn bench_prng_gaussian(c: &mut Criterion) {
    c.bench_function("prng_gaussian", |b| {
        b.iter(|| {
            (0..1000u32).fold(42u32, |s, _| {
                let (_, next) = prng_gaussian(black_box(s));
                next
            })
        })
    });
}

fn bench_poisson_disk_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("poisson_disk_2d");
    for size in [50.0f32, 100.0, 200.0] {
        group.bench_function(format!("{size}x{size}"), |b| {
            b.iter(|| poisson_disk_2d(black_box(42), size, size, 5.0, 30))
        });
    }
    group.finish();
}

fn bench_monte_carlo_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("monte_carlo_1d");
    for n in [100, 1_000, 10_000] {
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| monte_carlo_1d(black_box(42), |x| x.sin(), 0.0, std::f32::consts::PI, n))
        });
    }
    group.finish();
}

fn bench_van_der_corput(c: &mut Criterion) {
    c.bench_function("van_der_corput_1000", |b| {
        b.iter(|| {
            (0..1000u32).fold(0.0f32, |acc, i| acc + van_der_corput(black_box(i), 2))
        })
    });
}

fn bench_weighted_choice(c: &mut Criterion) {
    let mut group = c.benchmark_group("weighted_choice");
    for n in [10, 100, 1000] {
        let weights: Vec<f32> = (0..n).map(|i| (i + 1) as f32).collect();
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| weighted_choice(black_box(42), &weights))
        });
    }
    group.finish();
}

fn bench_halton_2d(c: &mut Criterion) {
    c.bench_function("halton_2d_1000", |b| {
        b.iter(|| {
            (0..1000u32).map(|i| halton_2d(black_box(i))).last()
        })
    });
}

criterion_group!(
    benches,
    bench_prng_next,
    bench_prng_gaussian,
    bench_poisson_disk_2d,
    bench_monte_carlo_convergence,
    bench_van_der_corput,
    bench_weighted_choice,
    bench_halton_2d,
);
criterion_main!(benches);
