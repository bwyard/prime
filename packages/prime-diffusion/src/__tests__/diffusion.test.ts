import { describe, it, expect } from 'vitest'
import { ouStep, ouStepSeeded, gbmStep, gbmStepSeeded } from '../index.js'

const EPS = 1e-4
const SEED = 0xDEAD_BEEF_1234_5678n

// ── ouStep ────────────────────────────────────────────────────────────────────

describe('ouStep', () => {
  it('zero dt — returns x unchanged', () => {
    expect(ouStep(1, 0, 0.5, 0.3, 0, 1)).toBeCloseTo(1, 5)
  })

  it('zero noise — noiseless convergence', () => {
    // x' = 1 + 0.5*(0-1)*0.1 = 1 - 0.05 = 0.95
    expect(ouStep(1, 0, 0.5, 0, 0.1, 0)).toBeCloseTo(0.95, 5)
  })

  it('zero sigma — deterministic decay', () => {
    // x' = 2 + 1*(1-2)*0.1 = 1.9
    expect(ouStep(2, 1, 1, 0, 0.1, 5)).toBeCloseTo(1.9, 5)
  })

  it('deterministic', () => {
    expect(ouStep(1, 0, 0.3, 0.1, 0.01, 0.5)).toBe(ouStep(1, 0, 0.3, 0.1, 0.01, 0.5))
  })

  it('noiseless mean reversion converges — theta=1.0, 1000 steps', () => {
    const x = Array.from<null>({ length: 1000 }).reduce(
      ([state]: [number]): [number] => [ouStep(state, 0, 1, 0, 0.01, 0)],
      [10] as [number],
    )[0]
    expect(Math.abs(x)).toBeLessThan(0.01)
  })
})

// ── ouStepSeeded ──────────────────────────────────────────────────────────────

describe('ouStepSeeded', () => {
  it('advances value', () => {
    const [x1] = ouStepSeeded(1, 0, 0.3, 0.1, 0.01, SEED)
    expect(Math.abs(x1 - 1)).toBeGreaterThan(Number.EPSILON)
  })

  it('advances seed', () => {
    const [, s1] = ouStepSeeded(1, 0, 0.3, 0.1, 0.01, SEED)
    expect(s1).not.toBe(SEED)
  })

  it('deterministic', () => {
    const a = ouStepSeeded(1, 0, 0.3, 0.1, 0.01, SEED)
    const b = ouStepSeeded(1, 0, 0.3, 0.1, 0.01, SEED)
    expect(a).toEqual(b)
  })

  it('100-step chain stays bounded near mu=0', () => {
    const [x] = Array.from<null>({ length: 100 }).reduce(
      ([x, s]: [number, bigint]): [number, bigint] => ouStepSeeded(x, 0, 0.3, 0.5, 0.01, s),
      [0, SEED] as [number, bigint],
    )
    expect(Math.abs(x)).toBeLessThan(5)
  })
})

// ── gbmStep ───────────────────────────────────────────────────────────────────

describe('gbmStep', () => {
  it('zero dt — returns x unchanged', () => {
    expect(gbmStep(1, 0.05, 0.2, 0, 0.5)).toBeCloseTo(1, 5)
  })

  it('zero sigma — deterministic growth', () => {
    // x' = 1 * exp(0.1 * 0.1) = exp(0.01)
    expect(gbmStep(1, 0.1, 0, 0.1, 0)).toBeCloseTo(Math.exp(0.01), 5)
  })

  it('always positive', () => {
    const x = Array.from<null>({ length: 100 }).reduce(
      ([x, i]: [number, number]): [number, number] => [gbmStep(x, 0, 0.3, 0.01, Math.sin(i * 0.1)), i + 1],
      [1, 0] as [number, number],
    )[0]
    expect(x).toBeGreaterThan(0)
  })

  it('deterministic', () => {
    expect(gbmStep(1, 0.05, 0.2, 0.01, 0.5)).toBe(gbmStep(1, 0.05, 0.2, 0.01, 0.5))
  })
})

// ── gbmStepSeeded ─────────────────────────────────────────────────────────────

describe('gbmStepSeeded', () => {
  it('result is positive', () => {
    const [x1] = gbmStepSeeded(1, 0.05, 0.2, 0.01, SEED)
    expect(x1).toBeGreaterThan(0)
  })

  it('deterministic', () => {
    const a = gbmStepSeeded(1, 0.05, 0.2, 0.01, SEED)
    const b = gbmStepSeeded(1, 0.05, 0.2, 0.01, SEED)
    expect(a).toEqual(b)
  })

  it('advances seed', () => {
    const [, s1] = gbmStepSeeded(1, 0.05, 0.2, 0.01, SEED)
    expect(s1).not.toBe(SEED)
  })

  it('100-step chain stays positive', () => {
    const [x] = Array.from<null>({ length: 100 }).reduce(
      ([x, s]: [number, bigint]): [number, bigint] => gbmStepSeeded(x, 0, 0.2, 0.01, s),
      [1, SEED] as [number, bigint],
    )
    expect(x).toBeGreaterThan(0)
  })
})

// ── Cross-language parity (values verified against Rust prime-diffusion) ──────

describe('cross-language parity', () => {
  it('ouStep(0, 0.5, 1.0, 0.1, 0.01, 0) matches Rust = 0.005', () =>
    expect(ouStep(0, 0.5, 1.0, 0.1, 0.01, 0)).toBeCloseTo(0.005, 5))
  it('ouStep zero noise with theta=1 drifts toward mu', () => {
    // x=1, mu=0, theta=1, sigma=0.1, dt=0.1, w=0: x + 1*(0-1)*0.1 = 0.9
    expect(ouStep(1, 0, 1.0, 0.1, 0.1, 0)).toBeCloseTo(0.9, 5)
  })
  it('gbmStep(100, 0.1, 0, 0.01, 0) matches Rust ≈ 100.1', () =>
    expect(gbmStep(100, 0.1, 0, 0.01, 0)).toBeCloseTo(100.10005, 3))
  it('gbmStep zero mu and sigma — unchanged', () =>
    expect(gbmStep(50, 0, 0, 0.01, 0)).toBeCloseTo(50, 5))
})
