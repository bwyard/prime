//! Triangle cell scatter-cull: Approach H.

use prime_random::prng_next;
use rayon::prelude::*;
use super::cull_to_min_dist;

// ── Approach H: Triangle Cell Scatter-Cull ───────────────────────────────────
//
// Cells are irregular triangles formed by splitting a jittered rectangular grid.
// Each rectangle is split along a diagonal — alternating direction per cell for
// visual variety.  Interior vertices are jittered ±jitter*cell_size; corner
// vertices (domain boundary) stay fixed so the tiling fills the domain exactly.
//
// Triangles tile the plane perfectly with no gaps or overlaps.  Area is computed
// exactly via the cross-product formula: A = ½ |( b−a ) × ( c−a )|.
//
// Scatter density is area-proportional: each triangle receives candidates
// proportional to its area relative to the domain, so triangles of different
// sizes receive candidate counts consistent with expected point density.
//
// Two variants:
//   scatter_cull_triangles  — per-cell cull then caller can do global cull (two-pass)
//   scatter_global_triangles — scatter only, single global cull (one-pass)

// ── Private helpers ───────────────────────────────────────────────────────────

/// Test whether point (px, py) lies inside or on the edge of triangle (a, b, c).
///
/// Uses sign of cross products — exact for degenerate cases.
/// Returns true if the point is inside or on the boundary.
#[inline]
fn point_in_triangle(
    px: f32, py: f32,
    ax: f32, ay: f32,
    bx: f32, by: f32,
    cx: f32, cy: f32,
) -> bool {
    let d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
    let d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
    let d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

/// Compute the area of a triangle with vertices a, b, c.
///
/// Area = ½ × |(b−a) × (c−a)|
#[inline]
fn triangle_area(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    // Cross product magnitude: (b-a) × (c-a)
    let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    cross.abs() * 0.5
}

/// Generate a jittered (cols+1)×(rows+1) grid of vertices for triangle tiling.
///
/// Corner vertices (on the domain boundary) are fixed.  Interior vertices are
/// jittered by ±`jitter` × cell_size using a seeded PRNG.
///
/// Returns a flat Vec of (x, y) in row-major order: index = row * (cols+1) + col.
fn jittered_grid_vertices(
    width: f32,
    height: f32,
    cols: usize,
    rows: usize,
    jitter: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    let cell_w = width  / cols as f32;
    let cell_h = height / rows as f32;
    let vcols = cols + 1;
    let vrows = rows + 1;

    let mut verts = Vec::with_capacity(vcols * vrows);
    let mut s = seed;

    for row in 0..vrows {
        for col in 0..vcols {
            let base_x = col as f32 * cell_w;
            let base_y = row as f32 * cell_h;

            // Only interior vertices are jittered; boundary vertices stay fixed.
            let on_boundary = col == 0 || col == cols || row == 0 || row == rows;
            if on_boundary || jitter == 0.0 {
                verts.push((base_x, base_y));
            } else {
                let (jx, s1) = prng_next(s);
                let (jy, s2) = prng_next(s1);
                s = s2;
                // jitter in [-jitter, +jitter] × cell_size
                let dx = (jx - 0.5) * 2.0 * jitter * cell_w;
                let dy = (jy - 0.5) * 2.0 * jitter * cell_h;
                verts.push((base_x + dx, base_y + dy));
            }
        }
    }

    verts
}

/// Build the list of triangles from the jittered grid.
///
/// Each rectangle (col, row) is split into 2 triangles.  Even cells split
/// top-left to bottom-right (TL→BR diagonal); odd cells split top-right to
/// bottom-left (TR→BL diagonal), creating an alternating pattern.
///
/// Returns Vec of (a, b, c) vertex triples.
fn build_triangles(
    verts: &[(f32, f32)],
    cols: usize,
    rows: usize,
) -> Vec<((f32, f32), (f32, f32), (f32, f32))> {
    let vcols = cols + 1;
    let mut triangles = Vec::with_capacity(2 * cols * rows);

    for row in 0..rows {
        for col in 0..cols {
            // Grid indices for the four corners of this rectangle
            let tl = row       * vcols + col;
            let tr = row       * vcols + col + 1;
            let bl = (row + 1) * vcols + col;
            let br = (row + 1) * vcols + col + 1;

            let v_tl = verts[tl];
            let v_tr = verts[tr];
            let v_bl = verts[bl];
            let v_br = verts[br];

            // Alternate diagonal direction per cell for visual irregularity
            let cell_idx = row * cols + col;
            if cell_idx % 2 == 0 {
                // TL→BR diagonal: upper-left tri (TL, TR, BL) and lower-right (TR, BR, BL)
                triangles.push((v_tl, v_tr, v_bl));
                triangles.push((v_tr, v_br, v_bl));
            } else {
                // TR→BL diagonal: upper-right tri (TL, TR, BR) and lower-left (TL, BR, BL)
                triangles.push((v_tl, v_tr, v_br));
                triangles.push((v_tl, v_br, v_bl));
            }
        }
    }

    triangles
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Scatter-cull Poisson disk sampling over irregular triangle cells (Approach H, two-pass).
///
/// Splits a jittered `cols × rows` rectangular grid into `2 × cols × rows` triangles.
/// Interior vertices are jittered by ±`jitter × cell_size` for visual irregularity;
/// boundary vertices stay fixed so the tiling covers the domain exactly.
///
/// Scatter density is area-proportional: each triangle receives candidates
/// proportional to its area relative to the full domain area.
///
/// **Two-pass:** candidates are culled per-cell first (intra-triangle min-dist),
/// then a global `global_cull_to_min_dist` pass resolves cross-triangle seam conflicts.
///
/// # Math
///
/// Triangle area: $A = \frac{1}{2} |(b - a) \times (c - a)|$
///
/// Candidates per triangle: $N_i = \frac{A_i}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Arguments
/// * `width`, `height` — domain dimensions
/// * `min_dist`        — minimum distance between accepted points
/// * `cols`, `rows`    — grid dimensions; produces `2 × cols × rows` triangles
/// * `jitter`          — interior vertex jitter factor (0.0 = regular grid, 0.2 = ±20%)
/// * `total_target`    — desired total output points
/// * `overage_ratio`   — scatter multiplier (>1.0 to compensate rejection and seam loss)
/// * `seed`            — PRNG seed for reproducibility
///
/// # Returns
/// One `Vec<(f32, f32)>` per triangle; each inner vec satisfies intra-cell min-dist.
/// Apply `global_cull_to_min_dist` to the flattened output for global min-dist.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_triangles;
/// let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 5.0, 42);
/// assert_eq!(cells.len(), 32); // 2 × 4 × 4
/// ```
pub fn scatter_cull_triangles(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    jitter: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    // Build jittered vertex grid and triangle list
    let grid_seed    = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let scatter_seed = seed.wrapping_mul(22_695_477u32).wrapping_add(1u32);

    let verts     = jittered_grid_vertices(width, height, cols, rows, jitter, grid_seed);
    let triangles = build_triangles(&verts, cols, rows);

    let domain_area = width * height;

    // Parallel outer loop: each triangle scatters and culls independently.
    // Sort candidates before cull to restore determinism across par_iter runs.
    triangles
        .par_iter()
        .enumerate()
        .map(|(i, &(a, b, c))| {
            let area = triangle_area(a, b, c);
            if area < 1e-9 {
                return Vec::new();
            }

            // Area-proportional candidate count; 2× over-generate for bounding-box rejection
            let drop_n = ((area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            // Bounding box for scatter
            let (ax, ay) = a;
            let (bx, by) = b;
            let (cx, cy) = c;
            let x0 = ax.min(bx).min(cx).max(0.0);
            let y0 = ay.min(by).min(cy).max(0.0);
            let x1 = ax.max(bx).max(cx).min(width);
            let y1 = ay.max(by).max(cy).min(height);
            let bw = (x1 - x0).max(0.0);
            let bh = (y1 - y0).max(0.0);

            // 2× over-generate: right triangle fills ~50% of its bounding box
            let mut candidates: Vec<(f32, f32)> = (0..drop_n * 2)
                .scan(cell_seed, |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    let px = x0 + xf * bw;
                    let py = y0 + yf * bh;
                    Some((px, py))
                })
                .filter(|&(px, py)| point_in_triangle(px, py, ax, ay, bx, by, cx, cy))
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

/// Scatter-cull Poisson disk sampling over irregular triangle cells — single global cull pass
/// (Approach H, one-pass).
///
/// Same triangle geometry and area-proportional scatter as `scatter_cull_triangles`, but skips
/// per-cell cull entirely.  All candidates from all triangles are flattened into one pool and
/// a single global `cull_to_min_dist` resolves conflicts.
///
/// This avoids the seam dead-zone density ceiling that two-pass approaches hit at high overage.
///
/// # Math
///
/// Triangle area: $A = \frac{1}{2} |(b - a) \times (c - a)|$
///
/// Candidates per triangle: $N_i = \frac{A_i}{A_\text{domain}} \times N_\text{total} \times \text{overage}$
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_global_triangles;
/// let pts = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 5.0, 42);
/// assert!(!pts.is_empty());
/// ```
pub fn scatter_global_triangles(
    width: f32,
    height: f32,
    min_dist: f32,
    cols: usize,
    rows: usize,
    jitter: f32,
    total_target: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || cols == 0 || rows == 0 || total_target == 0
    {
        return Vec::new();
    }

    let grid_seed    = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let scatter_seed = seed.wrapping_mul(22_695_477u32).wrapping_add(1u32);

    let verts     = jittered_grid_vertices(width, height, cols, rows, jitter, grid_seed);
    let triangles = build_triangles(&verts, cols, rows);

    let domain_area = width * height;

    // Parallel outer loop: each triangle generates candidates independently.
    // Sort the flattened candidates before global cull to restore determinism.
    let mut candidates: Vec<(f32, f32)> = triangles
        .par_iter()
        .enumerate()
        .flat_map(|(i, &(a, b, c))| {
            let area = triangle_area(a, b, c);
            if area < 1e-9 {
                return Vec::new();
            }

            let drop_n = ((area / domain_area) * total_target as f32 * overage_ratio) as usize;
            let drop_n = drop_n.max(4);

            let cell_seed = (scatter_seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i as u64) as u32;

            let (ax, ay) = a;
            let (bx, by) = b;
            let (cx, cy) = c;
            let x0 = ax.min(bx).min(cx).max(0.0);
            let y0 = ay.min(by).min(cy).max(0.0);
            let x1 = ax.max(bx).max(cx).min(width);
            let y1 = ay.max(by).max(cy).min(height);
            let bw = (x1 - x0).max(0.0);
            let bh = (y1 - y0).max(0.0);

            (0..drop_n * 2)
                .scan(cell_seed, move |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x0 + xf * bw, y0 + yf * bh))
                })
                .filter(move |&(px, py)| point_in_triangle(px, py, ax, ay, bx, by, cx, cy))
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── point_in_triangle ─────────────────────────────────────────────────────

    #[test]
    fn point_in_triangle_inside() {
        // Centre of a right triangle at origin
        assert!(point_in_triangle(0.25, 0.25, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn point_in_triangle_outside() {
        // Clearly outside
        assert!(!point_in_triangle(2.0, 2.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn point_in_triangle_on_edge() {
        // Mid-point of hypotenuse — on boundary
        assert!(point_in_triangle(0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0));
    }

    // ── triangle_area ─────────────────────────────────────────────────────────

    #[test]
    fn triangle_area_unit_right_triangle() {
        let a = triangle_area((0.0, 0.0), (1.0, 0.0), (0.0, 1.0));
        assert!((a - 0.5).abs() < 1e-6, "expected 0.5, got {a}");
    }

    #[test]
    fn triangle_area_degenerate_collinear() {
        let a = triangle_area((0.0, 0.0), (1.0, 0.0), (2.0, 0.0));
        assert!(a < 1e-6, "expected ~0, got {a}");
    }

    // ── scatter_cull_triangles ────────────────────────────────────────────────

    #[test]
    fn scatter_cull_triangles_invalid_inputs_return_empty() {
        assert!(scatter_cull_triangles(0.0,   100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_triangles(100.0, 100.0, 0.0, 4, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_triangles(100.0, 100.0, 5.0, 0, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_triangles(100.0, 100.0, 5.0, 4, 0, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 0,   3.0, 42).is_empty());
        assert!(scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 0.0, 42).is_empty());
    }

    #[test]
    fn scatter_cull_triangles_cell_count() {
        let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        // 2 triangles per rectangle × cols × rows
        assert_eq!(cells.len(), 2 * 4 * 4);
    }

    #[test]
    fn scatter_cull_triangles_cell_count_nonsquare() {
        let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 3, 5, 0.0, 300, 3.0, 42);
        assert_eq!(cells.len(), 2 * 3 * 5);
    }

    #[test]
    fn scatter_cull_triangles_min_dist_holds() {
        let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let d  = (dx * dx + dy * dy).sqrt();
                    assert!(d >= min_dist - 1e-4, "min-dist violated within cell: {d:.4}");
                }
            }
        }
    }

    #[test]
    fn scatter_cull_triangles_deterministic() {
        let a = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        let b = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_cull_triangles_produces_points() {
        let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        let total: usize = cells.iter().map(|c| c.len()).sum();
        assert!(total > 0, "expected non-empty output");
    }

    #[test]
    fn scatter_cull_triangles_all_points_in_domain() {
        let cells = scatter_cull_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        for cell in &cells {
            for &(x, y) in cell {
                assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
                assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
            }
        }
    }

    // ── scatter_global_triangles ──────────────────────────────────────────────

    #[test]
    fn scatter_global_triangles_invalid_inputs_return_empty() {
        assert!(scatter_global_triangles(0.0,   100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_triangles(100.0, 100.0, 0.0, 4, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_triangles(100.0, 100.0, 5.0, 0, 4, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_triangles(100.0, 100.0, 5.0, 4, 0, 0.2, 300, 3.0, 42).is_empty());
        assert!(scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 0,   3.0, 42).is_empty());
        assert!(scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 0.0, 42).is_empty());
    }

    #[test]
    fn scatter_global_triangles_produces_points() {
        let pts = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        assert!(!pts.is_empty());
    }

    #[test]
    fn scatter_global_triangles_min_dist_holds() {
        let pts = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
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
    fn scatter_global_triangles_deterministic() {
        let a = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        let b = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_global_triangles_all_points_in_domain() {
        let pts = scatter_global_triangles(100.0, 100.0, 5.0, 4, 4, 0.2, 300, 3.0, 42);
        for &(x, y) in &pts {
            assert!(x >= 0.0 && x < 100.0, "x={x} out of domain");
            assert!(y >= 0.0 && y < 100.0, "y={y} out of domain");
        }
    }
}
