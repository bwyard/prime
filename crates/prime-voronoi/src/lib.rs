//! `prime-voronoi` — Voronoi diagrams and Lloyd relaxation.
//!
//! All public functions are pure (LOAD + COMPUTE only). No `&mut`, no side effects,
//! no hidden state.
//!
//! # Temporal Assembly Model
//! - **LOAD** — read parameters (query point, seed points, sample points)
//! - **COMPUTE** — pure geometry (distance comparisons, centroid sums)
//! - **APPEND** — return new state as owned value
//!
//! STORE and JUMP do not exist here.
//!
//! # Included
//! - `voronoi_nearest_2d` — find nearest seed index and F1 distance
//! - `voronoi_f1_f2_2d` — F1 (nearest) and F2 (second-nearest) distances
//! - `lloyd_relax_step_2d` — one Lloyd relaxation step (sample-based)
//!
//! # Deferred to post-release
//! - Delaunay triangulation (Bowyer-Watson) — complex algorithm (~200 lines),
//!   deferred until API surface is stable.

// ── Voronoi nearest ───────────────────────────────────────────────────────────

/// Find the nearest seed and its distance from `query`.
///
/// Returns `None` if `seeds` is empty.
///
/// # Math
///
/// ```text
/// (index, dist) = argmin_i  sqrt((query.x - seeds[i].x)² + (query.y - seeds[i].y)²)
/// ```
///
/// Squared distances are compared during the scan to avoid unnecessary square roots.
/// The final distance is returned as the true Euclidean distance.
///
/// # Arguments
/// * `query`  — point to query `(x, y)`
/// * `seeds`  — slice of `(x, y)` seed points
///
/// # Returns
/// `Some((index, distance))` of the nearest seed, or `None` if `seeds` is empty.
///
/// # Edge cases
/// * Empty `seeds` → `None`
/// * Single seed → always returns `(0, dist_to_seed_0)`
///
/// # Example
/// ```rust
/// use prime_voronoi::voronoi_nearest_2d;
/// let seeds = [(0.0_f32, 0.0), (1.0, 0.0), (0.0, 1.0)];
/// let (idx, dist) = voronoi_nearest_2d((0.1, 0.1), &seeds).unwrap();
/// assert_eq!(idx, 0);
/// assert!(dist < 0.2);
/// ```
pub fn voronoi_nearest_2d(query: (f32, f32), seeds: &[(f32, f32)]) -> Option<(usize, f32)> {
    seeds.iter().enumerate().fold(None, |best, (i, &(sx, sy))| {
        let dx = query.0 - sx;
        let dy = query.1 - sy;
        let d2 = dx * dx + dy * dy;
        match best {
            None => Some((i, d2)),
            Some((_bi, bd2)) if d2 < bd2 => Some((i, d2)),
            other => other,
        }
    })
    .map(|(i, d2)| (i, d2.sqrt()))
}

// ── Voronoi F1 + F2 ───────────────────────────────────────────────────────────

/// Compute F1 (nearest) and F2 (second-nearest) Euclidean distances from `query`.
///
/// Returns `None` if `seeds` is empty. Returns `(f1, f1)` if only one seed.
///
/// F1 and F2 together are used to compute cellular noise patterns and edge
/// detection on Voronoi diagrams.
///
/// # Math
///
/// ```text
/// F1 = min_i  dist(query, seeds[i])
/// F2 = min_i  dist(query, seeds[i])  where i ≠ argmin_j dist(query, seeds[j])
/// ```
///
/// # Arguments
/// * `query`  — query point `(x, y)`
/// * `seeds`  — slice of seed points
///
/// # Returns
/// `Some((f1, f2))` or `None` if empty.
///
/// # Example
/// ```rust
/// use prime_voronoi::voronoi_f1_f2_2d;
/// let seeds = [(0.0_f32, 0.0), (1.0, 0.0)];
/// let (f1, f2) = voronoi_f1_f2_2d((0.3, 0.0), &seeds).unwrap();
/// assert!(f1 < f2);
/// ```
pub fn voronoi_f1_f2_2d(query: (f32, f32), seeds: &[(f32, f32)]) -> Option<(f32, f32)> {
    if seeds.is_empty() {
        return None;
    }

    let (f1_d2, f2_d2) = seeds.iter().fold(
        (f32::MAX, f32::MAX),
        |(f1, f2), &(sx, sy)| {
            let dx = query.0 - sx;
            let dy = query.1 - sy;
            let d2 = dx * dx + dy * dy;
            if d2 < f1 {
                (d2, f1)
            } else if d2 < f2 {
                (f1, d2)
            } else {
                (f1, f2)
            }
        },
    );

    Some((f1_d2.sqrt(), f2_d2.min(f32::MAX).sqrt()))
}

// ── Lloyd relaxation ──────────────────────────────────────────────────────────

/// One step of sample-based Lloyd relaxation in 2-D.
///
/// Moves each seed toward the centroid of its Voronoi cell, estimated by
/// distributing a set of `samples` (e.g., a regular grid or stratified random
/// points) among the seeds by nearest-neighbor assignment.
///
/// Seeds with no samples assigned to them remain at their original position
/// (they are "empty cells" — this is the standard behaviour for boundary seeds
/// that are out-competed by neighbours).
///
/// # Math
///
/// ```text
/// For each sample s:
///   assign s to nearest seed j
///
/// For each seed i:
///   centroid_i = mean of all samples assigned to seed i
///   if no samples → keep original position
///
/// new_seeds[i] = centroid_i
/// ```
///
/// # Arguments
/// * `seeds`   — current seed positions `&[(x, y)]`
/// * `samples` — evaluation points used to estimate Voronoi cell centroids
///
/// # Returns
/// New seed positions (same length as `seeds`).
/// Returns an empty `Vec` if `seeds` is empty.
///
/// # Edge cases
/// * Empty `seeds` → `vec![]`
/// * Empty `samples` → all seeds unchanged
///
/// # Example
/// ```rust
/// use prime_voronoi::lloyd_relax_step_2d;
///
/// // Two seeds, 4 samples: samples cluster around (0,0) and (1,0)
/// let seeds = vec![(0.1_f32, 0.0), (0.9, 0.0)];
/// let samples = vec![(0.0, 0.0), (0.2, 0.0), (0.8, 0.0), (1.0, 0.0)];
/// let relaxed = lloyd_relax_step_2d(&seeds, &samples);
/// assert_eq!(relaxed.len(), 2);
/// assert!((relaxed[0].0 - 0.1).abs() < 0.5);
/// ```
pub fn lloyd_relax_step_2d(seeds: &[(f32, f32)], samples: &[(f32, f32)]) -> Vec<(f32, f32)> {
    if seeds.is_empty() {
        return vec![];
    }

    // Accumulate (sum_x, sum_y, count) per seed.
    let init: Vec<(f32, f32, u32)> = seeds.iter().map(|_| (0.0, 0.0, 0)).collect();

    let accum = samples.iter().fold(init, |mut acc, &(sx, sy)| {
        // Find nearest seed to this sample
        let nearest = seeds.iter().enumerate().fold(
            (0usize, f32::MAX),
            |(bi, bd2), (i, &(px, py))| {
                let dx = sx - px;
                let dy = sy - py;
                let d2 = dx * dx + dy * dy;
                if d2 < bd2 { (i, d2) } else { (bi, bd2) }
            },
        ).0;

        acc[nearest].0 += sx;
        acc[nearest].1 += sy;
        acc[nearest].2 += 1;
        acc
    });

    // Compute centroids; fall back to original seed if no samples assigned.
    accum
        .iter()
        .enumerate()
        .map(|(i, &(sx, sy, count))| {
            if count == 0 {
                seeds[i]
            } else {
                (sx / count as f32, sy / count as f32)
            }
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    // ── voronoi_nearest_2d ────────────────────────────────────────────────────

    #[test]
    fn voronoi_nearest_empty_is_none() {
        assert!(voronoi_nearest_2d((0.0, 0.0), &[]).is_none());
    }

    #[test]
    fn voronoi_nearest_single_seed() {
        let (idx, dist) = voronoi_nearest_2d((1.0, 0.0), &[(0.0, 0.0)]).unwrap();
        assert_eq!(idx, 0);
        assert!((dist - 1.0).abs() < EPSILON);
    }

    #[test]
    fn voronoi_nearest_selects_closest() {
        let seeds = [(0.0_f32, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let (idx, _) = voronoi_nearest_2d((0.1, 0.1), &seeds).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn voronoi_nearest_boundary_case() {
        // Query exactly on seed 1
        let seeds = [(0.0_f32, 0.0), (1.0, 0.0)];
        let (idx, dist) = voronoi_nearest_2d((1.0, 0.0), &seeds).unwrap();
        assert_eq!(idx, 1);
        assert!(dist.abs() < EPSILON);
    }

    #[test]
    fn voronoi_nearest_deterministic() {
        let seeds = [(0.0_f32, 0.0), (1.0, 0.0), (0.5, 0.5)];
        let a = voronoi_nearest_2d((0.3, 0.3), &seeds);
        let b = voronoi_nearest_2d((0.3, 0.3), &seeds);
        assert_eq!(a, b);
    }

    // ── voronoi_f1_f2_2d ──────────────────────────────────────────────────────

    #[test]
    fn voronoi_f1_f2_empty_is_none() {
        assert!(voronoi_f1_f2_2d((0.0, 0.0), &[]).is_none());
    }

    #[test]
    fn voronoi_f1_f2_two_seeds_ordering() {
        let seeds = [(0.0_f32, 0.0), (1.0, 0.0)];
        let (f1, f2) = voronoi_f1_f2_2d((0.3, 0.0), &seeds).unwrap();
        assert!(f1 < f2, "f1={f1} f2={f2}");
        assert!((f1 - 0.3).abs() < EPSILON, "f1={f1}");
        assert!((f2 - 0.7).abs() < EPSILON, "f2={f2}");
    }

    #[test]
    fn voronoi_f1_f2_deterministic() {
        let seeds = [(0.0_f32, 0.0), (1.0, 0.0), (0.5, 0.5)];
        let a = voronoi_f1_f2_2d((0.3, 0.3), &seeds);
        let b = voronoi_f1_f2_2d((0.3, 0.3), &seeds);
        assert_eq!(a, b);
    }

    // ── lloyd_relax_step_2d ───────────────────────────────────────────────────

    #[test]
    fn lloyd_relax_empty_seeds() {
        assert!(lloyd_relax_step_2d(&[], &[(0.5, 0.5)]).is_empty());
    }

    #[test]
    fn lloyd_relax_no_samples_unchanged() {
        let seeds = vec![(0.0_f32, 0.0), (1.0, 0.0)];
        let relaxed = lloyd_relax_step_2d(&seeds, &[]);
        assert_eq!(relaxed[0], seeds[0]);
        assert_eq!(relaxed[1], seeds[1]);
    }

    #[test]
    fn lloyd_relax_preserves_length() {
        let seeds: Vec<(f32, f32)> = (0..5).map(|i| (i as f32 * 0.2, 0.0)).collect();
        let samples: Vec<(f32, f32)> = (0..100).map(|i| (i as f32 * 0.01, 0.0)).collect();
        let relaxed = lloyd_relax_step_2d(&seeds, &samples);
        assert_eq!(relaxed.len(), seeds.len());
    }

    #[test]
    fn lloyd_relax_two_seeds_move_toward_centroids() {
        // Seeds at 0.1 and 0.9; symmetric samples in [0,1] on x-axis
        let seeds = vec![(0.1_f32, 0.0), (0.9_f32, 0.0)];
        let samples: Vec<(f32, f32)> = (0..=100).map(|i| (i as f32 / 100.0, 0.0)).collect();
        let relaxed = lloyd_relax_step_2d(&seeds, &samples);
        // Each seed should move toward 0.25 and 0.75 (centroids of [0,0.5] and [0.5,1])
        assert!((relaxed[0].0 - 0.25).abs() < 0.05, "seed0={}", relaxed[0].0);
        assert!((relaxed[1].0 - 0.75).abs() < 0.05, "seed1={}", relaxed[1].0);
    }

    #[test]
    fn lloyd_relax_deterministic() {
        let seeds = vec![(0.1_f32, 0.2), (0.8_f32, 0.7)];
        let samples: Vec<(f32, f32)> = (0..5).flat_map(|i| {
            (0..5).map(move |j| (i as f32 * 0.25, j as f32 * 0.25))
        }).collect();
        let a = lloyd_relax_step_2d(&seeds, &samples);
        let b = lloyd_relax_step_2d(&seeds, &samples);
        assert_eq!(a, b);
    }
}
