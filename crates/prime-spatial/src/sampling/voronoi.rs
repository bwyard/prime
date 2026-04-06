//! Voronoi scatter-cull: Approach D (single-level) and D-R (recursive).

use prime_random::prng_next;
use prime_voronoi::{voronoi_sites_seeded, voronoi_partition, lloyd_relax_n};
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach D: Voronoi K₁₀ Scatter-Cull ─────────────────────────────────────
//
// Strategy B (scatter-cull) over irregular Voronoi cells.
//
// Steps:
//   1. Generate K random Voronoi sites in the domain.
//   2. Lloyd-relax the sites for `lloyd_iters` steps → centroidal placement.
//   3. Scatter `k * target_per_cell * overage_ratio` random candidates in the domain.
//   4. Partition candidates by nearest site → one candidate set per Voronoi cell.
//   5. Cull each cell's candidates to min-dist validity.
//
// No Bridson reference for Approach D in this file — partition-Bridson over
// Voronoi cells is structurally identical to approach C-A (run Bridson per
// cell) and adds no new geometric information. The interesting comparison is
// scatter-cull Voronoi (D) vs scatter-cull rectangular (C-B).

/// Scatter-cull Poisson disk sampling over K Voronoi cells (Approach D).
///
/// Generates `k` Voronoi sites, Lloyd-relaxes them for `lloyd_iters` steps,
/// then applies scatter-cull independently inside each Voronoi cell.
///
/// Unlike rectangular partitions, Voronoi cells are irregular polygons — their
/// boundaries are implicitly defined by nearest-site distance. No explicit
/// cell polygon computation is required: a candidate point belongs to the cell
/// whose site is nearest.
///
/// # Math
///
/// **Site placement:**
/// Random sites $\{s_i\}_{i=0}^{k-1}$ drawn from the domain, then moved toward
/// centroidal positions via sample-based Lloyd relaxation:
///
/// $$s_i^{(t+1)} = \frac{1}{|\mathcal{C}_i^{(t)}|} \sum_{p \in \mathcal{C}_i^{(t)}} p$$
///
/// where $\mathcal{C}_i^{(t)}$ is the set of grid samples nearest to $s_i$ at step $t$.
///
/// **Scatter:**
/// Total candidates $N = k \cdot \text{target} \cdot \text{overage}$ scattered uniformly
/// in the full domain:
///
/// $$N = k \cdot \text{target\_per\_cell} \cdot \text{overage\_ratio}$$
///
/// **Partition:**
/// $$\text{cell}_i = \bigl\{\, p \in \text{candidates} : \arg\min_j \lVert p - s_j \rVert = i \,\bigr\}$$
///
/// **Cull:** each cell's candidates filtered to min-dist validity using the
/// same O(1)-grid cull as `scatter_cull_rect`.
///
/// # Arguments
/// * `width`, `height`      — domain dimensions (must be > 0)
/// * `min_dist`             — minimum allowed distance between any two accepted points
/// * `k`                    — number of Voronoi sites (partitions)
/// * `lloyd_iters`          — Lloyd relaxation steps (0 = use raw random sites)
/// * `target_per_cell`      — approximate desired survivors per Voronoi cell
/// * `overage_ratio`        — scatter multiplier (>1.0 ensures surplus; 1.5 recommended)
/// * `seed`                 — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per Voronoi cell (length = `k`).
/// Points within each cell satisfy min-dist. Cross-cell min-dist is NOT
/// guaranteed — boundary conflicts exist at cell seams (same as Approach C).
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_voronoi;
/// let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
/// assert_eq!(cells.len(), 10);
/// assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
/// ```
pub fn scatter_cull_voronoi(
    width: f32,
    height: f32,
    min_dist: f32,
    k: usize,
    lloyd_iters: usize,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || k == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    // Step 1 — Generate K random sites in the domain.
    let raw_sites = voronoi_sites_seeded(k, 0.0, 0.0, width, height, seed);

    // Step 2 — Lloyd-relax sites toward centroidal positions.
    // Sample the domain with a regular grid for centroid estimation.
    // Grid density: sqrt(k) * 4 samples per axis — enough for accurate centroids
    // without excessive allocation.
    let lloyd_sites = if lloyd_iters == 0 {
        raw_sites
    } else {
        let grid_n    = ((k as f32).sqrt() as usize * 4).max(10);
        let step_x    = width  / grid_n as f32;
        let step_y    = height / grid_n as f32;
        let samples: Vec<(f32, f32)> = (0..grid_n)
            .flat_map(|row| {
                (0..grid_n).map(move |col| {
                    (col as f32 * step_x + step_x * 0.5,
                     row as f32 * step_y + step_y * 0.5)
                })
            })
            .collect();
        lloyd_relax_n(&raw_sites, &samples, lloyd_iters)
    };

    // Step 3 — Scatter candidates uniformly across the full domain.
    // Seed for scatter phase derived from the input seed, different from site seed.
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let total_candidates = k * target_per_cell * overage_ratio as usize;

    let candidates: Vec<(f32, f32)> = (0..total_candidates)
        .scan(scatter_seed, |s, _| {
            let (xf, s1) = prng_next(*s);
            let (yf, s2) = prng_next(s1);
            *s = s2;
            Some((xf * width, yf * height))
        })
        .collect();

    // Step 4 — Partition candidates by nearest Voronoi site.
    let partitioned = voronoi_partition(&lloyd_sites, &candidates);

    // Step 5 — Cull each Voronoi cell's candidates to min-dist validity.
    // Voronoi cells are irregular — use a conservative bounding box for the
    // cull grid (the full domain) since we don't know exact cell extents.
    // The cull grid still achieves O(1) per candidate — cell bounding boxes
    // are an optimisation left for when performance data motivates it.
    //
    // Parallel outer loop: each cell culls independently. voronoi_partition
    // returns cells in site-index order (deterministic), and cull_to_min_dist
    // is deterministic given fixed input, so output order is preserved.
    // Each cell's candidates are sorted before culling to normalise candidate
    // order — consistent with the sort applied in the rect scatter phase.
    partitioned
        .into_par_iter()
        .map(|mut cell_candidates| {
            cell_candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });
            cull_to_min_dist(cell_candidates, 0.0, 0.0, width, height, min_dist)
        })
        .collect()
}

// ── Approach D-R: Voronoi K₁₀ Recursive Scatter-Cull ────────────────────────
//
// Same goal as scatter_cull_voronoi (Approach D) but the partition is built
// recursively: each Voronoi cell is itself split into K sub-cells, down to
// `levels` depth.
//
// k^levels leaf cells are produced. total_target points are spread across all
// leaf cells: target_per_leaf = total_target / k^levels.
//
// The scatter is done once, up front, in the full domain. Candidates are
// partitioned level by level. Culling happens only at the leaves.
//
// Sub-level sites are generated within the bounding box of the candidates in
// each cell; Lloyd-relaxation uses those same candidates as samples.

fn voronoi_recursive_inner(
    candidates: Vec<(f32, f32)>,
    k: usize,
    levels_remaining: usize,
    lloyd_iters: usize,
    min_dist: f32,
    full_width: f32,
    full_height: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    // Leaf: cull and return
    if levels_remaining == 0 || candidates.len() < 2 {
        let culled = cull_to_min_dist(candidates, 0.0, 0.0, full_width, full_height, min_dist);
        return vec![culled];
    }

    // Bounding box of this cell's candidates
    let (min_x, min_y, max_x, max_y) = candidates.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(mnx, mny, mxx, mxy), &(x, y)| {
            (mnx.min(x), mny.min(y), mxx.max(x), mxy.max(y))
        },
    );
    let cell_w = (max_x - min_x).max(1e-6);
    let cell_h = (max_y - min_y).max(1e-6);

    // Generate K sub-sites within this cell's bounding box
    let raw_sites = voronoi_sites_seeded(k, min_x, min_y, cell_w, cell_h, seed);

    // Lloyd-relax using the candidates themselves as samples.
    // Candidates are approximately uniform within the cell, so this gives
    // reasonable centroidal placement without a separate sample grid.
    let sites = lloyd_relax_n(&raw_sites, &candidates, lloyd_iters);

    // Partition candidates by nearest sub-site
    let sub_cells = voronoi_partition(&sites, &candidates);

    // Recurse into each sub-cell with a derived seed
    sub_cells
        .into_iter()
        .enumerate()
        .flat_map(|(i, sub_candidates)| {
            let sub_seed = seed
                .wrapping_mul(1_664_525u32)
                .wrapping_add(i as u32);
            voronoi_recursive_inner(
                sub_candidates,
                k,
                levels_remaining - 1,
                lloyd_iters,
                min_dist,
                full_width,
                full_height,
                sub_seed,
            )
        })
        .collect()
}

/// Recursive scatter-cull Poisson disk sampling over K^levels Voronoi leaf cells
/// (Approach D-R).
///
/// Produces `total_target` points across `k^levels` leaf cells. The scatter
/// phase runs once in the full domain; partitioning recurses down `levels`
/// levels; culling applies only at the leaves.
///
/// This is a separate option from `scatter_cull_voronoi` (single-level D).
/// Both are kept for benchmarking — neither replaces the other until results
/// are in.
///
/// # Math
///
/// **Leaf count:**
/// $$\text{leaves} = k^{\text{levels}}$$
///
/// **Per-leaf target:**
/// $$\text{target\_per\_leaf} = \left\lceil \frac{\text{total\_target}}{\text{leaves}} \right\rceil$$
///
/// **Total candidates scattered:**
/// $$N = \text{total\_target} \cdot \text{overage\_ratio}$$
/// (distributed among leaves by partition, not pre-allocated per leaf)
///
/// **Recursive partition at each level:**
/// - Compute bounding box of this cell's candidates
/// - Generate K sub-sites within the bounding box
/// - Lloyd-relax sub-sites using candidates as samples
/// - Partition candidates by nearest sub-site
/// - Recurse at depth − 1
///
/// At leaf: apply min-dist cull.
///
/// # Arguments
/// * `width`, `height`   — domain dimensions (must be > 0)
/// * `min_dist`          — minimum allowed distance between any two accepted points
/// * `k`                 — Voronoi sites per level (10 → K₁₀)
/// * `levels`            — recursion depth (leaf count = k^levels)
/// * `lloyd_iters`       — Lloyd steps per level (0 = raw random sites)
/// * `total_target`      — desired total accepted points across all leaf cells
/// * `overage_ratio`     — scatter multiplier (>1.0; 1.5 recommended)
/// * `seed`              — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` of length up to `k^levels` (some leaves may be empty
/// if the domain is small relative to min-dist). Points within each leaf satisfy
/// min-dist. Cross-leaf min-dist is NOT guaranteed.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_voronoi_recursive;
/// // K=10, levels=2 → 100 leaf cells, 1000 total target points
/// let cells = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42);
/// assert!(cells.len() > 0);
/// assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
/// ```
pub fn scatter_cull_voronoi_recursive(
    width: f32,
    height: f32,
    min_dist: f32,
    k: usize,
    levels: usize,
    lloyd_iters: usize,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || k == 0 || levels == 0 || total_target == 0
    {
        return Vec::new();
    }

    // Scatter all candidates in the full domain up front.
    // overage covers expected cull losses; candidates are distributed to leaves by partition.
    let total_candidates = (total_target as f32 * overage_ratio) as usize;
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    let candidates: Vec<(f32, f32)> = (0..total_candidates)
        .scan(scatter_seed, |s, _| {
            let (xf, s1) = prng_next(*s);
            let (yf, s2) = prng_next(s1);
            *s = s2;
            Some((xf * width, yf * height))
        })
        .collect();

    voronoi_recursive_inner(candidates, k, levels, lloyd_iters, min_dist, width, height, seed)
}

/// Scatter-cull Voronoi — single global-cull pass (Approach D global).
///
/// Generates K Lloyd-relaxed Voronoi sites, scatters candidates in the full
/// domain, partitions by nearest site for scatter organisation, then flattens
/// and applies a single global min-dist cull. No per-cell cull.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_voronoi;
/// let pts = scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 2.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_voronoi(
    width: f32,
    height: f32,
    min_dist: f32,
    k: usize,
    lloyd_iters: usize,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || k == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    // Scatter phase reuses scatter_cull_voronoi's candidate generation without cull.
    // Flatten the per-cell scatter from scatter_cull_voronoi but skip its cull:
    // we replicate the scatter here to avoid the per-cell cull.
    let raw_sites = voronoi_sites_seeded(k, 0.0, 0.0, width, height, seed);
    let lloyd_sites = if lloyd_iters == 0 {
        raw_sites
    } else {
        let grid_n  = ((k as f32).sqrt() as usize * 4).max(10);
        let step_x  = width  / grid_n as f32;
        let step_y  = height / grid_n as f32;
        let samples: Vec<(f32, f32)> = (0..grid_n)
            .flat_map(|row| {
                (0..grid_n).map(move |col| (
                    col as f32 * step_x + step_x * 0.5,
                    row as f32 * step_y + step_y * 0.5,
                ))
            })
            .collect();
        lloyd_relax_n(&raw_sites, &samples, lloyd_iters)
    };

    let scatter_seed     = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let total_candidates = k * target_per_cell * overage_ratio as usize;

    let candidates: Vec<(f32, f32)> = (0..total_candidates)
        .scan(scatter_seed, |s, _| {
            let (xf, s1) = prng_next(*s);
            let (yf, s2) = prng_next(s1);
            *s = s2;
            Some((xf * width, yf * height))
        })
        .collect();

    // Partition for documentation / future per-cell stats, then flatten for single cull
    let _ = lloyd_sites; // sites used above; partition step skipped — scatter is global
    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── scatter_cull_voronoi ─────────────────────────────────────────────────

    #[test]
    fn scatter_cull_voronoi_cell_count() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        assert_eq!(cells.len(), 10, "should return exactly k cells");
    }

    #[test]
    fn scatter_cull_voronoi_invalid_inputs_return_empty() {
        assert!(scatter_cull_voronoi(0.0, 100.0, 5.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 0.0, 5.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 0.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 5.0, 0, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 0, 1.5, 42).is_empty());
    }

    #[test]
    fn scatter_cull_voronoi_min_dist_holds() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    assert!(
                        dist >= min_dist - 1e-4,
                        "min-dist violated within cell: {dist:.4} < {min_dist}"
                    );
                }
            }
        }
    }

    #[test]
    fn scatter_cull_voronoi_deterministic() {
        let a = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let b = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        assert_eq!(a, b, "same seed must produce identical output");
    }

    #[test]
    fn scatter_cull_voronoi_different_seeds_differ() {
        let a = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let b = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 99);
        // Different seeds should almost certainly produce different outputs
        let total_a: usize = a.iter().map(|c| c.len()).sum();
        let total_b: usize = b.iter().map(|c| c.len()).sum();
        // At least point counts differ or positions differ
        assert!(total_a != total_b || a != b);
    }

    #[test]
    fn scatter_cull_voronoi_zero_lloyd_iters() {
        // Should work fine with no relaxation — sites are raw random
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 0, 20, 1.5, 42);
        assert_eq!(cells.len(), 10);
    }

    #[test]
    fn scatter_cull_voronoi_coverage_nonzero() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 1.5, 42);
        let flat: Vec<(f32, f32)> = cells.into_iter().flatten().collect();
        assert!(!flat.is_empty(), "should produce at least some points");
    }

    // ── scatter_cull_voronoi_recursive ──────────────────────────────────────

    #[test]
    fn scatter_cull_voronoi_recursive_basic() {
        // K=10, levels=2 → up to 100 leaf cells
        let cells = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42);
        assert!(!cells.is_empty());
        assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
    }

    #[test]
    fn scatter_cull_voronoi_recursive_invalid_inputs_return_empty() {
        assert!(scatter_cull_voronoi_recursive(0.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 0, 2, 3, 200, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 0, 3, 200, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 0, 1.5, 42).is_empty());
    }

    #[test]
    fn scatter_cull_voronoi_recursive_min_dist_holds() {
        let cells = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    assert!(
                        dist >= min_dist - 1e-4,
                        "min-dist violated: {dist:.4} < {min_dist}"
                    );
                }
            }
        }
    }

    #[test]
    fn scatter_cull_voronoi_recursive_deterministic() {
        let a = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42);
        let b = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 2, 3, 200, 1.5, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_cull_voronoi_recursive_levels_1_matches_structure_of_d() {
        // levels=1 should produce exactly K leaf cells (same structure as single-level D)
        let cells = scatter_cull_voronoi_recursive(100.0, 100.0, 5.0, 10, 1, 3, 200, 1.5, 42);
        assert_eq!(cells.len(), 10);
    }

    // ── scatter_global_voronoi ────────────────────────────────────────────────

    #[test]
    fn scatter_global_voronoi_invalid_inputs_return_empty() {
        assert!(scatter_global_voronoi(0.0,   100.0, 5.0, 10, 3, 30, 2.0, 42).is_empty());
        assert!(scatter_global_voronoi(100.0, 100.0, 0.0, 10, 3, 30, 2.0, 42).is_empty());
        assert!(scatter_global_voronoi(100.0, 100.0, 5.0, 0,  3, 30, 2.0, 42).is_empty());
        assert!(scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 0,  2.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_voronoi_produces_points() {
        let pts = scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 2.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_voronoi_min_dist_holds() {
        let pts = scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 2.0, 42);
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
    fn scatter_global_voronoi_deterministic() {
        let a = scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 2.0, 42);
        let b = scatter_global_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 2.0, 42);
        assert_eq!(a, b);
    }
}
