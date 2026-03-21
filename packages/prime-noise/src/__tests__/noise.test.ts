/**
 * Tests for prime-noise TypeScript port.
 *
 * Rules enforced throughout:
 * - `const` only — no `let` anywhere
 * - No `for` loops — `Array.from + reduce/map` patterns
 * - All functions are pure; tests verify determinism and range contracts
 */

import { describe, it, expect } from 'vitest'
import { valueNoise2d, perlin2d, fbm2d, worley2d } from '../index'

const EPSILON = 1e-5

// ---------------------------------------------------------------------------
// valueNoise2d
// ---------------------------------------------------------------------------

describe('valueNoise2d', () => {
  it('returns values in [0, 1] for a range of coordinates', () => {
    const points: readonly [number, number][] = [
      [0.0, 0.0],
      [0.5, 0.5],
      [1.234, 5.678],
      [-3.1, 2.9],
      [100.0, 200.0],
      [0.999, 0.001],
    ]
    const allInRange = points.reduce(
      (ok, [x, y]) => ok && valueNoise2d(x, y) >= 0 && valueNoise2d(x, y) <= 1,
      true,
    )
    expect(allInRange).toBe(true)
  })

  it('is deterministic — same inputs produce same output', () => {
    const a = valueNoise2d(3.14, 2.71)
    const b = valueNoise2d(3.14, 2.71)
    expect(a).toBe(b)
  })

  it('at integer lattice points returns the raw hash value (fx=0, fy=0)', () => {
    // At integer coordinates smoothstep(0) = 0, so result = hash2d(xi, yi).
    // We verify it is consistent with multiple calls — exact hash value is
    // not known from TS side, but it must be stable and in [0,1].
    const v1 = valueNoise2d(2.0, 3.0)
    const v2 = valueNoise2d(2.0, 3.0)
    expect(Math.abs(v1 - v2)).toBeLessThan(EPSILON)
    expect(v1).toBeGreaterThanOrEqual(0)
    expect(v1).toBeLessThanOrEqual(1)
  })

  it('produces different values at distinct coordinates', () => {
    const a = valueNoise2d(0.5, 0.5)
    const b = valueNoise2d(1.5, 0.5)
    expect(a).not.toBe(b)
  })

  it('satisfies [0,1] range using Array.from + reduce across many points', () => {
    const xs = Array.from({ length: 10 }, (_, i) => i * 0.37 - 1.5)
    const ys = Array.from({ length: 10 }, (_, i) => i * 0.53 - 2.0)
    const pairs = xs.reduce<[number, number][]>(
      (acc, x) => [...acc, ...ys.map<[number, number]>((y) => [x, y])],
      [],
    )
    const allInRange = pairs.reduce(
      (ok, [x, y]) => {
        const v = valueNoise2d(x, y)
        return ok && v >= 0 && v <= 1
      },
      true,
    )
    expect(allInRange).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// perlin2d
// ---------------------------------------------------------------------------

describe('perlin2d', () => {
  it('returns values approximately in [-1.5, 1.5] for typical inputs', () => {
    const points: readonly [number, number][] = [
      [0.5, 0.5],
      [1.234, 5.678],
      [-3.1, 2.9],
      [10.0, 20.0],
      [0.1, 0.9],
    ]
    const allInRange = points.reduce(
      (ok, [x, y]) => ok && perlin2d(x, y) >= -1.5 && perlin2d(x, y) <= 1.5,
      true,
    )
    expect(allInRange).toBe(true)
  })

  it('is deterministic — same inputs produce same output', () => {
    const a = perlin2d(1.1, 2.2)
    const b = perlin2d(1.1, 2.2)
    expect(a).toBe(b)
  })

  it('returns exactly 0 at integer lattice points (all offsets are 0)', () => {
    const latticePoints: readonly [number, number][] = [
      [0, 0],
      [1, 2],
      [-3, 5],
      [10, -4],
    ]
    const allZero = latticePoints.reduce(
      (ok, [x, y]) => ok && Math.abs(perlin2d(x, y)) < EPSILON,
      true,
    )
    expect(allZero).toBe(true)
  })

  it('produces different values at distinct non-lattice coordinates', () => {
    const a = perlin2d(0.3, 0.7)
    const b = perlin2d(0.7, 0.3)
    expect(a).not.toBe(b)
  })

  it('covers both positive and negative values across sample set', () => {
    const xs = Array.from({ length: 8 }, (_, i) => i * 0.41 + 0.2)
    const ys = Array.from({ length: 8 }, (_, i) => i * 0.61 + 0.15)
    const values = xs.reduce<number[]>(
      (acc, x) => [...acc, ...ys.map((y) => perlin2d(x, y))],
      [],
    )
    const hasPositive = values.reduce((ok, v) => ok || v > 0, false)
    const hasNegative = values.reduce((ok, v) => ok || v < 0, false)
    expect(hasPositive).toBe(true)
    expect(hasNegative).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// fbm2d
// ---------------------------------------------------------------------------

describe('fbm2d', () => {
  it('returns 0 when octaves = 0', () => {
    expect(fbm2d(1.0, 2.0, 0, 2.0, 0.5)).toBe(0)
  })

  it('equals perlin2d with 1 octave (amplitude=1, freq=1)', () => {
    const x = 0.4
    const y = 0.8
    const fbmVal = fbm2d(x, y, 1, 2.0, 0.5)
    const perlinVal = perlin2d(x, y)
    expect(Math.abs(fbmVal - perlinVal)).toBeLessThan(EPSILON)
  })

  it('is deterministic — same inputs produce same output', () => {
    const a = fbm2d(0.5, 0.5, 6, 2.0, 0.5)
    const b = fbm2d(0.5, 0.5, 6, 2.0, 0.5)
    expect(a).toBe(b)
  })

  it('stays within plausible bounds for standard params (lacunarity=2, gain=0.5)', () => {
    const points: readonly [number, number][] = [
      [0.1, 0.2],
      [5.5, 3.3],
      [-1.0, -2.0],
    ]
    const allBounded = points.reduce(
      (ok, [x, y]) => ok && Math.abs(fbm2d(x, y, 8, 2.0, 0.5)) < 3.0,
      true,
    )
    expect(allBounded).toBe(true)
  })

  it('adding more octaves changes the result', () => {
    const x = 0.3
    const y = 0.7
    const one = fbm2d(x, y, 1, 2.0, 0.5)
    const six = fbm2d(x, y, 6, 2.0, 0.5)
    expect(one).not.toBe(six)
  })

  it('uses Array.from+reduce internally — verified externally by folding octave contributions', () => {
    // Manual fold over octaves matches fbm2d output
    const x = 0.6
    const y = 0.4
    const octaves = 4
    const lacunarity = 2.0
    const gain = 0.5

    const [expected] = Array.from<null>({ length: octaves }).reduce(
      ([acc, freq, amp]: [number, number, number]) => [
        acc + amp * perlin2d(x * freq, y * freq),
        freq * lacunarity,
        amp * gain,
      ],
      [0, 1, 1] as [number, number, number],
    )

    expect(Math.abs(fbm2d(x, y, octaves, lacunarity, gain) - expected)).toBeLessThan(EPSILON)
  })
})

// ---------------------------------------------------------------------------
// worley2d
// ---------------------------------------------------------------------------

describe('worley2d', () => {
  it('returns values in [0, 1] for various coordinates', () => {
    const points: readonly [number, number][] = [
      [0.0, 0.0],
      [0.5, 0.5],
      [3.14, 2.71],
      [-1.5, 4.2],
      [100.0, 200.0],
    ]
    const allInRange = points.reduce(
      (ok, [x, y]) => ok && worley2d(x, y, 42) >= 0 && worley2d(x, y, 42) <= 1,
      true,
    )
    expect(allInRange).toBe(true)
  })

  it('is deterministic — same inputs produce same output', () => {
    const a = worley2d(1.23, 4.56, 99)
    const b = worley2d(1.23, 4.56, 99)
    expect(a).toBe(b)
  })

  it('different seeds produce different feature fields', () => {
    const a = worley2d(0.5, 0.5, 0)
    const b = worley2d(0.5, 0.5, 1)
    expect(a).not.toBe(b)
  })

  it('always returns a non-negative value', () => {
    const v = worley2d(0.0, 0.0, 0)
    expect(v).toBeGreaterThanOrEqual(0)
  })

  it('returns a positive distance at cell centre', () => {
    const d = worley2d(0.5, 0.5, 7)
    expect(d).toBeGreaterThan(0)
  })

  it('satisfies [0,1] range using Array.from + reduce across many seeds', () => {
    const seeds = Array.from({ length: 20 }, (_, i) => i * 7)
    const allInRange = seeds.reduce(
      (ok, seed) => ok && worley2d(0.5, 0.5, seed) >= 0 && worley2d(0.5, 0.5, seed) <= 1,
      true,
    )
    expect(allInRange).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// Cross-function consistency
// ---------------------------------------------------------------------------

describe('cross-function consistency', () => {
  it('fbm2d with 1 octave and gain=0 still equals perlin2d', () => {
    const x = 0.7
    const y = 0.3
    // gain=0 means amplitude is 1.0 only for i=0 (amp_0=1, amp_1=0*1=0), so still equals perlin
    const fbmVal = fbm2d(x, y, 1, 2.0, 0.0)
    const perlinVal = perlin2d(x, y)
    expect(Math.abs(fbmVal - perlinVal)).toBeLessThan(EPSILON)
  })

  it('valueNoise2d and perlin2d return different values at the same non-lattice point', () => {
    const vn = valueNoise2d(0.5, 0.5)
    const pn = perlin2d(0.5, 0.5)
    // Different algorithms — should not coincidentally match
    expect(vn).not.toBeCloseTo(pn, 3)
  })
})
