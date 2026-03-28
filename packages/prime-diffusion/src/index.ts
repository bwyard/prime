/**
 * prime-diffusion — Stochastic processes: Ornstein-Uhlenbeck and geometric Brownian motion.
 *
 * All exported functions are pure (LOAD + COMPUTE only). No mutation, no side effects.
 * Noise is either caller-supplied (standard normal `w`) or generated deterministically
 * from a threaded `u32` seed via Mulberry32 + Box-Muller (matching prime-random).
 *
 * Temporal assembly:
 *   LOAD    ← state, parameters, seed
 *   COMPUTE ← stochastic update
 *   APPEND  ← return [nextValue, nextSeed] tuple
 */

// ── Internal noise helpers (Mulberry32 + Box-Muller) ─────────────────────────

/**
 * Mulberry32 pure step — matches prime-random's prngNext.
 * Returns [value in [0,1), nextSeed].
 */
const mulberry32 = (seed: number): [number, number] => {
  const z0 = (seed + 0x6D2B79F5) >>> 0
  const z1 = Math.imul(z0 ^ (z0 >>> 15), z0 | 1)
  const z2 = z1 ^ (z1 + Math.imul(z1 ^ (z1 >>> 7), z1 | 61))
  return [((z2 ^ (z2 >>> 14)) >>> 0) / 0x100000000, z0]
}

/**
 * Draw one standard-normal sample from a u32 seed via Box-Muller.
 *
 * @param seed - non-zero u32 RNG state
 * @returns [z, nextSeed] where z ~ N(0, 1)
 */
const gaussianFromSeed = (seed: number): [number, number] => {
  const [u1raw, s1] = mulberry32(seed)
  const [u2, s2] = mulberry32(s1)
  const u1 = u1raw < 1e-10 ? 1e-10 : u1raw
  return [Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2), s2]
}

// ── Ornstein-Uhlenbeck ────────────────────────────────────────────────────────

/**
 * Ornstein-Uhlenbeck step with caller-supplied noise.
 *
 * Math: `x' = x + θ(μ − x)dt + σ√dt · w`
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
 * Generates one N(0,1) sample from `seed` via Mulberry32 + Box-Muller,
 * applies ouStep, and returns the advanced seed.
 *
 * @param x, mu, theta, sigma, dt - same as ouStep
 * @param seed - u32 RNG seed (non-zero)
 * @returns [x_next, nextSeed]
 *
 * @example
 * const [x1, s1] = ouStepSeeded(1, 0, 0.5, 0.1, 0.01, 12345)
 * const [x2, s2] = ouStepSeeded(x1, 0, 0.5, 0.1, 0.01, s1)
 */
export const ouStepSeeded = (
  x: number,
  mu: number,
  theta: number,
  sigma: number,
  dt: number,
  seed: number,
): [number, number] => {
  const [w, nextSeed] = gaussianFromSeed(seed)
  return [ouStep(x, mu, theta, sigma, dt, w), nextSeed]
}

// ── Geometric Brownian Motion ─────────────────────────────────────────────────

/**
 * Geometric Brownian motion step with caller-supplied noise.
 *
 * Math (exact solution): `x' = x · exp((μ − σ²/2)dt + σ√dt · w)`
 *
 * @param x     - current value (must be > 0)
 * @param mu    - drift rate
 * @param sigma - volatility (> 0)
 * @param dt    - time step
 * @param w     - standard normal noise sample N(0, 1)
 * @returns next value x' (always positive when x > 0)
 *
 * @example
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
 * @param seed - u32 RNG seed
 * @returns [x_next, nextSeed]
 *
 * @example
 * const [x1, s1] = gbmStepSeeded(1, 0.05, 0.2, 0.01, 42)
 */
export const gbmStepSeeded = (
  x: number,
  mu: number,
  sigma: number,
  dt: number,
  seed: number,
): [number, number] => {
  const [w, nextSeed] = gaussianFromSeed(seed)
  return [gbmStep(x, mu, sigma, dt, w), nextSeed]
}
