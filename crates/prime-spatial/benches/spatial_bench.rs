use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use prime_spatial::{poisson_rect_partitioned, scatter_cull_rect};
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

criterion_group!(benches, bench_approach_c, bench_wei);
criterion_main!(benches);
