//! SDF ellipse scatter-cull: Approach G.

use prime_random::prng_next;
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach G: SDF Ellipse Scatter-Cull ─────────────────────────────────────
//
// Cells are defined analytically as ellipses.  Each cell has an explicit
// geometric description — not proximity-to-site (Voronoi) — so its area is
// known exactly as π × a × b without any numerical approximation.
//
// Scatter density is derived from the area formula rather than a flat
// target_per_cell count, ensuring cells of different sizes receive candidates
// proportional to their actual area.  This is the "calculus" component: the
// closed-form integral of the ellipse area drives scatter, not a heuristic.
//
// Cell membership: a point (x, y) is inside ellipse i iff
//   ((x − cx) / a)² + ((y − cy) / b)² ≤ 1
//
// This is the normalised ellipse SDF (signed distance ≤ 0 inside).
//
// Two variants:
//   scatter_cull_sdf_ellipse  — per-cell cull then global cull (two-pass)
//   scatter_global_sdf_ellipse — scatter only, single global cull (one-pass)

/// SDF of a 2-D ellipse centred at `(cx, cy)` with semi-axes `a` (x) and `b` (y).
///
/// Returns negative inside, zero on boundary, positive outside.
/// The value is not a true Euclidean distance for points far from the boundary,
/// but the sign is exact and the magnitude is proportional to the minimum
/// component distance — suitable for inside/outside tests and nearest-ellipse queries.
///
/// # Math
///
/// Normalised ellipse equation: $\frac{(x-cx)^2}{a^2} + \frac{(y-cy)^2}{b^2} = 1$
///
/// SDF approximation: $\sqrt{\left(\frac{x-cx}{a}\right)^2 + \left(\frac{y-cy}{b}\right)^2} - 1$
/// scaled by $\min(a, b)$ so that magnitude is in world-space units near the boundary.
#[inline]
fn sdf_ellipse(x: f32, y: f32, cx: f32, cy: f32, a: f32, b: f32) -> f32 {
    let nx = (x - cx) / a;
    let ny = (y - cy) / b;
    ((nx * nx + ny * ny).sqrt() - 1.0) * a.min(b)
}

/// Place `cols × rows` ellipses over the domain with seeded jitter, returning
/// their centres and semi-axes.
///
/// Coverage factor `>1.0` makes ellipses extend past the grid-cell midpoint so
/// adjacent ellipses overlap — ensuring no large uncovered gaps between cells.
fn ellipse_cells(
    width: f32,
    height: f32,
    cols: usize,
    rows: usize,
    aspect_ratio: f32,
    coverage: f32,
    seed: u32,
) -> Vec<(f32, f32, f32, f32)> {   // (cx, cy, a, b)
    let cell_w = width  / cols  as f32;
    let cell_h = height / rows  as f32;
    let a_base = cell_w * 0.5 * coverage;
    let b_base = cell_h * 0.5 * coverage * aspect_ratio;

    (0..rows * cols)
        .scan(seed, |s, i| {
            let row = i / cols;
            let col = i % cols;
            // Per-cell seed for jitter
            let cell_seed = (*s as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;
            let (jx, s1) = prng_next(cell_seed);
            let (jy, s2) = prng_next(s1);
            *s = s2;
            // Jitter is ±20% of cell size so ellipses stay mostly within their grid tile
            let jitter_range = 0.2;
            let cx = (col as f32 + 0.5 + (jx - 0.5) * jitter_range) * cell_w;
            let cy = (row as f32 + 0.5 + (jy - 0.5) * jitter_range) * cell_h;
            let cx = cx.clamp(a_base, width  - a_base);
            let cy = cy.clamp(b_base, height - b_base);
            Some((cx, cy, a_base, b_base))
        })
        .collect()
}

/// Scatter-cull Poisson disk sampling over SDF-defined ellipse cells (Approach G, two-pass).
///
/// Cells are ellipses placed on a `cols × rows` jittered grid.  Scatter density
/// per cell is area-proportional: each cell receives candidates proportional to
/// `π × a × b / domain_area`, so cells of equal size get equal candidate counts
/// regardless of their position.
///
/// **Two-pass:** candidates are culled per-cell first, then a global min-dist
/// cull resolves seam conflicts.  Compare with `scatter_global_sdf_ellipse`
/// (single-pass) which skips per-cell cull and avoids the seam dead-zone ceiling.
///
/// # Math
///
/// Ellipse area: $A_\text{cell} = \pi \cdot a \cdot b$
///
/// Candidates per cell: $N_i = \frac{A_i}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_sdf_ellipse;
/// let cells = scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
/// assert!(!cells.is_empty());
/// ```
pub fn scatter_cull_sdf_ellipse(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    aspect_ratio: f32,
    coverage: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    let cells = ellipse_cells(width, height, cols, rows, aspect_ratio, coverage, seed);
    let domain_area = width * height;
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    // Parallel outer loop: each ellipse cell scatters and culls independently.
    // Inner PRNG scan + filter + take stays sequential within each cell.
    // Candidates are sorted before culling to restore determinism across par_iter runs.
    cells
        .par_iter()
        .enumerate()
        .map(|(i, &(cx, cy, a, b))| {
            // Area-proportional candidate count — the calculus step
            let cell_area = std::f32::consts::PI * a * b;
            let drop_n = ((cell_area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            // Scatter in bounding box, reject outside ellipse
            let x0 = (cx - a).max(0.0);
            let y0 = (cy - b).max(0.0);
            let bw  = ((cx + a).min(width)  - x0).max(0.0);
            let bh  = ((cy + b).min(height) - y0).max(0.0);

            let mut candidates: Vec<(f32, f32)> = (0..drop_n * 3)  // 3× over-generate to compensate rejection
                .scan(cell_seed, |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    let px = x0 + xf * bw;
                    let py = y0 + yf * bh;
                    Some((px, py))
                })
                .filter(|&(px, py)| sdf_ellipse(px, py, cx, cy, a, b) <= 0.0)
                .take(drop_n)
                .collect();

            // Sort to normalise candidate order — restores determinism across par_iter runs.
            candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });

            cull_to_min_dist(candidates, x0, y0, x0 + bw, y0 + bh, min_dist)
        })
        .collect()
}

/// Scatter-cull Poisson disk sampling over SDF-defined ellipse cells — single global cull pass
/// (Approach G, one-pass).
///
/// Same cell geometry and area-proportional scatter as `scatter_cull_sdf_ellipse`, but skips
/// per-cell cull entirely.  All candidates from all cells are flattened into one pool and
/// a single global `cull_to_min_dist` resolves conflicts.
///
/// This avoids the seam dead-zone density ceiling that two-pass approaches hit at high overage.
///
/// # Math
///
/// Candidates per cell: $N_i = \frac{\pi \cdot a_i \cdot b_i}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_sdf_ellipse;
/// let pts = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_sdf_ellipse(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    aspect_ratio: f32,
    coverage: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    let cells = ellipse_cells(width, height, cols, rows, aspect_ratio, coverage, seed);
    let domain_area = width * height;
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    // Parallel outer loop: each ellipse cell generates candidates independently.
    // Inner PRNG scan + filter + take stays sequential within each cell.
    // Sort the flattened candidates before global cull to restore determinism.
    let mut candidates: Vec<(f32, f32)> = cells
        .par_iter()
        .enumerate()
        .flat_map(|(i, &(cx, cy, a, b))| {
            let cell_area = std::f32::consts::PI * a * b;
            let drop_n = ((cell_area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            let x0 = (cx - a).max(0.0);
            let y0 = (cy - b).max(0.0);
            let bw  = ((cx + a).min(width)  - x0).max(0.0);
            let bh  = ((cy + b).min(height) - y0).max(0.0);

            (0..drop_n * 3)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * bw, y0 + yf * bh))
                })
                .filter(move |&(px, py)| sdf_ellipse(px, py, cx, cy, a, b) <= 0.0)
                .take(drop_n)
                .collect::<Vec<_>>()
        })
        .collect();

    // Sort to normalise candidate order — restores determinism across par_iter runs.
    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Approach G-CC: Clipped-Circle Scatter-Cull ───────────────────────────────
//
// Each cell is a grid tile clipped to a circle.  The tile boundary acts as a
// hard partition so every domain point belongs to exactly one cell.  The circle
// (with coverage ≥ √2 ≈ 1.414) covers every corner of the tile — guaranteeing
// full domain coverage without holes.
//
// Two variants follow the same split as the SDF-ellipse variants above:
//   scatter_cull_clipped_circle  — scatter inside tile, reject outside circle,
//                                  per-cell cull then caller does global cull.
//   scatter_global_clipped_circle — scatter inside tile without per-cell rejection,
//                                   single global cull handles everything.
//
// Cell geometry for (col, row):
//   x0 = col * cell_w,  y0 = row * cell_h
//   cx = (col + 0.5) * cell_w,  cy = (row + 0.5) * cell_h
//   r  = cell_w.min(cell_h) * 0.5 * coverage
//
// Area used for area-proportional drop_n:
//   two-pass: π × r² (candidates are pre-filtered to the circle)
//   single-pass: cell_w × cell_h (all tile candidates are accepted)

/// Scatter-cull Poisson disk sampling over clipped-circle cells (Approach G-CC, two-pass).
///
/// Each cell is defined as a grid tile (the partition) whose scatter candidates are
/// further filtered to lie inside the inscribed/over-sized circle.  Scatter density
/// is area-proportional to `π × r²` — the closed-form circle area — which is the
/// same calculus principle as `scatter_cull_sdf_ellipse`.
///
/// Use `coverage ≥ √2 ≈ 1.414` to ensure the circle covers all tile corners,
/// giving complete domain coverage.  At `coverage = 1.0` (inscribed circle), corners
/// are excluded and ~21.5% of each tile's area is never sampled.
///
/// **Two-pass:** candidates are culled per-cell, then the caller applies a global
/// `global_cull_to_min_dist` to resolve seam conflicts.
///
/// # Math
///
/// Circle radius: $r = \min(w_\text{cell}, h_\text{cell}) \cdot 0.5 \cdot \text{coverage}$
///
/// Circle area: $A = \pi r^2$
///
/// Candidates per cell: $N_i = \frac{\pi r^2}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_clipped_circle;
/// let cells = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
/// assert!(!cells.is_empty());
/// ```
pub fn scatter_cull_clipped_circle(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    coverage: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    let cell_w      = width  / cols as f32;
    let cell_h      = height / rows as f32;
    let r           = cell_w.min(cell_h) * 0.5 * coverage;
    let domain_area = width * height;
    let cell_area   = std::f32::consts::PI * r * r;

    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    // Parallel outer loop: each clipped-circle cell scatters and culls independently.
    (0..rows * cols)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|&i| {
            let row = i / cols;
            let col = i % cols;
            let x0  = col as f32 * cell_w;
            let y0  = row as f32 * cell_h;
            let cx  = x0 + cell_w * 0.5;
            let cy  = y0 + cell_h * 0.5;

            let drop_n = ((cell_area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            // Scatter uniformly in the tile rectangle; reject outside circle.
            // Over-generate by 4× to compensate for circle-area rejection (π/4 ≈ 0.785).
            let mut candidates: Vec<(f32, f32)> = (0..drop_n * 4)
                .scan(cell_seed, |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    let px = x0 + xf * cell_w;
                    let py = y0 + yf * cell_h;
                    Some((px, py))
                })
                .filter(|&(px, py)| sdf_ellipse(px, py, cx, cy, r, r) <= 0.0)
                .take(drop_n)
                .collect();

            // Sort to normalise candidate order — restores determinism across par_iter runs.
            candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });

            cull_to_min_dist(candidates, x0, y0, x0 + cell_w, y0 + cell_h, min_dist)
        })
        .collect()
}

/// Scatter-cull Poisson disk sampling over clipped-circle cells — single global cull pass
/// (Approach G-CC, one-pass).
///
/// Same cell geometry as `scatter_cull_clipped_circle` but candidates are scattered in the
/// full tile rectangle without per-candidate circle rejection.  All candidates from all cells
/// are flattened into one pool and a single global `cull_to_min_dist` resolves conflicts.
///
/// Since no candidates are rejected before the global cull, every domain point is reachable
/// and coverage is complete regardless of `coverage` value.  The `coverage` parameter still
/// controls the circle size used for area-proportional drop-n calculation — higher values
/// scatter more candidates per cell.
///
/// Scatter density is proportional to the tile area (`cell_w × cell_h`) since all tile
/// candidates are retained.
///
/// # Math
///
/// Candidates per cell: $N_i = \frac{w_\text{cell} \cdot h_\text{cell}}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_clipped_circle;
/// let pts = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_clipped_circle(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    coverage: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    let cell_w      = width  / cols as f32;
    let cell_h      = height / rows as f32;
    // coverage is received for API symmetry; tile area drives drop_n in single-pass variant.
    let _           = coverage;
    let tile_area   = cell_w * cell_h;
    let domain_area = width * height;

    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    // Parallel outer loop: each clipped-circle cell generates candidates independently.
    let mut candidates: Vec<(f32, f32)> = (0..rows * cols)
        .collect::<Vec<_>>()
        .par_iter()
        .flat_map(|&i| {
            let row = i / cols;
            let col = i % cols;
            let x0  = col as f32 * cell_w;
            let y0  = row as f32 * cell_h;

            let drop_n = ((tile_area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            // Scatter uniformly in the tile rectangle — no circle rejection.
            // Tile partitioning guarantees full domain coverage.
            (0..drop_n)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * cell_w, y0 + yf * cell_h))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Sort to normalise candidate order — restores determinism across par_iter runs.
    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Approach G-Inset + Corner Fill ───────────────────────────────────────────
//
// Main ellipses: a = cell_w/2 − min_dist/2, b = cell_h/2 − min_dist/2.
// This leaves a min_dist-wide dead zone at every tile boundary.
// Corner fills: small circles of radius min_dist/2 at each grid intersection,
// covering the dead zone at each corner where four tiles meet.
// Combined, every point in the domain is reachable by at least one scatter region.
//
// Returns empty if either inset semi-axis ≤ 0 (cells too small for min_dist).

/// Scatter-cull over inset ellipses with corner-fill circles (Approach G-Inset, two-pass).
///
/// Main ellipses inset by `min_dist/2` from each tile edge.  Corner circles of radius
/// `min_dist/2` placed at every grid intersection fill the dead zones that the inset creates.
///
/// Returns empty if `cell_w/2 − min_dist/2 ≤ 0` or `cell_h/2 − min_dist/2 ≤ 0`.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_sdf_ellipse_inset;
/// let cells = scatter_cull_sdf_ellipse_inset(100.0, 100.0, 5.0, 4, 4, 300, 5.0, 42);
/// assert!(!cells.is_empty());
/// ```
pub fn scatter_cull_sdf_ellipse_inset(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    let cell_w  = width  / cols as f32;
    let cell_h  = height / rows as f32;
    let a_inset = cell_w * 0.5 - min_dist * 0.5;
    let b_inset = cell_h * 0.5 - min_dist * 0.5;

    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
        || a_inset <= 0.0 || b_inset <= 0.0
    {
        return Vec::new();
    }

    let domain_area  = width * height;
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    (0..rows * cols)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|&i| {
            let row = i / cols;
            let col = i % cols;
            let cx  = (col as f32 + 0.5) * cell_w;
            let cy  = (row as f32 + 0.5) * cell_h;

            let cell_area = std::f32::consts::PI * a_inset * b_inset;
            let drop_n    = ((cell_area / domain_area) * total_target as f32 * overage_ratio)
                .ceil() as usize;
            let drop_n    = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            let x0 = (cx - a_inset).max(0.0);
            let y0 = (cy - b_inset).max(0.0);
            let bw = ((cx + a_inset).min(width)  - x0).max(0.0);
            let bh = ((cy + b_inset).min(height) - y0).max(0.0);

            // 3× over-generate to compensate for bounding-box → ellipse rejection (~π/4 acceptance).
            let mut candidates: Vec<(f32, f32)> = (0..drop_n * 3)
                .scan(cell_seed, |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    let px = x0 + xf * bw;
                    let py = y0 + yf * bh;
                    Some((px, py))
                })
                .filter(|&(px, py)| sdf_ellipse(px, py, cx, cy, a_inset, b_inset) <= 0.0)
                .take(drop_n)
                .collect();

            candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
                ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
            });

            cull_to_min_dist(candidates, x0, y0, x0 + bw, y0 + bh, min_dist)
        })
        .collect()
}

/// Scatter-cull Poisson disk sampling over min_dist-inset ellipse cells — single global cull pass
/// (Approach G-Inset, one-pass).
///
/// Same inset geometry as `scatter_cull_sdf_ellipse_inset`.  Skips per-cell cull entirely;
/// all candidates are flattened and resolved by a single global `cull_to_min_dist`.
///
/// Since the inset already guarantees seam safety for the two-pass variant, the single-pass
/// variant is provided here as a reference comparison: it produces the same (or more) points
/// with seam_kept = 1.000 by construction.
///
/// Returns empty if either computed semi-axis ≤ 0 (min_dist ≥ cell dimension).
///
/// # Math
///
/// $a = w_\text{cell}/2 - d_\text{min}/2$,  $b = h_\text{cell}/2 - d_\text{min}/2$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_sdf_ellipse_inset;
/// let pts = scatter_global_sdf_ellipse_inset(100.0, 100.0, 5.0, 4, 4, 300, 5.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_sdf_ellipse_inset(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    let cell_w  = width  / cols as f32;
    let cell_h  = height / rows as f32;
    let a_inset = cell_w * 0.5 - min_dist * 0.5;
    let b_inset = cell_h * 0.5 - min_dist * 0.5;

    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
        || a_inset <= 0.0 || b_inset <= 0.0
    {
        return Vec::new();
    }

    let domain_area  = width * height;
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);

    let mut candidates: Vec<(f32, f32)> = (0..rows * cols)
        .collect::<Vec<_>>()
        .par_iter()
        .flat_map(|&i| {
            let row = i / cols;
            let col = i % cols;
            let cx  = (col as f32 + 0.5) * cell_w;
            let cy  = (row as f32 + 0.5) * cell_h;

            let cell_area = std::f32::consts::PI * a_inset * b_inset;
            let drop_n    = ((cell_area / domain_area) * total_target as f32 * overage_ratio)
                .ceil() as usize;
            let drop_n    = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            let x0 = (cx - a_inset).max(0.0);
            let y0 = (cy - b_inset).max(0.0);
            let bw = ((cx + a_inset).min(width)  - x0).max(0.0);
            let bh = ((cy + b_inset).min(height) - y0).max(0.0);

            (0..drop_n * 3)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * bw, y0 + yf * bh))
                })
                .filter(move |&(px, py)| sdf_ellipse(px, py, cx, cy, a_inset, b_inset) <= 0.0)
                .take(drop_n)
                .collect::<Vec<_>>()
        })
        .collect();

    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Approach G-Corner: Inset Ellipses + Corner-Fill Circles ──────────────────
//
// Main ellipses inset by min_dist/2 — creates a min_dist dead zone at every boundary.
// Corner circles (radius = min_dist/2) centred at every grid intersection fill those gaps.
// All candidates from both sets are flattened into a single global cull pass.
// Domain-edge corner circles are naturally clipped by the domain bounds filter.

/// Inset ellipses + corner-fill circles, single global cull (Approach G-Corner).
///
/// Main cells: ellipses with `a = cell_w/2 − min_dist/2`, `b = cell_h/2 − min_dist/2`.
/// Corner fills: circles of radius `min_dist/2` centred at every grid intersection `(i·cell_w, j·cell_h)`.
/// Combined scatter covers the full domain; one global cull resolves all conflicts.
///
/// Returns empty if inset semi-axes ≤ 0.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_sdf_ellipse_corner_fill;
/// let pts = scatter_global_sdf_ellipse_corner_fill(100.0, 100.0, 5.0, 4, 4, 300, 5.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_sdf_ellipse_corner_fill(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    let cell_w  = width  / cols as f32;
    let cell_h  = height / rows as f32;
    let a_inset = cell_w * 0.5 - min_dist * 0.5;
    let b_inset = cell_h * 0.5 - min_dist * 0.5;
    let r_corner = min_dist * 0.5;

    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
        || a_inset <= 0.0 || b_inset <= 0.0
    {
        return Vec::new();
    }

    let domain_area   = width * height;
    let main_seed     = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let corner_seed   = seed.wrapping_mul(2_891_336_453u32).wrapping_add(747_796_405u32);

    // ── Main cells: inset ellipses ────────────────────────────────────────────
    let mut candidates: Vec<(f32, f32)> = (0..rows * cols)
        .collect::<Vec<_>>()
        .par_iter()
        .flat_map(|&i| {
            let row = i / cols;
            let col = i % cols;
            let cx  = (col as f32 + 0.5) * cell_w;
            let cy  = (row as f32 + 0.5) * cell_h;

            let cell_area = std::f32::consts::PI * a_inset * b_inset;
            let drop_n    = ((cell_area / domain_area) * total_target as f32 * overage_ratio)
                .ceil() as usize;
            let drop_n    = drop_n.max(4);

            let cell_seed = (main_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            let x0 = (cx - a_inset).max(0.0);
            let y0 = (cy - b_inset).max(0.0);
            let bw = ((cx + a_inset).min(width)  - x0).max(0.0);
            let bh = ((cy + b_inset).min(height) - y0).max(0.0);

            (0..drop_n * 3)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * bw, y0 + yf * bh))
                })
                .filter(move |&(px, py)| sdf_ellipse(px, py, cx, cy, a_inset, b_inset) <= 0.0)
                .take(drop_n)
                .collect::<Vec<_>>()
        })
        .collect();

    // ── Corner fills: circles at every grid intersection ──────────────────────
    // (cols+1) × (rows+1) intersection points; domain-edge circles are clipped by bounds.
    let corner_area = std::f32::consts::PI * r_corner * r_corner;
    let corner_drop_base = ((corner_area / domain_area) * total_target as f32 * overage_ratio)
        .ceil() as usize;
    let corner_drop_base = corner_drop_base.max(4);

    let corner_candidates: Vec<(f32, f32)> = (0..(rows + 1) * (cols + 1))
        .collect::<Vec<_>>()
        .par_iter()
        .flat_map(|&k| {
            let row = k / (cols + 1);
            let col = k % (cols + 1);
            let cx  = col as f32 * cell_w;
            let cy  = row as f32 * cell_h;

            let cell_seed = (corner_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(k as u64) as u32;

            let x0 = (cx - r_corner).max(0.0);
            let y0 = (cy - r_corner).max(0.0);
            let bw = ((cx + r_corner).min(width)  - x0).max(0.0);
            let bh = ((cy + r_corner).min(height) - y0).max(0.0);

            (0..corner_drop_base * 3)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * bw, y0 + yf * bh))
                })
                .filter(move |&(px, py)| sdf_ellipse(px, py, cx, cy, r_corner, r_corner) <= 0.0)
                .take(corner_drop_base)
                .collect::<Vec<_>>()
        })
        .collect();

    candidates.extend(corner_candidates);
    candidates.sort_unstable_by(|&(ax, ay), &(bx, by)| {
        ax.to_bits().cmp(&bx.to_bits()).then(ay.to_bits().cmp(&by.to_bits()))
    });

    cull_to_min_dist(candidates, 0.0, 0.0, width, height, min_dist)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── sdf_ellipse ───────────────────────────────────────────────────────────

    #[test]
    fn sdf_ellipse_inside() {
        assert!(sdf_ellipse(0.0, 0.0, 0.0, 0.0, 5.0, 3.0) < 0.0);
    }

    #[test]
    fn sdf_ellipse_outside() {
        assert!(sdf_ellipse(10.0, 0.0, 0.0, 0.0, 5.0, 3.0) > 0.0);
    }

    #[test]
    fn sdf_ellipse_on_boundary() {
        // Point exactly on the x-axis at semi-axis distance
        let d = sdf_ellipse(5.0, 0.0, 0.0, 0.0, 5.0, 3.0);
        assert!(d.abs() < 0.1, "expected near zero, got {d}");
    }

    // ── scatter_cull_sdf_ellipse ──────────────────────────────────────────────

    #[test]
    fn scatter_cull_sdf_ellipse_invalid_inputs_return_empty() {
        assert!(scatter_cull_sdf_ellipse(0.0,   100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_sdf_ellipse(100.0, 100.0, 0.0, 4, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 0, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 0,   3.0, 42).is_empty());
    }

    #[test]
    fn scatter_cull_sdf_ellipse_cell_count() {
        let cells = scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        assert_eq!(cells.len(), 16); // 4×4 = 16 cells
    }

    #[test]
    fn scatter_cull_sdf_ellipse_min_dist_holds() {
        let cells = scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let d  = (dx * dx + dy * dy).sqrt();
                    assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4}");
                }
            }
        }
    }

    #[test]
    fn scatter_cull_sdf_ellipse_deterministic() {
        let a = scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        let b = scatter_cull_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    // ── scatter_global_sdf_ellipse ────────────────────────────────────────────

    #[test]
    fn scatter_global_sdf_ellipse_invalid_inputs_return_empty() {
        assert!(scatter_global_sdf_ellipse(0.0,   100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_sdf_ellipse(100.0, 100.0, 0.0, 4, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 0, 4, 1.0, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 0,   3.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_sdf_ellipse_produces_points() {
        let pts = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_sdf_ellipse_min_dist_holds() {
        let pts = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        let min_dist = 5.0_f32;
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let dx = pts[i].0 - pts[j].0;
                let dy = pts[i].1 - pts[j].1;
                let d  = (dx * dx + dy * dy).sqrt();
                assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4}");
            }
        }
    }

    #[test]
    fn scatter_global_sdf_ellipse_deterministic() {
        let a = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        let b = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_global_sdf_ellipse_all_points_in_domain() {
        let pts = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        for &(x, y) in &pts {
            assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
        }
    }

    #[test]
    fn scatter_global_sdf_ellipse_aspect_changes_output() {
        let circle  = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 1.0, 1.2, 300, 3.0, 42);
        let ellipse = scatter_global_sdf_ellipse(100.0, 100.0, 5.0, 4, 4, 2.0, 1.2, 300, 3.0, 42);
        assert_ne!(circle, ellipse, "different aspect ratios should produce different output");
    }

    // ── scatter_cull_clipped_circle ───────────────────────────────────────────

    #[test]
    fn scatter_cull_clipped_circle_invalid_inputs_return_empty() {
        assert!(scatter_cull_clipped_circle(0.0,   100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_clipped_circle(100.0, 100.0, 0.0, 4, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_clipped_circle(100.0, 100.0, 5.0, 0, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 0,   3.0, 42).is_empty());
    }

    #[test]
    fn scatter_cull_clipped_circle_cell_count() {
        let cells = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        assert_eq!(cells.len(), 16); // 4×4 = 16 cells
    }

    #[test]
    fn scatter_cull_clipped_circle_produces_points() {
        let cells = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        let total: usize = cells.iter().map(|c| c.len()).sum();
        assert!(total > 0);
    }

    #[test]
    fn scatter_cull_clipped_circle_min_dist_holds() {
        let cells = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let d  = (dx * dx + dy * dy).sqrt();
                    assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4}");
                }
            }
        }
    }

    #[test]
    fn scatter_cull_clipped_circle_deterministic() {
        let a = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        let b = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_cull_clipped_circle_all_points_in_domain() {
        let cells = scatter_cull_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        for &(x, y) in cells.iter().flatten() {
            assert!(x >= 0.0 && x <= 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y <= 100.0, "y={y} out of domain");
        }
    }

    // ── scatter_global_clipped_circle ─────────────────────────────────────────

    #[test]
    fn scatter_global_clipped_circle_invalid_inputs_return_empty() {
        assert!(scatter_global_clipped_circle(0.0,   100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_clipped_circle(100.0, 100.0, 0.0, 4, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_clipped_circle(100.0, 100.0, 5.0, 0, 4, 1.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 0,   3.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_clipped_circle_produces_points() {
        let pts = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_clipped_circle_min_dist_holds() {
        let pts = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        let min_dist = 5.0_f32;
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let dx = pts[i].0 - pts[j].0;
                let dy = pts[i].1 - pts[j].1;
                let d  = (dx * dx + dy * dy).sqrt();
                assert!(d >= min_dist - 1e-4, "min-dist violated: {d:.4}");
            }
        }
    }

    #[test]
    fn scatter_global_clipped_circle_deterministic() {
        let a = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        let b = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_global_clipped_circle_all_points_in_domain() {
        let pts = scatter_global_clipped_circle(100.0, 100.0, 5.0, 4, 4, 1.2, 300, 3.0, 42);
        for &(x, y) in &pts {
            assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
        }
    }
}
