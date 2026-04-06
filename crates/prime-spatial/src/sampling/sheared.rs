//! Sheared variable-size scatter-cull: Approach F.

use prime_random::prng_next;
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach F: Sheared Variable-Size Scatter-Cull ───────────────────────────
//
// Extends Approach C (rectangular scatter-cull) in two ways:
//
//   F-1: Non-equal cell sizes — column widths and row heights are drawn from a
//        seeded PRNG and normalised to sum to the domain dimensions. This breaks
//        the periodic vertical seam alignment present in Approach C.
//
//   F-2: Row shear — each row r is shifted right by r * shear_x, where
//        shear_x = shear_factor * mean_cell_width. Cells are parallelograms, not
//        rectangles. The top edge is horizontally offset from the bottom edge by
//        shear_x. This breaks horizontal seam alignment.
//
// Combined: neither the column boundaries nor the row seams repeat at regular
// intervals, predicting more uniform blue-noise character at partition boundaries.
//
// Scatter inside cell (col, row) uses the parallelogram basis:
//   basis_u = (widths[col], 0)          — across the cell
//   basis_v = (shear_x,    heights[row]) — up the cell
//
// A candidate at u, v ∈ [0,1] maps to world coordinates:
//   x = x_starts[col] + row*shear_x + u*widths[col] + v*shear_x
//   y = y_starts[row]                 + v*heights[row]
//
// The Jacobian of this transform is widths[col]*heights[row] — area-preserving.
// Candidates that land outside [0,width]×[0,height] are filtered out; the
// overage_ratio compensates for losses at sheared edges.

/// Generate `n` positive random weights from `seed`, normalised to sum to `total`.
/// Used to produce variable cell widths/heights.
fn seeded_variable_sizes(n: usize, total: f32, seed: u32) -> Vec<f32> {
    let raw: Vec<f32> = (0..n)
        .scan(seed, |s, _| {
            let (v, s2) = prng_next(*s);
            *s = s2;
            Some(v + 0.2)  // +0.2 floor: prevents degenerate near-zero cells
        })
        .collect();
    let sum = raw.iter().sum::<f32>();
    raw.iter().map(|&w| w / sum * total).collect()
}

/// Prefix sums: returns a Vec of length n where result[i] = sum of sizes[0..i].
fn prefix_sums(sizes: &[f32]) -> Vec<f32> {
    sizes
        .iter()
        .scan(0.0f32, |acc, &s| {
            let start = *acc;
            *acc += s;
            Some(start)
        })
        .collect()
}

/// Scatter-cull over sheared, variable-size parallelogram cells (Approach F).
///
/// Combines two geometric extensions to the rectangular Approach C:
///
/// - **F-1 — Variable sizes:** column widths and row heights are drawn from a
///   seeded distribution and normalised to span the domain. No two columns are
///   the same width; same for rows. This eliminates periodic vertical/horizontal
///   seam alignment.
///
/// - **F-2 — Shear:** each row $r$ is shifted right by $r \cdot s$, where
///   $s = \text{shear\_factor} \times \bar{w}$ and $\bar{w}$ is the mean column
///   width. Cells are parallelograms. At `shear_factor = 0.5` each row is offset
///   by half a mean cell width — the classic brick/offset pattern.
///
/// # Math
///
/// **Variable widths:** raw weights $\tilde{w}_i \sim \text{Uniform}(0.2, 1.2)$
/// (seeded), normalised: $w_i = \tilde{w}_i / \sum_j \tilde{w}_j \cdot W$.
///
/// **Scatter basis for cell $(c, r)$:**
/// $$\mathbf{u} = (w_c,\; 0), \quad \mathbf{v} = (s,\; h_r)$$
///
/// Point at $(u, v) \in [0,1]^2$:
/// $$\mathbf{p} = \bigl(x_c + r \cdot s + u \cdot w_c + v \cdot s,\; y_r + v \cdot h_r\bigr)$$
///
/// Jacobian: $|\det[\mathbf{u}, \mathbf{v}]| = w_c \cdot h_r$ — area-preserving.
///
/// # Arguments
/// * `width`, `height`    — domain dimensions (must be > 0)
/// * `min_dist`           — minimum allowed distance between any two accepted points
/// * `cols`, `rows`       — partition count (variable-size cells per axis)
/// * `shear_factor`       — x-shift per row as fraction of mean column width
///                          (0.0 = axis-aligned variable-size rectangles,
///                           0.5 = brick-pattern shear, 1.0 = full-cell shear)
/// * `target_per_cell`    — approximate desired survivors per cell
/// * `overage_ratio`      — scatter multiplier (>1.0; 1.5–2.0 recommended; edge cells
///                          lose candidates to domain clamp under shear)
/// * `seed`               — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per cell (length = `cols * rows`),
/// row-major order. Points within each cell satisfy min-dist. Cross-cell
/// min-dist is NOT guaranteed.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_sheared;
/// let cells = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
/// assert_eq!(cells.len(), 16);
/// assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
/// ```
pub fn scatter_cull_sheared(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    shear_factor: f32,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    // F-1: Variable-size column widths and row heights from seeded PRNG
    let width_seed  = seed;
    let height_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let col_widths  = seeded_variable_sizes(cols, width,  width_seed);
    let row_heights = seeded_variable_sizes(rows, height, height_seed);
    let x_starts    = prefix_sums(&col_widths);
    let y_starts    = prefix_sums(&row_heights);

    // F-2: Shear amount = shear_factor * mean column width
    let mean_cell_w = width / cols as f32;
    let shear_x     = shear_factor * mean_cell_w;

    let drop_n = (target_per_cell as f32 * overage_ratio) as usize;

    // Precompute per-cell parameters flat (row-major). Using a single flat index
    // avoids nested closure capture conflicts with x_starts/col_widths.
    // Each entry: (x0, y0, cell_w, cell_h, row_shear_offset, cell_seed)
    let cell_params: Vec<(f32, f32, f32, f32, f32, u32)> = (0..(rows * cols))
        .map(|i| {
            let row       = i / cols;
            let col       = i % cols;
            let cell_seed = seed
                .wrapping_mul(1_664_525u32)
                .wrapping_add(i as u32)
                .wrapping_add(0xDEAD_BEEF);
            (
                x_starts[col],
                y_starts[row],
                col_widths[col],
                row_heights[row],
                row as f32 * shear_x,
                cell_seed,
            )
        })
        .collect();

    // Parallel outer loop: each cell scatters and culls independently.
    // Inner PRNG scan stays sequential within each cell.
    // Candidates are sorted before culling to restore determinism across par_iter runs.
    cell_params
        .into_par_iter()
        .map(|(x0, y0, w, h, row_shear, cell_seed)| {
            // Scatter candidates in the parallelogram basis (u, v ∈ [0,1])
            // x = x0 + row_shear + u*w + v*shear_x
            // y = y0             + v*h
            let mut candidates: Vec<(f32, f32)> = (0..drop_n)
                .scan(cell_seed, |s, _| {
                    let (u, s1) = prng_next(*s);
                    let (v, s2) = prng_next(s1);
                    *s = s2;
                    let px = x0 + row_shear + u * w + v * shear_x;
                    let py = y0 + v * h;
                    Some((px, py))
                })
                // Filter: keep only candidates inside the domain
                .filter(|&(px, py)| px >= 0.0 && px < width && py >= 0.0 && py < height)
                .collect();

            // Sort to normalise candidate order — restores determinism across par_iter runs.
            candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });

            cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatter_cull_sheared_cell_count() {
        let cells = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
        assert_eq!(cells.len(), 16);
    }

    #[test]
    fn scatter_cull_sheared_invalid_inputs_return_empty() {
        assert!(scatter_cull_sheared(0.0,   100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42).is_empty());
        assert!(scatter_cull_sheared(100.0, 0.0,   5.0, 4, 4, 0.5, 20, 2.0, 42).is_empty());
        assert!(scatter_cull_sheared(100.0, 100.0, 0.0, 4, 4, 0.5, 20, 2.0, 42).is_empty());
        assert!(scatter_cull_sheared(100.0, 100.0, 5.0, 0, 4, 0.5, 20, 2.0, 42).is_empty());
        assert!(scatter_cull_sheared(100.0, 100.0, 5.0, 4, 0, 0.5, 20, 2.0, 42).is_empty());
        assert!(scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 0,  2.0, 42).is_empty());
    }

    #[test]
    fn scatter_cull_sheared_min_dist_holds() {
        let cells = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
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
    fn scatter_cull_sheared_deterministic() {
        let a = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
        let b = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_cull_sheared_zero_shear_is_variable_rect() {
        // shear_factor=0.0 → parallelogram degenerates to variable-size rectangles
        let cells = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.0, 20, 2.0, 42);
        assert_eq!(cells.len(), 16);
        assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
    }

    #[test]
    fn scatter_cull_sheared_all_points_in_domain() {
        let cells = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
        for cell in &cells {
            for &(x, y) in cell {
                assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
                assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
            }
        }
    }

    #[test]
    fn scatter_cull_sheared_different_seeds_differ() {
        let a = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 42);
        let b = scatter_cull_sheared(100.0, 100.0, 5.0, 4, 4, 0.5, 20, 2.0, 99);
        assert_ne!(a, b);
    }
}
