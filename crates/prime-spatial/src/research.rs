//! Research comparison implementations — **not for production use outside this investigation.**
//!
//! Functions in this module are comparison baselines for the scatter-cull hypothesis.
//! They may use internal mutation, convergence loops, or other patterns that do not
//! conform to PRIME's pure functional contract. Each function is annotated with why
//! it is here rather than in the main API.
//!
//! See `docs/research/parallel-spatial-sampling.md` for context.

use prime_random::prng_next;

// ── Wei 2008 — Phase-Based Parallel Poisson Disk Sampling ────────────────────
//
// RESEARCH NOTE: Wei's algorithm requires a convergence loop (run phases until
// no new point is placed in a full pass). This is data-dependent termination
// that cannot be expressed as a fixed-length fold without an arbitrary bound.
// It also requires a shared mutable global acceptance grid across all phases.
// These properties make a pure external contract impossible without the same
// grid-cloning cost that motivated the Bridson rewrite.
//
// Placed here, not in lib.rs, because:
//   (1) It exists purely as a research comparison baseline
//   (2) Its internal structure does not meet PRIME's production code standards
//   (3) It should never be used outside of benchmarking against scatter-cull

/// Wei's phase-based parallel Poisson disk sampling (Wei 2008).
///
/// **Research comparison only.** Do not use outside benchmarking.
///
/// # Math
///
/// Divides the domain into tiles of size $T = 2 \cdot d_{min}$. Tiles are coloured
/// with a 2×2 checkerboard (4 phases). All tiles in the same phase are separated
/// by $\geq d_{min}$, so they are conflict-free and can be processed in parallel.
///
/// Each pass attempts up to `max_attempts` dart throws per tile per phase.
/// Passes repeat until convergence — no point placed in a full 4-phase pass.
///
/// This single-threaded implementation preserves Wei's phase structure exactly.
/// In a parallel build, tiles within each phase would use `par_iter`.
///
/// # ADVANCE-EXCEPTION
/// The outer convergence loop has data-dependent termination — bounds are not
/// known in advance. Internal mutation (grid, points Vec) does not escape the
/// function. External contract: same inputs → same outputs.
///
/// # Arguments
/// * `width`, `height`   — domain dimensions (must be > 0)
/// * `min_dist`          — minimum distance between any two accepted points
/// * `max_attempts`      — dart throws per tile per pass (30 standard)
/// * `seed`              — deterministic seed
///
/// # Returns
/// `Vec<(f32, f32)>` of accepted points in world coordinates. Empty on invalid input.
///
/// # Example
/// ```rust
/// # use prime_spatial::research::poisson_disk_wei;
/// let pts = poisson_disk_wei(100.0, 100.0, 5.0, 30, 42);
/// assert!(!pts.is_empty());
/// for i in 0..pts.len() {
///     for j in (i+1)..pts.len() {
///         let dx = pts[i].0 - pts[j].0;
///         let dy = pts[i].1 - pts[j].1;
///         assert!((dx*dx + dy*dy).sqrt() >= 5.0 - 1e-4);
///     }
/// }
/// ```
pub fn poisson_disk_wei(
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
    seed: u32,
) -> Vec<(f32, f32)> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 {
        return Vec::new();
    }

    let tile_size   = min_dist * 2.0;
    let tile_cols   = (width  / tile_size).ceil() as usize + 1;
    let tile_rows   = (height / tile_size).ceil() as usize + 1;

    let cell_size   = min_dist / 2.0_f32.sqrt();
    let grid_cols   = (width  / cell_size).ceil() as usize + 1;
    let grid_rows   = (height / cell_size).ceil() as usize + 1;
    let min_dist_sq = min_dist * min_dist;

    // ADVANCE-EXCEPTION: convergence loop — terminates when no point is placed in a full pass.
    // Internal mutation (grid, points) does not escape. Same inputs → same outputs.
    let mut grid:   Vec<Option<usize>> = vec![None; grid_cols * grid_rows];
    let mut points: Vec<(f32, f32)>    = Vec::new();
    let mut s = seed;

    loop {
        let mut placed_this_pass = false;

        // 4 phases: (even_col, even_row), (odd_col, even_row),
        //           (even_col, odd_row),  (odd_col, odd_row)
        for phase in 0..4u32 {
            let phase_col_parity = (phase & 1) as usize;
            let phase_row_parity = ((phase >> 1) & 1) as usize;

            for tr in (phase_row_parity..tile_rows).step_by(2) {
                for tc in (phase_col_parity..tile_cols).step_by(2) {
                    let tx_start = tc as f32 * tile_size;
                    let ty_start = tr as f32 * tile_size;
                    let tx_end   = (tx_start + tile_size).min(width);
                    let ty_end   = (ty_start + tile_size).min(height);
                    let tw = tx_end - tx_start;
                    let th = ty_end - ty_start;
                    if tw <= 0.0 || th <= 0.0 { continue; }

                    let tile_idx = tr * tile_cols + tc;
                    let mut tile_s = s
                        .wrapping_mul(1_664_525u32)
                        .wrapping_add(tile_idx as u32);

                    for _ in 0..max_attempts {
                        let (xf, s1) = prng_next(tile_s);
                        let (yf, s2) = prng_next(s1);
                        tile_s = s2;

                        let cx = tx_start + xf * tw;
                        let cy = ty_start + yf * th;

                        if cx < 0.0 || cx >= width || cy < 0.0 || cy >= height {
                            continue;
                        }

                        let gcx = (cx / cell_size) as usize;
                        let gcy = (cy / cell_size) as usize;

                        let too_close = (gcy.saturating_sub(2)..(gcy + 3).min(grid_rows))
                            .flat_map(|gy| {
                                (gcx.saturating_sub(2)..(gcx + 3).min(grid_cols))
                                    .map(move |gx| (gx, gy))
                            })
                            .filter_map(|(gx, gy)| grid[gy * grid_cols + gx])
                            .any(|pi| {
                                let (qx, qy) = points[pi];
                                let dx = cx - qx;
                                let dy = cy - qy;
                                dx * dx + dy * dy < min_dist_sq
                            });

                        if !too_close {
                            let new_idx = points.len();
                            grid[gcy * grid_cols + gcx] = Some(new_idx);
                            points.push((cx, cy));
                            placed_this_pass = true;
                            break;
                        }
                    }

                    // Advance global seed per tile to vary draws across passes
                    let (_, s_next) = prng_next(s.wrapping_add(tile_idx as u32));
                    s = s_next;
                }
            }
        }

        if !placed_this_pass { break; }
    }

    points
}
