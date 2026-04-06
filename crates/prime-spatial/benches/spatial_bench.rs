use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_spatial::{
    poisson_rect_partitioned, scatter_cull_rect,
    scatter_cull_voronoi, scatter_cull_voronoi_recursive,
    scatter_cull_sheared, scatter_cull_half_heart,
    scatter_global_rect, scatter_global_voronoi, scatter_global_half_heart,
    scatter_cull_sdf_ellipse, scatter_global_sdf_ellipse,
    scatter_cull_clipped_circle, scatter_global_clipped_circle,
    scatter_cull_triangles, scatter_global_triangles,
    global_cull_to_min_dist,
};
use prime_random::poisson_disk;

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

fn bench_total_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("total_pipeline");
    let pi4 = std::f32::consts::FRAC_PI_4;

    // Reference: serial Bridson — the standard we calibrate against.
    // Scatter-cull approaches + global_cull_to_min_dist must beat this
    // wall-clock time to justify their existence at each domain size.
    for domain in [100.0f32, 200.0, 500.0] {
        group.bench_function(format!("bridson/{domain}x{domain}"), |b| {
            b.iter(|| poisson_disk(black_box(domain), black_box(domain), 5.0, 30, 42))
        });

        // C-B + global cull: calibrated overage ~3.0
        group.bench_function(format!("c_b_global/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_rect(black_box(domain), black_box(domain), 5.0, 4, 4, 30, 3.0, 42);
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });

        // D + global cull: calibrated overage ~5.0
        group.bench_function(format!("d_global/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_voronoi(black_box(domain), black_box(domain), 5.0, 10, 3, 50, 5.0, 42);
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });

        // E + global cull: calibrated overage ~3.0
        group.bench_function(format!("e_global/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_half_heart(black_box(domain), black_box(domain), 5.0, 5, pi4, -15.0, 10.0, 50, 3.0, 42);
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });
    }

    group.finish();
}

fn bench_single_pass(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_pass_global_cull");
    let pi4 = std::f32::consts::FRAC_PI_4;

    // Single-pass global-cull variants vs Bridson — direct comparison.
    // These scatter using cell structure for seed organisation only, then
    // apply one global min-dist cull across ALL candidates.
    // Expected: higher survivor density than per-cell-cull variants (no seam dead zones).
    for domain in [100.0f32, 200.0, 500.0] {
        // Bridson reference — the target to beat
        group.bench_function(format!("bridson/{domain}x{domain}"), |b| {
            b.iter(|| prime_random::poisson_disk(black_box(domain), black_box(domain), 5.0, 30, 42))
        });

        // Global-rect: rectangular scatter, no per-cell cull
        group.bench_function(format!("global_rect/{domain}x{domain}"), |b| {
            b.iter(|| scatter_global_rect(black_box(domain), black_box(domain), 5.0, 4, 4, 30, 2.0, 42))
        });

        // Global-voronoi: K=10 scatter, no per-cell cull
        group.bench_function(format!("global_voronoi/{domain}x{domain}"), |b| {
            b.iter(|| scatter_global_voronoi(black_box(domain), black_box(domain), 5.0, 10, 3, 30, 2.0, 42))
        });

        // Global-half-heart: half-heart sites scatter, no per-cell cull
        group.bench_function(format!("global_half_heart/{domain}x{domain}"), |b| {
            b.iter(|| scatter_global_half_heart(black_box(domain), black_box(domain), 5.0, 5, pi4, -15.0, 10.0, 30, 2.0, 42))
        });

        // Two-pass C-B + global_cull for direct comparison
        group.bench_function(format!("two_pass_c_b/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_rect(black_box(domain), black_box(domain), 5.0, 4, 4, 30, 2.0, 42);
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });
    }

    group.finish();
}

fn bench_approach_g(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_g_sdf_ellipse");

    for domain in [100.0f32, 200.0, 500.0] {
        // Two-pass: circle cells (aspect=1.0), coverage=1.2
        group.bench_function(format!("two_pass_circle/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_sdf_ellipse(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.0, 1.2, 300, 5.0, 42,
                );
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });

        // Single-pass: circle cells, coverage=1.2
        group.bench_function(format!("single_pass_circle/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_sdf_ellipse(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.0, 1.2, 300, 5.0, 42,
                )
            })
        });

        // Single-pass: ellipse cells (aspect=2.0), coverage=1.2
        group.bench_function(format!("single_pass_ellipse/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_sdf_ellipse(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 2.0, 1.2, 300, 5.0, 42,
                )
            })
        });

        // Two-pass clipped-circle, coverage=1.2
        group.bench_function(format!("two_pass_clipped_circle/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_clipped_circle(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.2, 300, 5.0, 42,
                );
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });

        // Single-pass clipped-circle, coverage=1.2
        group.bench_function(format!("single_pass_clipped_circle/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_clipped_circle(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.2, 300, 5.0, 42,
                )
            })
        });

        // Bridson reference
        group.bench_function(format!("bridson/{domain}x{domain}"), |b| {
            b.iter(|| prime_random::poisson_disk(black_box(domain), black_box(domain), 5.0, 30, 42))
        });
    }

    group.finish();
}

fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_scatter");
    let pi4 = std::f32::consts::FRAC_PI_4;

    // Benchmark the parallelised scatter functions against the Bridson serial baseline.
    // These are the same function calls as the serial benches above — they now use
    // Rayon par_iter internally. The benchmark measures the parallel speedup.
    for domain in [100.0f32, 200.0, 500.0] {
        // Bridson serial baseline
        group.bench_function(format!("bridson/{domain}x{domain}"), |b| {
            b.iter(|| poisson_disk(black_box(domain), black_box(domain), 5.0, 30, 42))
        });

        // Parallel rect scatter-cull (Approach C-B)
        group.bench_function(format!("par_scatter_cull_rect/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_cull_rect(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 30, 1.5, 42,
                )
            })
        });

        // Parallel rect global scatter (Approach C-B global)
        group.bench_function(format!("par_scatter_global_rect/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_rect(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 30, 2.0, 42,
                )
            })
        });

        // Parallel voronoi scatter-cull (Approach D)
        group.bench_function(format!("par_scatter_cull_voronoi/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_cull_voronoi(
                    black_box(domain), black_box(domain),
                    5.0, 10, 3, 25, 1.5, 42,
                )
            })
        });

        // Parallel half-heart scatter-cull (Approach E)
        group.bench_function(format!("par_scatter_cull_half_heart/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_cull_half_heart(
                    black_box(domain), black_box(domain),
                    5.0, 5, pi4, -9.0, 6.0, 20, 1.5, 42,
                )
            })
        });

        // Parallel half-heart global scatter (Approach E global)
        group.bench_function(format!("par_scatter_global_half_heart/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_half_heart(
                    black_box(domain), black_box(domain),
                    5.0, 5, pi4, -15.0, 10.0, 30, 2.0, 42,
                )
            })
        });

        // Parallel sheared scatter-cull (Approach F)
        group.bench_function(format!("par_scatter_cull_sheared/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_cull_sheared(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 0.5, 20, 2.0, 42,
                )
            })
        });

        // Parallel SDF ellipse scatter-cull (Approach G two-pass)
        group.bench_function(format!("par_scatter_cull_sdf_ellipse/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_cull_sdf_ellipse(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42,
                )
            })
        });

        // Parallel SDF ellipse global scatter (Approach G one-pass)
        group.bench_function(format!("par_scatter_global_sdf_ellipse/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_sdf_ellipse(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42,
                )
            })
        });
    }

    group.finish();
}

fn bench_approach_h(c: &mut Criterion) {
    let mut group = c.benchmark_group("approach_h_triangles");

    // 4×4 grid → 32 triangles; jitter=0.2 for irregular tiling
    for domain in [100.0f32, 200.0, 500.0] {
        // Two-pass: per-cell cull then global cull
        group.bench_function(format!("two_pass/{domain}x{domain}"), |b| {
            b.iter(|| {
                let cells = scatter_cull_triangles(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 0.2, 300, 5.0, 42,
                );
                let flat: Vec<_> = cells.into_iter().flatten().collect();
                global_cull_to_min_dist(flat, domain, domain, 5.0)
            })
        });

        // Single-pass: scatter only, one global cull
        group.bench_function(format!("single_pass/{domain}x{domain}"), |b| {
            b.iter(|| {
                scatter_global_triangles(
                    black_box(domain), black_box(domain),
                    5.0, 4, 4, 0.2, 300, 5.0, 42,
                )
            })
        });

        // Bridson reference
        group.bench_function(format!("bridson/{domain}x{domain}"), |b| {
            b.iter(|| prime_random::poisson_disk(black_box(domain), black_box(domain), 5.0, 30, 42))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_approach_c, bench_approach_d, bench_approach_d_recursive, bench_approach_f, bench_approach_e, bench_total_pipeline, bench_single_pass, bench_approach_g, bench_parallel, bench_approach_h);
criterion_main!(benches);
