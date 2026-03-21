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

// ---------------------------------------------------------------------------
// 3-D internal helpers
// ---------------------------------------------------------------------------

/**
 * Hash a 3-D integer lattice coordinate to a float in [0, 1].
 *
 * Mirrors Rust `hash_3d`: chains three `wrapping_add`s in u32 space.
 */
const hash3d = (xi: number, yi: number, zi: number): number => {
  const inner = ((hashU32(xi >>> 0) + (yi >>> 0)) >>> 0)
  const h = hashU32(((hashU32(inner) + (zi >>> 0)) >>> 0))
  return h / 4294967295
}

/**
 * Twelve gradient vectors: midpoints of the edges of a unit cube.
 *
 * Matches Rust `GRADIENTS_3D` exactly (all integer components ∈ {-1, 0, 1}).
 */
const GRADIENTS_3D: readonly [number, number, number][] = [
  [1, 1, 0],  [-1, 1, 0],  [1, -1, 0],  [-1, -1, 0],
  [1, 0, 1],  [-1, 0, 1],  [1, 0, -1],  [-1, 0, -1],
  [0, 1, 1],  [0, -1, 1],  [0, 1, -1],  [0, -1, -1],
]

/** Map a lattice hash in [0, 1] to one of the twelve 3-D gradient vectors. */
const gradient3d = (h: number): [number, number, number] =>
  GRADIENTS_3D[Math.floor(h * 12) % 12] as [number, number, number]

/** Dot product of a 3-D gradient with offset `(dx, dy, dz)`. */
const gradDot3d = (g: [number, number, number], dx: number, dy: number, dz: number): number =>
  g[0] * dx + g[1] * dy + g[2] * dz

// ---------------------------------------------------------------------------
// 3-D value noise
// ---------------------------------------------------------------------------

/**
 * Smooth value noise at a 3-D point.
 *
 * Trilinear interpolation of hash values at the 8 integer lattice corners,
 * smoothed with the smoothstep fade curve.
 *
 * @param x - x coordinate (any finite number)
 * @param y - y coordinate (any finite number)
 * @param z - z coordinate (any finite number)
 * @returns A value in [0, 1].
 *
 * @example
 * ```ts
 * const v = valueNoise3d(1.5, 2.3, 0.7)
 * // v is in [0, 1]
 * ```
 */
export const valueNoise3d = (x: number, y: number, z: number): number => {
  const xi = Math.floor(x)
  const yi = Math.floor(y)
  const zi = Math.floor(z)
  const fx = x - xi
  const fy = y - yi
  const fz = z - zi

  const tx = smoothstep(fx)
  const ty = smoothstep(fy)
  const tz = smoothstep(fz)

  const v000 = hash3d(xi,     yi,     zi    )
  const v100 = hash3d(xi + 1, yi,     zi    )
  const v010 = hash3d(xi,     yi + 1, zi    )
  const v110 = hash3d(xi + 1, yi + 1, zi    )
  const v001 = hash3d(xi,     yi,     zi + 1)
  const v101 = hash3d(xi + 1, yi,     zi + 1)
  const v011 = hash3d(xi,     yi + 1, zi + 1)
  const v111 = hash3d(xi + 1, yi + 1, zi + 1)

  const bot = lerp(lerp(v000, v100, tx), lerp(v010, v110, tx), ty)
  const top = lerp(lerp(v001, v101, tx), lerp(v011, v111, tx), ty)
  return lerp(bot, top, tz)
}

// ---------------------------------------------------------------------------
// 3-D Perlin noise
// ---------------------------------------------------------------------------

/**
 * Classic Perlin gradient noise at a 3-D point.
 *
 * Trilinear blend of gradient dot products from the 8 lattice corners.
 * Returns 0 at exact integer lattice points.
 *
 * @param x - x coordinate
 * @param y - y coordinate
 * @param z - z coordinate
 * @returns Approximately in [-1, 1]. Not clamped.
 *
 * @example
 * ```ts
 * const v = perlin3d(0.5, 0.5, 0.5)
 * perlin3d(1.0, 2.0, 3.0) // 0
 * ```
 */
export const perlin3d = (x: number, y: number, z: number): number => {
  const xi = Math.floor(x)
  const yi = Math.floor(y)
  const zi = Math.floor(z)
  const fx = x - xi
  const fy = y - yi
  const fz = z - zi

  const tx = smoothstep(fx)
  const ty = smoothstep(fy)
  const tz = smoothstep(fz)

  const g000 = gradient3d(hash3d(xi,     yi,     zi    ))
  const g100 = gradient3d(hash3d(xi + 1, yi,     zi    ))
  const g010 = gradient3d(hash3d(xi,     yi + 1, zi    ))
  const g110 = gradient3d(hash3d(xi + 1, yi + 1, zi    ))
  const g001 = gradient3d(hash3d(xi,     yi,     zi + 1))
  const g101 = gradient3d(hash3d(xi + 1, yi,     zi + 1))
  const g011 = gradient3d(hash3d(xi,     yi + 1, zi + 1))
  const g111 = gradient3d(hash3d(xi + 1, yi + 1, zi + 1))

  const n000 = gradDot3d(g000, fx,     fy,     fz    )
  const n100 = gradDot3d(g100, fx - 1, fy,     fz    )
  const n010 = gradDot3d(g010, fx,     fy - 1, fz    )
  const n110 = gradDot3d(g110, fx - 1, fy - 1, fz    )
  const n001 = gradDot3d(g001, fx,     fy,     fz - 1)
  const n101 = gradDot3d(g101, fx - 1, fy,     fz - 1)
  const n011 = gradDot3d(g011, fx,     fy - 1, fz - 1)
  const n111 = gradDot3d(g111, fx - 1, fy - 1, fz - 1)

  const bot = lerp(lerp(n000, n100, tx), lerp(n010, n110, tx), ty)
  const top = lerp(lerp(n001, n101, tx), lerp(n011, n111, tx), ty)
  return lerp(bot, top, tz)
}

/**
 * Fractional Brownian Motion over 3-D Perlin noise.
 *
 * @param x          - x coordinate
 * @param y          - y coordinate
 * @param z          - z coordinate
 * @param octaves    - noise layers (0 returns 0)
 * @param lacunarity - frequency multiplier per octave (typically 2.0)
 * @param gain       - amplitude multiplier per octave (typically 0.5)
 * @returns Sum of all octave contributions. Not clamped.
 *
 * @example
 * ```ts
 * const v = fbm3d(0.3, 0.7, 0.2, 6, 2.0, 0.5)
 * ```
 */
export const fbm3d = (
  x: number,
  y: number,
  z: number,
  octaves: number,
  lacunarity: number,
  gain: number,
): number => {
  const [value] = Array.from<null>({ length: octaves }).reduce(
    ([acc, freq, amp]: [number, number, number]): [number, number, number] => [
      acc + amp * perlin3d(x * freq, y * freq, z * freq),
      freq * lacunarity,
      amp * gain,
    ],
    [0, 1, 1] as [number, number, number],
  )
  return value
}

// ---------------------------------------------------------------------------
// Simplex noise (2-D and 3-D)
// ---------------------------------------------------------------------------

// Skew / unskew constants (match Rust SIMPLEX_F2 / SIMPLEX_G2 / SIMPLEX_F3 / SIMPLEX_G3).
const SIMPLEX_F2 = 0.3660254   // (sqrt(3) - 1) / 2
const SIMPLEX_G2 = 0.21132487  // (3 - sqrt(3)) / 6
const SIMPLEX_F3 = 1 / 3
const SIMPLEX_G3 = 1 / 6

/**
 * Simplex noise at a 2-D point.
 *
 * Uses triangular simplex cells — no directional artifacts, 3 corners
 * evaluated instead of 4.
 *
 * @param x - x coordinate (any finite number)
 * @param y - y coordinate (any finite number)
 * @returns Approximately in [-1, 1]. Not clamped.
 *
 * @example
 * ```ts
 * const v = simplex2d(0.5, 0.5)
 * ```
 */
export const simplex2d = (x: number, y: number): number => {
  const s = (x + y) * SIMPLEX_F2
  const i = Math.floor(x + s)
  const j = Math.floor(y + s)

  const t = (i + j) * SIMPLEX_G2
  const x0 = x - (i - t)
  const y0 = y - (j - t)

  const [i1, j1] = x0 > y0 ? [1, 0] : [0, 1]

  const x1 = x0 - i1 + SIMPLEX_G2
  const y1 = y0 - j1 + SIMPLEX_G2
  const x2 = x0 - 1 + 2 * SIMPLEX_G2
  const y2 = y0 - 1 + 2 * SIMPLEX_G2

  const contrib = (xi: number, yi: number, dx: number, dy: number): number => {
    const tc = 0.5 - dx * dx - dy * dy
    if (tc < 0) return 0
    const t2 = tc * tc
    return t2 * t2 * gradDot(gradient(hash2d(xi, yi)), dx, dy)
  }

  return 70 * (contrib(i, j, x0, y0) + contrib(i + i1, j + j1, x1, y1) + contrib(i + 1, j + 1, x2, y2))
}

/**
 * Simplex noise at a 3-D point.
 *
 * Evaluates 4 tetrahedral corners. Fewer evaluations and no axis-aligned
 * artifacts compared to 3-D Perlin.
 *
 * @param x - x coordinate (any finite number)
 * @param y - y coordinate (any finite number)
 * @param z - z coordinate (any finite number)
 * @returns Approximately in [-1, 1]. Not clamped.
 *
 * @example
 * ```ts
 * const v = simplex3d(0.5, 0.5, 0.5)
 * ```
 */
export const simplex3d = (x: number, y: number, z: number): number => {
  const s = (x + y + z) * SIMPLEX_F3
  const i = Math.floor(x + s)
  const j = Math.floor(y + s)
  const k = Math.floor(z + s)

  const t = (i + j + k) * SIMPLEX_G3
  const x0 = x - (i - t)
  const y0 = y - (j - t)
  const z0 = z - (k - t)

  // Which tetrahedron?
  const [i1, j1, k1, i2, j2, k2] = x0 >= y0
    ? y0 >= z0      ? [1, 0, 0, 1, 1, 0]
    : x0 >= z0      ? [1, 0, 0, 1, 0, 1]
    :                 [0, 0, 1, 1, 0, 1]
    : y0 < z0       ? [0, 0, 1, 0, 1, 1]
    : x0 < z0       ? [0, 1, 0, 0, 1, 1]
    :                 [0, 1, 0, 1, 1, 0]

  const x1 = x0 - i1 + SIMPLEX_G3
  const y1 = y0 - j1 + SIMPLEX_G3
  const z1 = z0 - k1 + SIMPLEX_G3
  const x2 = x0 - i2 + 2 * SIMPLEX_G3
  const y2 = y0 - j2 + 2 * SIMPLEX_G3
  const z2 = z0 - k2 + 2 * SIMPLEX_G3
  const x3 = x0 - 1 + 3 * SIMPLEX_G3
  const y3 = y0 - 1 + 3 * SIMPLEX_G3
  const z3 = z0 - 1 + 3 * SIMPLEX_G3

  const contrib = (xi: number, yi: number, zi: number, dx: number, dy: number, dz: number): number => {
    const tc = 0.6 - dx * dx - dy * dy - dz * dz
    if (tc < 0) return 0
    const t2 = tc * tc
    return t2 * t2 * gradDot3d(gradient3d(hash3d(xi, yi, zi)), dx, dy, dz)
  }

  return 32 * (
    contrib(i,      j,      k,      x0, y0, z0)
  + contrib(i + i1, j + j1, k + k1, x1, y1, z1)
  + contrib(i + i2, j + j2, k + k2, x2, y2, z2)
  + contrib(i + 1,  j + 1,  k + 1,  x3, y3, z3)
  )
}

// ---------------------------------------------------------------------------
// Domain warping
// ---------------------------------------------------------------------------

/**
 * Domain-warped FBM in 2-D.
 *
 * Samples two independent FBM fields as a warp vector then evaluates FBM at
 * the warped position. Produces swirling turbulent shapes.
 *
 * @param x          - x coordinate
 * @param y          - y coordinate
 * @param octaves    - FBM octave count
 * @param lacunarity - frequency multiplier per octave
 * @param gain       - amplitude multiplier per octave
 * @param warpScale  - domain displacement magnitude (typically 1.0–2.0)
 * @returns An FBM-range value. Not clamped.
 *
 * @example
 * ```ts
 * const v = domainWarp2d(0.3, 0.7, 6, 2.0, 0.5, 1.0)
 * ```
 */
export const domainWarp2d = (
  x: number,
  y: number,
  octaves: number,
  lacunarity: number,
  gain: number,
  warpScale: number,
): number => {
  const wx = fbm2d(x,       y,       octaves, lacunarity, gain)
  const wy = fbm2d(x + 5.2, y + 1.3, octaves, lacunarity, gain)
  return fbm2d(x + warpScale * wx, y + warpScale * wy, octaves, lacunarity, gain)
}

/**
 * Domain-warped FBM in 3-D.
 *
 * Three independent FBM fields provide the warp vector; a fourth FBM samples
 * the warped position.
 *
 * @param x          - x coordinate
 * @param y          - y coordinate
 * @param z          - z coordinate
 * @param octaves    - FBM octave count
 * @param lacunarity - frequency multiplier per octave
 * @param gain       - amplitude multiplier per octave
 * @param warpScale  - domain displacement magnitude
 * @returns An FBM-range value. Not clamped.
 *
 * @example
 * ```ts
 * const v = domainWarp3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0)
 * ```
 */
export const domainWarp3d = (
  x: number,
  y: number,
  z: number,
  octaves: number,
  lacunarity: number,
  gain: number,
  warpScale: number,
): number => {
  const wx = fbm3d(x,       y,       z,       octaves, lacunarity, gain)
  const wy = fbm3d(x + 5.2, y + 1.3, z + 2.7, octaves, lacunarity, gain)
  const wz = fbm3d(x + 3.1, y + 7.4, z + 0.9, octaves, lacunarity, gain)
  return fbm3d(
    x + warpScale * wx,
    y + warpScale * wy,
    z + warpScale * wz,
    octaves, lacunarity, gain,
  )
}
