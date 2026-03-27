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

/// Advance PRNG with external entropy mixed into the next seed.
///
/// # Math
///   Same as `prng_next`, but next_seed is XOR'd with caller-supplied entropy.
///   Pass `entropy = 0` for standard deterministic behavior.
///
/// # Arguments
/// * `seed` - Current thread position
/// * `entropy` - External entropy (XOR'd into next_seed)
///
/// # Returns
/// `(value, next_seed ^ entropy)` — value is identical to `prng_next(seed)`.
///
/// # Edge cases
/// * `entropy = 0` → identical to `prng_next`
///
/// # Example
/// ```rust
/// use prime_random::prng_next_with_entropy;
/// let (v, s1) = prng_next_with_entropy(42, 0xDEADBEEF);
/// assert!(v >= 0.0 && v < 1.0);
/// ```
pub fn prng_next_with_entropy(seed: u32, entropy: u32) -> (f32, u32) {
    let (value, next) = prng_next(seed);
    (value, next ^ entropy)
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

// ── Probability distributions ───────────────────────────────────────────────

/// Box-Muller transform — one standard normal sample N(0,1).
///
/// # Math
///   u1, u2 ~ Uniform(0, 1)
///   z = sqrt(-2 * ln(u1)) * cos(2 * pi * u2)
///
/// # Arguments
/// * `seed` - Thread position (consumes 2 PRNG draws)
///
/// # Returns
/// `(z, next_seed)` where z ~ N(0, 1).
///
/// # Edge cases
/// * u1 near 0 is clamped to `f32::EPSILON` to avoid `ln(0)`
///
/// # Example
/// ```rust
/// use prime_random::prng_gaussian;
/// let (z, s1) = prng_gaussian(42);
/// assert!(z.is_finite());
/// ```
pub fn prng_gaussian(seed: u32) -> (f32, u32) {
    let (u1, s1) = prng_next(seed);
    let (u2, s2) = prng_next(s1);
    let u1_safe = if u1 < f32::EPSILON { f32::EPSILON } else { u1 };
    let z = (-2.0 * u1_safe.ln()).sqrt() * (2.0 * PI * u2).cos();
    (z, s2)
}

/// Full Box-Muller — returns both Gaussian values from the pair.
///
/// # Math
///   r = sqrt(-2 * ln(u1))
///   z0 = r * cos(2 * pi * u2)
///   z1 = r * sin(2 * pi * u2)
///
/// # Arguments
/// * `seed` - Thread position (consumes 2 PRNG draws)
///
/// # Returns
/// `(z0, z1, next_seed)` where z0, z1 ~ N(0, 1) independently.
///
/// # Example
/// ```rust
/// use prime_random::prng_gaussian_pair;
/// let (z0, z1, s1) = prng_gaussian_pair(42);
/// assert!(z0.is_finite() && z1.is_finite());
/// ```
pub fn prng_gaussian_pair(seed: u32) -> (f32, f32, u32) {
    let (u1, s1) = prng_next(seed);
    let (u2, s2) = prng_next(s1);
    let u1_safe = if u1 < f32::EPSILON { f32::EPSILON } else { u1 };
    let r = (-2.0 * u1_safe.ln()).sqrt();
    let theta = 2.0 * PI * u2;
    (r * theta.cos(), r * theta.sin(), s2)
}

/// Exponential distribution sample via inverse CDF.
///
/// # Math
///   x = -ln(1 - u) / lambda
///
/// # Arguments
/// * `seed` - Thread position
/// * `lambda` - Rate parameter (must be > 0)
///
/// # Returns
/// `(x, next_seed)` where x ~ Exp(lambda). Always positive.
///
/// # Edge cases
/// * `lambda <= 0` → returns `(0.0, next_seed)`
///
/// # Example
/// ```rust
/// use prime_random::prng_exponential;
/// let (x, s1) = prng_exponential(42, 1.0);
/// assert!(x > 0.0);
/// ```
pub fn prng_exponential(seed: u32, lambda: f32) -> (f32, u32) {
    let (u, next) = prng_next(seed);
    if lambda <= 0.0 { return (0.0, next); }
    (-(1.0 - u).ln() / lambda, next)
}

/// Uniform random point inside a disk of given radius. No rejection sampling.
///
/// # Math
///   angle = 2 * pi * u1
///   dist = radius * sqrt(u2)   — sqrt gives area-uniform distribution
///   (x, y) = (dist * cos(angle), dist * sin(angle))
///
/// # Arguments
/// * `seed` - Thread position (consumes 2 PRNG draws)
/// * `radius` - Disk radius
///
/// # Returns
/// `(x, y, next_seed)` — point uniformly distributed in disk.
///
/// # Example
/// ```rust
/// use prime_random::prng_disk_uniform;
/// let (x, y, s1) = prng_disk_uniform(42, 5.0);
/// assert!(x * x + y * y <= 25.0 + 1e-5);
/// ```
pub fn prng_disk_uniform(seed: u32, radius: f32) -> (f32, f32, u32) {
    let (u1, s1) = prng_next(seed);
    let (u2, s2) = prng_next(s1);
    let angle = 2.0 * PI * u1;
    let dist = radius * u2.sqrt();
    (dist * angle.cos(), dist * angle.sin(), s2)
}

/// Uniform random point in annulus [r_inner, r_outer]. Area-uniform.
///
/// # Math
///   angle = 2 * pi * u1
///   dist = sqrt(r_inner^2 + u2 * (r_outer^2 - r_inner^2))
///
/// # Arguments
/// * `seed` - Thread position (consumes 2 PRNG draws)
/// * `r_inner` - Inner radius
/// * `r_outer` - Outer radius
///
/// # Returns
/// `(x, y, next_seed)` — point uniformly distributed in annulus.
///
/// # Edge cases
/// * `r_inner >= r_outer` → samples on circle of radius `r_inner`
///
/// # Example
/// ```rust
/// use prime_random::prng_annulus_uniform;
/// let (x, y, s1) = prng_annulus_uniform(42, 3.0, 6.0);
/// let d = (x * x + y * y).sqrt();
/// assert!(d >= 3.0 - 1e-5 && d <= 6.0 + 1e-5);
/// ```
pub fn prng_annulus_uniform(seed: u32, r_inner: f32, r_outer: f32) -> (f32, f32, u32) {
    let (u1, s1) = prng_next(seed);
    let (u2, s2) = prng_next(s1);
    let angle = 2.0 * PI * u1;
    let r_in2 = r_inner * r_inner;
    let r_out2 = r_outer * r_outer;
    let dist = (r_in2 + u2 * (r_out2 - r_in2)).sqrt();
    (dist * angle.cos(), dist * angle.sin(), s2)
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

// ── Quasi-random sequences ──────────────────────────────────────────────────

/// Van der Corput radical inverse — low-discrepancy sequence in [0, 1).
///
/// # Math
///   Reflect the base-b digits of n around the decimal point.
///   vdc(5, 2) = 0.625 because 5 = 101₂ → 0.101₂ = 5/8
///
/// # Arguments
/// * `n` - Sequence index (0-based)
/// * `base` - Number base (typically prime: 2, 3, 5, ...)
///
/// # Returns
/// Value in [0, 1). Returns 0.0 for n = 0.
///
/// # Example
/// ```rust
/// use prime_random::van_der_corput;
/// assert!((van_der_corput(1, 2) - 0.5).abs() < 1e-10);
/// assert!((van_der_corput(2, 2) - 0.25).abs() < 1e-10);
/// ```
pub fn van_der_corput(n: u32, base: u32) -> f32 {
    let mut result = 0.0_f64;
    let mut denom = 1_u64;
    let mut num = n;
    let b = base as u64;
    // ADVANCE-EXCEPTION: digit extraction terminates when num reaches 0
    while num > 0 {
        denom *= b;
        result += (num % base) as f64 / denom as f64;
        num /= base;
    }
    result as f32
}

/// 2D Halton sequence using bases 2 and 3.
///
/// # Math
///   (van_der_corput(n, 2), van_der_corput(n, 3))
///
/// # Arguments
/// * `n` - Sequence index (0-based)
///
/// # Returns
/// `(x, y)` in [0, 1)^2.
///
/// # Example
/// ```rust
/// use prime_random::halton_2d;
/// let (x, y) = halton_2d(1);
/// assert!((x - 0.5).abs() < 1e-5);
/// ```
pub fn halton_2d(n: u32) -> (f32, f32) {
    (van_der_corput(n, 2), van_der_corput(n, 3))
}

/// 3D Halton sequence using bases 2, 3, and 5.
///
/// # Arguments
/// * `n` - Sequence index (0-based)
///
/// # Returns
/// `(x, y, z)` in [0, 1)^3.
///
/// # Example
/// ```rust
/// use prime_random::halton_3d;
/// let (x, y, z) = halton_3d(1);
/// assert!((x - 0.5).abs() < 1e-5);
/// assert!((z - 0.2).abs() < 1e-5);
/// ```
pub fn halton_3d(n: u32) -> (f32, f32, f32) {
    (van_der_corput(n, 2), van_der_corput(n, 3), van_der_corput(n, 5))
}

// ── Monte Carlo integration ─────────────────────────────────────────────────

/// 1D Monte Carlo integration of f over [a, b].
///
/// # Math
///   estimate = (b - a) / n * Σ f(x_i)  where x_i ~ Uniform(a, b)
///
/// # Arguments
/// * `seed` - Thread position
/// * `f` - Integrand
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `n` - Number of samples
///
/// # Returns
/// `(estimate, final_seed)` — integral estimate and advanced seed.
///
/// # Example
/// ```rust
/// use prime_random::monte_carlo_1d;
/// let (est, _s) = monte_carlo_1d(42, |x| x.sin(), 0.0, std::f32::consts::PI, 10000);
/// assert!((est - 2.0).abs() < 0.1);
/// ```
pub fn monte_carlo_1d(seed: u32, f: fn(f32) -> f32, a: f32, b: f32, n: usize) -> (f32, u32) {
    let width = b - a;
    let (sum, final_seed) = (0..n).fold((0.0_f32, seed), |(acc, s), _| {
        let (u, next) = prng_next(s);
        (acc + f(a + u * width), next)
    });
    (width * sum / n as f32, final_seed)
}

/// 2D Monte Carlo integration over [x0, x1] × [y0, y1].
///
/// # Math
///   estimate = area / n * Σ f(x_i, y_i)
///
/// # Arguments
/// * `seed` - Thread position
/// * `f` - Integrand
/// * `x0`, `x1` - X bounds
/// * `y0`, `y1` - Y bounds
/// * `n` - Number of samples
///
/// # Returns
/// `(estimate, final_seed)`.
///
/// # Example
/// ```rust
/// use prime_random::monte_carlo_2d;
/// let (est, _s) = monte_carlo_2d(42, |x, y| x * y, 0.0, 1.0, 0.0, 1.0, 10000);
/// assert!((est - 0.25).abs() < 0.05);
/// ```
pub fn monte_carlo_2d(
    seed: u32,
    f: fn(f32, f32) -> f32,
    x0: f32, x1: f32,
    y0: f32, y1: f32,
    n: usize,
) -> (f32, u32) {
    let area = (x1 - x0) * (y1 - y0);
    let (sum, final_seed) = (0..n).fold((0.0_f32, seed), |(acc, s), _| {
        let (ux, s1) = prng_next(s);
        let (uy, s2) = prng_next(s1);
        (acc + f(x0 + ux * (x1 - x0), y0 + uy * (y1 - y0)), s2)
    });
    (area * sum / n as f32, final_seed)
}

/// 1D Monte Carlo with Welford's online variance estimate.
///
/// # Math
///   Uses Welford's algorithm for numerically stable running variance.
///   estimate = mean(f(x_i)) * (b - a)
///   variance = var(f(x_i)) * (b - a)^2
///
/// # Returns
/// `(estimate, variance, final_seed)`.
///
/// # Example
/// ```rust
/// use prime_random::monte_carlo_1d_with_variance;
/// let (est, var, _s) = monte_carlo_1d_with_variance(42, |x| x.sin(), 0.0, std::f32::consts::PI, 10000);
/// assert!((est - 2.0).abs() < 0.1);
/// assert!(var > 0.0);
/// ```
pub fn monte_carlo_1d_with_variance(
    seed: u32,
    f: fn(f32) -> f32,
    a: f32, b: f32,
    n: usize,
) -> (f32, f32, u32) {
    let width = b - a;
    let (mean, m2, _, final_seed) = (0..n).fold(
        (0.0_f32, 0.0_f32, 0_usize, seed),
        |(prev_mean, prev_m2, count, s), _| {
            let (u, next) = prng_next(s);
            let sample = f(a + u * width);
            let new_count = count + 1;
            let delta = sample - prev_mean;
            let new_mean = prev_mean + delta / new_count as f32;
            let delta2 = sample - new_mean;
            (new_mean, prev_m2 + delta * delta2, new_count, next)
        },
    );
    let variance = if n > 1 { m2 / (n - 1) as f32 * width * width } else { 0.0 };
    (mean * width, variance, final_seed)
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

#[derive(Clone)]
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

fn bridson_step(state: &BridsonState, p: &BridsonParams) -> BridsonState {
    if state.active.is_empty() { return state.clone(); }

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
            let r2 = p.min_dist * p.min_dist;
            let dist = (r2 + dist_f * 3.0 * r2).sqrt();
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
        BridsonState { grid: state.grid.clone(), active: new_active, points: state.points.clone(), seed: final_seed }
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
/// `(Vec<(f32, f32)>, u32)` — point pairs all at least `min_dist` apart, and the final seed.
///
/// # Example
/// ```rust
/// use prime_random::poisson_disk_2d;
/// let (pts, _seed) = poisson_disk_2d(42, 100.0, 100.0, 10.0, 30);
/// assert!(!pts.is_empty());
/// ```
pub fn poisson_disk_2d(
    seed: u32,
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
) -> (Vec<(f32, f32)>, u32) {
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

    // ADVANCE: pure state transition via successors; terminates when active list empties
    let final_state = std::iter::successors(Some(initial), |state| {
        if state.active.is_empty() {
            None
        } else {
            Some(bridson_step(state, &p))
        }
    })
    .last()
    .unwrap();

    (final_state.points, final_state.seed)
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
        let (pts, _) = poisson_disk_2d(42, 100.0, 100.0, min_dist, 30);
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
        let (pts, _) = poisson_disk_2d(1, 50.0, 80.0, 8.0, 30);
        pts.iter().for_each(|&(x, y)| {
            assert!(x >= 0.0 && x < 50.0);
            assert!(y >= 0.0 && y < 80.0);
        });
    }

    #[test]
    fn poisson_disk_deterministic() {
        let (a, _) = poisson_disk_2d(5, 60.0, 60.0, 8.0, 30);
        let (b, _) = poisson_disk_2d(5, 60.0, 60.0, 8.0, 30);
        assert_eq!(a.len(), b.len());
        a.iter().zip(b.iter()).for_each(|(pa, pb)| {
            assert!((pa.0 - pb.0).abs() < 1e-5);
            assert!((pa.1 - pb.1).abs() < 1e-5);
        });
    }

    #[test]
    fn poisson_disk_returns_seed() {
        let (_, seed) = poisson_disk_2d(42, 100.0, 100.0, 10.0, 30);
        assert_ne!(seed, 42);
    }

    // ── prng_shuffled ─────────────────────────────────────────────────────────

    #[test]
    fn prng_shuffled_preserves_length() {
        let v = vec![1, 2, 3, 4, 5];
        let (s, _) = prng_shuffled(0, &v);
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn prng_shuffled_original_unchanged() {
        let v = vec![1, 2, 3, 4, 5];
        let _ = prng_shuffled(0, &v);
        assert_eq!(v, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn prng_shuffled_contains_same_elements() {
        let v = vec![10u32, 20, 30, 40, 50];
        let (s, _) = prng_shuffled(42, &v);
        let mut sorted_orig = v.clone();
        let mut sorted_shuffled = s.clone();
        sorted_orig.sort();
        sorted_shuffled.sort();
        assert_eq!(sorted_orig, sorted_shuffled);
    }

    #[test]
    fn prng_shuffled_deterministic() {
        let v = vec![1, 2, 3, 4, 5];
        let (a, _) = prng_shuffled(7, &v);
        let (b, _) = prng_shuffled(7, &v);
        assert_eq!(a, b);
    }

    #[test]
    fn prng_shuffled_empty_returns_empty() {
        let v: Vec<i32> = vec![];
        let (s, _) = prng_shuffled(0, &v);
        assert!(s.is_empty());
    }

    // ── prng_choose ───────────────────────────────────────────────────────────

    #[test]
    fn prng_choose_returns_element_in_slice() {
        let v = vec!["a", "b", "c"];
        let (pick, _) = prng_choose(0, &v);
        assert!(pick.is_some());
        assert!(v.contains(pick.unwrap()));
    }

    #[test]
    fn prng_choose_empty_returns_none() {
        let v: Vec<i32> = vec![];
        let (pick, _) = prng_choose(0, &v);
        assert!(pick.is_none());
    }

    #[test]
    fn prng_choose_deterministic() {
        let v = vec![10, 20, 30, 40];
        let (a, _) = prng_choose(99, &v);
        let (b, _) = prng_choose(99, &v);
        assert_eq!(a, b);
    }

    #[test]
    fn prng_choose_single_element_always_returns_it() {
        let v = vec![42];
        let (pick, _) = prng_choose(0, &v);
        assert_eq!(*pick.unwrap(), 42);
    }

    // ── prng_next_with_entropy ───────────────────────────────────────────────

    #[test]
    fn entropy_zero_same_as_prng_next() {
        let (v1, s1) = prng_next(42);
        let (v2, s2) = prng_next_with_entropy(42, 0);
        assert_eq!(v1.to_bits(), v2.to_bits());
        assert_eq!(s1, s2);
    }

    #[test]
    fn entropy_changes_next_seed() {
        let (_, s1) = prng_next_with_entropy(42, 0xDEADBEEF);
        let (_, s2) = prng_next_with_entropy(42, 0);
        assert_ne!(s1, s2);
    }

    // ── prng_gaussian ────────────────────────────────────────────────────────

    #[test]
    fn gaussian_deterministic() {
        let (a, sa) = prng_gaussian(42);
        let (b, sb) = prng_gaussian(42);
        assert_eq!(a.to_bits(), b.to_bits());
        assert_eq!(sa, sb);
    }

    #[test]
    fn gaussian_mean_and_stddev() {
        let n = 10000usize;
        let (sum, sum_sq, _) = (0..n).fold((0.0_f32, 0.0_f32, 1u32), |(acc, sq, s), _| {
            let (g, next) = prng_gaussian(s);
            (acc + g, sq + g * g, next)
        });
        let mean = sum / n as f32;
        let variance = sum_sq / n as f32 - mean * mean;
        assert!(mean.abs() < 0.05, "mean={mean}");
        assert!((variance.sqrt() - 1.0).abs() < 0.1, "stddev={}", variance.sqrt());
    }

    #[test]
    fn gaussian_finite_for_many_seeds() {
        (0..1000u32).for_each(|i| {
            let (g, _) = prng_gaussian(i);
            assert!(g.is_finite(), "prng_gaussian({i}) = {g}");
        });
    }

    // ── prng_gaussian_pair ───────────────────────────────────────────────────

    #[test]
    fn gaussian_pair_deterministic() {
        let (a0, a1, sa) = prng_gaussian_pair(42);
        let (b0, b1, sb) = prng_gaussian_pair(42);
        assert_eq!(a0.to_bits(), b0.to_bits());
        assert_eq!(a1.to_bits(), b1.to_bits());
        assert_eq!(sa, sb);
    }

    // ── prng_exponential ─────────────────────────────────────────────────────

    #[test]
    fn exponential_always_positive() {
        (0..1000u32).for_each(|i| {
            let (x, _) = prng_exponential(i, 1.0);
            assert!(x > 0.0, "prng_exponential({i}) = {x}");
        });
    }

    #[test]
    fn exponential_mean_matches_lambda() {
        let lambda = 2.0_f32;
        let n = 10000usize;
        let (sum, _) = (0..n).fold((0.0_f32, 1u32), |(acc, s), _| {
            let (x, next) = prng_exponential(s, lambda);
            (acc + x, next)
        });
        let mean = sum / n as f32;
        assert!((mean - 1.0 / lambda).abs() < 0.05, "mean={mean}, expected={}", 1.0 / lambda);
    }

    #[test]
    fn exponential_deterministic() {
        let (a, sa) = prng_exponential(42, 3.0);
        let (b, sb) = prng_exponential(42, 3.0);
        assert_eq!(a.to_bits(), b.to_bits());
        assert_eq!(sa, sb);
    }

    // ── prng_disk_uniform ────────────────────────────────────────────────────

    #[test]
    fn disk_uniform_within_radius() {
        let radius = 5.0_f32;
        (0..1000u32).for_each(|i| {
            let (x, y, _) = prng_disk_uniform(i, radius);
            let d = (x * x + y * y).sqrt();
            assert!(d <= radius + 1e-5, "seed {i}: dist={d} > radius={radius}");
        });
    }

    #[test]
    fn disk_uniform_deterministic() {
        let (x1, y1, s1) = prng_disk_uniform(42, 10.0);
        let (x2, y2, s2) = prng_disk_uniform(42, 10.0);
        assert_eq!(x1.to_bits(), x2.to_bits());
        assert_eq!(y1.to_bits(), y2.to_bits());
        assert_eq!(s1, s2);
    }

    // ── prng_annulus_uniform ─────────────────────────────────────────────────

    #[test]
    fn annulus_uniform_within_bounds() {
        let r_inner = 3.0_f32;
        let r_outer = 7.0_f32;
        (0..1000u32).for_each(|i| {
            let (x, y, _) = prng_annulus_uniform(i, r_inner, r_outer);
            let d = (x * x + y * y).sqrt();
            assert!(d >= r_inner - 1e-5, "seed {i}: dist={d} < r_inner={r_inner}");
            assert!(d <= r_outer + 1e-5, "seed {i}: dist={d} > r_outer={r_outer}");
        });
    }

    #[test]
    fn annulus_uniform_deterministic() {
        let (x1, y1, s1) = prng_annulus_uniform(42, 2.0, 5.0);
        let (x2, y2, s2) = prng_annulus_uniform(42, 2.0, 5.0);
        assert_eq!(x1.to_bits(), x2.to_bits());
        assert_eq!(y1.to_bits(), y2.to_bits());
        assert_eq!(s1, s2);
    }

    // ── van_der_corput ───────────────────────────────────────────────────────

    #[test]
    fn vdc_known_values_base_2() {
        assert!((van_der_corput(1, 2) - 0.5).abs() < 1e-6);
        assert!((van_der_corput(2, 2) - 0.25).abs() < 1e-6);
        assert!((van_der_corput(3, 2) - 0.75).abs() < 1e-6);
        assert!((van_der_corput(4, 2) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn vdc_known_values_base_3() {
        assert!((van_der_corput(1, 3) - 1.0 / 3.0).abs() < 1e-6);
        assert!((van_der_corput(2, 3) - 2.0 / 3.0).abs() < 1e-6);
        assert!((van_der_corput(3, 3) - 1.0 / 9.0).abs() < 1e-6);
    }

    #[test]
    fn vdc_zero_returns_zero() {
        assert_eq!(van_der_corput(0, 2), 0.0);
        assert_eq!(van_der_corput(0, 3), 0.0);
    }

    // ── halton_2d ────────────────────────────────────────────────────────────

    #[test]
    fn halton_2d_known_values() {
        let (x, y) = halton_2d(1);
        assert!((x - 0.5).abs() < 1e-5);
        assert!((y - 1.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn halton_2d_zero_is_origin() {
        let (x, y) = halton_2d(0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }

    // ── halton_3d ────────────────────────────────────────────────────────────

    #[test]
    fn halton_3d_known_values() {
        let (x, y, z) = halton_3d(1);
        assert!((x - 0.5).abs() < 1e-5);
        assert!((y - 1.0 / 3.0).abs() < 1e-5);
        assert!((z - 0.2).abs() < 1e-5);
    }

    // ── monte_carlo_1d ───────────────────────────────────────────────────────

    #[test]
    fn mc_1d_sin_integral() {
        let (est, _) = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 50000);
        assert!((est - 2.0).abs() < 0.05, "est={est}");
    }

    #[test]
    fn mc_1d_deterministic() {
        let a = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 100);
        let b = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 100);
        assert_eq!(a.0.to_bits(), b.0.to_bits());
        assert_eq!(a.1, b.1);
    }

    // ── monte_carlo_2d ───────────────────────────────────────────────────────

    #[test]
    fn mc_2d_xy_integral() {
        let (est, _) = monte_carlo_2d(42, |x, y| x * y, 0.0, 1.0, 0.0, 1.0, 50000);
        assert!((est - 0.25).abs() < 0.02, "est={est}");
    }

    #[test]
    fn mc_2d_deterministic() {
        let a = monte_carlo_2d(42, |x, y| x * y, 0.0, 1.0, 0.0, 1.0, 100);
        let b = monte_carlo_2d(42, |x, y| x * y, 0.0, 1.0, 0.0, 1.0, 100);
        assert_eq!(a.0.to_bits(), b.0.to_bits());
        assert_eq!(a.1, b.1);
    }

    // ── monte_carlo_1d_with_variance ─────────────────────────────────────────

    #[test]
    fn mc_1d_variance_positive() {
        let (est, var, _) = monte_carlo_1d_with_variance(42, |x| x.sin(), 0.0, PI, 10000);
        assert!((est - 2.0).abs() < 0.1, "est={est}");
        assert!(var > 0.0, "variance should be positive, got {var}");
    }

    #[test]
    fn mc_1d_variance_zero_for_n1() {
        let (_, var, _) = monte_carlo_1d_with_variance(42, |x| x.sin(), 0.0, PI, 1);
        assert_eq!(var, 0.0);
    }
}
