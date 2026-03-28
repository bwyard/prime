import { describe, it, expect } from 'vitest'
import {
  prngNext,
  prngRange,
  prngRangeInt,
  prngBool,
  prngShuffled,
  prngChoose,
  weightedChoice,
  prngNextWithEntropy,
  prngGaussian,
  prngGaussianPair,
  prngExponential,
  prngDiskUniform,
  prngAnnulusUniform,
  vanDerCorput,
  halton2d,
  halton3d,
  monteCarlo1d,
  monteCarlo2d,
  monteCarlo1dWithVariance,
  poissonDisk2d,
  prngNextCausal,
  prngGaussianCausal,
  prngNext64,
  prngRange64,
  prngGaussian64,
  memoize1d,
} from '../index.js'
import type { CausalStep } from '../index.js'

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
    const seqA = Array.from<null>({ length: 10 }).reduce(
      ([vals, s]: [number[], number]): [number[], number] => {
        const [v, next] = prngNext(s)
        return [[...vals, v], next]
      },
      [[], 99] as [number[], number],
    )[0]
    const seqB = Array.from<null>({ length: 10 }).reduce(
      ([vals, s]: [number[], number]): [number[], number] => {
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
    const trues = Array.from<null>({ length: 10000 }).reduce(
      ([count, s]: [number, number]): [number, number] => {
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
    const counts = Array.from<null>({ length: n }).reduce(
      ([acc, s]: [number[], number]): [number[], number] => {
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

// ── prngNextWithEntropy ──────────────────────────────────────────────────────

describe('prngNextWithEntropy', () => {
  it('entropy=0 produces same value as prngNext', () => {
    const [valueA] = prngNextWithEntropy(42, 0)
    const [valueB] = prngNext(42)
    expect(valueA).toBe(valueB)
  })

  it('entropy=0 produces same next seed as prngNext', () => {
    const [, nextA] = prngNextWithEntropy(42, 0)
    const [, nextB] = prngNext(42)
    expect(nextA).toBe(nextB)
  })

  it('different entropy changes next seed', () => {
    const [, nextA] = prngNextWithEntropy(42, 0xDEADBEEF)
    const [, nextB] = prngNextWithEntropy(42, 0)
    expect(nextA).not.toBe(nextB)
  })

  it('is deterministic', () => {
    const [vA, sA] = prngNextWithEntropy(100, 0xCAFE)
    const [vB, sB] = prngNextWithEntropy(100, 0xCAFE)
    expect(vA).toBe(vB)
    expect(sA).toBe(sB)
  })
})

// ── prngGaussian ─────────────────────────────────────────────────────────────

describe('prngGaussian', () => {
  it('is deterministic', () => {
    const [a] = prngGaussian(42)
    const [b] = prngGaussian(42)
    expect(a).toBe(b)
  })

  it('mean ~0 and stddev ~1 over 10K samples', () => {
    const n = 10000
    const [sum, sumSq] = Array.from<null>({ length: n }).reduce(
      ([accSum, accSumSq, s]: [number, number, number]): [number, number, number] => {
        const [g, next] = prngGaussian(s)
        return [accSum + g, accSumSq + g * g, next]
      },
      [0, 0, 1] as [number, number, number],
    )
    const mean = sum / n
    const variance = sumSq / n - mean * mean
    expect(Math.abs(mean)).toBeLessThan(0.05)
    expect(Math.abs(Math.sqrt(variance) - 1)).toBeLessThan(0.1)
  })
})

// ── prngGaussianPair ─────────────────────────────────────────────────────────

describe('prngGaussianPair', () => {
  it('is deterministic', () => {
    const [a0, a1, aS] = prngGaussianPair(42)
    const [b0, b1, bS] = prngGaussianPair(42)
    expect(a0).toBe(b0)
    expect(a1).toBe(b1)
    expect(aS).toBe(bS)
  })

  it('both values are normally distributed', () => {
    const n = 10000
    const [sum0, sum1, sumSq0, sumSq1] = Array.from<null>({ length: n }).reduce(
      ([s0, s1, sq0, sq1, s]: [number, number, number, number, number]): [number, number, number, number, number] => {
        const [g0, g1, next] = prngGaussianPair(s)
        return [s0 + g0, s1 + g1, sq0 + g0 * g0, sq1 + g1 * g1, next]
      },
      [0, 0, 0, 0, 1] as [number, number, number, number, number],
    )
    const mean0 = sum0 / n
    const mean1 = sum1 / n
    const var0 = sumSq0 / n - mean0 * mean0
    const var1 = sumSq1 / n - mean1 * mean1
    expect(Math.abs(mean0)).toBeLessThan(0.05)
    expect(Math.abs(mean1)).toBeLessThan(0.05)
    expect(Math.abs(Math.sqrt(var0) - 1)).toBeLessThan(0.1)
    expect(Math.abs(Math.sqrt(var1) - 1)).toBeLessThan(0.1)
  })
})

// ── prngExponential ──────────────────────────────────────────────────────────

describe('prngExponential', () => {
  it('always positive', () => {
    Array.from({ length: 1000 }, (_, i) => prngExponential(i, 1.0)[0])
      .forEach(v => expect(v).toBeGreaterThan(0))
  })

  it('mean ~1/lambda over 10K samples', () => {
    const lambda = 2.0
    const n = 10000
    const [sum] = Array.from<null>({ length: n }).reduce(
      ([acc, s]: [number, number]): [number, number] => {
        const [e, next] = prngExponential(s, lambda)
        return [acc + e, next]
      },
      [0, 1] as [number, number],
    )
    const mean = sum / n
    expect(Math.abs(mean - 1 / lambda)).toBeLessThan(0.05)
  })

  it('is deterministic', () => {
    const [a, sA] = prngExponential(42, 3.0)
    const [b, sB] = prngExponential(42, 3.0)
    expect(a).toBe(b)
    expect(sA).toBe(sB)
  })
})

// ── prngDiskUniform ──────────────────────────────────────────────────────────

describe('prngDiskUniform', () => {
  it('all points within radius', () => {
    const radius = 5.0
    Array.from({ length: 1000 }, (_, i) => {
      const [x, y] = prngDiskUniform(i, radius)
      return Math.sqrt(x * x + y * y)
    }).forEach(dist => expect(dist).toBeLessThanOrEqual(radius + 1e-10))
  })

  it('is deterministic', () => {
    const [x1, y1, s1] = prngDiskUniform(42, 10.0)
    const [x2, y2, s2] = prngDiskUniform(42, 10.0)
    expect(x1).toBe(x2)
    expect(y1).toBe(y2)
    expect(s1).toBe(s2)
  })
})

// ── prngAnnulusUniform ───────────────────────────────────────────────────────

describe('prngAnnulusUniform', () => {
  it('all points in annulus', () => {
    const rInner = 3.0
    const rOuter = 7.0
    Array.from({ length: 1000 }, (_, i) => {
      const [x, y] = prngAnnulusUniform(i, rInner, rOuter)
      return Math.sqrt(x * x + y * y)
    }).forEach(dist => {
      expect(dist).toBeGreaterThanOrEqual(rInner - 1e-10)
      expect(dist).toBeLessThanOrEqual(rOuter + 1e-10)
    })
  })

  it('is deterministic', () => {
    const [x1, y1, s1] = prngAnnulusUniform(42, 2.0, 5.0)
    const [x2, y2, s2] = prngAnnulusUniform(42, 2.0, 5.0)
    expect(x1).toBe(x2)
    expect(y1).toBe(y2)
    expect(s1).toBe(s2)
  })
})

// ── vanDerCorput ─────────────────────────────────────────────────────────────

describe('vanDerCorput', () => {
  it('known values base 2', () => {
    expect(vanDerCorput(1, 2)).toBeCloseTo(0.5, 10)
    expect(vanDerCorput(2, 2)).toBeCloseTo(0.25, 10)
    expect(vanDerCorput(3, 2)).toBeCloseTo(0.75, 10)
    expect(vanDerCorput(4, 2)).toBeCloseTo(0.125, 10)
  })

  it('known values base 3', () => {
    expect(vanDerCorput(1, 3)).toBeCloseTo(1 / 3, 10)
    expect(vanDerCorput(2, 3)).toBeCloseTo(2 / 3, 10)
    expect(vanDerCorput(3, 3)).toBeCloseTo(1 / 9, 10)
  })

  it('returns 0 for n=0', () => {
    expect(vanDerCorput(0, 2)).toBe(0)
  })
})

// ── halton2d ─────────────────────────────────────────────────────────────────

describe('halton2d', () => {
  it('known first values', () => {
    const [x1, y1] = halton2d(1)
    expect(x1).toBeCloseTo(0.5, 10)
    expect(y1).toBeCloseTo(1 / 3, 10)

    const [x2, y2] = halton2d(2)
    expect(x2).toBeCloseTo(0.25, 10)
    expect(y2).toBeCloseTo(2 / 3, 10)
  })

  it('returns [0, 0] for n=0', () => {
    const [x, y] = halton2d(0)
    expect(x).toBe(0)
    expect(y).toBe(0)
  })
})

// ── halton3d ─────────────────────────────────────────────────────────────────

describe('halton3d', () => {
  it('known first value', () => {
    const [x, y, z] = halton3d(1)
    expect(x).toBeCloseTo(0.5, 10)
    expect(y).toBeCloseTo(1 / 3, 10)
    expect(z).toBeCloseTo(0.2, 10)
  })
})

// ── monteCarlo1d ─────────────────────────────────────────────────────────────

describe('monteCarlo1d', () => {
  it('sin(x) over [0, pi] ≈ 2.0', () => {
    const [estimate] = monteCarlo1d(42, Math.sin, 0, Math.PI, 50000)
    expect(Math.abs(estimate - 2.0)).toBeLessThan(0.05)
  })

  it('is deterministic', () => {
    const [a, sA] = monteCarlo1d(42, Math.sin, 0, Math.PI, 100)
    const [b, sB] = monteCarlo1d(42, Math.sin, 0, Math.PI, 100)
    expect(a).toBe(b)
    expect(sA).toBe(sB)
  })
})

// ── monteCarlo2d ─────────────────────────────────────────────────────────────

describe('monteCarlo2d', () => {
  it('x*y over [0,1]^2 ≈ 0.25', () => {
    const [estimate] = monteCarlo2d(42, (x, y) => x * y, 0, 1, 0, 1, 50000)
    expect(Math.abs(estimate - 0.25)).toBeLessThan(0.02)
  })

  it('is deterministic', () => {
    const [a, sA] = monteCarlo2d(42, (x, y) => x * y, 0, 1, 0, 1, 100)
    const [b, sB] = monteCarlo2d(42, (x, y) => x * y, 0, 1, 0, 1, 100)
    expect(a).toBe(b)
    expect(sA).toBe(sB)
  })
})

// ── monteCarlo1dWithVariance ─────────────────────────────────────────────────

describe('monteCarlo1dWithVariance', () => {
  it('sin(x) over [0, pi] ≈ 2.0 with positive variance', () => {
    const [estimate, variance] = monteCarlo1dWithVariance(42, Math.sin, 0, Math.PI, 50000)
    expect(Math.abs(estimate - 2.0)).toBeLessThan(0.05)
    expect(variance).toBeGreaterThan(0)
  })

  it('is deterministic', () => {
    const [a, vA, sA] = monteCarlo1dWithVariance(42, Math.sin, 0, Math.PI, 100)
    const [b, vB, sB] = monteCarlo1dWithVariance(42, Math.sin, 0, Math.PI, 100)
    expect(a).toBe(b)
    expect(vA).toBe(vB)
    expect(sA).toBe(sB)
  })

  it('variance is 0 for n=1', () => {
    const [, variance] = monteCarlo1dWithVariance(42, Math.sin, 0, Math.PI, 1)
    expect(variance).toBe(0)
  })
})

// ── poissonDisk2d ─────────────────────────────────────────────────────────────

describe('poissonDisk2d', () => {
  it('satisfies minimum distance', () => {
    const minDist = 10
    const [pts] = poissonDisk2d(42, 100, 100, minDist)
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
    const [pts] = poissonDisk2d(1, 50, 80, 8)
    pts.forEach(([x, y]) => {
      expect(x).toBeGreaterThanOrEqual(0)
      expect(x).toBeLessThan(50)
      expect(y).toBeGreaterThanOrEqual(0)
      expect(y).toBeLessThan(80)
    })
  })

  it('is deterministic', () => {
    const [ptsA, seedA] = poissonDisk2d(5, 60, 60, 8)
    const [ptsB, seedB] = poissonDisk2d(5, 60, 60, 8)
    expect(ptsA.length).toBe(ptsB.length)
    expect(seedA).toBe(seedB)
    ptsA.forEach((pa, i) => {
      expect(pa[0]).toBeCloseTo(ptsB[i][0], 5)
      expect(pa[1]).toBeCloseTo(ptsB[i][1], 5)
    })
  })

  it('returns final seed', () => {
    const [, seed] = poissonDisk2d(42, 100, 100, 10)
    expect(typeof seed).toBe('number')
    expect(seed).not.toBe(42)
  })
})

// ── CausalStep ──────────────────────────────────────────────────────────────────

describe('prngNextCausal', () => {
  it('records parent seed', () => {
    const step = prngNextCausal(42)
    expect(step.parentSeed).toBe(42)
    const [v] = prngNext(42)
    expect(step.value).toBe(v)
  })

  it('chain is traceable', () => {
    const s0 = prngNextCausal(42)
    const s1 = prngNextCausal(s0.nextSeed)
    const s2 = prngNextCausal(s1.nextSeed)
    expect(s1.parentSeed).toBe(s0.nextSeed)
    expect(s2.parentSeed).toBe(s1.nextSeed)
  })

  it('fold builds traceable log', () => {
    const history: CausalStep<number>[] = Array.from<null>({ length: 10 }).reduce(
      ([log, seed]: [CausalStep<number>[], number]): [CausalStep<number>[], number] => {
        const step = prngNextCausal(seed)
        return [[...log, step], step.nextSeed]
      },
      [[] as CausalStep<number>[], 42] as [CausalStep<number>[], number],
    )[0]
    Array.from({ length: history.length - 1 }, (_, i) => i + 1).forEach(i => {
      expect(history[i].parentSeed).toBe(history[i - 1].nextSeed)
    })
  })
})

describe('prngGaussianCausal', () => {
  it('records parent seed', () => {
    const step = prngGaussianCausal(42)
    expect(step.parentSeed).toBe(42)
    const [v] = prngGaussian(42)
    expect(step.value).toBe(v)
  })

  it('value is finite', () => {
    const step = prngGaussianCausal(42)
    expect(Number.isFinite(step.value)).toBe(true)
  })
})

// ── Receiver model: same seed, different contexts ───────────────────────────

describe('receiver model', () => {
  const SEED = 42

  // Every receiver extracts different information from the same seed
  const receivers = [
    { name: 'prngNext', fn: () => prngNext(SEED), type: 'uniform' },
    { name: 'prngBool(0.5)', fn: () => prngBool(SEED, 0.5), type: 'boolean' },
    { name: 'prngGaussian', fn: () => prngGaussian(SEED), type: 'gaussian' },
    { name: 'prngExponential(1)', fn: () => prngExponential(SEED, 1.0), type: 'exponential' },
    { name: 'prngDiskUniform(5)', fn: () => prngDiskUniform(SEED, 5.0), type: 'spatial' },
    { name: 'prngRangeInt(10)', fn: () => prngRangeInt(SEED, 10), type: 'discrete' },
  ] as const

  it('all receivers are deterministic on the same seed', () => {
    receivers.forEach(({ name, fn }) => {
      const a = fn()
      const b = fn()
      expect(a).toEqual(b)
    })
  })

  it('different receivers extract different values from the same seed', () => {
    const values = receivers.map(({ fn }) => JSON.stringify(fn()))
    const unique = new Set(values)
    expect(unique.size).toBe(receivers.length)
  })

  // Golden values — Rust is the reference. Update these if Rust changes.
  it('prngNext(42) matches Rust golden value', () => {
    const [v, next] = prngNext(42)
    expect(v).toBeCloseTo(0.6011038, 5)
    expect(next).toBe(1831565855)
  })

  it('prngGaussian(42) matches Rust golden value', () => {
    const [z] = prngGaussian(42)
    expect(z).toBeCloseTo(-0.95616, 3)
  })
})

// ── Cross-language parity ──────────────────────────────────────────────────────
//
// NOTE: Both TS and Rust use identical Mulberry32 algorithm.
// These tests verify TS-internal stability (regression guard) and structural contracts.

describe('cross-language parity', () => {
  it('prngRange(42, 10, 20) is in [10, 20)', () => {
    const [v] = prngRange(42, 10, 20)
    expect(v).toBeGreaterThanOrEqual(10)
    expect(v).toBeLessThan(20)
  })
  it('prngBool(42, 0.5) returns boolean', () => {
    const [v] = prngBool(42, 0.5)
    expect(typeof v).toBe('boolean')
  })
  it('prngShuffled preserves all elements', () => {
    const v = [10, 20, 30, 40, 50]
    const [s] = prngShuffled(42, v)
    expect([...s].sort((a, b) => a - b)).toEqual([10, 20, 30, 40, 50])
  })
  it('prngChoose from non-empty returns element in slice', () => {
    const v = ['a', 'b', 'c']
    const [pick] = prngChoose(42, v)
    expect(pick).not.toBeNull()
    expect(v.includes(pick!)).toBe(true)
  })
})
