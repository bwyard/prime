/**
 * prime-random — Seeded deterministic randomness.
 *
 * All public functions are LOAD + COMPUTE. No STORE. No JUMP. No exceptions.
 *
 * Thesis: the seed IS the thread. Who holds the seed controls who can
 * advance it. Consent revocation = stop threading the seed forward.
 * No DELETE needed — the sequence is causally inert without its key.
 */

// ── Pure PRNG primitives ────────────────────────────────────────────────────

/**
 * Mulberry32 pure step — LOAD + COMPUTE only.
 *
 * @remarks
 * The seed IS the thread position. Pass nextSeed forward to continue.
 * Stop passing it to end the thread — no DELETE required.
 *
 * State: z0 = (seed + 0x6D2B79F5) & 0xFFFFFFFF
 * Output: bit-mixed z0 → float in [0, 1)
 * Period: 2^32
 *
 * @param seed - Current thread position (32-bit unsigned integer)
 * @returns [value in [0,1), nextSeed] — thread the nextSeed forward
 *
 * @example
 * const [v1, s1] = prngNext(42)
 * const [v2, s2] = prngNext(s1)
 */
export const prngNext = (seed: number): [number, number] => {
  const z0 = (seed + 0x6D2B79F5) >>> 0
  const z1 = Math.imul(z0 ^ (z0 >>> 15), z0 | 1)
  const z2 = z1 ^ (z1 + Math.imul(z1 ^ (z1 >>> 7), z1 | 61))
  return [((z2 ^ (z2 >>> 14)) >>> 0) / 0x100000000, z0]
}

/**
 * Pure float in [min, max).
 * @returns [value, nextSeed]
 * @example
 * const [v, s1] = prngRange(0, 2.0, 5.0) // 2 ≤ v < 5
 */
export const prngRange = (seed: number, min: number, max: number): [number, number] => {
  if (min >= max) return [min, seed]
  const [v, next] = prngNext(seed)
  return [min + v * (max - min), next]
}

/**
 * Pure integer in [0, n).
 * @returns [value, nextSeed]
 * @example
 * const [i, s1] = prngRangeInt(0, 10) // 0 ≤ i < 10
 */
export const prngRangeInt = (seed: number, n: number): [number, number] => {
  const [v, next] = prngNext(seed)
  return [Math.floor(v * n), next]
}

/**
 * Pure boolean with probability p of true.
 * @returns [value, nextSeed]
 * @example
 * const [flip, s1] = prngBool(0, 0.5)
 */
export const prngBool = (seed: number, p: number): [boolean, number] => {
  const [v, next] = prngNext(seed)
  return [v < Math.max(0, Math.min(1, p)), next]
}

/**
 * Pure Fisher-Yates shuffle — returns new array, original unchanged.
 * @returns [shuffledArray, nextSeed]
 * @example
 * const [shuffled, s1] = prngShuffled(42, [1, 2, 3, 4, 5])
 */
export const prngShuffled = <T>(seed: number, arr: readonly T[]): [T[], number] =>
  Array.from({ length: arr.length - 1 }, (_, k) => arr.length - 1 - k).reduce(
    ([acc, s]: [T[], number], i) => {
      const [j, next] = prngRangeInt(s, i + 1)
      const copy = [...acc] as T[]
      ;[copy[i], copy[j]] = [copy[j], copy[i]]
      return [copy, next]
    },
    [[...arr] as T[], seed],
  )

/**
 * Pure random element from array.
 * @returns [element | undefined, nextSeed]
 * @example
 * const [pick, s1] = prngChoose(0, ['a', 'b', 'c'])
 */
export const prngChoose = <T>(seed: number, arr: readonly T[]): [T | undefined, number] => {
  if (arr.length === 0) return [undefined, seed]
  const [i, next] = prngRangeInt(seed, arr.length)
  return [arr[i], next]
}

// ── Pure higher-order functions ─────────────────────────────────────────────

/**
 * Weighted random choice — O(n) linear scan. Pure LOAD + COMPUTE.
 *
 * @remarks
 * Sample u ~ Uniform(0, sum(weights)).
 * Walk weights accumulating until accumulator ≤ 0 → return that index.
 *
 * @param seed - Thread position
 * @param weights - Non-negative weights. Must sum > 0.
 * @returns [chosenIndex, nextSeed]
 *
 * @example
 * const [i, s1] = weightedChoice(0, [1, 2, 1]) // index 1 is 2× likely
 */
export const weightedChoice = (seed: number, weights: readonly number[]): [number, number] => {
  if (weights.length === 0) return [0, seed]
  const total = weights.reduce((s, w) => s + w, 0)
  if (total <= 0) return [weights.length - 1, seed]
  const [u, s1] = prngRange(seed, 0, total)
  const { idx } = weights.reduce(
    ({ remaining, idx }, w, i) =>
      idx !== -1
        ? { remaining, idx }
        : remaining - w <= 0
          ? { remaining: remaining - w, idx: i }
          : { remaining: remaining - w, idx: -1 },
    { remaining: u, idx: -1 },
  )
  return [idx !== -1 ? idx : weights.length - 1, s1]
}

// ── Pure Bridson ────────────────────────────────────────────────────────────

type BridsonParams = {
  readonly width: number
  readonly height: number
  readonly minDist: number
  readonly maxAttempts: number
  readonly cols: number
  readonly rows: number
  readonly cellSize: number
}

type BridsonState = {
  readonly grid: readonly ([number, number] | null)[]
  readonly active: readonly number[]
  readonly points: readonly [number, number][]
  readonly seed: number
}

const bridsonTooClose = (
  x: number,
  y: number,
  grid: readonly ([number, number] | null)[],
  p: BridsonParams,
): boolean => {
  const cx = Math.floor(x / p.cellSize)
  const cy = Math.floor(y / p.cellSize)
  const r = 2
  return Array.from(
    { length: Math.min(p.rows, cy + r + 1) - Math.max(0, cy - r) },
    (_, dy) => Math.max(0, cy - r) + dy,
  ).some(gy =>
    Array.from(
      { length: Math.min(p.cols, cx + r + 1) - Math.max(0, cx - r) },
      (_, dx) => Math.max(0, cx - r) + dx,
    ).some(gx => {
      const pt = grid[gy * p.cols + gx]
      return pt !== null && (x - pt[0]) ** 2 + (y - pt[1]) ** 2 < p.minDist * p.minDist
    }),
  )
}

const bridsonStep = (state: BridsonState, p: BridsonParams): BridsonState => {
  if (state.active.length === 0) return state
  const [aiF, s1] = prngNext(state.seed)
  const ai = Math.floor(aiF * state.active.length)
  const [ax, ay] = state.points[state.active[ai]]

  const [candidate, finalSeed] = Array.from({ length: p.maxAttempts }).reduce(
    ([found, s]: [[number, number] | null, number]) => {
      if (found !== null) return [found, s] as [[number, number] | null, number]
      const [anglef, s2] = prngNext(s)
      const [distf, s3] = prngNext(s2)
      const angle = anglef * Math.PI * 2
      const dist = p.minDist + distf * p.minDist
      const cx = ax + Math.cos(angle) * dist
      const cy = ay + Math.sin(angle) * dist
      return cx >= 0 && cx < p.width && cy >= 0 && cy < p.height && !bridsonTooClose(cx, cy, state.grid, p)
        ? [[cx, cy] as [number, number], s3]
        : [null, s3]
    },
    [null, s1] as [[number, number] | null, number],
  )

  if (candidate !== null) {
    const cellIdx =
      Math.floor(candidate[1] / p.cellSize) * p.cols + Math.floor(candidate[0] / p.cellSize)
    return {
      grid: state.grid.map((v, i) => (i === cellIdx ? candidate : v)),
      active: [...state.active, state.points.length],
      points: [...state.points, candidate],
      seed: finalSeed,
    }
  }
  return {
    ...state,
    active: state.active.filter((_, i) => i !== ai),
    seed: finalSeed,
  }
}

/**
 * Poisson disk sampling — minimum distance spacing in 2D. Pure LOAD + COMPUTE.
 *
 * @remarks
 * Bridson's algorithm (2007) expressed as a pure state fold (ADVANCE).
 * Each step is an immutable state transition: (state) → newState.
 * The seed threads through every random draw — no hidden state.
 *
 * Performance note: each step copies the spatial grid (O(cols×rows)).
 * For typical game use (domain < 2000×2000, minDist > 5) this is negligible.
 *
 * @param seed - Thread position — same seed → same point distribution
 * @param width - Sampling domain width
 * @param height - Sampling domain height
 * @param minDist - Minimum distance between any two points
 * @param maxAttempts - Candidates per active point (30 is standard)
 * @returns Array of [x, y] tuples
 *
 * @example
 * const pts = poissonDisk2d(42, 100, 100, 10)
 */
export const poissonDisk2d = (
  seed: number,
  width: number,
  height: number,
  minDist: number,
  maxAttempts = 30,
): [number, number][] => {
  const cellSize = minDist / Math.SQRT2
  const cols = Math.ceil(width / cellSize) + 1
  const rows = Math.ceil(height / cellSize) + 1
  const p: BridsonParams = { width, height, minDist, maxAttempts, cols, rows, cellSize }

  const [x0f, s1] = prngNext(seed)
  const [y0f, s2] = prngNext(s1)
  const x0 = x0f * width
  const y0 = y0f * height
  const cellIdx0 = Math.floor(y0 / cellSize) * cols + Math.floor(x0 / cellSize)

  const initial: BridsonState = {
    grid: Array.from({ length: cols * rows }, (_, i) =>
      i === cellIdx0 ? ([x0, y0] as [number, number]) : null,
    ),
    active: [0],
    points: [[x0, y0]],
    seed: s2,
  }

  // Upper bound on steps: each point is added once and removed from active once.
  const maxPoints = Math.ceil((width * height) / (Math.PI * (minDist / 2) ** 2)) * 4
  const maxSteps = maxPoints * 2

  const final = Array.from({ length: maxSteps }).reduce(
    (state: BridsonState) => (state.active.length === 0 ? state : bridsonStep(state, p)),
    initial,
  )

  return [...final.points]
}
