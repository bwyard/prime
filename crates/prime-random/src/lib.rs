//! prime-random — Seeded deterministic randomness.
//!
//! All public functions are LOAD + COMPUTE. No STORE. No JUMP. No exceptions.
//!
//! Thesis: the seed IS the thread. Who holds the seed controls who can
//! advance it. Consent revocation = stop threading the seed forward.
//! No DELETE needed — the sequence is causally inert without its key.
//!
//! # Receiver Model
//!
//! Each function is a **receiver** (context function) that extracts typed
//! information from the same causal datum (a u32 seed). The same seed
//! produces different valid outputs depending on which receiver reads it:
//!
//! | Receiver | Output | Interpretation |
//! |----------|--------|----------------|
//! | `prng_next` | `f32` | Uniform probability in [0, 1) |
//! | `prng_bool` | `bool` | Bernoulli trial with threshold p |
//! | `prng_gaussian` | `f32` | Standard normal N(0, 1) |
//! | `prng_exponential` | `f32` | Waiting time with rate λ |
//! | `prng_disk_uniform` | `(f32, f32)` | Spatial coordinate in disk |
//! | `prng_annulus_uniform` | `(f32, f32)` | Spatial coordinate in annulus |
//! | `prng_choose` | `&T` | Element selection from collection |
//! | `weighted_choice` | `usize` | Weighted element index |
//!
//! This is the thesis **Context primitive**: `I(data, receiver)` not `I(data)`.
//! Information is relational — a property of the relationship between the
//! causal datum and the function that reads it.

use std::f32::consts::PI;
// Note: im::Vector was benchmarked but Vec clone is faster at typical grid sizes
// (<5600 cells). Persistent data structures win at >10K elements. See ADR-001.

// ── Pure PRNG primitives ─────────────────────────────────────────────────────

/// Mulberry32 pure step. `(seed) → (value in [0,1), next_seed)`.
///
/// Cross-language parity: identical algorithm in TypeScript.
///
/// # Example
/// ```rust
/// use prime_random::prng_next;
/// let (v, s1) = prng_next(42);
/// assert!(v >= 0.0 && v < 1.0);
/// ```
pub fn prng_next(seed: u32) -> (f32, u32) {
    let z0 = seed.wrapping_add(0x6D2B79F5);
    let z1 = (z0 ^ (z0 >> 15)).wrapping_mul(z0 | 1);
    let z2 = z1 ^ z1.wrapping_add((z1 ^ (z1 >> 7)).wrapping_mul(z1 | 61));
    let value = (z2 ^ (z2 >> 14)) as f64 / 4_294_967_296.0;
    (value as f32, z0)
}

/// Float in `[min, max)`. Returns `(min, seed)` when `min >= max`.
/// ```rust
/// # use prime_random::prng_range_f32;
/// let (v, _) = prng_range_f32(0, 2.0, 5.0);
/// assert!(v >= 2.0 && v < 5.0);
/// ```
pub fn prng_range_f32(seed: u32, min: f32, max: f32) -> (f32, u32) {
    if min >= max { return (min, seed); }
    let (v, next) = prng_next(seed);
    (min + v * (max - min), next)
}

/// Integer in `[0, n)`.
/// ```rust
/// # use prime_random::prng_range_usize;
/// let (i, _) = prng_range_usize(0, 10);
/// assert!(i < 10);
/// ```
pub fn prng_range_usize(seed: u32, n: usize) -> (usize, u32) {
    let (v, next) = prng_next(seed);
    ((v * n as f32) as usize, next)
}

/// Bool with probability `p` of true. Clamps `p` to `[0, 1]`.
/// ```rust
/// # use prime_random::prng_bool;
/// let (b, _) = prng_bool(0, 1.0);
/// assert!(b);
/// ```
pub fn prng_bool(seed: u32, p: f32) -> (bool, u32) {
    let (v, next) = prng_next(seed);
    (v < p.clamp(0.0, 1.0), next)
}

/// PRNG step with external entropy XOR'd into next_seed. `entropy = 0` is identical to [`prng_next`].
/// ```rust
/// # use prime_random::prng_next_with_entropy;
/// let (v, _) = prng_next_with_entropy(42, 0xDEADBEEF);
/// assert!(v >= 0.0 && v < 1.0);
/// ```
pub fn prng_next_with_entropy(seed: u32, entropy: u32) -> (f32, u32) {
    let (value, next) = prng_next(seed);
    (value, next ^ entropy)
}

// ── 64-bit PRNG ─────────────────────────────────────────────────────────────

/// SplitMix64 pure step — 64-bit PRNG with period 2^64.
///
/// For applications needing longer sequences than Mulberry32's 2^32 period.
/// An idle game at 100 draws/frame × 60fps exhausts Mulberry32 in ~8 days.
/// SplitMix64 lasts ~97 billion years at the same rate.
///
/// Same thesis contract: `(seed) -> (value, nextSeed)`.
/// ```rust
/// # use prime_random::prng_next_64;
/// let (v, s1) = prng_next_64(42u64);
/// assert!(v >= 0.0 && v < 1.0);
/// ```
pub fn prng_next_64(seed: u64) -> (f64, u64) {
    let z0 = seed.wrapping_add(0x9E3779B97F4A7C15);
    let z1 = (z0 ^ (z0 >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    let z2 = (z1 ^ (z1 >> 27)).wrapping_mul(0x94D049BB133111EB);
    let z3 = z2 ^ (z2 >> 31);
    (z3 as f64 / u64::MAX as f64, z0)
}

/// 64-bit float in [min, max).
/// ```rust
/// # use prime_random::prng_range_f64;
/// let (v, _) = prng_range_f64(42u64, 10.0, 20.0);
/// assert!(v >= 10.0 && v < 20.0);
/// ```
pub fn prng_range_f64(seed: u64, min: f64, max: f64) -> (f64, u64) {
    if min >= max { return (min, seed); }
    let (v, next) = prng_next_64(seed);
    (min + v * (max - min), next)
}

/// 64-bit Gaussian via Box-Muller. Higher precision than f32 variant.
/// ```rust
/// # use prime_random::prng_gaussian_64;
/// let (z, _) = prng_gaussian_64(42u64);
/// assert!(z.is_finite());
/// ```
pub fn prng_gaussian_64(seed: u64) -> (f64, u64) {
    let (u1, s1) = prng_next_64(seed);
    let (u2, s2) = prng_next_64(s1);
    let u1_safe = if u1 < f64::EPSILON { f64::EPSILON } else { u1 };
    let z = (-2.0 * u1_safe.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    (z, s2)
}

// ── Causal traceability ─────────────────────────────────────────────────────

/// A value with its causal parent recorded.
///
/// Enables backward traversal of deterministic sequences. The `parent_seed`
/// records which seed produced this value, allowing replay verification
/// without storing the full history.
///
/// # Thesis
/// The fold pattern preserves causal sequence but doesn't record ancestry.
/// `CausalStep` makes the parent explicit — you can always answer
/// "what input produced this output?" without replaying from genesis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CausalStep<T> {
    /// The computed value at this step.
    pub value: T,
    /// The seed that was the causal parent of this value.
    pub parent_seed: u32,
    /// The next seed (causal successor).
    pub next_seed: u32,
}

/// `prng_next` with causal ancestry recorded.
/// ```rust
/// # use prime_random::{prng_next_causal, CausalStep};
/// let step = prng_next_causal(42);
/// assert_eq!(step.parent_seed, 42);
/// assert!(step.value >= 0.0 && step.value < 1.0);
/// ```
pub fn prng_next_causal(seed: u32) -> CausalStep<f32> {
    let (value, next_seed) = prng_next(seed);
    CausalStep { value, parent_seed: seed, next_seed }
}

/// `prng_gaussian` with causal ancestry recorded.
/// ```rust
/// # use prime_random::{prng_gaussian_causal, CausalStep};
/// let step = prng_gaussian_causal(42);
/// assert!(step.value.is_finite());
/// assert_eq!(step.parent_seed, 42);
/// ```
pub fn prng_gaussian_causal(seed: u32) -> CausalStep<f32> {
    let (value, next_seed) = prng_gaussian(seed);
    CausalStep { value, parent_seed: seed, next_seed }
}

/// Fisher-Yates shuffle. Returns new Vec, original unchanged. O(n).
/// ```rust
/// # use prime_random::prng_shuffled;
/// let v = vec![1, 2, 3, 4, 5];
/// let (s, _) = prng_shuffled(0, &v);
/// assert_eq!(s.len(), 5);
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

/// Random element from slice. `None` if empty.
/// ```rust
/// # use prime_random::prng_choose;
/// let (pick, _) = prng_choose(0, &["a", "b", "c"]);
/// assert!(pick.is_some());
/// ```
pub fn prng_choose<T>(seed: u32, slice: &[T]) -> (Option<&T>, u32) {
    if slice.is_empty() { return (None, seed); }
    let (i, next) = prng_range_usize(seed, slice.len());
    (Some(&slice[i]), next)
}

// ── Probability distributions ───────────────────────────────────────────────

/// Standard normal sample N(0,1) via Box-Muller. Consumes 2 draws.
///
/// `z = sqrt(-2 ln(u1)) * cos(2π u2)`. Clamps u1 away from 0.
/// ```rust
/// # use prime_random::prng_gaussian;
/// let (z, _) = prng_gaussian(42);
/// assert!(z.is_finite());
/// ```
pub fn prng_gaussian(seed: u32) -> (f32, u32) {
    let (u1, s1) = prng_next(seed);
    let (u2, s2) = prng_next(s1);
    let u1_safe = if u1 < f32::EPSILON { f32::EPSILON } else { u1 };
    let z = (-2.0 * u1_safe.ln()).sqrt() * (2.0 * PI * u2).cos();
    (z, s2)
}

/// Full Box-Muller pair: `(z0, z1, next_seed)` where both are N(0,1).
/// ```rust
/// # use prime_random::prng_gaussian_pair;
/// let (z0, z1, _) = prng_gaussian_pair(42);
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

/// Exponential sample: `x = -ln(1-u)/λ`. Returns 0 when `lambda <= 0`.
/// ```rust
/// # use prime_random::prng_exponential;
/// let (x, _) = prng_exponential(42, 1.0);
/// assert!(x > 0.0);
/// ```
pub fn prng_exponential(seed: u32, lambda: f32) -> (f32, u32) {
    let (u, next) = prng_next(seed);
    if lambda <= 0.0 { return (0.0, next); }
    (-(1.0 - u).ln() / lambda, next)
}

/// Uniform random point inside a disk. No rejection sampling.
///
/// `dist = radius * sqrt(u2)` gives area-uniform distribution.
/// ```rust
/// # use prime_random::prng_disk_uniform;
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

/// Uniform random point in annulus `[r_inner, r_outer]`. Area-uniform.
///
/// `dist = sqrt(r_inner^2 + u2 * (r_outer^2 - r_inner^2))`
/// ```rust
/// # use prime_random::prng_annulus_uniform;
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

/// Weighted random choice — O(n) linear scan.
///
/// Sample `u ~ Uniform(0, sum(weights))`, return first index where cumulative sum >= u.
/// Empty weights returns 0; all-zero weights returns last index.
/// ```rust
/// # use prime_random::weighted_choice;
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

/// Van der Corput radical inverse — low-discrepancy sequence in `[0, 1)`.
///
/// Reflects the base-b digits of `n` around the decimal point:
/// `vdc(5, 2) = 0.101₂ = 5/8`.
/// ```rust
/// # use prime_random::van_der_corput;
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

/// 2D Halton sequence using bases 2 and 3. Returns `(x, y)` in `[0, 1)^2`.
/// ```rust
/// # use prime_random::halton_2d;
/// let (x, y) = halton_2d(1);
/// assert!((x - 0.5).abs() < 1e-5);
/// ```
pub fn halton_2d(n: u32) -> (f32, f32) {
    (van_der_corput(n, 2), van_der_corput(n, 3))
}

/// 3D Halton sequence using bases 2, 3, and 5. Returns `(x, y, z)` in `[0, 1)^3`.
/// ```rust
/// # use prime_random::halton_3d;
/// let (x, y, z) = halton_3d(1);
/// assert!((x - 0.5).abs() < 1e-5);
/// assert!((z - 0.2).abs() < 1e-5);
/// ```
pub fn halton_3d(n: u32) -> (f32, f32, f32) {
    (van_der_corput(n, 2), van_der_corput(n, 3), van_der_corput(n, 5))
}

// ── Monte Carlo integration ─────────────────────────────────────────────────

/// 1D Monte Carlo integration of `f` over `[a, b]`.
///
/// `estimate = (b - a) / n * Σ f(x_i)` where `x_i ~ Uniform(a, b)`.
/// ```rust
/// # use prime_random::monte_carlo_1d;
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

/// 2D Monte Carlo integration over `[x0, x1] x [y0, y1]`.
///
/// `estimate = area / n * Σ f(x_i, y_i)`.
/// ```rust
/// # use prime_random::monte_carlo_2d;
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

/// 1D stratified Monte Carlo — divides [a, b] into n equal strata, samples one point per stratum.
///
/// Converges at O(1/n) for smooth functions vs O(1/√n) for plain MC.
/// ```rust
/// # use prime_random::monte_carlo_1d_stratified;
/// let (est, _s) = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, std::f32::consts::PI, 100);
/// assert!((est - 2.0).abs() < 0.01);
/// ```
pub fn monte_carlo_1d_stratified(
    seed: u32,
    f: fn(f32) -> f32,
    a: f32, b: f32,
    n: usize,
) -> (f32, u32) {
    let width = b - a;
    let stratum = width / n as f32;
    let (sum, final_seed) = (0..n).fold((0.0_f32, seed), |(acc, s), i| {
        let (u, next) = prng_next(s);
        let x = a + (i as f32 + u) * stratum;
        (acc + f(x), next)
    });
    (width * sum / n as f32, final_seed)
}

/// 1D Monte Carlo with Welford's online variance estimate.
///
/// `estimate = mean(f(x_i)) * (b - a)`, variance via numerically stable running sum.
/// ```rust
/// # use prime_random::monte_carlo_1d_with_variance;
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

// ── Memoization ─────────────────────────────────────────────────────────────

/// Evaluate f(x) with memoization over a precomputed lookup table.
///
/// Builds a table of n evenly-spaced samples in [a, b], then interpolates.
/// Thesis-compatible: the table is computed once (LOAD+COMPUTE), then
/// lookups are O(1) with linear interpolation (pure COMPUTE).
///
/// # Returns
/// A closure that maps x -> f(x) approximately, with O(1) lookup cost.
///
/// # Example
/// ```rust
/// use prime_random::memoize_1d;
/// let fast_sin = memoize_1d(|x: f32| x.sin(), 0.0, std::f32::consts::PI, 1000);
/// assert!((fast_sin(1.0) - 1.0_f32.sin()).abs() < 0.01);
/// ```
pub fn memoize_1d(f: fn(f32) -> f32, a: f32, b: f32, n: usize) -> impl Fn(f32) -> f32 {
    let table: Vec<f32> = (0..=n).map(|i| {
        let t = i as f32 / n as f32;
        f(a + t * (b - a))
    }).collect();
    let step = (b - a) / n as f32;
    move |x: f32| {
        let t = (x - a) / step;
        let i = (t as usize).min(n - 1);
        let frac = t - i as f32;
        table[i] * (1.0 - frac) + table[i + 1] * frac
    }
}

// ── Pure Bridson ──────────────────────────────────────────────────────────────

struct BridsonParams {
    width: f32,
    height: f32,
    min_dist_sq: f32,
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
                dx * dx + dy * dy < p.min_dist_sq
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
            let dist = (p.min_dist_sq + dist_f * 3.0 * p.min_dist_sq).sqrt();
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
        let new_pt_idx = state.points.len();
        BridsonState {
            grid: state.grid.iter().enumerate()
                .map(|(i, v)| if i == cell_idx { Some((cx, cy)) } else { *v })
                .collect(),
            active: state.active.iter().copied()
                .chain(std::iter::once(new_pt_idx))
                .collect(),
            points: state.points.iter().copied()
                .chain(std::iter::once((cx, cy)))
                .collect(),
            seed: final_seed,
        }
    } else {
        BridsonState {
            grid: state.grid.clone(),  // O(1) for persistent vector
            active: state.active.iter().enumerate()
                .filter(|(i, _)| *i != ai)
                .map(|(_, &v)| v)
                .collect(),
            points: state.points.clone(),
            seed: final_seed,
        }
    }
}

/// Poisson disk sampling — minimum-distance spacing in 2D.
///
/// Bridson's algorithm (2007) as a pure state fold (ADVANCE).
/// Each step is `(state) -> new_state`. No mutable shared state.
///
/// Performance: each step clones the spatial grid O(cols x rows).
/// Typical game domains (< 2000x2000, min_dist > 5) are negligible.
/// ```rust
/// # use prime_random::poisson_disk_2d;
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
    let min_dist_sq = min_dist * min_dist;
    let p = BridsonParams { width, height, min_dist_sq, max_attempts, cols, rows, cell_size };

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

    // ── monte_carlo_1d_stratified ───────────────────────────────────────────

    #[test]
    fn mc_1d_stratified_sin_integral() {
        // Stratified should be much more accurate than plain MC at same n
        let (est, _) = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, PI, 100);
        assert!((est - 2.0).abs() < 0.01, "stratified est={est}");
    }

    #[test]
    fn mc_1d_stratified_beats_plain() {
        let n = 100;
        let (plain, _) = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, n);
        let (strat, _) = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, PI, n);
        let plain_err = (plain - 2.0).abs();
        let strat_err = (strat - 2.0).abs();
        assert!(strat_err < plain_err, "stratified err={strat_err} should beat plain err={plain_err}");
    }

    #[test]
    fn mc_1d_stratified_deterministic() {
        let a = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, PI, 100);
        let b = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, PI, 100);
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

    // ── CausalStep ───────────────────────────────────────────────────────────

    #[test]
    fn causal_step_records_parent() {
        let step = prng_next_causal(42);
        assert_eq!(step.parent_seed, 42);
        let (v, _) = prng_next(42);
        assert_eq!(step.value.to_bits(), v.to_bits());
    }

    #[test]
    fn causal_step_chain_is_traceable() {
        let s0 = prng_next_causal(42);
        let s1 = prng_next_causal(s0.next_seed);
        let s2 = prng_next_causal(s1.next_seed);
        // Chain: 42 -> s0.next_seed -> s1.next_seed -> s2.next_seed
        assert_eq!(s1.parent_seed, s0.next_seed);
        assert_eq!(s2.parent_seed, s1.next_seed);
    }

    #[test]
    fn causal_gaussian_records_parent() {
        let step = prng_gaussian_causal(42);
        assert_eq!(step.parent_seed, 42);
        let (v, _) = prng_gaussian(42);
        assert_eq!(step.value.to_bits(), v.to_bits());
    }

    #[test]
    fn causal_step_in_fold() {
        // Build a causal log via fold
        let history: Vec<CausalStep<f32>> = (0..10).fold(
            (vec![], 42u32),
            |(mut log, seed), _| {
                let step = prng_next_causal(seed);
                let next = step.next_seed;
                log.push(step);
                (log, next)
            },
        ).0;
        // Every step's parent is the previous step's next_seed
        (1..history.len()).for_each(|i| {
            assert_eq!(history[i].parent_seed, history[i - 1].next_seed);
        });
    }

    // ── Statistical validation tests ────────────────────────────────────────

    #[test]
    fn prng_next_chi_square_uniform() {
        // Divide [0,1) into 10 bins. 10K samples should distribute ~1000 per bin.
        let n = 10000usize;
        let bins = 10usize;
        let counts = (0..n).fold(([0usize; 10], 1u32), |(mut acc, s), _| {
            let (v, next) = prng_next(s);
            let bin = (v * bins as f32).min((bins - 1) as f32) as usize;
            acc[bin] += 1;
            (acc, next)
        }).0;
        let expected = n as f32 / bins as f32;
        let chi_sq: f32 = counts.iter()
            .map(|&c| (c as f32 - expected).powi(2) / expected)
            .sum();
        // Chi-square with 9 dof: critical value at p=0.01 is 21.67
        assert!(chi_sq < 21.67, "chi_sq={chi_sq} — PRNG fails uniformity test");
    }

    #[test]
    fn prng_next_ks_uniform() {
        // Max deviation from CDF of uniform should be small
        let n = 1000usize;
        let mut samples: Vec<f32> = (0..n).fold((vec![], 42u32), |(mut v, s), _| {
            let (val, next) = prng_next(s);
            v.push(val);
            (v, next)
        }).0;
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let max_dev = samples.iter().enumerate()
            .map(|(i, &v)| {
                let empirical = (i + 1) as f32 / n as f32;
                (v - empirical).abs()
            })
            .fold(0.0_f32, f32::max);
        // KS critical value at p=0.01 for n=1000: ~0.0408
        assert!(max_dev < 0.05, "KS max deviation={max_dev} — PRNG fails uniformity");
    }

    #[test]
    fn gaussian_jarque_bera_normality() {
        let n = 10000usize;
        let (sum, sum2, sum3, sum4, _) = (0..n).fold(
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 1u32),
            |(s1, s2, s3, s4, seed), _| {
                let (g, next) = prng_gaussian(seed);
                let g = g as f64;
                (s1 + g, s2 + g * g, s3 + g * g * g, s4 + g * g * g * g, next)
            },
        );
        let mean = sum / n as f64;
        let m2 = sum2 / n as f64 - mean * mean;
        let m3 = sum3 / n as f64 - 3.0 * mean * sum2 / n as f64 + 2.0 * mean.powi(3);
        let m4 = sum4 / n as f64 - 4.0 * mean * sum3 / n as f64 + 6.0 * mean.powi(2) * sum2 / n as f64 - 3.0 * mean.powi(4);
        let skewness = m3 / m2.powf(1.5);
        let kurtosis = m4 / (m2 * m2) - 3.0; // excess kurtosis
        let jb = (n as f64 / 6.0) * (skewness.powi(2) + kurtosis.powi(2) / 4.0);
        // JB critical value at p=0.01 with 2 dof: 9.21
        assert!(jb < 9.21, "JB={jb}, skew={skewness}, kurt={kurtosis} — Gaussian fails normality");
    }

    #[test]
    fn exponential_rate_chi_square() {
        let lambda = 2.0_f32;
        let n = 10000usize;
        let bins = 10usize;
        // Exponential CDF: F(x) = 1 - exp(-lambda*x)
        // Bin boundaries at F^{-1}(i/bins) = -ln(1 - i/bins) / lambda
        let boundaries: Vec<f32> = (1..bins)
            .map(|i| -(1.0 - i as f32 / bins as f32).ln() / lambda)
            .collect();
        let counts = (0..n).fold(([0usize; 10], 1u32), |(mut acc, s), _| {
            let (x, next) = prng_exponential(s, lambda);
            let bin = boundaries.iter().position(|&b| x < b).unwrap_or(bins - 1);
            acc[bin] += 1;
            (acc, next)
        }).0;
        let expected = n as f32 / bins as f32;
        let chi_sq: f32 = counts.iter()
            .map(|&c| (c as f32 - expected).powi(2) / expected)
            .sum();
        assert!(chi_sq < 21.67, "chi_sq={chi_sq} — Exponential fails distribution test");
    }

    #[test]
    fn disk_uniform_area_coverage() {
        // Points in inner half of disk (r < R/2) should be ~25% (area ratio)
        let n = 10000usize;
        let radius = 10.0_f32;
        let inner_count = (0..n).fold((0usize, 1u32), |(count, s), _| {
            let (x, y, next) = prng_disk_uniform(s, radius);
            let r = (x * x + y * y).sqrt();
            (count + if r < radius / 2.0 { 1 } else { 0 }, next)
        }).0;
        let ratio = inner_count as f32 / n as f32;
        // Expected: (R/2)^2 / R^2 = 0.25, tolerance 3%
        assert!((ratio - 0.25).abs() < 0.03, "inner ratio={ratio}, expected ~0.25");
    }

    #[test]
    fn annulus_uniform_area_coverage() {
        // Points in inner half of annulus area should be ~50%
        let n = 10000usize;
        let r_inner = 3.0_f32;
        let r_outer = 7.0_f32;
        let r_mid_sq = (r_inner * r_inner + r_outer * r_outer) / 2.0;
        let r_mid = r_mid_sq.sqrt();
        let inner_count = (0..n).fold((0usize, 1u32), |(count, s), _| {
            let (x, y, next) = prng_annulus_uniform(s, r_inner, r_outer);
            let r = (x * x + y * y).sqrt();
            (count + if r < r_mid { 1 } else { 0 }, next)
        }).0;
        let ratio = inner_count as f32 / n as f32;
        assert!((ratio - 0.5).abs() < 0.03, "inner ratio={ratio}, expected ~0.5");
    }

    #[test]
    fn halton_lower_discrepancy_than_prng() {
        // Halton should fill [0,1)^2 more uniformly than pseudo-random
        let n = 256usize;
        let bins = 4usize; // 4x4 grid = 16 cells
        // Halton
        let halton_counts = (0..n).fold([0usize; 16], |mut acc, i| {
            let (x, y) = halton_2d(i as u32);
            let bx = (x * bins as f32).min((bins - 1) as f32) as usize;
            let by = (y * bins as f32).min((bins - 1) as f32) as usize;
            acc[by * bins + bx] += 1;
            acc
        });
        // PRNG
        let prng_counts = (0..n).fold(([0usize; 16], 42u32), |(mut acc, s), _| {
            let (x, s1) = prng_next(s);
            let (y, s2) = prng_next(s1);
            let bx = (x * bins as f32).min((bins - 1) as f32) as usize;
            let by = (y * bins as f32).min((bins - 1) as f32) as usize;
            acc[by * bins + bx] += 1;
            (acc, s2)
        }).0;
        let expected = n as f32 / 16.0;
        let halton_chi: f32 = halton_counts.iter().map(|&c| (c as f32 - expected).powi(2) / expected).sum();
        let prng_chi: f32 = prng_counts.iter().map(|&c| (c as f32 - expected).powi(2) / expected).sum();
        assert!(halton_chi < prng_chi, "Halton chi={halton_chi} should be < PRNG chi={prng_chi}");
    }

    #[test]
    fn poisson_disk_packing_density() {
        let width = 100.0_f32;
        let height = 100.0_f32;
        let min_dist = 5.0_f32;
        let (pts, _) = poisson_disk_2d(42, width, height, min_dist, 30);
        // Theoretical max: area / (pi * (r/2)^2) where r = min_dist
        let theoretical_max = (width * height) / (PI * (min_dist / 2.0).powi(2));
        let density = pts.len() as f32 / theoretical_max;
        // Bridson typically achieves 60-80% of theoretical max
        assert!(density > 0.50, "density={density} ({} points), expected >50%", pts.len());
        assert!(density < 0.95, "density={density} — suspiciously high");
    }

    // ── Production / academic-grade statistical tests ────────────────────────

    #[test]
    fn prng_next_serial_correlation() {
        // Pearson correlation between consecutive values should be near 0
        let n = 10000usize;
        let (values, _) = (0..n).fold((vec![], 42u32), |(mut v, s), _| {
            let (val, next) = prng_next(s);
            v.push(val);
            (v, next)
        });
        let mean: f32 = values.iter().sum::<f32>() / n as f32;
        let var: f32 = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n as f32;
        let cov: f32 = values.windows(2)
            .map(|w| (w[0] - mean) * (w[1] - mean))
            .sum::<f32>() / (n - 1) as f32;
        let correlation = cov / var;
        assert!(correlation.abs() < 0.03, "serial correlation={correlation}, expected near 0");
    }

    #[test]
    fn prng_next_runs_test() {
        // Count runs above/below median. Should be ~n/2 ± sqrt(n).
        let n = 10000usize;
        let (values, _) = (0..n).fold((vec![], 42u32), |(mut v, s), _| {
            let (val, next) = prng_next(s);
            v.push(val);
            (v, next)
        });
        let median = 0.5_f32; // theoretical median of Uniform(0,1)
        let above: Vec<bool> = values.iter().map(|&v| v > median).collect();
        let runs = 1 + above.windows(2).filter(|w| w[0] != w[1]).count();
        let n1 = above.iter().filter(|&&b| b).count() as f32;
        let n2 = above.iter().filter(|&&b| !b).count() as f32;
        let expected_runs = 1.0 + 2.0 * n1 * n2 / (n1 + n2);
        let std_runs = ((2.0 * n1 * n2 * (2.0 * n1 * n2 - n1 - n2))
            / ((n1 + n2).powi(2) * (n1 + n2 - 1.0))).sqrt();
        let z = (runs as f32 - expected_runs) / std_runs;
        // z should be in [-2.58, 2.58] at 99% confidence
        assert!(z.abs() < 2.58, "runs z-score={z}, expected |z|<2.58");
    }

    #[test]
    fn prng_next_multi_seed_uniformity() {
        // First output from 10000 different seeds should be uniformly distributed
        let n = 10000usize;
        let bins = 10usize;
        let counts = (0..n as u32).fold([0usize; 10], |mut acc, seed| {
            let (v, _) = prng_next(seed);
            let bin = (v * bins as f32).min((bins - 1) as f32) as usize;
            acc[bin] += 1;
            acc
        });
        let expected = n as f32 / bins as f32;
        let chi_sq: f32 = counts.iter()
            .map(|&c| (c as f32 - expected).powi(2) / expected)
            .sum();
        assert!(chi_sq < 21.67, "multi-seed chi_sq={chi_sq} — first values not uniform across seeds");
    }

    #[test]
    fn disk_uniform_multi_radius_coverage() {
        let n = 20000usize;
        let radius = 10.0_f32;
        // Test at r/4, r/2, 3r/4 — areas should be 6.25%, 25%, 56.25%
        let (q1, q2, q3, _) = (0..n).fold((0usize, 0usize, 0usize, 1u32), |(c1, c2, c3, s), _| {
            let (x, y, next) = prng_disk_uniform(s, radius);
            let r = (x * x + y * y).sqrt();
            (
                c1 + if r < radius * 0.25 { 1 } else { 0 },
                c2 + if r < radius * 0.5 { 1 } else { 0 },
                c3 + if r < radius * 0.75 { 1 } else { 0 },
                next,
            )
        });
        let r1 = q1 as f32 / n as f32;
        let r2 = q2 as f32 / n as f32;
        let r3 = q3 as f32 / n as f32;
        assert!((r1 - 0.0625).abs() < 0.02, "r/4 ratio={r1}, expected ~0.0625");
        assert!((r2 - 0.25).abs() < 0.02, "r/2 ratio={r2}, expected ~0.25");
        assert!((r3 - 0.5625).abs() < 0.02, "3r/4 ratio={r3}, expected ~0.5625");
    }

    #[test]
    fn mc_1d_convergence_rate() {
        // Error should decrease as O(1/sqrt(n)). Compare error at n=100 vs n=10000.
        // Ratio of errors should be ~sqrt(100) = 10.
        let true_value = 2.0_f32; // integral of sin(x) over [0, pi]
        let (est_100, _) = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 100);
        let (est_10k, _) = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 10000);
        let err_100 = (est_100 - true_value).abs();
        let err_10k = (est_10k - true_value).abs();
        // 100x more samples should give ~10x less error
        // Allow wide tolerance since single-run MC is noisy
        assert!(err_10k < err_100, "10K should be more accurate than 100");
        let improvement = err_100 / err_10k;
        assert!(improvement > 3.0, "improvement ratio={improvement}, expected >3 for O(1/sqrt(n))");
    }

    #[test]
    fn mc_stratified_convergence_rate_superiority() {
        // Stratified at n=100 should be as accurate as plain MC at n=1000+
        let true_value = 2.0_f32;
        let (strat_100, _) = monte_carlo_1d_stratified(42, |x| x.sin(), 0.0, PI, 100);
        let (plain_1000, _) = monte_carlo_1d(42, |x| x.sin(), 0.0, PI, 1000);
        let strat_err = (strat_100 - true_value).abs();
        let plain_err = (plain_1000 - true_value).abs();
        // Stratified at n=100 should beat or match plain at n=1000
        assert!(strat_err < plain_err * 2.0,
            "strat_100 err={strat_err}, plain_1000 err={plain_err}");
    }

    // ── prng_next_64 ─────────────────────────────────────────────────────────

    #[test]
    fn prng_next_64_in_range() {
        (0..1000u64).for_each(|i| {
            let (v, _) = prng_next_64(i);
            assert!(v >= 0.0 && v < 1.0, "prng_next_64({i}) = {v}");
        });
    }

    #[test]
    fn prng_next_64_deterministic() {
        let (a, sa) = prng_next_64(42);
        let (b, sb) = prng_next_64(42);
        assert_eq!(a.to_bits(), b.to_bits());
        assert_eq!(sa, sb);
    }

    #[test]
    fn prng_next_64_different_from_32() {
        let (v32, _) = prng_next(42);
        let (v64, _) = prng_next_64(42);
        // Different algorithms, different values (both valid)
        assert_ne!(v32 as f64, v64);
    }

    #[test]
    fn prng_gaussian_64_finite() {
        (0..1000u64).for_each(|i| {
            let (g, _) = prng_gaussian_64(i);
            assert!(g.is_finite(), "prng_gaussian_64({i}) = {g}");
        });
    }

    // ── memoize_1d ───────────────────────────────────────────────────────────

    #[test]
    fn memoize_1d_approximates_sin() {
        let fast_sin = memoize_1d(|x| x.sin(), 0.0, std::f32::consts::PI, 1000);
        // Check at several points
        for i in 0..100 {
            let x = i as f32 * std::f32::consts::PI / 100.0;
            assert!((fast_sin(x) - x.sin()).abs() < 0.01, "x={x}");
        }
    }

    #[test]
    fn memoize_1d_deterministic() {
        let f = memoize_1d(|x| x * x, 0.0, 10.0, 100);
        assert_eq!(f(5.0).to_bits(), f(5.0).to_bits());
    }

    #[test]
    fn gaussian_anderson_darling() {
        let n = 1000usize;
        let mut samples: Vec<f32> = (0..n).fold((vec![], 42u32), |(mut v, s), _| {
            let (g, next) = prng_gaussian(s);
            v.push(g);
            (v, next)
        }).0;
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // Phi (standard normal CDF) — Abramowitz & Stegun rational approximation
        let phi = |x: f32| -> f32 {
            let x = x as f64;
            let t = 1.0 / (1.0 + 0.2316419 * x.abs());
            let d = 0.3989422804014327; // 1/sqrt(2*pi)
            let p = d * (-x * x / 2.0).exp();
            let poly = t * (0.319381530
                + t * (-0.356563782
                + t * (1.781477937
                + t * (-1.821255978
                + t * 1.330274429))));
            let result = if x >= 0.0 { 1.0 - p * poly } else { p * poly };
            result as f32
        };
        let a2: f32 = -(1..=n).map(|i| {
            let p = phi(samples[i - 1]);
            let q = phi(samples[n - i]);
            let p = p.clamp(1e-10, 1.0 - 1e-10);
            let q = q.clamp(1e-10, 1.0 - 1e-10);
            (2 * i - 1) as f32 * (p.ln() + (1.0 - q).ln())
        }).sum::<f32>() / n as f32 - n as f32;
        // Adjusted statistic
        let a2_star = a2 * (1.0 + 0.75 / n as f32 + 2.25 / (n * n) as f32);
        // Critical value at p=0.01: 1.035
        assert!(a2_star < 1.035, "A2*={a2_star} — Gaussian fails Anderson-Darling at p=0.01");
    }
}
