//! Sampling strategies: Poisson-disk scatter-cull over various cell geometries.
//!
//! All strategies share the same two-phase pipeline:
//!   1. Scatter — pure random drop of candidates
//!   2. Cull    — spatial acceptance grid enforcing min-dist
//!
//! Shared helper `cull_to_min_dist` is `pub(crate)` for use by all submodules.
//! `global_cull_to_min_dist` is the public single-pass variant over a full domain.

pub mod rect;
pub mod voronoi;
pub mod half_heart;
pub mod sheared;
pub mod sdf;
pub mod triangle;

pub use rect::*;
pub use voronoi::*;
pub use half_heart::*;
pub use sheared::*;
pub use sdf::*;
pub use triangle::*;

// Shared: build a spatial acceptance grid and cull a point set down to min-dist validity.
// Used by scatter_cull_rect's cull phase. Returns the surviving points.
// ADVANCE-EXCEPTION: cull loop terminates when all candidates are processed — bounded.
pub(crate) fn cull_to_min_dist(
    candidates: Vec<(f32, f32)>,
    x_start: f32,
    y_start: f32,
    width: f32,
    height: f32,
    min_dist: f32,
) -> Vec<(f32, f32)> {
    let cell_size   = min_dist / 2.0_f32.sqrt();
    let cols        = (width  / cell_size).ceil() as usize + 1;
    let rows        = (height / cell_size).ceil() as usize + 1;
    let min_dist_sq = min_dist * min_dist;

    let mut grid:     Vec<Option<usize>> = vec![None; cols * rows];
    let mut accepted: Vec<(f32, f32)>    = Vec::new();

    for (wx, wy) in candidates {
        // Translate to local partition coordinates for grid indexing
        let lx = wx - x_start;
        let ly = wy - y_start;

        if lx < 0.0 || lx >= width || ly < 0.0 || ly >= height {
            continue;
        }

        let gcx = (lx / cell_size) as usize;
        let gcy = (ly / cell_size) as usize;

        let too_close = (gcy.saturating_sub(2)..(gcy + 3).min(rows))
            .flat_map(|gy| (gcx.saturating_sub(2)..(gcx + 3).min(cols))
                .map(move |gx| (gx, gy)))
            .filter_map(|(gx, gy)| grid[gy * cols + gx])
            .any(|pi| {
                let (qx, qy) = accepted[pi];
                let dx = wx - qx;
                let dy = wy - qy;
                dx * dx + dy * dy < min_dist_sq
            });

        if !too_close {
            let idx      = accepted.len();
            grid[gcy * cols + gcx] = Some(idx);
            accepted.push((wx, wy));
        }
    }

    accepted
}

// ── Global min-dist cull ──────────────────────────────────────────────────────
//
// All scatter-cull approaches guarantee min-dist WITHIN each cell only.
// Cross-cell seam violations exist because cells are culled independently.
//
// To produce output equivalent to serial Bridson (global min-dist), apply
// global_cull_to_min_dist as a final pass over the flattened point set.
//
// The seam violation rate (points removed by global cull / total intra-cell points)
// is a research metric — it measures how well each approach's geometry contains
// boundary conflicts within the overage surplus.

/// Apply a global min-dist cull to a flat set of points.
///
/// Scans points in input order, accepting each if it is at least `min_dist`
/// from all previously accepted points. Uses the same O(1)-per-point spatial
/// grid as the per-cell cull.
///
/// Use this as a final pass after any scatter-cull approach to guarantee
/// global min-dist — equivalent to what `poisson_disk` produces, without
/// the sequential constraint that makes Bridson slow.
///
/// # Math
///
/// Same acceptance grid as `cull_to_min_dist`: cell size $c = d / \sqrt{2}$.
/// For each candidate $p$: accept iff
/// $$\forall q \in \text{accepted}: \lVert p - q \rVert \geq d_{\min}$$
///
/// # Arguments
/// * `points`    — candidate points in any order; typically the flattened output
///                 of a scatter-cull function
/// * `width`, `height` — domain dimensions
/// * `min_dist`  — global minimum distance constraint
///
/// # Returns
/// Subset of `points` satisfying global min-dist. Order matches input order.
///
/// # Example
/// ```rust
/// use prime_spatial::{scatter_cull_rect, global_cull_to_min_dist};
/// let cells = scatter_cull_rect(100.0, 100.0, 5.0, 4, 4, 30, 1.5, 42);
/// let flat: Vec<_> = cells.into_iter().flatten().collect();
/// let global = global_cull_to_min_dist(flat, 100.0, 100.0, 5.0);
/// // global satisfies min-dist across ALL points, not just within cells
/// for i in 0..global.len() {
///     for j in (i+1)..global.len() {
///         let dx = global[i].0 - global[j].0;
///         let dy = global[i].1 - global[j].1;
///         assert!((dx*dx + dy*dy).sqrt() >= 5.0 - 1e-4);
///     }
/// }
/// ```
pub fn global_cull_to_min_dist(
    points: Vec<(f32, f32)>,
    width: f32,
    height: f32,
    min_dist: f32,
) -> Vec<(f32, f32)> {
    cull_to_min_dist(points, 0.0, 0.0, width, height, min_dist)
}
