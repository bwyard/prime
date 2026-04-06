//! Half-heart diagonal scatter-cull: Approach E.

use prime_random::prng_next;
use prime_voronoi::voronoi_partition;
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach E: Half-Heart Diagonal Scatter-Cull ─────────────────────────────
//
// Site generation strategy:
//   1. Place N seed points evenly spaced along a diagonal line through the domain.
//   2. Derive N "shifted" sites by adding (shift_x, shift_y) to each seed.
//   3. All 2N sites become Voronoi sites for scatter-cull.
//
// The diagonal + shift geometry produces elongated paired cells. Adjacent
// original/shifted site pairs create cells with a lobe that tilts toward the
// shift direction — the "half-heart" character. The exact cell shapes emerge
// from the Voronoi partition of the structured 2N sites.
//
// This is the first observational pass. Parameters are varied to see what cell
// geometries and coverage patterns emerge. Explicit Bézier arc boundaries are
// deferred until we understand the natural partition shapes from data.
//
// From the handoff: shift vector is "point at (4,4) moving diagonally to (-5,10)
// or something — play with the shift later, purely observational."
// We parameterise (shift_x, shift_y) and (diagonal_angle) here.

/// Generate 2*n_seeds Voronoi sites for the half-heart layout.
///
/// N seeds placed at equal intervals along a line through the domain at
/// `diagonal_angle`. Each seed is paired with a shifted copy at
/// `(seed.x + shift_x, seed.y + shift_y)`. Returned interleaved:
/// [s0, s0', s1, s1', ...] — pairs are adjacent in the slice.
///
/// # Math
///
/// Diagonal unit vector: $\hat{d} = (\cos\theta, \sin\theta)$
///
/// Seeds centred on the domain, spanning 80% of the domain diagonal:
/// $$s_i = \text{origin} + i \cdot \Delta \cdot \hat{d}$$
///
/// Shifted sites: $s'_i = s_i + (\text{shift\_x},\; \text{shift\_y})$
fn half_heart_sites(
    width: f32,
    height: f32,
    n_seeds: usize,
    diagonal_angle: f32,
    shift_x: f32,
    shift_y: f32,
) -> Vec<(f32, f32)> {
    if n_seeds == 0 { return vec![]; }

    let cos_a  = diagonal_angle.cos();
    let sin_a  = diagonal_angle.sin();

    // Centre the diagonal span on the domain centre, using 80% of the domain extent
    let origin_x = width  * 0.5 - cos_a * (width  * 0.4);
    let origin_y = height * 0.5 - sin_a * (height * 0.4);
    let diag_len = ((width * 0.8) * cos_a.abs() + (height * 0.8) * sin_a.abs()).max(1.0);
    let step     = if n_seeds > 1 { diag_len / (n_seeds - 1) as f32 } else { 0.0 };

    (0..n_seeds)
        .flat_map(|i| {
            let sx = origin_x + cos_a * (i as f32 * step);
            let sy = origin_y + sin_a * (i as f32 * step);
            [(sx, sy), (sx + shift_x, sy + shift_y)]
        })
        .collect()
}

/// Scatter-cull Poisson disk sampling over half-heart diagonal cells (Approach E).
///
/// Generates `2 * n_seeds` Voronoi sites using the half-heart layout: N seeds
/// evenly spaced along a diagonal, each paired with a shifted copy. The paired
/// site layout creates elongated lobe-shaped cells tilted in the shift direction.
///
/// This is the first observational pass. Parameters `diagonal_angle`,
/// `shift_x`, and `shift_y` are all varied experimentally to observe emergent
/// cell geometry and coverage characteristics. No claims are made about the
/// optimal shift — that is determined from data.
///
/// # Math
///
/// **Site layout:** see `half_heart_sites`.
///
/// **Scatter-cull:** identical to Approach D — scatter candidates uniformly
/// in the full domain, partition by nearest Voronoi site, cull to min-dist.
///
/// **Cell count:** `2 * n_seeds`. For n_seeds=5 → 10 cells.
///
/// # Arguments
/// * `width`, `height`    — domain dimensions (must be > 0)
/// * `min_dist`           — minimum allowed distance between any two accepted points
/// * `n_seeds`            — seeds along diagonal (total sites = `2 * n_seeds`)
/// * `diagonal_angle`     — angle of seed line in radians (π/4 = 45°, 0 = horizontal)
/// * `shift_x`, `shift_y` — shift vector applied to each seed to derive paired site
/// * `target_per_cell`    — approximate desired survivors per cell
/// * `overage_ratio`      — scatter multiplier (>1.0; 1.5 recommended)
/// * `seed`               — deterministic PRNG seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` of length `2 * n_seeds`, interleaved (original, shifted)
/// per seed pair. Points within each cell satisfy min-dist. Cross-cell min-dist
/// is NOT guaranteed.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_half_heart;
/// let cells = scatter_cull_half_heart(
///     100.0, 100.0, 5.0,
///     5, std::f32::consts::FRAC_PI_4, -9.0, 6.0,
///     20, 1.5, 42,
/// );
/// assert_eq!(cells.len(), 10);
/// assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
/// ```
pub fn scatter_cull_half_heart(
    width: f32,
    height: f32,
    min_dist: f32,
    n_seeds: usize,
    diagonal_angle: f32,
    shift_x: f32,
    shift_y: f32,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || n_seeds == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    let sites = half_heart_sites(width, height, n_seeds, diagonal_angle, shift_x, shift_y);
    let k     = sites.len(); // = 2 * n_seeds

    // Scatter candidates uniformly in the full domain
    let total_candidates  = k * target_per_cell * overage_ratio as usize;
    let scatter_seed      = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    let candidates: Vec<(f32, f32)> = (0..total_candidates)
        .scan(scatter_seed, |s, _| {
            let (xf, s1) = prng_next(*s);
            let (yf, s2) = prng_next(s1);
            *s = s2;
            Some((xf * width, yf * height))
        })
        .collect();

    // Partition by nearest site, cull each cell.
    // Parallel outer loop: cells cull independently; voronoi_partition produces
    // cells in site-index order (deterministic). Sort each cell's candidates
    // before culling to normalise order and ensure determinism across par_iter runs.
    voronoi_partition(&sites, &candidates)
        .into_par_iter()
        .map(|mut cell| {
            cell.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });
            cull_to_min_dist(cell, 0.0, 0.0, width, height, min_dist)
        })
        .collect()
}

/// Scatter-cull half-heart — single global-cull pass (Approach E global).
///
/// Generates 2*n_seeds structured diagonal sites, scatters candidates in the
/// full domain, then applies a single global min-dist cull with no cell boundaries.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_half_heart;
/// let pts = scatter_global_half_heart(
///     100.0, 100.0, 5.0, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 10, 2.0, 42
/// );
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_half_heart(
    width: f32,
    height: f32,
    min_dist: f32,
    n_seeds: usize,
    diagonal_angle: f32,
    shift_x: f32,
    shift_y: f32,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || n_seeds == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    // Build half-heart Voronoi sites (same as scatter_cull_half_heart)
    let sites = half_heart_sites(width, height, n_seeds, diagonal_angle, shift_x, shift_y);
    if sites.is_empty() {
        return Vec::new();
    }

    let k      = sites.len();
    let drop_n = (target_per_cell as f32 * overage_ratio) as usize;

    // Scatter uniformly across the full domain, one PRNG stream per site.
    // Site assignment is used only for seed mixing (each cell → different PRNG subsequence),
    // NOT as a bounding box.  This ensures uniform domain coverage — no gap zones between
    // site bounding boxes.  No per-cell cull: all candidates flatten to one global pass.
    //
    // Parallel outer loop: each cell's inner PRNG scan stays sequential.
    // Sort before global cull to restore determinism across par_iter runs.
    let mut candidates: Vec<(f32, f32)> = (0..k)
        .into_par_iter()
        .flat_map(|i| {
            let cell_seed = (seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;
            (0..drop_n).scan(cell_seed, move |s, _| {
                let (xf, s1) = prng_next(*s);
                let (yf, s2) = prng_next(s1);
                *s = s2;
                Some((xf * width, yf * height))
            }).collect::<Vec<_>>()
        })
        .collect();

    // Sort to normalise candidate order — restores determinism across par_iter runs.
    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── scatter_cull_half_heart (Approach E) ─────────────────────────────────

    #[test]
    fn scatter_cull_half_heart_cell_count() {
        let cells = scatter_cull_half_heart(
            100.0, 100.0, 5.0, 5,
            std::f32::consts::FRAC_PI_4, -9.0, 6.0,
            20, 1.5, 42,
        );
        // 5 seeds → 10 cells
        assert_eq!(cells.len(), 10);
    }

    #[test]
    fn scatter_cull_half_heart_invalid_inputs_return_empty() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        assert!(scatter_cull_half_heart(0.0,   100.0, 5.0, 5, pi4, -9.0, 6.0, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_half_heart(100.0, 100.0, 5.0, 0, pi4, -9.0, 6.0, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 0,  1.5, 42).is_empty());
        assert!(scatter_cull_half_heart(100.0, 100.0, 0.0, 5, pi4, -9.0, 6.0, 20, 1.5, 42).is_empty());
    }

    #[test]
    fn scatter_cull_half_heart_min_dist_holds() {
        let cells = scatter_cull_half_heart(
            100.0, 100.0, 5.0, 5,
            std::f32::consts::FRAC_PI_4, -9.0, 6.0,
            20, 1.5, 42,
        );
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let d  = (dx * dx + dy * dy).sqrt();
                    assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4} < {min_dist}");
                }
            }
        }
    }

    #[test]
    fn scatter_cull_half_heart_deterministic() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let a = scatter_cull_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 20, 1.5, 42);
        let b = scatter_cull_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 20, 1.5, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_cull_half_heart_produces_points() {
        let cells = scatter_cull_half_heart(
            100.0, 100.0, 5.0, 5,
            std::f32::consts::FRAC_PI_4, -9.0, 6.0,
            20, 1.5, 42,
        );
        let total: usize = cells.iter().map(|c| c.len()).sum();
        assert!(total > 0, "should produce at least some accepted points");
    }

    #[test]
    fn scatter_cull_half_heart_all_points_in_domain() {
        let cells = scatter_cull_half_heart(
            100.0, 100.0, 5.0, 5,
            std::f32::consts::FRAC_PI_4, -9.0, 6.0,
            20, 1.5, 42,
        );
        for cell in &cells {
            for &(x, y) in cell {
                assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
                assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
            }
        }
    }

    #[test]
    fn scatter_cull_half_heart_shift_changes_output() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let a = scatter_cull_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0,  6.0, 20, 1.5, 42);
        let b = scatter_cull_half_heart(100.0, 100.0, 5.0, 5, pi4, -15.0, 10.0, 20, 1.5, 42);
        // Different shift vectors should produce different cell structures
        assert_ne!(a, b);
    }

    // ── scatter_global_half_heart ─────────────────────────────────────────────

    #[test]
    fn scatter_global_half_heart_invalid_inputs_return_empty() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        assert!(scatter_global_half_heart(0.0,   100.0, 5.0, 5, pi4, -9.0, 6.0, 20, 2.0, 42).is_empty());
        assert!(scatter_global_half_heart(100.0, 100.0, 0.0, 5, pi4, -9.0, 6.0, 20, 2.0, 42).is_empty());
        assert!(scatter_global_half_heart(100.0, 100.0, 5.0, 0, pi4, -9.0, 6.0, 20, 2.0, 42).is_empty());
        assert!(scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 0,  2.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_half_heart_produces_points() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let pts = scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 30, 2.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_half_heart_min_dist_holds() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let pts = scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 30, 2.0, 42);
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
    fn scatter_global_half_heart_deterministic() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let a = scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 30, 2.0, 42);
        let b = scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 30, 2.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_global_half_heart_all_points_in_domain() {
        let pi4 = std::f32::consts::FRAC_PI_4;
        let pts = scatter_global_half_heart(100.0, 100.0, 5.0, 5, pi4, -9.0, 6.0, 30, 2.0, 42);
        for &(x, y) in &pts {
            assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
        }
    }
}
