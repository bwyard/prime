//! prime-random — Seeded deterministic randomness.
//!
//! All public functions are LOAD + COMPUTE. No STORE. No JUMP. No exceptions.
//!
//! Thesis: the seed IS the thread. Who holds the seed controls who can
//! advance it. Consent revocation = stop threading the seed forward.
//! No DELETE needed — the sequence is causally inert without its key.

use std::f32::consts::PI;

// ── Pure PRNG primitives ─────────────────────────────────────────────────────

/// Mulberry32 pure step — LOAD + COMPUTE only.
///
/// # Math
///   z0 = (seed + 0x6D2B79F5) mod 2^32
///   z1 = (z0 XOR (z0 >> 15)) * (z0 | 1)
///   z2 = z1 XOR (z1 + (z1 XOR (z1 >> 7)) * (z1 | 61))
///   out = (z2 XOR (z2 >> 14)) / 2^32
///
/// Same algorithm as TypeScript prime-random for cross-language parity.
/// Same seed → same sequence in both languages.
///
/// # Arguments
/// * `seed` - Current thread position (u32)
///
/// # Returns
/// `(value, next_seed)` — thread next_seed forward to continue.
///
/// # Example
/// ```rust
/// use prime_random::prng_next;
/// let (v1, s1) = prng_next(42);
/// let (v2, s2) = prng_next(s1);
/// assert!(v1 >= 0.0 && v1 < 1.0);
/// assert!(v2 >= 0.0 && v2 < 1.0);
/// ```
pub fn prng_next(seed: u32) -> (f32, u32) {
    let z0 = seed.wrapping_add(0x6D2B79F5);
    let z1 = (z0 ^ (z0 >> 15)).wrapping_mul(z0 | 1);
    let z2 = z1 ^ z1.wrapping_add((z1 ^ (z1 >> 7)).wrapping_mul(z1 | 61));
    let value = (z2 ^ (z2 >> 14)) as f64 / 4_294_967_296.0;
    (value as f32, z0)
}

/// Pure float in [min, max). Returns (value, next_seed).
///
/// # Example
/// ```rust
/// use prime_random::prng_range_f32;
/// let (v, _s1) = prng_range_f32(0, 2.0, 5.0);
/// assert!(v >= 2.0 && v < 5.0);
/// ```
pub fn prng_range_f32(seed: u32, min: f32, max: f32) -> (f32, u32) {
    if min >= max { return (min, seed); }
    let (v, next) = prng_next(seed);
    (min + v * (max - min), next)
}

/// Pure usize in [0, n). Returns (value, next_seed).
///
/// # Example
/// ```rust
/// use prime_random::prng_range_usize;
/// let (i, _s1) = prng_range_usize(0, 10);
/// assert!(i < 10);
/// ```
pub fn prng_range_usize(seed: u32, n: usize) -> (usize, u32) {
    let (v, next) = prng_next(seed);
    ((v * n as f32) as usize, next)
}

/// Pure bool with probability p of true. Returns (value, next_seed).
///
/// # Example
/// ```rust
/// use prime_random::prng_bool;
/// let (b, _s1) = prng_bool(0, 1.0);
/// assert!(b);
/// ```
pub fn prng_bool(seed: u32, p: f32) -> (bool, u32) {
    let (v, next) = prng_next(seed);
    (v < p.clamp(0.0, 1.0), next)
}

/// Pure Fisher-Yates shuffle — returns new Vec, original unchanged.
///
/// # Math
/// For i from n-1 down to 1: j = randInt(0, i+1); swap(arr[i], arr[j])
/// Every permutation equally probable. O(n).
///
/// # Returns
/// (shuffled_vec, next_seed)
///
/// # Example
/// ```rust
/// use prime_random::prng_shuffled;
/// let v = vec![1, 2, 3, 4, 5];
/// let (s, _seed) = prng_shuffled(0, &v);
/// assert_eq!(s.len(), 5);
/// assert_eq!(v, vec![1, 2, 3, 4, 5]); // original unchanged
/// ```
pub fn prng_shuffled<T: Clone>(seed: u32, slice: &[T]) -> (Vec<T>, u32) {
    (1..slice.len()).rev().fold(
        (slice.to_vec(), seed),
        |(mut out, s), i| {
            let (j, next) = prng_range_usize(s, i + 1);
            out.swap(i, j);
            (out, next)
        },
    )
}

/// Pure random element from slice. Returns (Some(&element), next_seed) or (None, seed).
///
/// # Example
/// ```rust
/// use prime_random::prng_choose;
/// let v = vec!["a", "b", "c"];
/// let (pick, _s1) = prng_choose(0, &v);
/// assert!(pick.is_some());
/// ```
pub fn prng_choose<T>(seed: u32, slice: &[T]) -> (Option<&T>, u32) {
    if slice.is_empty() { return (None, seed); }
    let (i, next) = prng_range_usize(seed, slice.len());
    (Some(&slice[i]), next)
}

// ── Pure higher-order functions ───────────────────────────────────────────────

/// Weighted random choice — O(n) linear scan. Pure LOAD + COMPUTE.
///
/// # Math
/// Sample u ~ Uniform(0, sum(weights)).
/// Walk weights; return first index where cumulative sum ≥ u.
///
/// # Arguments
/// * `seed` - Thread position
/// * `weights` - Non-negative f32 weights. Must sum > 0.
///
/// # Returns
/// (chosen_index, next_seed)
///
/// # Edge cases
/// * Empty → (0, seed)
/// * All zero → (last_index, seed)
///
/// # Example
/// ```rust
/// use prime_random::weighted_choice;
/// let (i, _s1) = weighted_choice(0, &[1.0, 2.0, 1.0]);
/// assert!(i < 3);
/// ```
pub fn weighted_choice(seed: u32, weights: &[f32]) -> (usize, u32) {
    if weights.is_empty() { return (0, seed); }
    let total: f32 = weights.iter().sum();
    if total <= 0.0 { return (weights.len() - 1, seed); }
    let (u, s1) = prng_range_f32(seed, 0.0, total);
    let result = weights.iter().enumerate().try_fold(u, |remaining, (i, &w)| {
        let r = remaining - w;
        if r <= 0.0 { Err(i) } else { Ok(r) }
    });
    let idx = match result {
        Err(i) => i,
        Ok(_) => weights.len() - 1,
    };
    (idx, s1)
}

// ── Pure Bridson ──────────────────────────────────────────────────────────────

struct BridsonParams {
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
    cols: usize,
    rows: usize,
    cell_size: f32,
}

struct BridsonState {
    grid: Vec<Option<(f32, f32)>>,
    active: Vec<usize>,
    points: Vec<(f32, f32)>,
    seed: u32,
}

fn bridson_too_close(x: f32, y: f32, grid: &[Option<(f32, f32)>], p: &BridsonParams) -> bool {
    let cx = (x / p.cell_size) as usize;
    let cy = (y / p.cell_size) as usize;
    let r = 2usize;
    let x0 = cx.saturating_sub(r);
    let y0 = cy.saturating_sub(r);
    let x1 = (cx + r + 1).min(p.cols);
    let y1 = (cy + r + 1).min(p.rows);
    (y0..y1).any(|gy|
        (x0..x1).any(|gx|
            grid[gy * p.cols + gx].is_some_and(|(px, py)| {
                let dx = x - px;
                let dy = y - py;
                dx * dx + dy * dy < p.min_dist * p.min_dist
            })
        )
    )
}

fn bridson_step(state: BridsonState, p: &BridsonParams) -> BridsonState {
    if state.active.is_empty() { return state; }

    let (ai_f, s1) = prng_next(state.seed);
    let ai = (ai_f * state.active.len() as f32) as usize;
    let (ax, ay) = state.points[state.active[ai]];

    let (candidate, final_seed) = (0..p.max_attempts).fold(
        (None::<(f32, f32)>, s1),
        |(found, s), _| {
            if found.is_some() { return (found, s); }
            let (angle_f, s2) = prng_next(s);
            let (dist_f, s3) = prng_next(s2);
            let angle = angle_f * PI * 2.0;
            let dist = p.min_dist + dist_f * p.min_dist;
            let cx = ax + angle.cos() * dist;
            let cy = ay + angle.sin() * dist;
            if cx < 0.0 || cx >= p.width || cy < 0.0 || cy >= p.height {
                return (None, s3);
            }
            if bridson_too_close(cx, cy, &state.grid, p) {
                return (None, s3);
            }
            (Some((cx, cy)), s3)
        },
    );

    if let Some((cx, cy)) = candidate {
        let cell_idx = (cy / p.cell_size) as usize * p.cols + (cx / p.cell_size) as usize;
        let mut new_grid = state.grid.clone();
        new_grid[cell_idx] = Some((cx, cy));
        let mut new_active = state.active.clone();
        new_active.push(state.points.len());
        let mut new_points = state.points.clone();
        new_points.push((cx, cy));
        BridsonState { grid: new_grid, active: new_active, points: new_points, seed: final_seed }
    } else {
        let new_active: Vec<usize> = state.active.iter().enumerate()
            .filter(|(i, _)| *i != ai)
            .map(|(_, &v)| v)
            .collect();
        BridsonState { grid: state.grid, active: new_active, points: state.points, seed: final_seed }
    }
}

/// Poisson disk sampling — minimum distance spacing in 2D. Pure LOAD + COMPUTE.
///
/// # Math
/// Bridson's algorithm (2007) expressed as a pure state fold (ADVANCE).
/// Each step is (state) → new_state. No mutable shared state.
///
/// Performance note: each step clones the spatial grid O(cols×rows).
/// Typical game domains (< 2000×2000, min_dist > 5) are negligible.
///
/// # Arguments
/// * `seed` - Thread position — same seed → same distribution
/// * `width` - Sampling domain width
/// * `height` - Sampling domain height
/// * `min_dist` - Minimum distance between any two points
/// * `max_attempts` - Candidates per active point (30 is standard)
///
/// # Returns
/// Vec of (x, y) point pairs, all at least `min_dist` apart.
///
/// # Example
/// ```rust
/// use prime_random::poisson_disk_2d;
/// let pts = poisson_disk_2d(42, 100.0, 100.0, 10.0, 30);
/// assert!(!pts.is_empty());
/// ```
pub fn poisson_disk_2d(
    seed: u32,
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
) -> Vec<(f32, f32)> {
    let cell_size = min_dist / 2.0_f32.sqrt();
    let cols = (width / cell_size).ceil() as usize + 1;
    let rows = (height / cell_size).ceil() as usize + 1;
    let p = BridsonParams { width, height, min_dist, max_attempts, cols, rows, cell_size };

    let (x0f, s1) = prng_next(seed);
    let (y0f, s2) = prng_next(s1);
    let x0 = x0f * width;
    let y0 = y0f * height;
    let cell_idx0 = (y0 / cell_size) as usize * cols + (x0 / cell_size) as usize;

    let initial_grid: Vec<Option<(f32, f32)>> = (0..cols * rows)
        .map(|i| if i == cell_idx0 { Some((x0, y0)) } else { None })
        .collect();

    let initial = BridsonState {
        grid: initial_grid,
        active: vec![0],
        points: vec![(x0, y0)],
        seed: s2,
    };

    // Upper bound: each point added once and removed from active once.
    let max_points = ((width * height) / (PI * (min_dist / 2.0).powi(2))).ceil() as usize * 4;
    let max_steps = max_points * 2;

    let final_state = (0..max_steps).fold(initial, |state, _| {
        if state.active.is_empty() { state } else { bridson_step(state, &p) }
    });

    final_state.points
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── prng_next ────────────────────────────────────────────────────────────

    #[test]
    fn prng_next_in_range() {
        (0..1000u32).for_each(|i| {
            let (v, _) = prng_next(i);
            assert!(v >= 0.0 && v < 1.0, "prng_next({i}) = {v} out of [0,1)");
        });
    }

    #[test]
    fn prng_next_same_seed_same_value() {
        let (a, _) = prng_next(42);
        let (b, _) = prng_next(42);
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn prng_next_threads_deterministically() {
        let seq_a: Vec<f32> = (0..10).fold((vec![], 99u32), |(mut v, s), _| {
            let (val, next) = prng_next(s);
            v.push(val);
            (v, next)
        }).0;
        let seq_b: Vec<f32> = (0..10).fold((vec![], 99u32), |(mut v, s), _| {
            let (val, next) = prng_next(s);
            v.push(val);
            (v, next)
        }).0;
        assert_eq!(seq_a, seq_b);
    }

    // ── weighted_choice ───────────────────────────────────────────────────────

    #[test]
    fn weighted_choice_empty_returns_zero() {
        let (idx, _) = weighted_choice(0, &[]);
        assert_eq!(idx, 0);
    }

    #[test]
    fn weighted_choice_single_element() {
        (0..10u32).for_each(|i| {
            let (idx, _) = weighted_choice(i, &[1.0]);
            assert_eq!(idx, 0);
        });
    }

    #[test]
    fn weighted_choice_all_weight_on_one() {
        (0..20u32).for_each(|i| {
            let (idx, _) = weighted_choice(i, &[0.0, 0.0, 1.0, 0.0]);
            assert_eq!(idx, 2);
        });
    }

    #[test]
    fn weighted_choice_distribution_matches_weights() {
        let n = 10000usize;
        let counts = (0..n).fold(([0usize; 3], 777u32), |(mut acc, s), _| {
            let (idx, next) = weighted_choice(s, &[1.0, 2.0, 1.0]);
            acc[idx] += 1;
            (acc, next)
        }).0;
        let tolerance = (n as f32 * 0.05) as usize;
        assert!((counts[0] as isize - (n / 4) as isize).unsigned_abs() < tolerance);
        assert!((counts[1] as isize - (n / 2) as isize).unsigned_abs() < tolerance);
        assert!((counts[2] as isize - (n / 4) as isize).unsigned_abs() < tolerance);
    }

    // ── poisson_disk_2d ───────────────────────────────────────────────────────

    #[test]
    fn poisson_disk_min_distance_satisfied() {
        let min_dist = 10.0f32;
        let pts = poisson_disk_2d(42, 100.0, 100.0, min_dist, 30);
        assert!(!pts.is_empty());
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let dx = pts[i].0 - pts[j].0;
                let dy = pts[i].1 - pts[j].1;
                assert!((dx * dx + dy * dy).sqrt() >= min_dist - 1e-4);
            }
        }
    }

    #[test]
    fn poisson_disk_points_within_bounds() {
        let pts = poisson_disk_2d(1, 50.0, 80.0, 8.0, 30);
        pts.iter().for_each(|&(x, y)| {
            assert!(x >= 0.0 && x < 50.0);
            assert!(y >= 0.0 && y < 80.0);
        });
    }

    #[test]
    fn poisson_disk_deterministic() {
        let a = poisson_disk_2d(5, 60.0, 60.0, 8.0, 30);
        let b = poisson_disk_2d(5, 60.0, 60.0, 8.0, 30);
        assert_eq!(a.len(), b.len());
        a.iter().zip(b.iter()).for_each(|(pa, pb)| {
            assert!((pa.0 - pb.0).abs() < 1e-5);
            assert!((pa.1 - pb.1).abs() < 1e-5);
        });
    }
}
