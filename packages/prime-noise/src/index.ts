/**
 * prime-noise — Value noise, Perlin gradient noise, FBM, and Worley cellular noise.
 *
 * All exported functions are pure (LOAD + COMPUTE only). No mutation, no side effects,
 * no hidden state. Same inputs always produce the same output.
 *
 * @remarks
 * Implements the temporal assembly thesis: LOAD (read parameters) + COMPUTE (pure math).
 * STORE and JUMP do not exist here. TypeScript port mirrors `prime-noise` Rust crate exactly
 * for cross-language determinism.
 *
 * @module prime-noise
 */

// ---------------------------------------------------------------------------
// Internal hash utilities — mirror Rust `hash_u32`, `hash_2d`, `hash_2d_seeded`
// ---------------------------------------------------------------------------

/**
 * Mulberry32-variant hash: maps a u32 to a pseudo-random u32.
 *
 * @remarks
 * Operates entirely in unsigned 32-bit integer space via `>>> 0` coercions
 * and `Math.imul` for wrapping multiplication. Matches the Rust implementation
 * bit-for-bit.
 */
const hashU32 = (x: number): number => {
  const z0 = (x + 0x6D2B79F5) >>> 0
  const z1 = Math.imul(z0 ^ (z0 >>> 15), z0 | 1) >>> 0
  const z2 = (z1 ^ (z1 + Math.imul(z1 ^ (z1 >>> 7), z1 | 61))) >>> 0
  return (z2 ^ (z2 >>> 14)) >>> 0
}

/**
 * Hash a 2-D integer lattice coordinate to a float in [0, 1].
 *
 * @remarks
 * Divides by `u32::MAX` (4294967295) to match Rust's `(h as f32) / (u32::MAX as f32)`.
 * This is `0xFFFFFFFF`, not `0x100000000`.
 */
const hash2d = (xi: number, yi: number): number => {
  const h = hashU32((hashU32(xi >>> 0) + (yi >>> 0)) >>> 0)
  return h / 4294967295
}

/**
 * Hash a 2-D integer lattice coordinate with an additional seed word.
 *
 * @remarks
 * Mirrors Rust: `hash_u32(hash_u32(xi as u32).wrapping_add(yi as u32).wrapping_add(seed))`.
 * The seed is added to the inner sum (not XORed with xi), matching Rust exactly.
 */
const hash2dSeeded = (xi: number, yi: number, seed: number): number => {
  const inner = ((hashU32(xi >>> 0) + (yi >>> 0)) >>> 0)
  const withSeed = (inner + (seed >>> 0)) >>> 0
  const h = hashU32(withSeed)
  return h / 4294967295
}

// ---------------------------------------------------------------------------
// Interpolation helpers
// ---------------------------------------------------------------------------

/**
 * Smoothstep fade curve: `t*t*(3 - 2*t)`.
 *
 * Maps t in [0, 1] to [0, 1] with zero derivative at both endpoints.
 */
const smoothstep = (t: number): number => t * t * (3 - 2 * t)

/**
 * Linear interpolation: `a + t * (b - a)`.
 */
const lerp = (a: number, b: number, t: number): number => a + t * (b - a)

// ---------------------------------------------------------------------------
// Gradient table — Perlin noise (matches Rust GRADIENTS exactly)
// ---------------------------------------------------------------------------

/**
 * Eight gradient vectors evenly spaced around the circle.
 *
 * @remarks
 * Matches Rust exactly: cardinal directions (1,0),(-1,0),(0,1),(0,-1) and
 * normalized diagonals (±0.7071068, ±0.7071068). Index is derived from a
 * lattice hash, giving eight possible dot-product orientations.
 */
const GRADIENTS: readonly [number, number][] = [
  [1.0, 0.0],
  [0.7071068, 0.7071068],
  [0.0, 1.0],
  [-0.7071068, 0.7071068],
  [-1.0, 0.0],
  [-0.7071068, -0.7071068],
  [0.0, -1.0],
  [0.7071068, -0.7071068],
]

/**
 * Map a lattice hash value in [0, 1] to one of the eight gradient vectors.
 */
const gradient = (h: number): [number, number] => GRADIENTS[Math.floor(h * 8) % 8]

/**
 * Dot product of gradient `g` with offset `(dx, dy)`.
 */
const gradDot = (g: [number, number], dx: number, dy: number): number =>
  g[0] * dx + g[1] * dy

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Smooth value noise at a 2-D point.
 *
 * @remarks
 * Bilinear interpolation of pseudo-random values at integer lattice corners,
 * smoothed with the smoothstep fade curve:
 *
 * ```
 * xi = floor(x),  yi = floor(y)
 * fx = fract(x),  fy = fract(y)
 * tx = smoothstep(fx),  ty = smoothstep(fy)
 *
 * v00 = hash2d(xi,   yi  )
 * v10 = hash2d(xi+1, yi  )
 * v01 = hash2d(xi,   yi+1)
 * v11 = hash2d(xi+1, yi+1)
 *
 * result = lerp(lerp(v00, v10, tx), lerp(v01, v11, tx), ty)
 * ```
 *
 * At exact integer lattice points `fx = 0`, so `smoothstep(0) = 0` and the
 * result equals `hash2d(xi, yi)` directly.
 *
 * @param x - Horizontal coordinate (any finite number).
 * @param y - Vertical coordinate (any finite number).
 * @returns A value in [0, 1].
 *
 * @example
 * ```ts
 * const v = valueNoise2d(1.5, 2.3)
 * // v is in [0, 1]
 * ```
 */
export const valueNoise2d = (x: number, y: number): number => {
  const xi = Math.floor(x)
  const yi = Math.floor(y)
  const fx = x - xi
  const fy = y - yi

  const tx = smoothstep(fx)
  const ty = smoothstep(fy)

  const v00 = hash2d(xi, yi)
  const v10 = hash2d(xi + 1, yi)
  const v01 = hash2d(xi, yi + 1)
  const v11 = hash2d(xi + 1, yi + 1)

  const bottom = lerp(v00, v10, tx)
  const top = lerp(v01, v11, tx)
  return lerp(bottom, top, ty)
}

/**
 * Classic Perlin gradient noise at a 2-D point.
 *
 * @remarks
 * For each of the four lattice corners, a gradient vector is selected via the
 * hash table and dot-producted with the offset from that corner. Results are
 * bilinearly blended with the smoothstep fade curve:
 *
 * ```
 * xi = floor(x),  yi = floor(y)
 * fx = fract(x),  fy = fract(y)
 * tx = smoothstep(fx),  ty = smoothstep(fy)
 *
 * For each corner (xi+dx, yi+dy), dx,dy ∈ {0,1}:
 *   g  = gradient(hash2d(xi+dx, yi+dy))
 *   n  = dot(g, (fx-dx, fy-dy))
 *
 * result = lerp(lerp(n00, n10, tx), lerp(n01, n11, tx), ty)
 * ```
 *
 * At exact integer lattice points all offset components are 0, so all four
 * dot products are 0 and the function returns exactly 0.
 *
 * @param x - Horizontal coordinate (any finite number).
 * @param y - Vertical coordinate (any finite number).
 * @returns A value approximately in [-1, 1]. Not clamped. Exact bounds depend on
 *   gradient directions; diagonal gradients (magnitude ≈ 0.707) limit the range.
 *
 * @example
 * ```ts
 * const v = perlin2d(0.5, 0.5)
 * // v is approximately in [-1, 1]
 *
 * // Returns exactly 0 at integer lattice points
 * const zero = perlin2d(2.0, 3.0) // 0
 * ```
 */
export const perlin2d = (x: number, y: number): number => {
  const xi = Math.floor(x)
  const yi = Math.floor(y)
  const fx = x - xi
  const fy = y - yi

  const tx = smoothstep(fx)
  const ty = smoothstep(fy)

  const g00 = gradient(hash2d(xi, yi))
  const g10 = gradient(hash2d(xi + 1, yi))
  const g01 = gradient(hash2d(xi, yi + 1))
  const g11 = gradient(hash2d(xi + 1, yi + 1))

  const n00 = gradDot(g00, fx, fy)
  const n10 = gradDot(g10, fx - 1, fy)
  const n01 = gradDot(g01, fx, fy - 1)
  const n11 = gradDot(g11, fx - 1, fy - 1)

  const bottom = lerp(n00, n10, tx)
  const top = lerp(n01, n11, tx)
  return lerp(bottom, top, ty)
}

/**
 * Fractional Brownian Motion: layered octaves of Perlin noise.
 *
 * @remarks
 * Sums multiple octaves of Perlin noise, each with increasing frequency and
 * decreasing amplitude. The fold pattern is ADVANCE — no mutable loop variable:
 *
 * ```
 * result = Σ_{i=0}^{octaves-1}  amplitude_i * perlin2d(x * freq_i, y * freq_i)
 *
 * where:
 *   freq_0      = 1.0
 *   amplitude_0 = 1.0
 *   freq_i      = freq_{i-1} * lacunarity
 *   amplitude_i = amplitude_{i-1} * gain
 * ```
 *
 * State `[accumulated, frequency, amplitude]` is threaded through
 * `Array.from({length: octaves}).reduce(...)` — no `let`, no `for`.
 *
 * With `lacunarity=2.0` and `gain=0.5` the theoretical amplitude bound is
 * `1 * (1 - 0.5^octaves) / 0.5` ≈ 2.0 for large octave counts.
 *
 * @param x - Horizontal coordinate.
 * @param y - Vertical coordinate.
 * @param octaves - Number of noise layers. 0 returns 0. Typical range: 1–8.
 * @param lacunarity - Frequency multiplier per octave (typically 2.0).
 * @param gain - Amplitude multiplier per octave (typically 0.5).
 * @returns Sum of all octave contributions. Not clamped.
 *
 * @example
 * ```ts
 * const v = fbm2d(0.3, 0.7, 6, 2.0, 0.5)
 * // |v| < 3.0 for standard params
 * ```
 */
export const fbm2d = (
  x: number,
  y: number,
  octaves: number,
  lacunarity: number,
  gain: number,
): number => {
  const [value] = Array.from<null>({ length: octaves }).reduce(
    ([acc, freq, amp]: [number, number, number]): [number, number, number] => [
      acc + amp * perlin2d(x * freq, y * freq),
      freq * lacunarity,
      amp * gain,
    ],
    [0, 1, 1] as [number, number, number],
  )
  return value
}

/**
 * Worley (cellular) noise at a 2-D point.
 *
 * @remarks
 * Finds the Euclidean distance to the nearest pseudo-random feature point by
 * searching 9 neighbouring cells. Guarantees the nearest feature is found because
 * any feature point at distance > √2 cannot be nearest.
 *
 * ```
 * cell = (floor(x), floor(y))
 *
 * For each of the 9 neighbouring cells (cell.x+dx, cell.y+dy), dx,dy ∈ {-1,0,1}:
 *   fx_cell = hash2dSeeded(cx, cy, seed)       — feature x offset in [0,1]
 *   fy_cell = hash2dSeeded(cx, cy, seed+1)     — feature y offset in [0,1]
 *   feature = (cx + fx_cell, cy + fy_cell)
 *   dist    = sqrt((x - feature.x)^2 + (y - feature.y)^2)
 *
 * result = min(dist over all 9 cells), clamped to [0, 1]
 * ```
 *
 * The 9-cell fold uses `Array.from` + `reduce` — no `for`, no `let`.
 * Seeds `seed` and `seed+1` (wrapping u32) are used for x and y offsets respectively.
 *
 * @param x - Horizontal coordinate (any finite number).
 * @param y - Vertical coordinate (any finite number).
 * @param seed - Unsigned 32-bit seed; different seeds yield independent feature fields.
 *   Internally `seed` is used for x offsets and `seed+1` (wrapping) for y offsets.
 * @returns Distance to nearest feature point, clamped to [0, 1]. Maximum unclamped
 *   distance is approximately √2 ≈ 1.414.
 *
 * @example
 * ```ts
 * const d = worley2d(0.5, 0.5, 42)
 * // d is in [0, 1]
 * ```
 */
export const worley2d = (x: number, y: number, seed: number): number => {
  const xi = Math.floor(x)
  const yi = Math.floor(y)

  const offsets: readonly [number, number][] = [
    [-1, -1], [-1, 0], [-1, 1],
    [0, -1],  [0, 0],  [0, 1],
    [1, -1],  [1, 0],  [1, 1],
  ]

  const seedU32 = seed >>> 0
  const seedPlus1 = (seedU32 + 1) >>> 0

  const minDist = offsets.reduce((minSoFar, [dx, dy]) => {
    const cx = xi + dx
    const cy = yi + dy
    const fx = hash2dSeeded(cx, cy, seedU32)
    const fy = hash2dSeeded(cx, cy, seedPlus1)
    const featX = cx + fx
    const featY = cy + fy
    const ddx = x - featX
    const ddy = y - featY
    const dist = Math.sqrt(ddx * ddx + ddy * ddy)
    return dist < minSoFar ? dist : minSoFar
  }, Number.MAX_VALUE)

  return Math.min(Math.max(minDist, 0), 1)
}
