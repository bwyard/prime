/**
 * prime-dynamics — Dynamical systems and numerical integration.
 *
 * Pure functions for integrating ODEs and simulating nonlinear dynamical
 * systems. No mutation, no hidden state. Same inputs always produce the same
 * output.
 *
 * Temporal assembly:
 *   LOAD    ← state, t, dt, derivative fn
 *   COMPUTE ← RK4 or Euler update rule
 *   APPEND  ← return new state as tuple
 *   ADVANCE ← caller folds over time steps with render/reduce
 */

// ── RK4 integrator ────────────────────────────────────────────────────────────

/**
 * Advance a scalar state by one 4th-order Runge-Kutta step.
 *
 * Math:
 *   k1 = f(t,        s)
 *   k2 = f(t + dt/2, s + dt/2 * k1)
 *   k3 = f(t + dt/2, s + dt/2 * k2)
 *   k4 = f(t + dt,   s + dt   * k3)
 *   s' = s + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
 *
 * @param state - current scalar state
 * @param t     - current time (seconds)
 * @param dt    - time step (seconds)
 * @param f     - derivative `f(t, state) → ds/dt`
 * @returns new state after one RK4 step
 *
 * @example
 * // Exponential decay: ds/dt = -s → s(t) = e^(-t)
 * const s1 = rk4Step(1, 0, 0.01, (_t, s) => -s)
 * // s1 ≈ e^(-0.01) ≈ 0.99005
 */
export const rk4Step = (
  state: number,
  t: number,
  dt: number,
  f: (t: number, state: number) => number,
): number => {
  const k1 = f(t, state)
  const k2 = f(t + dt * 0.5, state + dt * 0.5 * k1)
  const k3 = f(t + dt * 0.5, state + dt * 0.5 * k2)
  const k4 = f(t + dt, state + dt * k3)
  return state + (dt / 6) * (k1 + 2 * k2 + 2 * k3 + k4)
}

/**
 * Advance a `[x, y, z]` state by one 4th-order Runge-Kutta step.
 *
 * Used for 3D dynamical systems like Lorenz, Rössler, and Duffing.
 *
 * @param state - `[x, y, z]` current state
 * @param t     - current time (seconds)
 * @param dt    - time step (seconds)
 * @param f     - derivative `f(t, [x,y,z]) → [dx, dy, dz]`
 * @returns new `[x, y, z]` state after one RK4 step
 *
 * @example
 * // Circular motion in the XY plane
 * const [x1, y1] = rk4Step3([1, 0, 0], 0, 0.01, (_t, [x, y]) => [-y, x, 0])
 */
export const rk4Step3 = (
  state: [number, number, number],
  t: number,
  dt: number,
  f: (t: number, s: [number, number, number]) => [number, number, number],
): [number, number, number] => {
  const add = ([ax, ay, az]: [number, number, number], [bx, by, bz]: [number, number, number]): [number, number, number] =>
    [ax + bx, ay + by, az + bz]
  const scale = ([dx, dy, dz]: [number, number, number], s: number): [number, number, number] =>
    [dx * s, dy * s, dz * s]

  const k1 = f(t, state)
  const k2 = f(t + dt * 0.5, add(state, scale(k1, dt * 0.5)))
  const k3 = f(t + dt * 0.5, add(state, scale(k2, dt * 0.5)))
  const k4 = f(t + dt, add(state, scale(k3, dt)))

  const [sx, sy, sz] = state
  const [k1x, k1y, k1z] = k1
  const [k2x, k2y, k2z] = k2
  const [k3x, k3y, k3z] = k3
  const [k4x, k4y, k4z] = k4
  return [
    sx + (dt / 6) * (k1x + 2 * k2x + 2 * k3x + k4x),
    sy + (dt / 6) * (k1y + 2 * k2y + 2 * k3y + k4y),
    sz + (dt / 6) * (k1z + 2 * k2z + 2 * k3z + k4z),
  ]
}

/**
 * Advance a scalar state by one forward Euler step.
 *
 * First-order accuracy. Use `rk4Step` for higher precision.
 *
 * @param state - current scalar state
 * @param t     - current time
 * @param dt    - time step
 * @param f     - derivative `f(t, state) → ds/dt`
 * @returns `state + dt * f(t, state)`
 *
 * @example
 * const s1 = eulerStep(1, 0, 0.1, (_t, _s) => -1)
 * // s1 = 0.9
 */
export const eulerStep = (
  state: number,
  t: number,
  dt: number,
  f: (t: number, state: number) => number,
): number => state + dt * f(t, state)

// ── Lorenz attractor ──────────────────────────────────────────────────────────

/** Lorenz canonical sigma (Prandtl number) = 10 */
export const LORENZ_SIGMA = 10
/** Lorenz canonical rho (Rayleigh number) = 28 */
export const LORENZ_RHO = 28
/** Lorenz canonical beta (geometric factor) = 8/3 */
export const LORENZ_BETA = 8 / 3

/**
 * Advance the Lorenz attractor state by one RK4 step.
 *
 * The Lorenz system (1963) is the canonical example of deterministic chaos.
 * Form uses it to drive procedural animation: x=lateral sway, y=vertical bob,
 * z=sagittal rotation.
 *
 * Math:
 *   dx/dt = σ(y - x)
 *   dy/dt = x(ρ - z) - y
 *   dz/dt = xy - βz
 *
 * @param state - `[x, y, z]` current attractor state
 * @param sigma - Prandtl number (typically 10)
 * @param rho   - Rayleigh number (typically 28; chaos above ~24.74)
 * @param beta  - geometric factor (typically 8/3 ≈ 2.667)
 * @param dt    - time step (keep ≤ 0.01 for numerical stability)
 * @returns new `[x, y, z]` state
 *
 * @example
 * const s1 = lorenzStep([1, 1, 1], LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
 */
export const lorenzStep = (
  state: [number, number, number],
  sigma: number,
  rho: number,
  beta: number,
  dt: number,
): [number, number, number] =>
  rk4Step3(state, 0, dt, (_t, [x, y, z]) => [
    sigma * (y - x),
    x * (rho - z) - y,
    x * y - beta * z,
  ])

// ── Rössler attractor ─────────────────────────────────────────────────────────

/**
 * Advance the Rössler attractor state by one RK4 step.
 *
 * Simpler single-scroll chaotic attractor, smoother than Lorenz.
 *
 * Math:
 *   dx/dt = -(y + z)
 *   dy/dt = x + a*y
 *   dz/dt = b + z*(x - c)
 *
 * @param state - `[x, y, z]` current state
 * @param a     - typically 0.2
 * @param b     - typically 0.2
 * @param c     - typically 5.7 (chaos above ~3.0)
 * @param dt    - time step (keep ≤ 0.05)
 * @returns new `[x, y, z]` Rössler state
 *
 * @example
 * const s1 = rosslerStep([1, 0, 0], 0.2, 0.2, 5.7, 0.01)
 */
export const rosslerStep = (
  state: [number, number, number],
  a: number,
  b: number,
  c: number,
  dt: number,
): [number, number, number] =>
  rk4Step3(state, 0, dt, (_t, [x, y, z]) => [
    -(y + z),
    x + a * y,
    b + z * (x - c),
  ])

// ── Duffing oscillator ────────────────────────────────────────────────────────

/** Duffing oscillator parameters. */
export interface DuffingParams {
  /** Damping coefficient (typically 0.3) */
  delta: number
  /** Linear stiffness (typically -1.0) */
  alpha: number
  /** Cubic stiffness (typically 1.0) */
  beta: number
  /** Driving amplitude (typically 0.37) */
  gamma: number
  /** Driving frequency (typically 1.2) */
  omega: number
}

/**
 * Advance the Duffing oscillator state by one RK4 step.
 *
 * Models a damped, driven nonlinear spring. State is `[position, velocity]`.
 *
 * Math:
 *   dx/dt = v
 *   dv/dt = -δv - αx - βx³ + γcos(ωt)
 *
 * @param state - `[x, v]` position and velocity
 * @param t     - current time (seconds; used for driving term)
 * @param p     - Duffing parameters
 * @param dt    - time step
 * @returns new `[x, v]` state
 *
 * @example
 * const p = { delta: 0.3, alpha: -1, beta: 1, gamma: 0.37, omega: 1.2 }
 * const [, v1] = duffingStep([0, 0], 0, p, 0.01)
 * // v1 > 0 (driving force kicks it)
 */
// ── Logistic map ─────────────────────────────────────────────────────────────

/**
 * Logistic map: x_{n+1} = r * x * (1 - x). Exhibits chaos for r > 3.57.
 *
 * @param x - current value in [0, 1]
 * @param r - growth rate parameter
 * @returns next value
 *
 * @example
 * logistic(0.5, 2) // 0.5 (fixed point)
 */
export const logistic = (x: number, r: number): number => r * x * (1 - x)

// ── Lotka-Volterra (predator-prey) ───────────────────────────────────────────

/**
 * Lotka-Volterra predator-prey step via Euler.
 *
 * dx/dt = alpha*x - beta*x*y  (prey growth - predation)
 * dy/dt = delta*x*y - gamma*y (predator growth - death)
 *
 * @param x     - prey population
 * @param y     - predator population
 * @param alpha - prey birth rate
 * @param beta  - predation rate
 * @param delta - predator growth from predation
 * @param gamma - predator death rate
 * @param dt    - time step
 * @returns [nextX, nextY]
 */
export const lotkaVolterraStep = (
  x: number,
  y: number,
  alpha: number,
  beta: number,
  delta: number,
  gamma: number,
  dt: number,
): [number, number] => {
  const dx = (alpha * x - beta * x * y) * dt
  const dy = (delta * x * y - gamma * y) * dt
  return [x + dx, y + dy]
}

// ── SIR epidemiological model ────────────────────────────────────────────────

/**
 * SIR epidemiological model step.
 *
 * dS/dt = -beta*S*I, dI/dt = beta*S*I - gamma*I, dR/dt = gamma*I
 *
 * @param s     - susceptible fraction
 * @param i     - infected fraction
 * @param r     - recovered fraction
 * @param beta  - infection rate
 * @param gamma - recovery rate
 * @param dt    - time step
 * @returns [nextS, nextI, nextR]
 */
export const sirStep = (
  s: number,
  i: number,
  r: number,
  beta: number,
  gamma: number,
  dt: number,
): [number, number, number] => {
  const ds = -beta * s * i * dt
  const di = (beta * s * i - gamma * i) * dt
  const dr = gamma * i * dt
  return [s + ds, i + di, r + dr]
}

// ── Gray-Scott reaction-diffusion ────────────────────────────────────────────

/**
 * Gray-Scott reaction-diffusion step for a single cell.
 *
 * du/dt = Du*laplacian_u - u*v^2 + f*(1-u)
 * dv/dt = Dv*laplacian_v + u*v^2 - (f+k)*v
 *
 * @param u          - concentration of U
 * @param v          - concentration of V
 * @param laplacianU - discrete Laplacian of U
 * @param laplacianV - discrete Laplacian of V
 * @param f          - feed rate
 * @param k          - kill rate
 * @param dt         - time step
 * @returns [nextU, nextV]
 */
export const grayScottStep = (
  u: number,
  v: number,
  laplacianU: number,
  laplacianV: number,
  f: number,
  k: number,
  dt: number,
): [number, number] => {
  const duDt = laplacianU - u * v * v + f * (1 - u)
  const dvDt = laplacianV + u * v * v - (f + k) * v
  return [u + duDt * dt, v + dvDt * dt]
}

// ── L-systems ────────────────────────────────────────────────────────────────

/** Single L-system production rule: maps a character to its replacement string. */
export type LRule = { readonly symbol: string; readonly replacement: string }

/**
 * Apply one L-system generation step. Pure LOAD + COMPUTE + APPEND.
 *
 * Each character in `axiom` is replaced by its matching rule's replacement string.
 * Characters without matching rules are copied unchanged (identity rule).
 *
 * Math:
 *   L-system: G = (V, w, P) where V = alphabet, w = axiom, P = production rules.
 *   Each generation: sigma(w) = P(c1) ++ P(c2) ++ ... ++ P(cn)
 *
 * @param axiom - current string state
 * @param rules - production rules
 * @returns next generation string
 */
export const lsystemStep = (axiom: string, rules: readonly LRule[]): string =>
  Array.from(axiom).map(c => {
    const rule = rules.find(r => r.symbol === c)
    return rule ? rule.replacement : c
  }).join('')

/**
 * Apply n generations of L-system rules. Pure fold over generations.
 *
 * Math: sigma^n(w) = sigma(sigma(...sigma(w)...)) applied `generations` times.
 *
 * @param axiom       - initial axiom string
 * @param rules       - production rules
 * @param generations - number of generations to apply
 * @returns string after n generations
 */
export const lsystemGenerate = (axiom: string, rules: readonly LRule[], generations: number): string =>
  Array.from({ length: generations }).reduce<string>(
    (current) => lsystemStep(current, rules),
    axiom,
  )

// ── Numerical differentiation ────────────────────────────────────────────────

/**
 * Numerical derivative via central difference. `f'(x) ≈ (f(x+h) - f(x-h)) / 2h`.
 *
 * More accurate than forward difference (O(h²) vs O(h) error).
 *
 * @param f - scalar function to differentiate
 * @param x - point at which to evaluate derivative
 * @param h - step size
 * @returns approximate f'(x)
 *
 * @example
 * const d = derivative(x => x * x, 3, 1e-5)
 * // d ≈ 6 (d/dx(x²) = 2x = 6 at x=3)
 */
export const derivative = (
  f: (x: number) => number,
  x: number,
  h: number,
): number => (f(x + h) - f(x - h)) / (2 * h)

/**
 * Second derivative via central difference. `f''(x) ≈ (f(x+h) - 2f(x) + f(x-h)) / h²`.
 *
 * @param f - scalar function
 * @param x - point at which to evaluate
 * @param h - step size
 * @returns approximate f''(x)
 *
 * @example
 * const d2 = derivative2(x => x * x * x, 2, 1e-4)
 * // d2 ≈ 12 (d²/dx²(x³) = 6x = 12 at x=2)
 */
export const derivative2 = (
  f: (x: number) => number,
  x: number,
  h: number,
): number => (f(x + h) - 2 * f(x) + f(x - h)) / (h * h)

/**
 * Numerical gradient of a 2D function via central differences.
 *
 * @param f - function of (x, y)
 * @param x - x coordinate
 * @param y - y coordinate
 * @param h - step size
 * @returns [df/dx, df/dy]
 *
 * @example
 * const [gx, gy] = gradient2d((x, y) => x * x + y * y, 3, 4, 1e-5)
 * // gx ≈ 6, gy ≈ 8
 */
export const gradient2d = (
  f: (x: number, y: number) => number,
  x: number,
  y: number,
  h: number,
): [number, number] => {
  const dx = (f(x + h, y) - f(x - h, y)) / (2 * h)
  const dy = (f(x, y + h) - f(x, y - h)) / (2 * h)
  return [dx, dy]
}

// ── Numerical integration ────────────────────────────────────────────────────

/**
 * Trapezoidal rule integration of f over [a, b] with n subdivisions.
 *
 * `∫f(x)dx ≈ h/2 * (f(a) + 2*f(x₁) + 2*f(x₂) + ... + f(b))`
 *
 * @param f - integrand
 * @param a - lower bound
 * @param b - upper bound
 * @param n - number of subdivisions
 * @returns approximate integral
 *
 * @example
 * const area = integrateTrapezoidal(x => x * x, 0, 1, 1000)
 * // area ≈ 1/3
 */
export const integrateTrapezoidal = (
  f: (x: number) => number,
  a: number,
  b: number,
  n: number,
): number => {
  const h = (b - a) / n
  const interior = Array.from({ length: n - 1 }).reduce<number>(
    (sum, _, idx) => sum + f(a + (idx + 1) * h),
    0,
  )
  return h * (f(a) / 2 + interior + f(b) / 2)
}

/**
 * Simpson's rule integration of f over [a, b] with n subdivisions (n must be even).
 *
 * `∫f(x)dx ≈ h/3 * (f(a) + 4*f(x₁) + 2*f(x₂) + 4*f(x₃) + ... + f(b))`
 *
 * O(h⁴) error — much more accurate than trapezoidal for smooth functions.
 *
 * @param f - integrand
 * @param a - lower bound
 * @param b - upper bound
 * @param n - number of subdivisions (rounded up to even if odd)
 * @returns approximate integral
 *
 * @example
 * const area = integrateSimpson(x => x * x, 0, 1, 100)
 * // area ≈ 1/3
 */
export const integrateSimpson = (
  f: (x: number) => number,
  a: number,
  b: number,
  n: number,
): number => {
  const ne = n % 2 === 1 ? n + 1 : n // ensure even
  const h = (b - a) / ne
  const sum = Array.from({ length: ne - 1 }).reduce<number>(
    (acc, _, idx) => {
      const i = idx + 1
      const coeff = i % 2 === 0 ? 2 : 4
      return acc + coeff * f(a + i * h)
    },
    0,
  )
  return (h / 3) * (f(a) + sum + f(b))
}

// ── Van der Pol oscillator ───────────────────────────────────────────────────

/**
 * Van der Pol oscillator step via RK4.
 *
 * `x'' - μ(1 - x²)x' + x = 0`
 *
 * Relaxation oscillator — self-sustaining oscillations with nonlinear damping.
 * μ=0 is a simple harmonic oscillator. μ>0 exhibits limit cycle behavior.
 *
 * @param x  - current position
 * @param v  - current velocity
 * @param mu - nonlinearity parameter
 * @param dt - time step
 * @returns [nextX, nextV]
 *
 * @example
 * const [x1, v1] = vanDerPolStep(1, 0, 1, 0.01)
 */
export const vanDerPolStep = (
  x: number,
  v: number,
  mu: number,
  dt: number,
): [number, number] => {
  const f = (_t: number, state: [number, number]): [number, number] => {
    const [sx, sv] = state
    return [sv, mu * (1 - sx * sx) * sv - sx]
  }
  const k1 = f(0, [x, v])
  const k2 = f(0, [x + 0.5 * dt * k1[0], v + 0.5 * dt * k1[1]])
  const k3 = f(0, [x + 0.5 * dt * k2[0], v + 0.5 * dt * k2[1]])
  const k4 = f(0, [x + dt * k3[0], v + dt * k3[1]])
  return [
    x + (dt / 6) * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
    v + (dt / 6) * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]),
  ]
}

// ── Duffing oscillator ────────────────────────────────────────────────────────

/**
 * Advance the Duffing oscillator state by one RK4 step.
 *
 * Models a damped, driven nonlinear spring. State is `[position, velocity]`.
 *
 * Math:
 *   dx/dt = v
 *   dv/dt = -δv - αx - βx³ + γcos(ωt)
 *
 * @param state - `[x, v]` position and velocity
 * @param t     - current time (seconds; used for driving term)
 * @param p     - Duffing parameters (see `DuffingParams`)
 * @param dt    - time step
 * @returns new `[x, v]` state
 *
 * @example
 * const p = { delta: 0.3, alpha: -1, beta: 1, gamma: 0.37, omega: 1.2 }
 * const [x1, v1] = duffingStep([0, 0], 0, p, 0.01)
 */
export const duffingStep = (
  state: [number, number],
  t: number,
  p: DuffingParams,
  dt: number,
): [number, number] => {
  const { delta, alpha, beta, gamma, omega } = p
  const deriv = (ti: number, [x, v]: [number, number]): [number, number] => [
    v,
    -delta * v - alpha * x - beta * x ** 3 + gamma * Math.cos(omega * ti),
  ]

  const [x0, v0] = state
  const [k1x, k1v] = deriv(t, state)
  const [k2x, k2v] = deriv(t + dt * 0.5, [x0 + dt * 0.5 * k1x, v0 + dt * 0.5 * k1v])
  const [k3x, k3v] = deriv(t + dt * 0.5, [x0 + dt * 0.5 * k2x, v0 + dt * 0.5 * k2v])
  const [k4x, k4v] = deriv(t + dt, [x0 + dt * k3x, v0 + dt * k3v])

  return [
    x0 + (dt / 6) * (k1x + 2 * k2x + 2 * k3x + k4x),
    v0 + (dt / 6) * (k1v + 2 * k2v + 2 * k3v + k4v),
  ]
}

// ── Adaptive Simpson's quadrature ────────────────────────────────────────────

/**
 * Adaptive Simpson's quadrature — automatically refines subdivisions
 * where the integrand varies rapidly.
 *
 * O(h^4) per panel. Subdivides until |S_fine - S_coarse| < 15 * tol,
 * or maxDepth is reached.
 *
 * Math: S(a,b) = (b-a)/6 * (f(a) + 4*f(m) + f(b))
 *
 * @param f - integrand
 * @param a - lower bound
 * @param b - upper bound
 * @param tol - error tolerance (e.g., 1e-6)
 * @param maxDepth - maximum recursion depth (e.g., 20)
 * @returns approximate integral
 */
export const integrateAdaptive = (
  f: (x: number) => number,
  a: number,
  b: number,
  tol: number,
  maxDepth: number,
): number => {
  const simpson = (a: number, b: number): number => {
    const m = (a + b) / 2
    return ((b - a) / 6) * (f(a) + 4 * f(m) + f(b))
  }

  // ADVANCE-EXCEPTION: recursion depth bounded by maxDepth
  const recurse = (
    a: number,
    b: number,
    tol: number,
    whole: number,
    depth: number,
  ): number => {
    const m = (a + b) / 2
    const left = simpson(a, m)
    const right = simpson(m, b)
    const refined = left + right
    if (depth === 0 || Math.abs(refined - whole) < 15 * tol) {
      return refined + (refined - whole) / 15 // Richardson extrapolation
    }
    const halfTol = tol / 2
    return recurse(a, m, halfTol, left, depth - 1)
      + recurse(m, b, halfTol, right, depth - 1)
  }

  const whole = simpson(a, b)
  return recurse(a, b, tol, whole, maxDepth)
}

// ── Adaptive RK45 (Dormand-Prince) ODE solver ───────────────────────────────

/**
 * Dormand-Prince RK45 adaptive ODE solver.
 *
 * Automatically adjusts step size to maintain error within tolerance.
 * Uses two RK evaluations (4th and 5th order) to estimate local error.
 *
 * @param state - initial state
 * @param t0 - start time
 * @param tEnd - end time
 * @param dtInitial - initial step size guess
 * @param tol - error tolerance per step
 * @param f - ODE right-hand side: dy/dt = f(t, y)
 * @returns [finalState, finalTime, stepsTaken]
 */
export const rk45Adaptive = (
  state: number,
  t0: number,
  tEnd: number,
  dtInitial: number,
  tol: number,
  f: (t: number, y: number) => number,
): [number, number, number] => {
  // Dormand-Prince coefficients
  const a2 = 1 / 5, a3 = 3 / 10, a4 = 4 / 5, a5 = 8 / 9
  const b21 = 1 / 5
  const b31 = 3 / 40, b32 = 9 / 40
  const b41 = 44 / 45, b42 = -56 / 15, b43 = 32 / 9
  const b51 = 19372 / 6561, b52 = -25360 / 2187, b53 = 64448 / 6561, b54 = -212 / 729
  const b61 = 9017 / 3168, b62 = -355 / 33, b63 = 46732 / 5247, b64 = 49 / 176, b65 = -5103 / 18656

  // 5th order weights
  const c1 = 35 / 384, c3 = 500 / 1113, c4 = 125 / 192, c5 = -2187 / 6784, c6 = 11 / 84
  // 4th order weights
  const d1 = 5179 / 57600, d3 = 7571 / 16695, d4 = 393 / 640, d5 = -92097 / 339200, d6 = 187 / 2100, d7 = 1 / 40

  // ADVANCE-EXCEPTION: adaptive step loop with bounded iteration
  let y = state
  let t = t0
  let dt = dtInitial
  let steps = 0
  const maxSteps = 100_000

  while (t < tEnd && steps < maxSteps) {
    const dtActual = Math.min(dt, tEnd - t)

    const k1 = f(t, y)
    const k2 = f(t + a2 * dtActual, y + dtActual * b21 * k1)
    const k3 = f(t + a3 * dtActual, y + dtActual * (b31 * k1 + b32 * k2))
    const k4 = f(t + a4 * dtActual, y + dtActual * (b41 * k1 + b42 * k2 + b43 * k3))
    const k5 = f(t + a5 * dtActual, y + dtActual * (b51 * k1 + b52 * k2 + b53 * k3 + b54 * k4))
    const k6 = f(t + dtActual, y + dtActual * (b61 * k1 + b62 * k2 + b63 * k3 + b64 * k4 + b65 * k5))

    const y5 = y + dtActual * (c1 * k1 + c3 * k3 + c4 * k4 + c5 * k5 + c6 * k6)

    const k7 = f(t + dtActual, y5)
    const y4 = y + dtActual * (d1 * k1 + d3 * k3 + d4 * k4 + d5 * k5 + d6 * k6 + d7 * k7)

    const error = Math.abs(y5 - y4)

    if (error <= tol || dtActual <= 1e-10) {
      y = y5
      t += dtActual
      steps += 1
    }

    if (error > 0) {
      const factor = 0.9 * Math.pow(tol / error, 0.2)
      dt = dtActual * Math.max(0.1, Math.min(5.0, factor))
    }
  }

  return [y, t, steps]
}

// ── Newton-Raphson root finding ──────────────────────────────────────────────

/**
 * Newton-Raphson root finding. Finds x where f(x) ≈ 0.
 *
 * Uses numerical derivative (central difference) for the Jacobian.
 *
 * @param f - function to find root of
 * @param x0 - initial guess
 * @param tol - convergence tolerance
 * @param maxIter - maximum iterations
 * @returns [root, iterations]
 */
export const newtonRaphson = (
  f: (x: number) => number,
  x0: number,
  tol: number,
  maxIter: number,
): [number, number] => {
  const h = 1e-5
  // ADVANCE-EXCEPTION: convergence loop with bounded iteration
  let x = x0
  for (let i = 0; i < maxIter; i++) {
    const fx = f(x)
    if (Math.abs(fx) < tol) {
      return [x, i]
    }
    const dfx = (f(x + h) - f(x - h)) / (2 * h)
    if (Math.abs(dfx) < 1e-12) {
      return [x, i]
    }
    x -= fx / dfx
  }
  return [x, maxIter]
}

// ── Bisection root finding ───────────────────────────────────────────────────

/**
 * Bisection root finding. Guaranteed convergence for continuous f with f(a)*f(b) < 0.
 *
 * Slower than Newton but always converges. O(log((b-a)/tol)) iterations.
 *
 * @param f - function to find root of
 * @param a - lower bracket
 * @param b - upper bracket
 * @param tol - convergence tolerance
 * @param maxIter - maximum iterations
 * @returns [root, iterations]
 */
export const bisection = (
  f: (x: number) => number,
  a: number,
  b: number,
  tol: number,
  maxIter: number,
): [number, number] => {
  // ADVANCE-EXCEPTION: convergence loop
  let lo = a
  let hi = b
  let fLo = f(lo)
  for (let i = 0; i < maxIter; i++) {
    const mid = (lo + hi) / 2
    const fMid = f(mid)
    if (Math.abs(fMid) < tol || (hi - lo) < tol) {
      return [mid, i]
    }
    if (fLo * fMid < 0) {
      hi = mid
    } else {
      lo = mid
      fLo = fMid
    }
  }
  return [(lo + hi) / 2, maxIter]
}
