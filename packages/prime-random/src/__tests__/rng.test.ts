import { describe, it, expect } from 'vitest'
import {
  prngNext,
  prngRange,
  prngRangeInt,
  prngBool,
  prngShuffled,
  prngChoose,
  weightedChoice,
  poissonDisk2d,
} from '../index.js'

// ── prngNext ─────────────────────────────────────────────────────────────────

describe('prngNext', () => {
  it('returns value in [0, 1)', () => {
    Array.from({ length: 1000 }, (_, i) => prngNext(i)[0])
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(0)
        expect(v).toBeLessThan(1)
      })
  })

  it('same seed → same value', () => {
    const [a] = prngNext(42)
    const [b] = prngNext(42)
    expect(a).toBe(b)
  })

  it('different seeds → different values', () => {
    const vals = Array.from({ length: 20 }, (_, i) => prngNext(i)[0])
    expect(new Set(vals).size).toBeGreaterThan(15)
  })

  it('nextSeed differs from seed', () => {
    const [, next] = prngNext(42)
    expect(next).not.toBe(42)
  })

  it('threads forward — sequence is deterministic', () => {
    const seqA = Array.from({ length: 10 }).reduce(
      ([vals, s]: [number[], number]) => {
        const [v, next] = prngNext(s)
        return [[...vals, v], next]
      },
      [[], 99] as [number[], number],
    )[0]
    const seqB = Array.from({ length: 10 }).reduce(
      ([vals, s]: [number[], number]) => {
        const [v, next] = prngNext(s)
        return [[...vals, v], next]
      },
      [[], 99] as [number[], number],
    )[0]
    expect(seqA).toEqual(seqB)
  })
})

// ── prngRange ─────────────────────────────────────────────────────────────────

describe('prngRange', () => {
  it('returns values in [min, max)', () => {
    Array.from({ length: 1000 }, (_, i) => prngRange(i, 2, 5)[0])
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(2)
        expect(v).toBeLessThan(5)
      })
  })

  it('returns min when min equals max', () => {
    expect(prngRange(0, 3, 3)[0]).toBe(3)
  })
})

// ── prngRangeInt ──────────────────────────────────────────────────────────────

describe('prngRangeInt', () => {
  it('returns integers in [0, n)', () => {
    Array.from({ length: 1000 }, (_, i) => prngRangeInt(i, 7)[0])
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(0)
        expect(v).toBeLessThan(7)
        expect(Number.isInteger(v)).toBe(true)
      })
  })
})

// ── prngBool ──────────────────────────────────────────────────────────────────

describe('prngBool', () => {
  it('always true at p=1', () => {
    Array.from({ length: 100 }, (_, i) => prngBool(i, 1)[0])
      .forEach(v => expect(v).toBe(true))
  })

  it('always false at p=0', () => {
    Array.from({ length: 100 }, (_, i) => prngBool(i, 0)[0])
      .forEach(v => expect(v).toBe(false))
  })

  it('roughly half at p=0.5', () => {
    const trues = Array.from({ length: 10000 }).reduce(
      ([count, s]: [number, number]) => {
        const [b, next] = prngBool(s, 0.5)
        return [count + (b ? 1 : 0), next]
      },
      [0, 99] as [number, number],
    )[0]
    expect(trues).toBeGreaterThan(4500)
    expect(trues).toBeLessThan(5500)
  })
})

// ── prngShuffled ──────────────────────────────────────────────────────────────

describe('prngShuffled', () => {
  it('preserves all elements', () => {
    const original = [1, 2, 3, 4, 5, 6, 7, 8]
    const [result] = prngShuffled(0, original)
    expect([...result].sort((a, b) => a - b)).toEqual(original)
  })

  it('does not mutate original', () => {
    const original = [1, 2, 3, 4, 5]
    prngShuffled(0, original)
    expect(original).toEqual([1, 2, 3, 4, 5])
  })

  it('handles empty array', () => {
    const [result] = prngShuffled(0, [])
    expect(result).toEqual([])
  })

  it('at least one permutation differs from original', () => {
    const original = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    const anyDifferent = Array.from({ length: 20 }, (_, seed) =>
      prngShuffled(seed, original)[0].join(',') !== original.join(','),
    ).some(Boolean)
    expect(anyDifferent).toBe(true)
  })
})

// ── prngChoose ────────────────────────────────────────────────────────────────

describe('prngChoose', () => {
  it('returns element from array', () => {
    const arr = [10, 20, 30, 40]
    Array.from({ length: 100 }, (_, i) => prngChoose(i, arr)[0])
      .forEach(pick => expect(arr).toContain(pick))
  })

  it('returns undefined for empty array', () => {
    expect(prngChoose(0, [])[0]).toBeUndefined()
  })
})

// ── weightedChoice ────────────────────────────────────────────────────────────

describe('weightedChoice', () => {
  it('returns 0 for empty weights', () => {
    const [idx] = weightedChoice(0, [])
    expect(idx).toBe(0)
  })

  it('always picks the only non-zero weight', () => {
    Array.from({ length: 20 }, (_, i) => weightedChoice(i, [0, 0, 1, 0])[0])
      .forEach(i => expect(i).toBe(2))
  })

  it('distribution matches weights (1:2:1)', () => {
    const n = 10000
    const counts = Array.from({ length: n }).reduce(
      ([acc, s]: [number[], number]) => {
        const [idx, next] = weightedChoice(s, [1, 2, 1])
        return [acc.map((c, j) => (j === idx ? c + 1 : c)), next]
      },
      [[0, 0, 0], 777] as [number[], number],
    )[0]
    const tolerance = n * 0.05
    expect(Math.abs(counts[0] - n / 4)).toBeLessThan(tolerance)
    expect(Math.abs(counts[1] - n / 2)).toBeLessThan(tolerance)
    expect(Math.abs(counts[2] - n / 4)).toBeLessThan(tolerance)
  })
})

// ── poissonDisk2d ─────────────────────────────────────────────────────────────

describe('poissonDisk2d', () => {
  it('satisfies minimum distance', () => {
    const minDist = 10
    const pts = poissonDisk2d(42, 100, 100, minDist)
    expect(pts.length).toBeGreaterThan(0)
    pts.forEach((pi, i) =>
      pts.slice(i + 1).forEach(pj => {
        const dx = pi[0] - pj[0]
        const dy = pi[1] - pj[1]
        expect(Math.sqrt(dx * dx + dy * dy)).toBeGreaterThanOrEqual(minDist - 1e-4)
      }),
    )
  })

  it('all points within bounds', () => {
    const pts = poissonDisk2d(1, 50, 80, 8)
    pts.forEach(([x, y]) => {
      expect(x).toBeGreaterThanOrEqual(0)
      expect(x).toBeLessThan(50)
      expect(y).toBeGreaterThanOrEqual(0)
      expect(y).toBeLessThan(80)
    })
  })

  it('is deterministic', () => {
    const ptsA = poissonDisk2d(5, 60, 60, 8)
    const ptsB = poissonDisk2d(5, 60, 60, 8)
    expect(ptsA.length).toBe(ptsB.length)
    ptsA.forEach((pa, i) => {
      expect(pa[0]).toBeCloseTo(ptsB[i][0], 5)
      expect(pa[1]).toBeCloseTo(ptsB[i][1], 5)
    })
  })
})
