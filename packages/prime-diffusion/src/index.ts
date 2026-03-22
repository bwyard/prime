/**
 * prime-diffusion — Stochastic processes: Ornstein-Uhlenbeck and geometric Brownian motion.
 *
 * All exported functions are pure (LOAD + COMPUTE only). No mutation, no side effects.
 * Noise is either caller-supplied (standard normal `w`) or generated deterministically
 * from a threaded `bigint` seed (mirrors Rust `u64` seed threading).
 *
 * Temporal assembly:
 *   LOAD    ← state, parameters, seed
 *   COMPUTE ← stochastic update
 *   APPEND  ← return [nextValue, nextSeed] tuple
 */

// ── Internal noise helpers ────────────────────────────────────────────────────

/**
 * Xorshift64 PRNG: maps a bigint seed to [uniform_01, nextSeed].
 *
 * Uses BigInt arithmetic to mirror Rust's wrapping u64 xorshift.
 * Period = 2⁶⁴ − 1.
 */
const xorshift64 = (seed: bigint): [number, bigint] => {
  const MASK = 0xFFFF_FFFF_FFFF_FFFFn
  const s0 = (seed ^ ((seed << 13n) & MASK)) & MASK
  const s1 = (s0 ^ (s0 >> 7n)) & MASK
  const s2 = (s1 ^ ((s1 << 17n) & MASK)) & MASK
  const u = Number(s2 & 0xFFFF_FFFFn) / 4294967295
  return [u, s2]
}

/**
 * Box-Muller transform: two independent U(0,1) samples → standard normal.
 *
 * @param u1 - uniform in (0, 1] (must be > 0)
 * @param u2 - uniform in [0, 1)
 * @returns one sample from N(0, 1)
 */
const boxMuller = (u1: number, u2: number): number =>
  Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2)

/**
 * Draw one standard-normal sample from a bigint seed.
 *
 * @param seed - non-zero bigint RNG state
 * @returns [z, nextSeed] where z ~ N(0, 1)
 */
const normalFromSeed = (seed: bigint): [number, bigint] => {
  const [u1raw, s1] = xorshift64(seed)
  const [u2, s2] = xorshift64(s1)
  const u1 = u1raw < Number.EPSILON ? Number.EPSILON : u1raw
  return [boxMuller(u1, u2), s2]
}

// ── Ornstein-Uhlenbeck ────────────────────────────────────────────────────────

/**
 * Ornstein-Uhlenbeck step with caller-supplied noise.
 *
 * The O-U process is the canonical mean-reverting stochastic process.
 * Useful for economy curves, rival activity, and weather systems in simulations.
 *
 * Math:
 * ```
 * x' = x + θ(μ − x)dt + σ√dt · w
 * ```
 *
 * @param x     - current value
 * @param mu    - long-run mean (equilibrium point)
 * @param theta - mean-reversion speed (> 0; typical 0.1–1.0)
 * @param sigma - volatility / noise scale (> 0)
 * @param dt    - time step
 * @param w     - standard normal noise sample N(0, 1)
 * @returns next value x'
 *
 * @example
 * // Noiseless convergence toward mu=0 from x=1
 * ouStep(1, 0, 0.5, 0, 0.1, 0) // 0.95
 */
export const ouStep = (
  x: number,
  mu: number,
  theta: number,
  sigma: number,
  dt: number,
  w: number,
): number => x + theta * (mu - x) * dt + sigma * Math.sqrt(dt) * w

/**
 * Ornstein-Uhlenbeck step with deterministic seeded noise.
 *
 * Generates one N(0,1) sample from `seed` via Box-Muller, applies ouStep,
 * and returns the advanced seed. Chain calls to build a stochastic time series.
 *
 * @param x, mu, theta, sigma, dt - same as ouStep
 * @param seed - bigint RNG seed (non-zero); mirrors Rust u64
 * @returns [x_next, nextSeed]
 *
 * @example
 * const [x1, s1] = ouStepSeeded(1, 0, 0.5, 0.1, 0.01, 12345n)
 * const [x2, s2] = ouStepSeeded(x1, 0, 0.5, 0.1, 0.01, s1)
 */
export const ouStepSeeded = (
  x: number,
  mu: number,
  theta: number,
  sigma: number,
  dt: number,
  seed: bigint,
): [number, bigint] => {
  const [w, nextSeed] = normalFromSeed(seed)
  return [ouStep(x, mu, theta, sigma, dt, w), nextSeed]
}

// ── Geometric Brownian Motion ─────────────────────────────────────────────────

/**
 * Geometric Brownian motion step with caller-supplied noise.
 *
 * GBM models multiplicative processes (always positive): resource prices,
 * guild reputation, skill multipliers.
 *
 * Math (exact solution for one step):
 * ```
 * x' = x · exp((μ − σ²/2)dt + σ√dt · w)
 * ```
 *
 * @param x     - current value (must be > 0)
 * @param mu    - drift rate
 * @param sigma - volatility (> 0)
 * @param dt    - time step
 * @param w     - standard normal noise sample N(0, 1)
 * @returns next value x' (always positive when x > 0)
 *
 * @example
 * // Zero drift, zero noise → unchanged
 * gbmStep(1, 0, 0, 0.1, 0) // 1.0
 */
export const gbmStep = (
  x: number,
  mu: number,
  sigma: number,
  dt: number,
  w: number,
): number => x * Math.exp((mu - 0.5 * sigma * sigma) * dt + sigma * Math.sqrt(dt) * w)

/**
 * Geometric Brownian motion step with deterministic seeded noise.
 *
 * @param x, mu, sigma, dt - same as gbmStep
 * @param seed - bigint RNG seed
 * @returns [x_next, nextSeed]
 *
 * @example
 * const [x1, s1] = gbmStepSeeded(1, 0.05, 0.2, 0.01, 42n)
 */
export const gbmStepSeeded = (
  x: number,
  mu: number,
  sigma: number,
  dt: number,
  seed: bigint,
): [number, bigint] => {
  const [w, nextSeed] = normalFromSeed(seed)
  return [gbmStep(x, mu, sigma, dt, w), nextSeed]
}
