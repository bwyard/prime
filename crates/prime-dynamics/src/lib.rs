//! `prime-dynamics` — Dynamical systems and numerical integration.
//!
//! Pure functions for integrating ODEs and simulating nonlinear dynamical
//! systems. No mutation, no hidden state. Same inputs always produce the same
//! output.
//!
//! # Included
//! - `rk4_step` / `rk4_step3` — generic 4th-order Runge-Kutta integrator
//! - `euler_step` — generic forward Euler integrator
//! - `lorenz_step` — Lorenz attractor via RK4
//! - `rossler_step` — Rössler attractor via RK4
//! - `duffing_step` — Duffing oscillator via RK4

// ── RK4 integrator ────────────────────────────────────────────────────────────

/// Advance a scalar state by one 4th-order Runge-Kutta step.
///
/// # Math
///
/// ```text
/// k1 = f(t,        s)
/// k2 = f(t + dt/2, s + dt/2 * k1)
/// k3 = f(t + dt/2, s + dt/2 * k2)
/// k4 = f(t + dt,   s + dt   * k3)
/// s' = s + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
/// ```
///
/// # Arguments
/// * `state` — current scalar state
/// * `t`     — current time (seconds)
/// * `dt`    — time step (seconds)
/// * `f`     — derivative `f(t, state) → ds/dt`
///
/// # Returns
/// New state after one RK4 step.
///
/// # Example
/// ```rust
/// use prime_dynamics::rk4_step;
///
/// // Exponential decay: ds/dt = -s → s(t) = e^(-t)
/// let s1 = rk4_step(1.0_f32, 0.0, 0.01, |_t, s| -s);
/// assert!((s1 - (-0.01_f32).exp()).abs() < 1e-6);
/// ```
pub fn rk4_step(state: f32, t: f32, dt: f32, f: impl Fn(f32, f32) -> f32) -> f32 {
    let k1 = f(t, state);
    let k2 = f(t + dt * 0.5, state + dt * 0.5 * k1);
    let k3 = f(t + dt * 0.5, state + dt * 0.5 * k2);
    let k4 = f(t + dt, state + dt * k3);
    state + dt / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
}

/// Advance a `(x, y, z)` state by one 4th-order Runge-Kutta step.
///
/// Used for 3D dynamical systems like Lorenz, Rössler, and Duffing.
///
/// # Arguments
/// * `state` — `(x, y, z)` current state
/// * `t`     — current time (seconds)
/// * `dt`    — time step (seconds)
/// * `f`     — derivative `f(t, (x,y,z)) → (dx, dy, dz)`
///
/// # Returns
/// New `(x, y, z)` state after one RK4 step.
///
/// # Example
/// ```rust
/// use prime_dynamics::rk4_step3;
///
/// // Circular motion in the XY plane
/// let (x1, y1, _) = rk4_step3((1.0, 0.0, 0.0), 0.0, 0.01, |_t, (x, y, _z)| (-y, x, 0.0));
/// assert!((x1*x1 + y1*y1 - 1.0).abs() < 1e-5);
/// ```
pub fn rk4_step3(
    state: (f32, f32, f32),
    t: f32,
    dt: f32,
    f: impl Fn(f32, (f32, f32, f32)) -> (f32, f32, f32),
) -> (f32, f32, f32) {
    let add = |(ax, ay, az): (f32, f32, f32), (bx, by, bz): (f32, f32, f32)| {
        (ax + bx, ay + by, az + bz)
    };
    let scale = |(dx, dy, dz): (f32, f32, f32), s: f32| (dx * s, dy * s, dz * s);

    let k1 = f(t, state);
    let k2 = f(t + dt * 0.5, add(state, scale(k1, dt * 0.5)));
    let k3 = f(t + dt * 0.5, add(state, scale(k2, dt * 0.5)));
    let k4 = f(t + dt, add(state, scale(k3, dt)));

    let (sx, sy, sz) = state;
    let (k1x, k1y, k1z) = k1;
    let (k2x, k2y, k2z) = k2;
    let (k3x, k3y, k3z) = k3;
    let (k4x, k4y, k4z) = k4;
    (
        sx + dt / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x),
        sy + dt / 6.0 * (k1y + 2.0 * k2y + 2.0 * k3y + k4y),
        sz + dt / 6.0 * (k1z + 2.0 * k2z + 2.0 * k3z + k4z),
    )
}

/// Advance a scalar state by one forward Euler step.
///
/// First-order accuracy. Use [`rk4_step`] for higher precision.
///
/// # Arguments
/// * `state` — current scalar state
/// * `t`     — current time
/// * `dt`    — time step
/// * `f`     — derivative `f(t, state) → ds/dt`
///
/// # Returns
/// `state + dt * f(t, state)`
///
/// # Example
/// ```rust
/// use prime_dynamics::euler_step;
///
/// let s1 = euler_step(1.0_f32, 0.0, 0.1, |_t, _s| -1.0);
/// assert!((s1 - 0.9).abs() < 1e-6);
/// ```
pub fn euler_step(state: f32, t: f32, dt: f32, f: impl Fn(f32, f32) -> f32) -> f32 {
    state + dt * f(t, state)
}

// ── Lorenz attractor ──────────────────────────────────────────────────────────

/// Lorenz canonical sigma (Prandtl number) = 10.
pub const LORENZ_SIGMA: f32 = 10.0;
/// Lorenz canonical rho (Rayleigh number) = 28.
pub const LORENZ_RHO: f32 = 28.0;
/// Lorenz canonical beta (geometric factor) = 8/3.
pub const LORENZ_BETA: f32 = 8.0 / 3.0;

/// Advance the Lorenz attractor state by one RK4 step.
///
/// The Lorenz system (1963) is the canonical example of deterministic chaos.
/// Form uses it to drive procedural animation: x=lateral sway, y=vertical bob,
/// z=sagittal rotation.
///
/// # Math
///
/// ```text
/// dx/dt = σ(y - x)
/// dy/dt = x(ρ - z) - y
/// dz/dt = xy - βz
/// ```
///
/// # Arguments
/// * `state`  — `(x, y, z)` current attractor state
/// * `sigma`  — Prandtl number (typically 10.0)
/// * `rho`    — Rayleigh number (typically 28.0; chaos above ~24.74)
/// * `beta`   — geometric factor (typically 8/3 ≈ 2.667)
/// * `dt`     — time step (keep ≤ 0.01 for numerical stability)
///
/// # Returns
/// New `(x, y, z)` state.
///
/// # Edge cases
/// * `dt = 0` → returns `state` unchanged
///
/// # Example
/// ```rust
/// use prime_dynamics::{lorenz_step, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA};
///
/// let s0 = (1.0_f32, 1.0_f32, 1.0_f32);
/// let s1 = lorenz_step(s0, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01);
/// assert!(s0 != s1);
/// ```
pub fn lorenz_step(
    state: (f32, f32, f32),
    sigma: f32,
    rho: f32,
    beta: f32,
    dt: f32,
) -> (f32, f32, f32) {
    rk4_step3(state, 0.0, dt, |_t, (x, y, z)| {
        (sigma * (y - x), x * (rho - z) - y, x * y - beta * z)
    })
}

// ── Rössler attractor ─────────────────────────────────────────────────────────

/// Advance the Rössler attractor state by one RK4 step.
///
/// Simpler single-scroll chaotic attractor, smoother than Lorenz.
///
/// # Math
///
/// ```text
/// dx/dt = -(y + z)
/// dy/dt = x + a*y
/// dz/dt = b + z*(x - c)
/// ```
///
/// # Arguments
/// * `state` — `(x, y, z)` current state
/// * `a`     — typically 0.2
/// * `b`     — typically 0.2
/// * `c`     — typically 5.7 (chaos above ~3.0)
/// * `dt`    — time step (keep ≤ 0.05)
///
/// # Returns
/// New `(x, y, z)` Rössler state.
///
/// # Example
/// ```rust
/// use prime_dynamics::rossler_step;
///
/// let s1 = rossler_step((1.0, 0.0, 0.0), 0.2, 0.2, 5.7, 0.01);
/// assert!(s1 != (1.0_f32, 0.0_f32, 0.0_f32));
/// ```
pub fn rossler_step(
    state: (f32, f32, f32),
    a: f32,
    b: f32,
    c: f32,
    dt: f32,
) -> (f32, f32, f32) {
    rk4_step3(state, 0.0, dt, |_t, (x, y, z)| {
        (-(y + z), x + a * y, b + z * (x - c))
    })
}

// ── Duffing oscillator ────────────────────────────────────────────────────────

/// Advance the Duffing oscillator state by one RK4 step.
///
/// Models a damped, driven nonlinear spring. State is `(position, velocity)`.
///
/// # Math
///
/// ```text
/// dx/dt = v
/// dv/dt = -δv - αx - βx³ + γcos(ωt)
/// ```
///
/// # Arguments
/// * `state` — `(x, v)` position and velocity
/// * `t`     — current time (seconds; used for driving term)
/// * `delta` — damping (typically 0.3)
/// * `alpha` — linear stiffness (typically -1.0)
/// * `beta`  — cubic stiffness (typically 1.0)
/// * `gamma` — driving amplitude (typically 0.37)
/// * `omega` — driving frequency (typically 1.2)
/// * `dt`    — time step
///
/// # Returns
/// New `(x, v)` state.
///
/// # Example
/// ```rust
/// use prime_dynamics::{duffing_step, DuffingParams};
///
/// let p = DuffingParams { delta: 0.3, alpha: -1.0, beta: 1.0, gamma: 0.37, omega: 1.2 };
/// let (_, v1) = duffing_step((0.0, 0.0), 0.0, p, 0.01);
/// assert!(v1.abs() > 0.0);
/// ```
/// Duffing oscillator parameters.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DuffingParams {
    /// Damping coefficient (typically 0.3)
    pub delta: f32,
    /// Linear stiffness (typically -1.0)
    pub alpha: f32,
    /// Cubic stiffness (typically 1.0)
    pub beta: f32,
    /// Driving amplitude (typically 0.37)
    pub gamma: f32,
    /// Driving frequency (typically 1.2)
    pub omega: f32,
}

pub fn duffing_step(
    state: (f32, f32),
    t: f32,
    p: DuffingParams,
    dt: f32,
) -> (f32, f32) {
    let DuffingParams { delta, alpha, beta, gamma, omega } = p;
    let deriv = |t: f32, (x, v): (f32, f32)| {
        (v, -delta * v - alpha * x - beta * x.powi(3) + gamma * (omega * t).cos())
    };

    let (x0, v0) = state;
    let (k1x, k1v) = deriv(t, state);
    let (k2x, k2v) = deriv(t + dt * 0.5, (x0 + dt * 0.5 * k1x, v0 + dt * 0.5 * k1v));
    let (k3x, k3v) = deriv(t + dt * 0.5, (x0 + dt * 0.5 * k2x, v0 + dt * 0.5 * k2v));
    let (k4x, k4v) = deriv(t + dt,       (x0 + dt * k3x,        v0 + dt * k3v));

    (
        x0 + dt / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x),
        v0 + dt / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v),
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    // ── rk4_step ──────────────────────────────────────────────────────────────

    #[test]
    fn rk4_exponential_decay() {
        let s = (0..100).fold(1.0_f32, |s, i| rk4_step(s, i as f32 * 0.01, 0.01, |_t, x| -x));
        assert!((s - (-1.0_f32).exp()).abs() < EPSILON, "s={}", s);
    }

    #[test]
    fn rk4_zero_dt_no_change() {
        let s = rk4_step(3.14_f32, 0.0, 0.0, |_t, x| -x);
        assert!((s - 3.14).abs() < EPSILON);
    }

    #[test]
    fn rk4_constant_derivative() {
        let s = rk4_step(0.0_f32, 0.0, 1.0, |_t, _s| 1.0);
        assert!((s - 1.0).abs() < EPSILON);
    }

    #[test]
    fn rk4_deterministic() {
        let a = rk4_step(1.0_f32, 0.0, 0.01, |_t, x| -x);
        let b = rk4_step(1.0_f32, 0.0, 0.01, |_t, x| -x);
        assert!((a - b).abs() < EPSILON);
    }

    // ── rk4_step3 ─────────────────────────────────────────────────────────────

    #[test]
    fn rk4_step3_circular_motion_radius() {
        let (x, y, _) = (0..1000).fold((1.0_f32, 0.0_f32, 0.0_f32), |s, i| {
            rk4_step3(s, i as f32 * 0.01, 0.01, |_t, (x, y, _z)| (-y, x, 0.0))
        });
        let r = (x * x + y * y).sqrt();
        assert!((r - 1.0).abs() < 1e-3, "radius={}", r);
    }

    #[test]
    fn rk4_step3_zero_dt() {
        let s = rk4_step3((1.0, 2.0, 3.0), 0.0, 0.0, |_t, s| s);
        assert_eq!(s, (1.0_f32, 2.0_f32, 3.0_f32));
    }

    #[test]
    fn rk4_step3_deterministic() {
        let f = |_t: f32, (x, y, z): (f32, f32, f32)| (y - x, x - z, z - y);
        let a = rk4_step3((1.0, 0.5, -0.5), 0.0, 0.01, f);
        let b = rk4_step3((1.0, 0.5, -0.5), 0.0, 0.01, f);
        assert_eq!(a, b);
    }

    // ── euler_step ────────────────────────────────────────────────────────────

    #[test]
    fn euler_step_linear() {
        let s = euler_step(1.0_f32, 0.0, 0.1, |_t, _s| -1.0);
        assert!((s - 0.9).abs() < EPSILON);
    }

    #[test]
    fn euler_step_zero_dt() {
        let s = euler_step(5.0_f32, 0.0, 0.0, |_t, x| x * 100.0);
        assert!((s - 5.0).abs() < EPSILON);
    }

    #[test]
    fn euler_step_deterministic() {
        let a = euler_step(1.0_f32, 0.5, 0.01, |t, s| t - s);
        let b = euler_step(1.0_f32, 0.5, 0.01, |t, s| t - s);
        assert!((a - b).abs() < EPSILON);
    }

    // ── lorenz_step ───────────────────────────────────────────────────────────

    #[test]
    fn lorenz_step_moves_state() {
        let s0 = (1.0_f32, 1.0_f32, 1.0_f32);
        let s1 = lorenz_step(s0, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01);
        assert!(s0 != s1);
    }

    #[test]
    fn lorenz_step_zero_dt_no_change() {
        let s0 = (1.0_f32, 2.0_f32, 3.0_f32);
        let s1 = lorenz_step(s0, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.0);
        assert_eq!(s0, s1);
    }

    #[test]
    fn lorenz_step_bounded_short_run() {
        let (x, y, z) = (0..1000).fold((1.0_f32, 1.0_f32, 1.0_f32), |s, _| {
            lorenz_step(s, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
        });
        assert!(x.abs() < 100.0 && y.abs() < 100.0 && z.abs() < 100.0,
            "x={} y={} z={}", x, y, z);
    }

    #[test]
    fn lorenz_step_deterministic() {
        let a = lorenz_step((1.0, 0.0, 0.0), LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01);
        let b = lorenz_step((1.0, 0.0, 0.0), LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01);
        assert_eq!(a, b);
    }

    #[test]
    fn lorenz_sensitive_to_initial_conditions() {
        // f32 precision limits sensitivity; use 1e-3 perturbation over 3000 steps
        let n = 3000;
        let s1 = (0..n).fold((1.0_f32, 1.0_f32, 1.0_f32), |s, _| {
            lorenz_step(s, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
        });
        let s2 = (0..n).fold((1.001_f32, 1.0_f32, 1.0_f32), |s, _| {
            lorenz_step(s, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
        });
        assert!((s1.0 - s2.0).abs() > 0.5, "Expected chaos divergence");
    }

    // ── rossler_step ──────────────────────────────────────────────────────────

    #[test]
    fn rossler_step_moves_state() {
        let s0 = (1.0_f32, 0.0_f32, 0.0_f32);
        let s1 = rossler_step(s0, 0.2, 0.2, 5.7, 0.01);
        assert!(s0 != s1);
    }

    #[test]
    fn rossler_step_bounded() {
        let (x, y, z) = (0..1000).fold((1.0_f32, 0.0_f32, 0.0_f32), |s, _| {
            rossler_step(s, 0.2, 0.2, 5.7, 0.01)
        });
        assert!(x.abs() < 50.0 && y.abs() < 50.0 && z.abs() < 50.0,
            "x={} y={} z={}", x, y, z);
    }

    #[test]
    fn rossler_step_deterministic() {
        let a = rossler_step((1.0, 0.0, 0.0), 0.2, 0.2, 5.7, 0.01);
        let b = rossler_step((1.0, 0.0, 0.0), 0.2, 0.2, 5.7, 0.01);
        assert_eq!(a, b);
    }

    // ── duffing_step ──────────────────────────────────────────────────────────

    #[test]
    fn duffing_step_nonzero_drive() {
        let (_, v1) = duffing_step((0.0, 0.0), 0.0, DuffingParams { delta: 0.3, alpha: -1.0, beta: 1.0, gamma: 0.37, omega: 1.2 }, 0.01);
        assert!(v1.abs() > 0.0, "v1={}", v1);
    }

    #[test]
    fn duffing_step_zero_dt() {
        let p = DuffingParams { delta: 0.3, alpha: -1.0, beta: 1.0, gamma: 0.37, omega: 1.2 };
        let s0 = (1.0_f32, 0.5_f32);
        let s1 = duffing_step(s0, 0.0, p, 0.0);
        assert!((s1.0 - s0.0).abs() < EPSILON && (s1.1 - s0.1).abs() < EPSILON);
    }

    #[test]
    fn duffing_step_deterministic() {
        let p = DuffingParams { delta: 0.3, alpha: -1.0, beta: 1.0, gamma: 0.37, omega: 1.2 };
        let a = duffing_step((1.0, 0.0), 0.5, p, 0.01);
        let b = duffing_step((1.0, 0.0), 0.5, p, 0.01);
        assert_eq!(a, b);
    }
}
