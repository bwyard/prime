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

// ── Logistic map ─────────────────────────────────────────────────────────────

/// Logistic map: x_{n+1} = r * x * (1 - x). Exhibits chaos for r > 3.57.
///
/// ```rust
/// # use prime_dynamics::logistic;
/// let x = logistic(0.5, 3.9);
/// assert!(x > 0.0 && x < 1.0);
/// ```
pub fn logistic(x: f32, r: f32) -> f32 {
    r * x * (1.0 - x)
}

// ── Lotka-Volterra (predator-prey) ───────────────────────────────────────────

/// Lotka-Volterra predator-prey step via Euler.
///
/// dx/dt = alpha*x - beta*x*y  (prey growth - predation)
/// dy/dt = delta*x*y - gamma*y (predator growth - death)
///
/// ```rust
/// # use prime_dynamics::lotka_volterra_step;
/// let (x, y) = lotka_volterra_step(1.0, 0.5, 1.1, 0.4, 0.1, 0.4, 0.01);
/// assert!(x > 0.0 && y > 0.0);
/// ```
pub fn lotka_volterra_step(
    x: f32,
    y: f32,
    alpha: f32,
    beta: f32,
    delta: f32,
    gamma: f32,
    dt: f32,
) -> (f32, f32) {
    let dx = (alpha * x - beta * x * y) * dt;
    let dy = (delta * x * y - gamma * y) * dt;
    (x + dx, y + dy)
}

// ── SIR epidemiological model ────────────────────────────────────────────────

/// SIR epidemiological model step.
///
/// dS/dt = -beta*S*I, dI/dt = beta*S*I - gamma*I, dR/dt = gamma*I
///
/// ```rust
/// # use prime_dynamics::sir_step;
/// let (s, i, r) = sir_step(0.99, 0.01, 0.0, 0.3, 0.1, 0.1);
/// assert!((s + i + r - 1.0).abs() < 1e-4); // population conserved
/// ```
pub fn sir_step(s: f32, i: f32, r: f32, beta: f32, gamma: f32, dt: f32) -> (f32, f32, f32) {
    let ds = -beta * s * i * dt;
    let di = (beta * s * i - gamma * i) * dt;
    let dr = gamma * i * dt;
    (s + ds, i + di, r + dr)
}

// ── Gray-Scott reaction-diffusion ────────────────────────────────────────────

/// Gray-Scott reaction-diffusion step for a single cell.
///
/// du/dt = Du*laplacian_u - u*v^2 + f*(1-u)
/// dv/dt = Dv*laplacian_v + u*v^2 - (f+k)*v
///
/// ```rust
/// # use prime_dynamics::gray_scott_step;
/// let (u, v) = gray_scott_step(1.0, 0.0, 0.0, 0.0, 0.04, 0.06, 0.01);
/// assert!(u >= 0.0);
/// ```
pub fn gray_scott_step(
    u: f32,
    v: f32,
    laplacian_u: f32,
    laplacian_v: f32,
    f: f32,
    k: f32,
    dt: f32,
) -> (f32, f32) {
    let du_dt = laplacian_u - u * v * v + f * (1.0 - u);
    let dv_dt = laplacian_v + u * v * v - (f + k) * v;
    (u + du_dt * dt, v + dv_dt * dt)
}

// ── L-systems ────────────────────────────────────────────────────────────────

/// Single L-system production rule: maps a character to its replacement string.
#[derive(Debug, Clone)]
pub struct LRule {
    /// The character to match.
    pub symbol: char,
    /// The replacement string for this symbol.
    pub replacement: &'static str,
}

/// Apply one L-system generation step. Pure LOAD + COMPUTE + APPEND.
///
/// Each character in `axiom` is replaced by its matching rule's replacement string.
/// Characters without matching rules are copied unchanged (identity rule).
///
/// # Math
/// L-system: G = (V, ω, P) where V = alphabet, ω = axiom, P = production rules.
/// Each generation: σ(ω) = P(c₁) ++ P(c₂) ++ ... ++ P(cₙ)
///
/// # Example
/// ```rust
/// use prime_dynamics::{LRule, lsystem_step};
/// let rules = vec![
///     LRule { symbol: 'A', replacement: "AB" },
///     LRule { symbol: 'B', replacement: "A" },
/// ];
/// let gen1 = lsystem_step("A", &rules);
/// assert_eq!(gen1, "AB");
/// let gen2 = lsystem_step(&gen1, &rules);
/// assert_eq!(gen2, "ABA");
/// ```
pub fn lsystem_step(axiom: &str, rules: &[LRule]) -> String {
    // ADVANCE-EXCEPTION: fold builds the output string incrementally.
    // Pure from caller's perspective: &str + &[LRule] -> String
    let mut result = String::with_capacity(axiom.len() * 2);
    for c in axiom.chars() {
        match rules.iter().find(|r| r.symbol == c) {
            Some(rule) => result.push_str(rule.replacement),
            None => result.push(c),
        }
    }
    result
}

/// Apply n generations of L-system rules. Pure fold over generations.
///
/// # Math
/// σⁿ(ω) = σ(σ(...σ(ω)...)) applied `generations` times.
///
/// # Example
/// ```rust
/// use prime_dynamics::{LRule, lsystem_generate};
/// let rules = vec![
///     LRule { symbol: 'A', replacement: "AB" },
///     LRule { symbol: 'B', replacement: "A" },
/// ];
/// let gen5 = lsystem_generate("A", &rules, 5);
/// assert_eq!(gen5, "ABAABABAABAAB");
/// ```
pub fn lsystem_generate(axiom: &str, rules: &[LRule], generations: usize) -> String {
    (0..generations).fold(axiom.to_string(), |current, _| lsystem_step(&current, rules))
}

// ── Numerical differentiation ────────────────────────────────────────────────

/// Numerical derivative via central difference. `f'(x) ≈ (f(x+h) - f(x-h)) / 2h`.
///
/// More accurate than forward difference (O(h²) vs O(h) error).
/// For f32, use h ≈ 1e-3 to balance truncation and rounding error.
/// ```rust
/// # use prime_dynamics::derivative;
/// let d = derivative(|x| x * x, 3.0, 1e-3);
/// assert!((d - 6.0).abs() < 1e-3); // d/dx(x²) = 2x = 6 at x=3
/// ```
pub fn derivative(f: fn(f32) -> f32, x: f32, h: f32) -> f32 {
    (f(x + h) - f(x - h)) / (2.0 * h)
}

/// Second derivative via central difference. `f''(x) ≈ (f(x+h) - 2f(x) + f(x-h)) / h²`.
///
/// For f32, use h ≈ 1e-2 to balance truncation and rounding error.
/// ```rust
/// # use prime_dynamics::derivative2;
/// let d2 = derivative2(|x| x * x * x, 2.0, 1e-2);
/// assert!((d2 - 12.0).abs() < 0.1); // d²/dx²(x³) = 6x = 12 at x=2
/// ```
pub fn derivative2(f: fn(f32) -> f32, x: f32, h: f32) -> f32 {
    (f(x + h) - 2.0 * f(x) + f(x - h)) / (h * h)
}

/// Numerical gradient of a 2D function via central differences.
/// ```rust
/// # use prime_dynamics::gradient_2d;
/// let (gx, gy) = gradient_2d(|x, y| x * x + y * y, 3.0, 4.0, 1e-3);
/// assert!((gx - 6.0).abs() < 1e-3); // df/dx = 2x = 6
/// assert!((gy - 8.0).abs() < 1e-3); // df/dy = 2y = 8
/// ```
pub fn gradient_2d(f: fn(f32, f32) -> f32, x: f32, y: f32, h: f32) -> (f32, f32) {
    let dx = (f(x + h, y) - f(x - h, y)) / (2.0 * h);
    let dy = (f(x, y + h) - f(x, y - h)) / (2.0 * h);
    (dx, dy)
}

// ── Numerical integration ────────────────────────────────────────────────────

/// Trapezoidal rule integration of f over [a, b] with n subdivisions.
///
/// `∫f(x)dx ≈ h/2 * (f(a) + 2*f(x₁) + 2*f(x₂) + ... + f(b))`
/// ```rust
/// # use prime_dynamics::integrate_trapezoidal;
/// let area = integrate_trapezoidal(|x| x * x, 0.0, 1.0, 1000);
/// assert!((area - 1.0/3.0).abs() < 1e-4);
/// ```
pub fn integrate_trapezoidal(f: fn(f32) -> f32, a: f32, b: f32, n: usize) -> f32 {
    let h = (b - a) / n as f32;
    let interior: f32 = (1..n).map(|i| f(a + i as f32 * h)).sum();
    h * (f(a) / 2.0 + interior + f(b) / 2.0)
}

/// Simpson's rule integration of f over [a, b] with n subdivisions (n must be even).
///
/// `∫f(x)dx ≈ h/3 * (f(a) + 4*f(x₁) + 2*f(x₂) + 4*f(x₃) + ... + f(b))`
///
/// O(h⁴) error — much more accurate than trapezoidal for smooth functions.
/// ```rust
/// # use prime_dynamics::integrate_simpson;
/// let area = integrate_simpson(|x| x * x, 0.0, 1.0, 100);
/// assert!((area - 1.0/3.0).abs() < 1e-6);
/// ```
pub fn integrate_simpson(f: fn(f32) -> f32, a: f32, b: f32, n: usize) -> f32 {
    let n = if n % 2 == 1 { n + 1 } else { n }; // ensure even
    let h = (b - a) / n as f32;
    let sum: f32 = (1..n)
        .map(|i| {
            let coeff = if i % 2 == 0 { 2.0 } else { 4.0 };
            coeff * f(a + i as f32 * h)
        })
        .sum();
    h / 3.0 * (f(a) + sum + f(b))
}

// ── Van der Pol oscillator ───────────────────────────────────────────────────

/// Van der Pol oscillator step via RK4.
///
/// `x'' - μ(1 - x²)x' + x = 0`
///
/// Relaxation oscillator — self-sustaining oscillations with nonlinear damping.
/// μ=0 is a simple harmonic oscillator. μ>0 exhibits limit cycle behavior.
/// ```rust
/// # use prime_dynamics::van_der_pol_step;
/// let (x, v) = van_der_pol_step(1.0, 0.0, 1.0, 0.01);
/// assert!(x.is_finite() && v.is_finite());
/// ```
pub fn van_der_pol_step(x: f32, v: f32, mu: f32, dt: f32) -> (f32, f32) {
    // System: dx/dt = v, dv/dt = mu*(1-x²)*v - x
    let f = |_t: f32, state: (f32, f32)| -> (f32, f32) {
        let (x, v) = state;
        (v, mu * (1.0 - x * x) * v - x)
    };
    // RK4 for 2D system
    let k1 = f(0.0, (x, v));
    let k2 = f(0.0, (x + 0.5 * dt * k1.0, v + 0.5 * dt * k1.1));
    let k3 = f(0.0, (x + 0.5 * dt * k2.0, v + 0.5 * dt * k2.1));
    let k4 = f(0.0, (x + dt * k3.0, v + dt * k3.1));
    (
        x + dt / 6.0 * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0),
        v + dt / 6.0 * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1),
    )
}

// ── Adaptive Simpson's quadrature ────────────────────────────────────────────

/// Adaptive Simpson's quadrature — automatically refines subdivisions
/// where the integrand varies rapidly.
///
/// O(h^4) per panel. Subdivides until |S_fine - S_coarse| < 15 * tol,
/// or max_depth is reached.
///
/// # Arguments
/// * `f` - Integrand
/// * `a`, `b` - Integration bounds
/// * `tol` - Error tolerance (e.g., 1e-6)
/// * `max_depth` - Maximum recursion depth (e.g., 20)
///
/// # Math
/// Uses the 3-point Simpson estimate S(a,b) = (b-a)/6 * (f(a) + 4*f(m) + f(b))
/// and compares against S(a,m) + S(m,b). Subdivides when they disagree.
/// ```rust
/// # use prime_dynamics::integrate_adaptive;
/// let area = integrate_adaptive(|x| x.sin(), 0.0, std::f32::consts::PI, 1e-6, 20);
/// assert!((area - 2.0).abs() < 1e-5);
/// ```
pub fn integrate_adaptive(
    f: fn(f32) -> f32,
    a: f32,
    b: f32,
    tol: f32,
    max_depth: u32,
) -> f32 {
    fn simpson(f: fn(f32) -> f32, a: f32, b: f32) -> f32 {
        let m = (a + b) / 2.0;
        (b - a) / 6.0 * (f(a) + 4.0 * f(m) + f(b))
    }

    // ADVANCE-EXCEPTION: recursion depth is bounded by max_depth
    fn recurse(
        f: fn(f32) -> f32,
        a: f32,
        b: f32,
        tol: f32,
        whole: f32,
        depth: u32,
    ) -> f32 {
        let m = (a + b) / 2.0;
        let left = simpson(f, a, m);
        let right = simpson(f, m, b);
        let refined = left + right;
        if depth == 0 || (refined - whole).abs() < 15.0 * tol {
            refined + (refined - whole) / 15.0 // Richardson extrapolation
        } else {
            let half_tol = tol / 2.0;
            recurse(f, a, m, half_tol, left, depth - 1)
                + recurse(f, m, b, half_tol, right, depth - 1)
        }
    }

    let whole = simpson(f, a, b);
    recurse(f, a, b, tol, whole, max_depth)
}

// ── Adaptive RK45 (Dormand-Prince) ODE solver ──────────────────────────────

/// Dormand-Prince RK45 adaptive ODE solver.
///
/// Automatically adjusts step size to maintain error within tolerance.
/// Uses two RK evaluations (4th and 5th order) to estimate local error.
///
/// # Arguments
/// * `state` - Initial state
/// * `t0` - Start time
/// * `t_end` - End time
/// * `dt_initial` - Initial step size guess
/// * `tol` - Error tolerance per step
/// * `f` - ODE right-hand side: dy/dt = f(t, y)
///
/// # Returns
/// `(final_state, final_time, steps_taken)` — state at t_end with step count.
/// ```rust
/// # use prime_dynamics::rk45_adaptive;
/// // dy/dt = -y, exact solution y = exp(-t)
/// let (y, t, steps) = rk45_adaptive(1.0, 0.0, 1.0, 0.1, 1e-6, |_t, y| -y);
/// assert!((y - (-1.0_f32).exp()).abs() < 1e-4);
/// assert!((t - 1.0).abs() < 1e-6);
/// assert!(steps > 0);
/// ```
pub fn rk45_adaptive(
    state: f32,
    t0: f32,
    t_end: f32,
    dt_initial: f32,
    tol: f32,
    f: impl Fn(f32, f32) -> f32,
) -> (f32, f32, u32) {
    // Dormand-Prince coefficients (Butcher tableau)
    let a2 = 1.0 / 5.0;
    let a3 = 3.0 / 10.0;
    let a4 = 4.0 / 5.0;
    let a5 = 8.0 / 9.0;

    let b21 = 1.0 / 5.0;
    let b31 = 3.0 / 40.0;
    let b32 = 9.0 / 40.0;
    let b41 = 44.0 / 45.0;
    let b42 = -56.0 / 15.0;
    let b43 = 32.0 / 9.0;
    let b51 = 19372.0 / 6561.0;
    let b52 = -25360.0 / 2187.0;
    let b53 = 64448.0 / 6561.0;
    let b54 = -212.0 / 729.0;
    let b61 = 9017.0 / 3168.0;
    let b62 = -355.0 / 33.0;
    let b63 = 46732.0 / 5247.0;
    let b64 = 49.0 / 176.0;
    let b65 = -5103.0 / 18656.0;

    // 5th order weights
    let c1 = 35.0 / 384.0;
    let c3 = 500.0 / 1113.0;
    let c4 = 125.0 / 192.0;
    let c5 = -2187.0 / 6784.0;
    let c6 = 11.0 / 84.0;

    // 4th order weights (for error estimate)
    let d1 = 5179.0 / 57600.0;
    let d3 = 7571.0 / 16695.0;
    let d4 = 393.0 / 640.0;
    let d5 = -92097.0 / 339200.0;
    let d6 = 187.0 / 2100.0;
    let d7 = 1.0 / 40.0;

    // ADVANCE-EXCEPTION: adaptive step loop with bounded iteration
    let mut y = state;
    let mut t = t0;
    let mut dt = dt_initial;
    let mut steps = 0u32;
    let max_steps = 100_000u32;

    while t < t_end && steps < max_steps {
        let dt_actual = dt.min(t_end - t);

        let k1 = f(t, y);
        let k2 = f(t + a2 * dt_actual, y + dt_actual * b21 * k1);
        let k3 = f(
            t + a3 * dt_actual,
            y + dt_actual * (b31 * k1 + b32 * k2),
        );
        let k4 = f(
            t + a4 * dt_actual,
            y + dt_actual * (b41 * k1 + b42 * k2 + b43 * k3),
        );
        let k5 = f(
            t + a5 * dt_actual,
            y + dt_actual * (b51 * k1 + b52 * k2 + b53 * k3 + b54 * k4),
        );
        let k6 = f(
            t + dt_actual,
            y + dt_actual * (b61 * k1 + b62 * k2 + b63 * k3 + b64 * k4 + b65 * k5),
        );

        // 5th order solution
        let y5 = y + dt_actual * (c1 * k1 + c3 * k3 + c4 * k4 + c5 * k5 + c6 * k6);

        // 4th order solution (for error estimate)
        let k7 = f(t + dt_actual, y5);
        let y4 =
            y + dt_actual * (d1 * k1 + d3 * k3 + d4 * k4 + d5 * k5 + d6 * k6 + d7 * k7);

        let error = (y5 - y4).abs();

        if error <= tol || dt_actual <= 1e-10 {
            // Accept step
            y = y5;
            t += dt_actual;
            steps += 1;
        }

        // Adjust step size
        if error > 0.0 {
            let factor = 0.9 * (tol / error).powf(0.2);
            dt = dt_actual * factor.clamp(0.1, 5.0);
        }
    }

    (y, t, steps)
}

// ── Newton-Raphson root finding ─────────────────────────────────────────────

/// Newton-Raphson root finding. Finds x where f(x) ≈ 0.
///
/// Uses numerical derivative (central difference) for the Jacobian.
///
/// # Arguments
/// * `f` - Function to find root of
/// * `x0` - Initial guess
/// * `tol` - Convergence tolerance
/// * `max_iter` - Maximum iterations
///
/// # Returns
/// `(root, iterations)` — the root and how many steps it took.
/// ```rust
/// # use prime_dynamics::newton_raphson;
/// let (root, _iters) = newton_raphson(|x| x * x - 2.0, 1.0, 1e-6, 50);
/// assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
/// ```
pub fn newton_raphson(f: fn(f32) -> f32, x0: f32, tol: f32, max_iter: u32) -> (f32, u32) {
    let h = 1e-5_f32;
    // ADVANCE-EXCEPTION: convergence loop with bounded iteration
    let mut x = x0;
    for i in 0..max_iter {
        let fx = f(x);
        if fx.abs() < tol {
            return (x, i);
        }
        let dfx = (f(x + h) - f(x - h)) / (2.0 * h);
        if dfx.abs() < 1e-12 {
            return (x, i); // derivative too small, return best guess
        }
        x -= fx / dfx;
    }
    (x, max_iter)
}

// ── Bisection root finding ──────────────────────────────────────────────────

/// Bisection root finding. Guaranteed convergence for continuous f with f(a)*f(b) < 0.
///
/// Slower than Newton but always converges. O(log((b-a)/tol)) iterations.
/// ```rust
/// # use prime_dynamics::bisection;
/// let (root, _iters) = bisection(|x| x * x - 2.0, 1.0, 2.0, 1e-6, 100);
/// assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5);
/// ```
pub fn bisection(f: fn(f32) -> f32, a: f32, b: f32, tol: f32, max_iter: u32) -> (f32, u32) {
    // ADVANCE-EXCEPTION: convergence loop
    let mut lo = a;
    let mut hi = b;
    let mut f_lo = f(lo);
    for i in 0..max_iter {
        let mid = (lo + hi) / 2.0;
        let f_mid = f(mid);
        if f_mid.abs() < tol || (hi - lo) < tol {
            return (mid, i);
        }
        if f_lo * f_mid < 0.0 {
            hi = mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    ((lo + hi) / 2.0, max_iter)
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

    // ── logistic ──────────────────────────────────────────────────────────────

    #[test]
    fn logistic_fixed_point_r2() {
        // r=2, x=0.5 is a fixed point: 2 * 0.5 * (1 - 0.5) = 0.5
        let x = logistic(0.5, 2.0);
        assert!((x - 0.5).abs() < EPSILON, "x={}", x);
    }

    #[test]
    fn logistic_r4_stays_in_unit() {
        // r=4 is fully chaotic but maps [0,1] → [0,1]
        let x = (0..1000).fold(0.1_f32, |x, _| logistic(x, 4.0));
        assert!(x >= 0.0 && x <= 1.0, "x={}", x);
    }

    #[test]
    fn logistic_deterministic() {
        let a = logistic(0.3, 3.7);
        let b = logistic(0.3, 3.7);
        assert!((a - b).abs() < EPSILON);
    }

    #[test]
    fn logistic_zero_input() {
        // x=0 is always a fixed point regardless of r
        let x = logistic(0.0, 3.9);
        assert!((x).abs() < EPSILON, "x={}", x);
    }

    // ── lotka_volterra_step ───────────────────────────────────────────────────

    #[test]
    fn lotka_volterra_populations_positive() {
        let (x, y) = (0..1000).fold((1.0_f32, 0.5_f32), |(x, y), _| {
            lotka_volterra_step(x, y, 1.1, 0.4, 0.1, 0.4, 0.01)
        });
        assert!(x > 0.0 && y > 0.0, "x={} y={}", x, y);
    }

    #[test]
    fn lotka_volterra_small_dt_bounded() {
        let (x, y) = (0..100).fold((2.0_f32, 1.0_f32), |(x, y), _| {
            lotka_volterra_step(x, y, 1.1, 0.4, 0.1, 0.4, 0.001)
        });
        assert!(x < 100.0 && y < 100.0, "x={} y={}", x, y);
    }

    #[test]
    fn lotka_volterra_deterministic() {
        let a = lotka_volterra_step(1.0, 0.5, 1.1, 0.4, 0.1, 0.4, 0.01);
        let b = lotka_volterra_step(1.0, 0.5, 1.1, 0.4, 0.1, 0.4, 0.01);
        assert!((a.0 - b.0).abs() < EPSILON && (a.1 - b.1).abs() < EPSILON);
    }

    // ── sir_step ──────────────────────────────────────────────────────────────

    #[test]
    fn sir_population_conserved() {
        let (s, i, r) = (0..1000).fold((0.99_f32, 0.01_f32, 0.0_f32), |(s, i, r), _| {
            sir_step(s, i, r, 0.3, 0.1, 0.1)
        });
        let total = s + i + r;
        assert!((total - 1.0).abs() < EPSILON, "total={}", total);
    }

    #[test]
    fn sir_no_infected_no_change() {
        // With i=0, nothing should change
        let (s, i, r) = sir_step(1.0, 0.0, 0.0, 0.3, 0.1, 0.1);
        assert!((s - 1.0).abs() < EPSILON);
        assert!((i).abs() < EPSILON);
        assert!((r).abs() < EPSILON);
    }

    #[test]
    fn sir_deterministic() {
        let a = sir_step(0.99, 0.01, 0.0, 0.3, 0.1, 0.1);
        let b = sir_step(0.99, 0.01, 0.0, 0.3, 0.1, 0.1);
        assert!((a.0 - b.0).abs() < EPSILON);
        assert!((a.1 - b.1).abs() < EPSILON);
        assert!((a.2 - b.2).abs() < EPSILON);
    }

    // ── gray_scott_step ───────────────────────────────────────────────────────

    #[test]
    fn gray_scott_stable_without_v() {
        // u=1, v=0: no reaction occurs, u drifts toward 1 via feed term
        let (u, v) = gray_scott_step(1.0, 0.0, 0.0, 0.0, 0.04, 0.06, 0.01);
        assert!((u - 1.0).abs() < EPSILON, "u={}", u);
        assert!((v).abs() < EPSILON, "v={}", v);
    }

    #[test]
    fn gray_scott_reaction_with_v() {
        // When both u and v are present, reaction should change concentrations
        let (u1, v1) = gray_scott_step(0.5, 0.25, 0.0, 0.0, 0.04, 0.06, 0.1);
        assert!((u1 - 0.5).abs() > EPSILON || (v1 - 0.25).abs() > EPSILON,
            "u1={} v1={}", u1, v1);
    }

    #[test]
    fn gray_scott_deterministic() {
        let a = gray_scott_step(0.5, 0.25, 0.1, -0.05, 0.04, 0.06, 0.01);
        let b = gray_scott_step(0.5, 0.25, 0.1, -0.05, 0.04, 0.06, 0.01);
        assert!((a.0 - b.0).abs() < EPSILON && (a.1 - b.1).abs() < EPSILON);
    }

    #[test]
    fn gray_scott_zero_dt_no_change() {
        let (u, v) = gray_scott_step(0.5, 0.3, 0.1, 0.1, 0.04, 0.06, 0.0);
        assert!((u - 0.5).abs() < EPSILON && (v - 0.3).abs() < EPSILON);
    }

    // ── lsystem_step / lsystem_generate ──────────────────────────────────────

    #[test]
    fn lsystem_algae() {
        // Classic Lindenmayer algae: A->AB, B->A
        let rules = vec![
            LRule { symbol: 'A', replacement: "AB" },
            LRule { symbol: 'B', replacement: "A" },
        ];
        assert_eq!(lsystem_step("A", &rules), "AB");
        assert_eq!(lsystem_step("AB", &rules), "ABA");
        assert_eq!(lsystem_step("ABA", &rules), "ABAAB");
    }

    #[test]
    fn lsystem_fibonacci_length() {
        // L-system produces Fibonacci sequence in string lengths
        let rules = vec![
            LRule { symbol: 'A', replacement: "AB" },
            LRule { symbol: 'B', replacement: "A" },
        ];
        let lengths: Vec<usize> = (0..8).fold((vec![1], "A".to_string()), |(mut lens, s), _| {
            let next = lsystem_step(&s, &rules);
            lens.push(next.len());
            (lens, next)
        }).0;
        // Fibonacci: 1, 2, 3, 5, 8, 13, 21, 34
        assert_eq!(&lengths[..8], &[1, 2, 3, 5, 8, 13, 21, 34]);
    }

    #[test]
    fn lsystem_koch_curve() {
        // Koch curve: F->F+F-F-F+F
        let rules = vec![
            LRule { symbol: 'F', replacement: "F+F-F-F+F" },
        ];
        let gen1 = lsystem_step("F", &rules);
        assert_eq!(gen1, "F+F-F-F+F");
        // + and - are preserved (no rule)
    }

    #[test]
    fn lsystem_identity_for_unmapped() {
        let rules = vec![
            LRule { symbol: 'A', replacement: "B" },
        ];
        assert_eq!(lsystem_step("AXA", &rules), "BXB");
    }

    #[test]
    fn lsystem_empty_axiom() {
        let rules = vec![LRule { symbol: 'A', replacement: "B" }];
        assert_eq!(lsystem_step("", &rules), "");
    }

    #[test]
    fn lsystem_generate_multi_step() {
        let rules = vec![
            LRule { symbol: 'A', replacement: "AB" },
            LRule { symbol: 'B', replacement: "A" },
        ];
        assert_eq!(lsystem_generate("A", &rules, 0), "A");
        assert_eq!(lsystem_generate("A", &rules, 1), "AB");
        assert_eq!(lsystem_generate("A", &rules, 5), "ABAABABAABAAB");
    }

    #[test]
    fn lsystem_deterministic() {
        let rules = vec![
            LRule { symbol: 'F', replacement: "FF" },
        ];
        assert_eq!(lsystem_generate("F", &rules, 3), lsystem_generate("F", &rules, 3));
    }

    // ── derivative ───────────────────────────────────────────────────────────

    #[test]
    fn derivative_x_squared() {
        let d = derivative(|x| x * x, 3.0, 1e-3);
        assert!((d - 6.0).abs() < 1e-3, "d={d}");
    }

    #[test]
    fn derivative_sin() {
        let d = derivative(|x| x.sin(), 0.0, 1e-3);
        assert!((d - 1.0).abs() < 1e-3, "d={d}"); // cos(0) = 1
    }

    #[test]
    fn derivative2_x_cubed() {
        let d2 = derivative2(|x| x * x * x, 2.0, 1e-2);
        assert!((d2 - 12.0).abs() < 0.1, "d2={d2}"); // 6x = 12 at x=2
    }

    #[test]
    fn gradient_2d_paraboloid() {
        let (gx, gy) = gradient_2d(|x, y| x * x + y * y, 3.0, 4.0, 1e-3);
        assert!((gx - 6.0).abs() < 1e-3, "gx={gx}");
        assert!((gy - 8.0).abs() < 1e-3, "gy={gy}");
    }

    // ── integration ──────────────────────────────────────────────────────────

    #[test]
    fn trapezoidal_x_squared() {
        let area = integrate_trapezoidal(|x| x * x, 0.0, 1.0, 1000);
        assert!((area - 1.0 / 3.0).abs() < 1e-4, "area={area}");
    }

    #[test]
    fn trapezoidal_sin() {
        let area = integrate_trapezoidal(|x| x.sin(), 0.0, std::f32::consts::PI, 1000);
        assert!((area - 2.0).abs() < 1e-4, "area={area}");
    }

    #[test]
    fn simpson_x_squared() {
        let area = integrate_simpson(|x| x * x, 0.0, 1.0, 100);
        assert!((area - 1.0 / 3.0).abs() < 1e-6, "area={area}");
    }

    #[test]
    fn simpson_more_accurate_than_trapezoidal() {
        let true_val = 1.0_f32 / 3.0;
        let trap = integrate_trapezoidal(|x| x * x, 0.0, 1.0, 100);
        let simp = integrate_simpson(|x| x * x, 0.0, 1.0, 100);
        assert!((simp - true_val).abs() < (trap - true_val).abs());
    }

    // ── van_der_pol ──────────────────────────────────────────────────────────

    #[test]
    fn van_der_pol_mu_zero_is_harmonic() {
        // mu=0: simple harmonic oscillator, energy conserved
        let (x, v) = (0..10000).fold((1.0_f32, 0.0_f32), |(x, v), _| {
            van_der_pol_step(x, v, 0.0, 0.001)
        });
        let energy = x * x + v * v;
        assert!((energy - 1.0).abs() < 0.01, "energy={energy}");
    }

    #[test]
    fn van_der_pol_limit_cycle() {
        // mu>0: should converge to limit cycle (bounded amplitude)
        let (x, v) = (0..50000).fold((0.1_f32, 0.0_f32), |(x, v), _| {
            van_der_pol_step(x, v, 1.0, 0.001)
        });
        assert!(x.abs() < 3.0 && v.abs() < 5.0, "x={x}, v={v} — should be bounded");
    }

    #[test]
    fn van_der_pol_deterministic() {
        let a = van_der_pol_step(1.0, 0.0, 1.0, 0.01);
        let b = van_der_pol_step(1.0, 0.0, 1.0, 0.01);
        assert_eq!(a, b);
    }

    // ── integrate_adaptive ──────────────────────────────────────────────────

    #[test]
    fn adaptive_sin_over_pi() {
        let area = integrate_adaptive(|x| x.sin(), 0.0, std::f32::consts::PI, 1e-6, 20);
        assert!((area - 2.0).abs() < 1e-5, "area={area}");
    }

    #[test]
    fn adaptive_x_squared() {
        let area = integrate_adaptive(|x| x * x, 0.0, 1.0, 1e-6, 20);
        assert!((area - 1.0 / 3.0).abs() < 1e-5, "area={area}");
    }

    #[test]
    fn adaptive_more_accurate_than_fixed_simpson() {
        // Integrate a rapidly varying function where adaptive should shine
        let f: fn(f32) -> f32 = |x| (10.0 * x).sin();
        let true_val = (1.0 - (10.0_f32).cos()) / 10.0;
        let adaptive = integrate_adaptive(f, 0.0, 1.0, 1e-6, 20);
        let fixed = integrate_simpson(f, 0.0, 1.0, 10); // same low panel count
        assert!(
            (adaptive - true_val).abs() < (fixed - true_val).abs(),
            "adaptive={adaptive} fixed={fixed} true={true_val}"
        );
    }

    #[test]
    fn adaptive_constant_function() {
        let area = integrate_adaptive(|_| 5.0, 0.0, 3.0, 1e-6, 20);
        assert!((area - 15.0).abs() < 1e-4, "area={area}");
    }

    #[test]
    fn adaptive_deterministic() {
        let a = integrate_adaptive(|x| x.sin(), 0.0, 1.0, 1e-6, 20);
        let b = integrate_adaptive(|x| x.sin(), 0.0, 1.0, 1e-6, 20);
        assert!((a - b).abs() < EPSILON);
    }

    // ── rk45_adaptive ───────────────────────────────────────────────────────

    #[test]
    fn rk45_exponential_decay() {
        let (y, t, steps) = rk45_adaptive(1.0, 0.0, 1.0, 0.1, 1e-6, |_t, y| -y);
        assert!((y - (-1.0_f32).exp()).abs() < 1e-4, "y={y}");
        assert!((t - 1.0).abs() < 1e-6, "t={t}");
        assert!(steps > 0, "steps={steps}");
    }

    #[test]
    fn rk45_linear_ode() {
        // dy/dt = 1 → y = t + y0
        let (y, t, _) = rk45_adaptive(0.0, 0.0, 2.0, 0.1, 1e-6, |_t, _y| 1.0);
        assert!((y - 2.0).abs() < 1e-4, "y={y}");
        assert!((t - 2.0).abs() < 1e-6, "t={t}");
    }

    #[test]
    fn rk45_reasonable_step_count() {
        // For a smooth ODE, should not take an absurd number of steps
        let (_, _, steps) = rk45_adaptive(1.0, 0.0, 1.0, 0.1, 1e-6, |_t, y| -y);
        assert!(steps < 1000, "steps={steps} — too many for smooth ODE");
    }

    #[test]
    fn rk45_deterministic() {
        let a = rk45_adaptive(1.0, 0.0, 1.0, 0.1, 1e-6, |_t, y| -y);
        let b = rk45_adaptive(1.0, 0.0, 1.0, 0.1, 1e-6, |_t, y| -y);
        assert!((a.0 - b.0).abs() < EPSILON);
        assert_eq!(a.2, b.2);
    }

    // ── newton_raphson ──────────────────────────────────────────────────────

    #[test]
    fn newton_sqrt2() {
        let (root, iters) = newton_raphson(|x| x * x - 2.0, 1.0, 1e-6, 50);
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5, "root={root}");
        assert!(iters < 20, "iters={iters}");
    }

    #[test]
    fn newton_cube_root() {
        let (root, _) = newton_raphson(|x| x * x * x - 8.0, 1.0, 1e-6, 50);
        assert!((root - 2.0).abs() < 1e-4, "root={root}");
    }

    #[test]
    fn newton_converges_fast() {
        // Quadratic convergence — should find sqrt(2) in very few iterations
        let (_, iters) = newton_raphson(|x| x * x - 2.0, 1.0, 1e-6, 50);
        assert!(iters <= 10, "iters={iters} — Newton should converge fast");
    }

    #[test]
    fn newton_deterministic() {
        let a = newton_raphson(|x| x * x - 2.0, 1.0, 1e-6, 50);
        let b = newton_raphson(|x| x * x - 2.0, 1.0, 1e-6, 50);
        assert!((a.0 - b.0).abs() < EPSILON);
        assert_eq!(a.1, b.1);
    }

    // ── bisection ───────────────────────────────────────────────────────────

    #[test]
    fn bisection_sqrt2() {
        let (root, _) = bisection(|x| x * x - 2.0, 1.0, 2.0, 1e-6, 100);
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5, "root={root}");
    }

    #[test]
    fn bisection_guaranteed_convergence() {
        // Bisection always converges if bracket is valid
        let (root, iters) = bisection(|x| x * x - 2.0, 0.0, 10.0, 1e-6, 100);
        assert!((root - std::f32::consts::SQRT_2).abs() < 1e-5, "root={root}");
        assert!(iters < 100, "iters={iters}");
    }

    #[test]
    fn bisection_cos_root() {
        // cos(x) = 0 in [1, 2] → x = π/2
        let (root, _) = bisection(|x| x.cos(), 1.0, 2.0, 1e-6, 100);
        assert!(
            (root - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "root={root}"
        );
    }

    #[test]
    fn bisection_deterministic() {
        let a = bisection(|x| x * x - 2.0, 1.0, 2.0, 1e-6, 100);
        let b = bisection(|x| x * x - 2.0, 1.0, 2.0, 1e-6, 100);
        assert!((a.0 - b.0).abs() < EPSILON);
        assert_eq!(a.1, b.1);
    }
}
