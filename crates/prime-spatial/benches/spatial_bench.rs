use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_spatial::{
    poisson_rect_partitioned, scatter_cull_rect,
    scatter_cull_voronoi, scatter_cull_voronoi_recursive,
    scatter_cull_sheared, scatter_cull_half_heart,
};
use prime_spatial::research::poisson_disk_wei;

// Research benchmark: Approach C — rectangular partitions
// Strategy A (partition-Bridson) vs Strategy B (scatter-cull) vs serial Bridson baseline
// Parameters from handoff v2: 40 partitions, ~250 survivors/partition, overage_ratio=1.5

fn bench_approach_c(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_c_rect");

    for (domain, cols, rows) in [(100.0f32, 4, 4), (200.0, 6, 6), (500.0, 8, 8)] {
        let label = format!("{domain}x{domain}_{}x{}", cols, rows);

        // Strategy A — partition-Bridson
        group.bench_function(format!("partition_bridson/{label}"), |b| {
            b.iter(|| {
                poisson_rect_partitioned(
                    black_box(domain), black_box(domain),
                    5.0, 30, cols, rows, 42,
                )
            })
        });

        // Strategy B — scatter-cull
        // target_per_partition sized to match expected Bridson output per cell
        let target = 250 / (cols * rows).max(1);
        group.bench_function(format!("scatter_cull/{label}"), |b| {
            b.iter(|| {
                scatter_cull_rect(
                    black_box(domain), black_box(domain),
                    5.0, cols, rows, target.max(20), 1.5, 42,
                )
            })
        });
    }

    group.finish();
}

fn bench_wei(c: &mut Criterion) {
    let mut group = c.benchmark_group("wei_2008");
    for domain in [100.0f32, 200.0, 500.0] {
        group.bench_function(format!("{domain}x{domain}"), |b| {
            b.iter(|| poisson_disk_wei(black_box(domain), black_box(domain), 5.0, 30, 42))
        });
    }
    group.finish();
}

fn bench_approach_d(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_d_voronoi");

    // K=10 sites, 3 Lloyd iterations, overage_ratio=1.5
    // target_per_cell scaled to match C-B target counts for fair comparison
    for (domain, k) in [(100.0f32, 10), (200.0, 10), (500.0, 10)] {
        let label    = format!("{domain}x{domain}_k{k}");
        let target   = 25usize; // ~same total as C-B at equivalent domain sizes

        group.bench_function(format!("scatter_cull_voronoi/{label}"), |b| {
            b.iter(|| {
                scatter_cull_voronoi(
                    black_box(domain), black_box(domain),
                    5.0, k, 3, target, 1.5, 42,
                )
            })
        });
    }

    group.finish();
}

fn bench_approach_d_recursive(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_d_recursive");

    // K=10, levels=1 → 10 leaf cells; levels=2 → 100 leaf cells
    // total_target=200 → ~20 per leaf (levels=1) or ~2 per leaf (levels=2)
    // Kept small so benchmark completes quickly and results are comparable
    for (domain, levels, total) in [
        (100.0f32, 1usize, 200usize),
        (100.0,    2,      200),
        (200.0,    1,      500),
        (200.0,    2,      500),
        (500.0,    1,      1000),
        (500.0,    2,      1000),
    ] {
        let label = format!("{domain}x{domain}_L{levels}");
        group.bench_function(format!("scatter_cull_recursive/{label}"), |b| {
            b.iter(|| {
                scatter_cull_voronoi_recursive(
                    black_box(domain), black_box(domain),
                    5.0, 10, levels, 3, total, 1.5, 42,
                )
            })
        });
    }

    group.finish();
}

fn bench_approach_f(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_f_sheared");

    // shear_factor=0.5 (brick pattern) vs shear_factor=0.0 (variable-rect, no shear)
    // Same partition count and target as C-B for direct comparison
    for (domain, cols, rows) in [(100.0f32, 4, 4), (200.0, 6, 6), (500.0, 8, 8)] {
        let label  = format!("{domain}x{domain}_{}x{}", cols, rows);
        let target = 250 / (cols * rows).max(1);

        group.bench_function(format!("shear_0.5/{label}"), |b| {
            b.iter(|| {
                scatter_cull_sheared(
                    black_box(domain), black_box(domain),
                    5.0, cols, rows, 0.5, target.max(10), 2.0, 42,
                )
            })
        });

        group.bench_function(format!("variable_rect/{label}"), |b| {
            b.iter(|| {
                scatter_cull_sheared(
                    black_box(domain), black_box(domain),
                    5.0, cols, rows, 0.0, target.max(10), 2.0, 42,
                )
            })
        });
    }

    group.finish();
}

fn bench_approach_e(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_e_half_heart");

    // Observational: vary shift vector and n_seeds to see cell geometry effects.
    // shift(-9, 6) — from handoff example; shift(-15, 10) — steeper diagonal shift
    let pi4 = std::f32::consts::FRAC_PI_4;

    for (domain, n_seeds) in [(100.0f32, 5usize), (200.0, 10), (500.0, 20)] {
        let target = 20usize;

        group.bench_function(format!("shift_d9_6/{domain}x{domain}_n{n_seeds}"), |b| {
            b.iter(|| {
                scatter_cull_half_heart(
                    black_box(domain), black_box(domain),
                    5.0, n_seeds, pi4, -9.0, 6.0, target, 1.5, 42,
                )
            })
        });

        group.bench_function(format!("shift_d15_10/{domain}x{domain}_n{n_seeds}"), |b| {
            b.iter(|| {
                scatter_cull_half_heart(
                    black_box(domain), black_box(domain),
                    5.0, n_seeds, pi4, -15.0, 10.0, target, 1.5, 42,
                )
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_approach_c, bench_approach_d, bench_approach_d_recursive, bench_approach_f, bench_approach_e, bench_wei);
criterion_main!(benches);
