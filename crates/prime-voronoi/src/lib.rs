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
//! - `delaunay_2d` — Delaunay triangulation via Bowyer-Watson

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

// ── Delaunay triangulation ────────────────────────────────────────────────────

/// Circumcircle test: is point `(px, py)` strictly inside the circumcircle of
/// triangle `(ax,ay)`, `(bx,by)`, `(cx,cy)`?
///
/// Orientation-independent: works for both CW and CCW triangles by checking
/// the sign of the cross product and adjusting accordingly.
///
/// # Math
/// ```text
/// | dx  dy  dx²+dy² |
/// | ex  ey  ex²+ey² | > 0  ⟹  point inside circumcircle (CCW triangle)
/// | fx  fy  fx²+fy² |
/// ```
/// where `(dx,dy) = a - p`, `(ex,ey) = b - p`, `(fx,fy) = c - p`.
/// For CW triangles the sign is flipped.
#[allow(clippy::too_many_arguments)]
pub(crate) fn in_circumcircle(
    px: f32, py: f32,
    ax: f32, ay: f32,
    bx: f32, by: f32,
    cx: f32, cy: f32,
) -> bool {
    let dx = ax - px;
    let dy = ay - py;
    let ex = bx - px;
    let ey = by - py;
    let fx = cx - px;
    let fy = cy - py;

    let dx2_dy2 = dx * dx + dy * dy;
    let ex2_ey2 = ex * ex + ey * ey;
    let fx2_fy2 = fx * fx + fy * fy;

    let det = dx * (ey * fx2_fy2 - fy * ex2_ey2)
            - dy * (ex * fx2_fy2 - fx * ex2_ey2)
            + dx2_dy2 * (ex * fy - ey * fx);

    // Check triangle orientation (sign of cross product)
    let orient = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);

    // If CCW (orient > 0): det > 0 means inside
    // If CW  (orient < 0): det < 0 means inside
    if orient > 0.0 { det > 0.0 } else { det < 0.0 }
}

/// Delaunay triangulation via Bowyer-Watson. Returns list of triangle index triples.
///
/// Each triangle is `(i, j, k)` where `i`, `j`, `k` are indices into the input `points` slice.
/// Points on the super-triangle boundary are excluded from the output.
///
/// # Math
/// Bowyer-Watson incrementally inserts points, removing triangles whose circumcircle
/// contains the new point, then re-triangulates the resulting polygonal hole.
///
/// # Example
/// ```rust
/// use prime_voronoi::delaunay_2d;
/// let points = vec![(0.0_f32, 0.0), (1.0, 0.0), (0.5, 1.0)];
/// let tris = delaunay_2d(&points);
/// assert_eq!(tris.len(), 1);
/// // Triangle indices reference the input points (order may vary)
/// let (a, b, c) = tris[0];
/// assert!(a < 3 && b < 3 && c < 3);
/// assert_ne!(a, b);
/// assert_ne!(b, c);
/// ```
pub fn delaunay_2d(points: &[(f32, f32)]) -> Vec<(usize, usize, usize)> {
    if points.len() < 3 {
        return vec![];
    }

    // ADVANCE-EXCEPTION: Bowyer-Watson requires triangle set mutation.
    // Internal only — public API is pure: &[(f32,f32)] -> Vec<(usize,usize,usize)>

    // Compute bounding box
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in points {
        if x < min_x { min_x = x; }
        if y < min_y { min_y = y; }
        if x > max_x { max_x = x; }
        if y > max_y { max_y = y; }
    }

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let d_max = dx.max(dy).max(1e-6);
    let mid_x = (min_x + max_x) * 0.5;
    let mid_y = (min_y + max_y) * 0.5;

    // Super-triangle vertices (indices: n, n+1, n+2)
    let n = points.len();
    let s0 = (mid_x - 20.0 * d_max, mid_y - d_max);
    let s1 = (mid_x, mid_y + 20.0 * d_max);
    let s2 = (mid_x + 20.0 * d_max, mid_y - d_max);

    // Extended point list: original points + 3 super-triangle vertices
    let mut all_points: Vec<(f32, f32)> = points.to_vec();
    all_points.push(s0);
    all_points.push(s1);
    all_points.push(s2);

    // Start with super-triangle
    let mut triangles: Vec<(usize, usize, usize)> = vec![(n, n + 1, n + 2)];

    for i in 0..n {
        let (px, py) = all_points[i];

        // Find bad triangles (circumcircle contains point i)
        let mut bad: Vec<usize> = Vec::new();
        for (t, &(a, b, c)) in triangles.iter().enumerate() {
            let (ax, ay) = all_points[a];
            let (bx, by) = all_points[b];
            let (cx, cy) = all_points[c];
            if in_circumcircle(px, py, ax, ay, bx, by, cx, cy) {
                bad.push(t);
            }
        }

        // Find boundary polygon (edges that appear in exactly one bad triangle)
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for &t in &bad {
            let (a, b, c) = triangles[t];
            let tri_edges = [(a, b), (b, c), (c, a)];
            for &(e0, e1) in &tri_edges {
                // Check if this edge is shared with another bad triangle
                let shared = bad.iter().any(|&other| {
                    other != t && {
                        let (oa, ob, oc) = triangles[other];
                        let other_edges = [(oa, ob), (ob, oc), (oc, oa)];
                        other_edges.iter().any(|&(o0, o1)| {
                            (e0 == o0 && e1 == o1) || (e0 == o1 && e1 == o0)
                        })
                    }
                });
                if !shared {
                    edges.push((e0, e1));
                }
            }
        }

        // Remove bad triangles (reverse order to preserve indices)
        bad.sort_unstable();
        for &t in bad.iter().rev() {
            triangles.swap_remove(t);
        }

        // Create new triangles from boundary edges to inserted point
        for &(e0, e1) in &edges {
            triangles.push((i, e0, e1));
        }
    }

    // Remove triangles that reference super-triangle vertices
    triangles
        .into_iter()
        .filter(|&(a, b, c)| a < n && b < n && c < n)
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

    // ── delaunay_2d ──────────────────────────────────────────────────────────

    #[test]
    fn delaunay_single_triangle() {
        let pts = vec![(0.0_f32, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let tris = delaunay_2d(&pts);
        assert_eq!(tris.len(), 1);
    }

    #[test]
    fn delaunay_four_points_two_triangles() {
        let pts = vec![(0.0_f32, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let tris = delaunay_2d(&pts);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn delaunay_empty() {
        let pts: Vec<(f32, f32)> = vec![];
        let tris = delaunay_2d(&pts);
        assert!(tris.is_empty());
    }

    #[test]
    fn delaunay_two_points() {
        let pts = vec![(0.0_f32, 0.0), (1.0, 0.0)];
        let tris = delaunay_2d(&pts);
        assert!(tris.is_empty());
    }

    #[test]
    fn delaunay_circumcircle_property() {
        // For every Delaunay triangle, no other point should be inside its circumcircle
        let pts = vec![(0.0_f32, 0.0), (4.0, 0.0), (2.0, 3.0), (1.0, 1.0), (3.0, 1.0)];
        let tris = delaunay_2d(&pts);
        assert!(!tris.is_empty());
        for &(i, j, k) in &tris {
            let (ax, ay) = pts[i];
            let (bx, by) = pts[j];
            let (cx, cy) = pts[k];
            for (m, &(px, py)) in pts.iter().enumerate() {
                if m == i || m == j || m == k { continue; }
                assert!(
                    !in_circumcircle(px, py, ax, ay, bx, by, cx, cy),
                    "point {m} inside circumcircle of triangle ({i},{j},{k})"
                );
            }
        }
    }

    #[test]
    fn delaunay_deterministic() {
        let pts = vec![(0.0_f32, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)];
        let a = delaunay_2d(&pts);
        let b = delaunay_2d(&pts);
        assert_eq!(a, b);
    }
}
