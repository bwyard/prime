//! `prime-diffusion` — Stochastic processes: Ornstein-Uhlenbeck and geometric Brownian motion.
//!
//! All public functions are pure (LOAD + COMPUTE only). No `&mut`, no side effects,
//! no hidden state. Noise is either supplied externally (caller-provided standard normal `w`)
//! or generated deterministically from a threaded `u32` seed via `prime_random::prng_gaussian`.
//!
//! # Temporal Assembly Model
//! - **LOAD** — read parameters + state
//! - **COMPUTE** — stochastic update formula
//! - **APPEND** — return `(next_value, next_seed)` as a tuple
//!
//! STORE and JUMP do not exist here. Seeded variants thread the seed forward like
//! `prime_random::prng_next` — pure state machines.
//!
//! # Included
//! - `ou_step` — Ornstein-Uhlenbeck step (caller-supplied noise)
//! - `ou_step_seeded` — OU step with deterministic Gaussian noise from threaded seed
//! - `gbm_step` — Geometric Brownian motion step (caller-supplied noise)
//! - `gbm_step_seeded` — GBM step with deterministic noise from threaded seed

// ── Ornstein-Uhlenbeck ────────────────────────────────────────────────────────

/// Ornstein-Uhlenbeck step with caller-supplied noise.
///
/// The O-U process is the canonical mean-reverting stochastic process. It is
/// used in Idle Hero for economy curves (resource prices, rival activity) that
/// should wander but always return to a set point.
///
/// # Math
///
/// ```text
/// dx  =  θ(μ - x) dt + σ √dt · w
/// x'  =  x + θ(μ - x) dt + σ √dt · w
/// ```
///
/// where `w` is a sample from a standard normal distribution N(0, 1).
///
/// # Arguments
/// * `x`     — current value
/// * `mu`    — long-run mean (equilibrium point)
/// * `theta` — mean-reversion speed (> 0; typical 0.1–1.0)
/// * `sigma` — volatility / noise scale (> 0)
/// * `dt`    — time step
/// * `w`     — standard normal noise sample N(0, 1)
///
/// # Returns
/// Next value `x'`.
///
/// # Edge cases
/// * `dt = 0` → returns `x` unchanged
/// * `theta = 0` → no mean reversion; pure random walk `x + σ√dt·w`
/// * `sigma = 0` → deterministic decay to `mu`: `x + θ(μ−x)dt`
///
/// # Example
/// ```rust
/// use prime_diffusion::ou_step;
/// // No noise — converges toward mu=0 from x=1.0
/// let x1 = ou_step(1.0, 0.0, 0.5, 0.0, 0.1, 0.0);
/// assert!((x1 - 0.95).abs() < 1e-5);
/// ```
pub fn ou_step(x: f32, mu: f32, theta: f32, sigma: f32, dt: f32, w: f32) -> f32 {
    x + theta * (mu - x) * dt + sigma * dt.sqrt() * w
}

/// Ornstein-Uhlenbeck step with deterministic seeded noise.
///
/// Generates one standard-normal sample from `seed` via `prime_random::prng_gaussian`,
/// then applies [`ou_step`]. Threads the seed forward so callers can chain steps
/// without external RNG state.
///
/// # Arguments
/// * `x`, `mu`, `theta`, `sigma`, `dt` — same as [`ou_step`]
/// * `seed` — `u32` RNG state (non-zero); threads forward deterministically
///
/// # Returns
/// `(x_next, next_seed)` — new value and advanced seed.
///
/// # Example
/// ```rust
/// use prime_diffusion::ou_step_seeded;
/// let (x1, s1) = ou_step_seeded(1.0, 0.0, 0.5, 0.1, 0.01, 12345_u32);
/// let (x2, _)  = ou_step_seeded(x1,  0.0, 0.5, 0.1, 0.01, s1);
/// assert!(x1 != 1.0);
/// ```
pub fn ou_step_seeded(x: f32, mu: f32, theta: f32, sigma: f32, dt: f32, seed: u32) -> (f32, u32) {
    let (w, next_seed) = prime_random::prng_gaussian(seed);
    (ou_step(x, mu, theta, sigma, dt, w), next_seed)
}

// ── Geometric Brownian Motion ─────────────────────────────────────────────────

/// Geometric Brownian motion step with caller-supplied noise.
///
/// GBM models multiplicative processes where the quantity is always positive —
/// resource stockpiles, market prices, skill multipliers.
///
/// # Math
///
/// ```text
/// Exact solution for one step:
/// x'  =  x · exp((μ − σ²/2) dt + σ √dt · w)
/// ```
///
/// # Arguments
/// * `x`     — current value (must be > 0)
/// * `mu`    — drift rate (annualised, or per unit time)
/// * `sigma` — volatility (> 0)
/// * `dt`    — time step
/// * `w`     — standard normal noise sample N(0, 1)
///
/// # Returns
/// Next value `x'` (always positive when `x > 0`).
///
/// # Edge cases
/// * `dt = 0` → returns `x` unchanged
/// * `sigma = 0` → deterministic exponential growth: `x · exp(μ·dt)`
/// * `x = 0` → returns 0 (absorbing state)
///
/// # Example
/// ```rust
/// use prime_diffusion::gbm_step;
/// // Zero drift, no noise → x unchanged
/// let x1 = gbm_step(1.0, 0.0, 0.0, 0.1, 0.0);
/// assert!((x1 - 1.0).abs() < 1e-5);
/// ```
pub fn gbm_step(x: f32, mu: f32, sigma: f32, dt: f32, w: f32) -> f32 {
    x * ((mu - 0.5 * sigma * sigma) * dt + sigma * dt.sqrt() * w).exp()
}

/// Geometric Brownian motion step with deterministic seeded noise.
///
/// Identical to [`gbm_step`] but generates noise from `seed` via
/// `prime_random::prng_gaussian` and threads the seed forward.
///
/// # Returns
/// `(x_next, next_seed)`.
///
/// # Example
/// ```rust
/// use prime_diffusion::gbm_step_seeded;
/// let (x1, s1) = gbm_step_seeded(1.0, 0.05, 0.2, 0.01, 42_u32);
/// assert!(x1 > 0.0);
/// ```
pub fn gbm_step_seeded(x: f32, mu: f32, sigma: f32, dt: f32, seed: u32) -> (f32, u32) {
    let (w, next_seed) = prime_random::prng_gaussian(seed);
    (gbm_step(x, mu, sigma, dt, w), next_seed)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;
    const SEED: u32 = 0xDEAD_BEEF;

    // ── ou_step ───────────────────────────────────────────────────────────────

    #[test]
    fn ou_step_zero_dt_no_change() {
        let x = ou_step(1.0, 0.0, 0.5, 0.3, 0.0, 1.0);
        assert!((x - 1.0).abs() < EPSILON);
    }

    #[test]
    fn ou_step_zero_noise_converges() {
        // dx = θ(μ - x)dt, no noise
        // x' = x + 0.5*(0 - 1)*0.1 = 1 - 0.05 = 0.95
        let x = ou_step(1.0, 0.0, 0.5, 0.0, 0.1, 0.0);
        assert!((x - 0.95).abs() < EPSILON, "x={x}");
    }

    #[test]
    fn ou_step_zero_sigma_deterministic_decay() {
        // With sigma=0, same as noiseless
        let x = ou_step(2.0, 1.0, 1.0, 0.0, 0.1, 5.0);
        // x' = 2 + 1*(1-2)*0.1 = 2 - 0.1 = 1.9
        assert!((x - 1.9).abs() < EPSILON, "x={x}");
    }

    #[test]
    fn ou_step_deterministic() {
        let a = ou_step(1.0, 0.0, 0.3, 0.1, 0.01, 0.5);
        let b = ou_step(1.0, 0.0, 0.3, 0.1, 0.01, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn ou_step_mean_reversion() {
        // Noiseless OU: x(t) = x0 * exp(-θt) → 0
        // theta=1.0, 1000 steps, dt=0.01 → x = 10 * exp(-10) ≈ 4.5e-4
        let x = (0..1000).fold(10.0_f32, |x, _| ou_step(x, 0.0, 1.0, 0.0, 0.01, 0.0));
        assert!(x.abs() < 0.01, "x={x} — should be near 0 after 1000 steps");
    }

    // ── ou_step_seeded ────────────────────────────────────────────────────────

    #[test]
    fn ou_step_seeded_advances_value() {
        let (x1, _) = ou_step_seeded(1.0, 0.0, 0.3, 0.1, 0.01, SEED);
        assert!((x1 - 1.0).abs() > f32::EPSILON, "seeded step should produce movement");
    }

    #[test]
    fn ou_step_seeded_threads_seed_forward() {
        let (_, s1) = ou_step_seeded(1.0, 0.0, 0.3, 0.1, 0.01, SEED);
        assert_ne!(s1, SEED, "seed must advance");
    }

    #[test]
    fn ou_step_seeded_deterministic() {
        let a = ou_step_seeded(1.0, 0.0, 0.3, 0.1, 0.01, SEED);
        let b = ou_step_seeded(1.0, 0.0, 0.3, 0.1, 0.01, SEED);
        assert_eq!(a, b);
    }

    #[test]
    fn ou_step_seeded_chain_100_steps_bounded() {
        // O-U process should stay near mu=0 over many steps
        let (x, _) = (0..100).fold((0.0_f32, SEED), |(x, s), _| {
            ou_step_seeded(x, 0.0, 0.3, 0.5, 0.01, s)
        });
        assert!(x.abs() < 5.0, "O-U should stay bounded; x={x}");
    }

    // ── gbm_step ──────────────────────────────────────────────────────────────

    #[test]
    fn gbm_step_zero_dt_no_change() {
        let x = gbm_step(1.0, 0.05, 0.2, 0.0, 0.5);
        assert!((x - 1.0).abs() < EPSILON, "x={x}");
    }

    #[test]
    fn gbm_step_zero_sigma_deterministic_growth() {
        // x' = x * exp(mu * dt) = 1.0 * exp(0.1 * 0.1) = exp(0.01)
        let x = gbm_step(1.0, 0.1, 0.0, 0.1, 0.0);
        let expected = (0.01_f32).exp();
        assert!((x - expected).abs() < EPSILON, "x={x}, expected={expected}");
    }

    #[test]
    fn gbm_step_always_positive() {
        // GBM with positive x should never produce x ≤ 0
        let x = (0..100).fold(1.0_f32, |x, i| gbm_step(x, 0.0, 0.3, 0.01, (i as f32 * 0.1).sin()));
        assert!(x > 0.0, "GBM must stay positive; x={x}");
    }

    #[test]
    fn gbm_step_deterministic() {
        let a = gbm_step(1.0, 0.05, 0.2, 0.01, 0.5);
        let b = gbm_step(1.0, 0.05, 0.2, 0.01, 0.5);
        assert_eq!(a, b);
    }

    // ── gbm_step_seeded ───────────────────────────────────────────────────────

    #[test]
    fn gbm_step_seeded_positive() {
        let (x1, _) = gbm_step_seeded(1.0, 0.05, 0.2, 0.01, SEED);
        assert!(x1 > 0.0, "GBM result must be positive; x={x1}");
    }

    #[test]
    fn gbm_step_seeded_deterministic() {
        let a = gbm_step_seeded(1.0, 0.05, 0.2, 0.01, SEED);
        let b = gbm_step_seeded(1.0, 0.05, 0.2, 0.01, SEED);
        assert_eq!(a, b);
    }

    #[test]
    fn gbm_step_seeded_threads_seed() {
        let (_, s1) = gbm_step_seeded(1.0, 0.05, 0.2, 0.01, SEED);
        assert_ne!(s1, SEED);
    }

    #[test]
    fn gbm_step_seeded_chain_100_stays_positive() {
        let (x, _) = (0..100).fold((1.0_f32, SEED), |(x, s), _| {
            gbm_step_seeded(x, 0.0, 0.2, 0.01, s)
        });
        assert!(x > 0.0, "GBM chain must stay positive; x={x}");
    }

    // ── ou_step / gbm_step edge cases ─────────────────────────────────────────

    #[test]
    fn ou_step_zero_theta_no_reversion() {
        // theta=0 → no mean-reversion, only diffusion term.
        let x = ou_step(5.0, 0.0, 0.0, 0.1, 0.01, 1.0);
        // Should not snap to mu; should be near 5.0 + small diffusion.
        assert!((x - 5.0).abs() < 0.5, "theta=0: x moved too far: {x}");
    }

    #[test]
    fn gbm_step_zero_mu_sigma_unchanged() {
        // mu=0, sigma=0, w=anything → GBM exponent = 0 → x unchanged.
        let x = gbm_step(2.5, 0.0, 0.0, 0.01, 1.0);
        assert!((x - 2.5).abs() < EPSILON, "gbm_step with sigma=mu=0 should return x; got {x}");
    }
}
