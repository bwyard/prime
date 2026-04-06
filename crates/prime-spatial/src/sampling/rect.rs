//! Rectangular partition scatter-cull: Approach C strategies A and B.

use prime_random::{poisson_disk, prng_next};
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach C: Rectangular Partitions ───────────────────────────────────────
//
// Two strategies over the same geometry, kept side by side for benchmarking:
//
//   poisson_rect_partitioned  — Strategy A: run Bridson inside each partition
//   scatter_cull_rect         — Strategy B: scatter-cull inside each partition
//
// Both return Vec<Vec<(f32, f32)>> — one inner Vec per partition.
// Points within each partition satisfy min_dist from each other.
// Seam handling differs per strategy (see each fn).

/// Approach C — Strategy A: Rectangular partitions with Bridson per cell.
///
/// # Math
///
/// Divides the domain into `partition_cols × partition_rows` equal rectangles.
/// Runs `poisson_disk` independently inside each cell with a seam inset of
/// $\frac{d_{min}}{2}$ on shared edges, so no cross-partition distance checks
/// are needed. Per-partition seed:
///
/// $$s_{r,c} = s_0 \cdot 6364136223846793005 + (r \cdot P_c + c)$$
///
/// # Arguments
/// * `width`, `height`       — domain dimensions (must be > 0)
/// * `min_dist`              — minimum distance between any two points
/// * `max_attempts`          — Bridson candidate attempts per active point (30 standard)
/// * `partition_cols`        — number of columns in the rectangular grid
/// * `partition_rows`        — number of rows in the rectangular grid
/// * `seed`                  — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per partition (row-major order).
/// Points are in world coordinates. Empty Vec on invalid inputs.
///
/// # Edge cases
/// Returns empty if any dimension or `min_dist` is ≤ 0, or if partition count is 0.
///
/// # Example
/// ```rust
/// # use prime_spatial::poisson_rect_partitioned;
/// let partitions = poisson_rect_partitioned(100.0, 100.0, 5.0, 30, 4, 4, 42);
/// assert_eq!(partitions.len(), 16);
/// assert!(partitions.iter().all(|p| !p.is_empty()));
/// ```
pub fn poisson_rect_partitioned(
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
    partition_cols: usize,
    partition_rows: usize,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || partition_cols == 0 || partition_rows == 0
    {
        return Vec::new();
    }

    let cell_w = width  / partition_cols as f32;
    let cell_h = height / partition_rows as f32;
    let inset  = min_dist / 2.0;

    (0..partition_rows).flat_map(|row| {
        (0..partition_cols).map(move |col| {
            // Per-partition seed — wrapping_mul mix avoids correlation
            let p_seed = (seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((row * partition_cols + col) as u64)
                as u32;

            let x_start = col as f32 * cell_w + inset;
            let y_start = row as f32 * cell_h + inset;
            let p_width  = (cell_w - inset * 2.0).max(0.0);
            let p_height = (cell_h - inset * 2.0).max(0.0);

            if p_width <= min_dist || p_height <= min_dist {
                return Vec::new();
            }

            // Run Bridson in local coordinates, translate to world
            poisson_disk(p_width, p_height, min_dist, max_attempts, p_seed)
                .into_iter()
                .map(|(x, y)| (x + x_start, y + y_start))
                .collect()
        })
    })
    .collect()
}

/// Approach C — Strategy B: Rectangular partitions with scatter-cull per cell.
///
/// # Math
///
/// Divides the domain into `partition_cols × partition_rows` equal rectangles.
/// Each cell receives a pure random drop of $N = \lfloor target \cdot r \rfloor$ points
/// where $r$ is the overage ratio. A cull pass then removes points violating $d_{min}$.
/// No seam inset — boundary conflicts are resolved from surplus during the cull pass.
///
/// Per-partition seed uses the same mixing strategy as `poisson_rect_partitioned`.
///
/// # Arguments
/// * `width`, `height`          — domain dimensions (must be > 0)
/// * `min_dist`                 — minimum distance between any two points
/// * `partition_cols`           — number of columns
/// * `partition_rows`           — number of rows
/// * `target_per_partition`     — desired survivors per partition
/// * `overage_ratio`            — drop multiplier, e.g. 1.5 for 50% overage
/// * `seed`                     — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per partition (row-major order).
/// Points are in world coordinates. Empty Vec on invalid inputs.
///
/// # Edge cases
/// Returns empty if any dimension, `min_dist`, or `overage_ratio` is ≤ 0,
/// or if partition count or target is 0.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_rect;
/// let partitions = scatter_cull_rect(100.0, 100.0, 5.0, 4, 4, 250, 1.5, 42);
/// assert_eq!(partitions.len(), 16);
/// ```
pub fn scatter_cull_rect(
    width: f32,
    height: f32,
    min_dist: f32,
    partition_cols: usize,
    partition_rows: usize,
    target_per_partition: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || partition_cols == 0 || partition_rows == 0 || target_per_partition == 0
    {
        return Vec::new();
    }

    let cell_w   = width  / partition_cols as f32;
    let cell_h   = height / partition_rows as f32;
    let drop_n   = (target_per_partition as f32 * overage_ratio) as usize;
    let n_cells  = partition_rows * partition_cols;

    // Phase 1 — Scatter (parallel outer loop): each cell generates its candidates
    // independently; inner PRNG scan remains sequential within each cell.
    // Candidates are sorted by (x.to_bits(), y.to_bits()) after collection to restore
    // determinism — par_iter does not guarantee ordering across threads.
    (0..n_cells).into_par_iter().map(|i| {
        let row = i / partition_cols;
        let col = i % partition_cols;
        let p_seed = (seed as u64)
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add((row * partition_cols + col) as u64)
            as u32;

        let x_start = col as f32 * cell_w;
        let y_start = row as f32 * cell_h;

        // Phase 1 — Scatter: pure random drop, no distance checking.
        // scan threads PRNG state forward — the canonical pure pattern for
        // generating a sequence of seeded values.
        let mut candidates: Vec<(f32, f32)> = (0..drop_n)
            .scan(p_seed, |s, _| {
                let (xf, s1) = prng_next(*s);
                let (yf, s2) = prng_next(s1);
                *s = s2;
                Some((x_start + xf * cell_w, y_start + yf * cell_h))
            })
            .collect();

        // Sort to normalise candidate order before cull — restores determinism
        // across par_iter runs (same seed → same candidates → same sorted order → same cull).
        candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
            ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
        });

        // Phase 2 — Cull: apply min-dist constraint as post-pass
        cull_to_min_dist(candidates, x_start, y_start, cell_w, cell_h, min_dist)
    })
    .collect()
}

// ── Single-pass global-cull variants ─────────────────────────────────────────
//
// The two-phase pipeline (per-cell cull → global cull) imposes a structural
// density ceiling: the per-cell cull prematurely eliminates candidates near
// seams, leaving dead zones that the global cull cannot fill.
//
// The single-pass variants skip the per-cell cull entirely:
//   1. Scatter candidates using the cell structure (organisational only)
//   2. Flatten — cell boundaries are removed
//   3. Run one global min-dist cull across ALL candidates
//
// This lets candidates from adjacent cells compete at seam regions, allowing
// the cull to select the globally best subset. Output is a flat Vec<(f32,f32)>
// with global min-dist guaranteed — equivalent to serial Bridson's output type.

/// Scatter-cull rectangular — single global-cull pass (Approach C-B global).
///
/// Scatters candidates using the rectangular cell structure for seed mixing,
/// then flattens and applies a single global min-dist cull. No per-cell cull.
///
/// This eliminates the structural density ceiling caused by per-cell culling:
/// candidates from both sides of a seam compete in the same cull pass,
/// allowing near-seam regions to fill as densely as the interior.
///
/// # Math
///
/// Total candidates scattered: $N = P_c \times P_r \times \text{target} \times \text{overage}$
///
/// Single global cull: same O(1)-per-point acceptance grid as `cull_to_min_dist`,
/// applied once over the full domain with all $N$ candidates.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_rect;
/// let pts = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
/// // flat output — no cell structure
/// for i in 0..pts.len() {
///     for j in (i+1)..pts.len() {
///         let dx = pts[i].0 - pts[j].0;
///         let dy = pts[i].1 - pts[j].1;
///         assert!((dx*dx+dy*dy).sqrt() >= 5.0 - 1e-4);
///     }
/// }
/// ```
pub fn scatter_global_rect(
    width: f32,
    height: f32,
    min_dist: f32,
    partition_cols: usize,
    partition_rows: usize,
    target_per_partition: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || partition_cols == 0 || partition_rows == 0 || target_per_partition == 0
    {
        return Vec::new();
    }

    let cell_w = width  / partition_cols as f32;
    let cell_h = height / partition_rows as f32;
    let drop_n = (target_per_partition as f32 * overage_ratio) as usize;

    // Scatter phase (parallel outer loop) — same seed mixing as scatter_cull_rect, no cull.
    // Each cell's inner PRNG scan stays sequential. Candidates are collected per cell,
    // then flattened. Sort before global cull to restore determinism across par_iter runs.
    let mut candidates: Vec<(f32, f32)> = (0..(partition_rows * partition_cols))
        .into_par_iter()
        .flat_map(|i| {
            let row     = i / partition_cols;
            let col     = i % partition_cols;
            let p_seed  = (seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;
            let x_start = col as f32 * cell_w;
            let y_start = row as f32 * cell_h;
            (0..drop_n).scan(p_seed, move |s, _| {
                let (xf, s1) = prng_next(*s);
                let (yf, s2) = prng_next(s1);
                *s = s2;
                Some((x_start + xf * cell_w, y_start + yf * cell_h))
            }).collect::<Vec<_>>()
        })
        .collect();

    // Sort to normalise candidate order — restores determinism across par_iter runs.
    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    // Single global cull — no cell boundaries
    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatter_global_rect_invalid_inputs_return_empty() {
        assert!(scatter_global_rect(0.0,   100.0, 5.0, 4, 4, 20, 2.0, 42).is_empty());
        assert!(scatter_global_rect(100.0, 100.0, 0.0, 4, 4, 20, 2.0, 42).is_empty());
        assert!(scatter_global_rect(100.0, 100.0, 5.0, 0, 4, 20, 2.0, 42).is_empty());
        assert!(scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 0,  2.0, 42).is_empty());
        assert!(scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 20, 0.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_rect_produces_points() {
        let pts = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_rect_min_dist_holds() {
        let pts = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
        let min_dist = 5.0_f32;
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let dx = pts[i].0 - pts[j].0;
                let dy = pts[i].1 - pts[j].1;
                let d  = (dx * dx + dy * dy).sqrt();
                assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4} < {min_dist}");
            }
        }
    }

    #[test]
    fn scatter_global_rect_deterministic() {
        let a = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
        let b = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_global_rect_all_points_in_domain() {
        let pts = scatter_global_rect(100.0, 100.0, 5.0, 4, 4, 30, 2.0, 42);
        for &(x, y) in &pts {
            assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
        }
    }
}
