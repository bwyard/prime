/**
 * prime-interp — Interpolation, easing, and smoothstep functions.
 */

// ── Basic ─────────────────────────────────────────────────────────────────

/**
 * Linear interpolation between a and b.
 * @remarks lerp(a, b, t) = a + t × (b − a)
 * @param a - Start value
 * @param b - End value
 * @param t - Factor (not clamped — extrapolates outside [0,1])
 * @example lerp(0, 10, 0.5) // 5
 */
export const lerp = (a: number, b: number, t: number): number => a + t * (b - a)

/**
 * Lerp with t clamped to [0, 1].
 * @param a - Start value
 * @param b - End value
 * @param t - Factor (clamped to [0, 1])
 * @example lerpClamped(0, 10, 1.5) // 10
 */
export const lerpClamped = (a: number, b: number, t: number): number =>
  lerp(a, b, Math.max(0, Math.min(1, t)))

/**
 * Inverse lerp — the t that produces v between a and b.
 * @remarks inv_lerp(a, b, v) = (v − a) / (b − a)
 * @param a - Range start
 * @param b - Range end (must not equal a)
 * @param v - Value to find t for
 * @example invLerp(0, 10, 5) // 0.5
 */
export const invLerp = (a: number, b: number, v: number): number => {
  if (Math.abs(b - a) < Number.EPSILON) return 0
  return (v - a) / (b - a)
}

/**
 * Remap a value from one range to another.
 * @remarks lerp(outMin, outMax, invLerp(inMin, inMax, v))
 * @example remap(5, 0, 10, 0, 100) // 50
 */
export const remap = (
  v: number,
  inMin: number,
  inMax: number,
  outMin: number,
  outMax: number,
): number => lerp(outMin, outMax, invLerp(inMin, inMax, v))

// ── Smooth ────────────────────────────────────────────────────────────────

/**
 * Hermite smoothstep — S-curve, zero derivative at both edges.
 * @remarks t = clamp((x−e0)/(e1−e0)); return t²×(3−2t)
 * @param edge0 - Lower edge
 * @param edge1 - Upper edge
 * @param x - Input value
 * @example smoothstep(0, 1, 0.5) // 0.5
 */
export const smoothstep = (edge0: number, edge1: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - edge0) / (edge1 - edge0)))
  return t * t * (3 - 2 * t)
}

/**
 * Ken Perlin's smootherstep — C2 continuity (zero 1st AND 2nd derivative).
 * @remarks t³×(t×(6t−15)+10)
 * @example smootherstep(0, 1, 0.5) // 0.5
 */
export const smootherstep = (edge0: number, edge1: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - edge0) / (edge1 - edge0)))
  return t * t * t * (t * (6 * t - 15) + 10)
}

// ── Easing — Quad ─────────────────────────────────────────────────────────

/** Quadratic ease-in. t² */
export const easeInQuad = (t: number): number => t * t

/** Quadratic ease-out. t×(2−t) */
export const easeOutQuad = (t: number): number => t * (2 - t)

/** Quadratic ease-in-out. */
export const easeInOutQuad = (t: number): number =>
  t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2

// ── Easing — Cubic ────────────────────────────────────────────────────────

/** Cubic ease-in. t³ */
export const easeInCubic = (t: number): number => t ** 3

/** Cubic ease-out. 1−(1−t)³ */
export const easeOutCubic = (t: number): number => 1 - (1 - t) ** 3

/** Cubic ease-in-out. */
export const easeInOutCubic = (t: number): number =>
  t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2

// ── Easing — Quart ────────────────────────────────────────────────────────

/** Quartic ease-in. t⁴ */
export const easeInQuart = (t: number): number => t ** 4

/** Quartic ease-out. 1−(1−t)⁴ */
export const easeOutQuart = (t: number): number => 1 - (1 - t) ** 4

/** Quartic ease-in-out. */
export const easeInOutQuart = (t: number): number =>
  t < 0.5 ? 8 * t ** 4 : 1 - (-2 * t + 2) ** 4 / 2

// ── Easing — Quint ────────────────────────────────────────────────────────

/** Quintic ease-in. t⁵ */
export const easeInQuint = (t: number): number => t ** 5

/** Quintic ease-out. 1−(1−t)⁵ */
export const easeOutQuint = (t: number): number => 1 - (1 - t) ** 5

/** Quintic ease-in-out. */
export const easeInOutQuint = (t: number): number =>
  t < 0.5 ? 16 * t ** 5 : 1 - (-2 * t + 2) ** 5 / 2

// ── Easing — Sine ─────────────────────────────────────────────────────────

/** Sine ease-in. 1−cos(t×π/2) */
export const easeInSine = (t: number): number => 1 - Math.cos((t * Math.PI) / 2)

/** Sine ease-out. sin(t×π/2) */
export const easeOutSine = (t: number): number => Math.sin((t * Math.PI) / 2)

/** Sine ease-in-out. −(cos(πt)−1)/2 */
export const easeInOutSine = (t: number): number => -(Math.cos(Math.PI * t) - 1) / 2

// ── Easing — Expo ─────────────────────────────────────────────────────────

/** Exponential ease-in. 2^(10t−10) */
export const easeInExpo = (t: number): number =>
  t === 0 ? 0 : 2 ** (10 * t - 10)

/** Exponential ease-out. 1−2^(−10t) */
export const easeOutExpo = (t: number): number =>
  t === 1 ? 1 : 1 - 2 ** (-10 * t)

/** Exponential ease-in-out. */
export const easeInOutExpo = (t: number): number => {
  if (t === 0) return 0
  if (t === 1) return 1
  return t < 0.5 ? 2 ** (20 * t - 10) / 2 : (2 - 2 ** (-20 * t + 10)) / 2
}

// ── Easing — Circ ─────────────────────────────────────────────────────────

/** Circular ease-in. 1−√(1−t²) */
export const easeInCirc = (t: number): number => 1 - Math.sqrt(1 - t ** 2)

/** Circular ease-out. √(1−(t−1)²) */
export const easeOutCirc = (t: number): number => Math.sqrt(1 - (t - 1) ** 2)

/** Circular ease-in-out. */
export const easeInOutCirc = (t: number): number =>
  t < 0.5
    ? (1 - Math.sqrt(1 - (2 * t) ** 2)) / 2
    : (Math.sqrt(1 - (-2 * t + 2) ** 2) + 1) / 2

// ── Easing — Elastic ──────────────────────────────────────────────────────

/** Elastic ease-in. May return values outside [0, 1]. */
export const easeInElastic = (t: number): number => {
  if (t === 0) return 0
  if (t === 1) return 1
  const c4 = (2 * Math.PI) / 3
  return -(2 ** (10 * t - 10)) * Math.sin((10 * t - 10.75) * c4)
}

/** Elastic ease-out. May return values outside [0, 1]. */
export const easeOutElastic = (t: number): number => {
  if (t === 0) return 0
  if (t === 1) return 1
  const c4 = (2 * Math.PI) / 3
  return 2 ** (-10 * t) * Math.sin((10 * t - 0.75) * c4) + 1
}

/** Elastic ease-in-out. */
export const easeInOutElastic = (t: number): number => {
  if (t === 0) return 0
  if (t === 1) return 1
  const c5 = (2 * Math.PI) / 4.5
  return t < 0.5
    ? -(2 ** (20 * t - 10) * Math.sin((20 * t - 11.125) * c5)) / 2
    : (2 ** (-20 * t + 10) * Math.sin((20 * t - 11.125) * c5)) / 2 + 1
}

// ── Easing — Bounce ───────────────────────────────────────────────────────

/** Bounce ease-out — bounces at the end. */
export const easeOutBounce = (t: number): number => {
  const n1 = 7.5625
  const d1 = 2.75
  if (t < 1 / d1) return n1 * t * t
  if (t < 2 / d1) { const t2 = t - 1.5 / d1; return n1 * t2 * t2 + 0.75 }
  if (t < 2.5 / d1) { const t2 = t - 2.25 / d1; return n1 * t2 * t2 + 0.9375 }
  const t2 = t - 2.625 / d1; return n1 * t2 * t2 + 0.984375
}

/** Bounce ease-in — bounces at the start. */
export const easeInBounce = (t: number): number => 1 - easeOutBounce(1 - t)

/** Bounce ease-in-out. */
export const easeInOutBounce = (t: number): number =>
  t < 0.5
    ? (1 - easeOutBounce(1 - 2 * t)) / 2
    : (1 + easeOutBounce(2 * t - 1)) / 2

// ── Easing — Back ─────────────────────────────────────────────────────────

/** Ease in with overshoot (back). s = 1.70158. */
export const easeInBack = (t: number): number => {
  const s = 1.70158
  return t * t * ((s + 1) * t - s)
}

/** Ease out with overshoot (back). s = 1.70158. */
export const easeOutBack = (t: number): number => {
  const s = 1.70158
  const t1 = t - 1
  return t1 * t1 * ((s + 1) * t1 + s) + 1
}

// ── Repeat / Pingpong ────────────────────────────────────────────────────

/**
 * Repeat: wraps t into [0, length).
 * @param t - Input value
 * @param length - Period length
 * @example repeat(2.5, 1.0) // 0.5
 */
export const repeat = (t: number, length: number): number => {
  if (length === 0) return 0
  return t - Math.floor(t / length) * length
}

/**
 * Ping-pong: t bounces between 0 and length.
 * @param t - Input value
 * @param length - Bounce range
 * @example pingpong(2.5, 1.0) // 0.5
 */
export const pingpong = (t: number, length: number): number => {
  if (length === 0) return 0
  const r = repeat(t, length * 2)
  return length - Math.abs(r - length)
}
