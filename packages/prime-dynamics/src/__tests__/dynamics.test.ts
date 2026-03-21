import { describe, it, expect } from 'vitest'
import {
  rk4Step,
  rk4Step3,
  eulerStep,
  lorenzStep,
  rosslerStep,
  duffingStep,
  LORENZ_SIGMA,
  LORENZ_RHO,
  LORENZ_BETA,
} from '../index.js'

const EPS = 1e-4

// ── rk4Step ───────────────────────────────────────────────────────────────────

describe('rk4Step', () => {
  it('exponential decay — 100 steps matches e^(-1)', () => {
    const s = Array.from<null>({ length: 100 }).reduce(
      ([state, n]: [number, number]): [number, number] => [
        rk4Step(state, n * 0.01, 0.01, (_t, x) => -x),
        n + 1,
      ],
      [1, 0] as [number, number],
    )[0]
    expect(Math.abs(s - Math.exp(-1))).toBeLessThan(EPS)
  })

  it('zero dt — returns state unchanged', () => {
    expect(rk4Step(3.14, 0, 0, (_t, x) => -x)).toBeCloseTo(3.14, 5)
  })

  it('constant derivative — s + dt*1', () => {
    expect(rk4Step(0, 0, 1, () => 1)).toBeCloseTo(1, 5)
  })

  it('deterministic', () => {
    const a = rk4Step(1, 0, 0.01, (_t, x) => -x)
    const b = rk4Step(1, 0, 0.01, (_t, x) => -x)
    expect(a).toBe(b)
  })

  it('cross-language parity — matches Rust rk4_step(1.0, 0.0, 0.01, |_t, x| -x)', () => {
    // Rust: rk4_step(1.0_f32, 0.0, 0.01, |_t, x| -x) ≈ 0.99004983
    const s = rk4Step(1, 0, 0.01, (_t, x) => -x)
    expect(Math.abs(s - Math.exp(-0.01))).toBeLessThan(1e-7)
  })
})

// ── rk4Step3 ──────────────────────────────────────────────────────────────────

describe('rk4Step3', () => {
  it('circular motion preserves radius over 1000 steps', () => {
    const [x, y] = Array.from<null>({ length: 1000 }).reduce(
      ([s, n]: [[number, number, number], number]): [[number, number, number], number] => [
        rk4Step3(s, n * 0.01, 0.01, (_t, [sx, sy]) => [-sy, sx, 0]),
        n + 1,
      ],
      [[1, 0, 0] as [number, number, number], 0] as [[number, number, number], number],
    )[0]
    expect(Math.abs(Math.sqrt(x * x + y * y) - 1)).toBeLessThan(1e-3)
  })

  it('zero dt — returns state unchanged', () => {
    const s = rk4Step3([1, 2, 3], 0, 0, (_t, sv) => sv)
    expect(s).toEqual([1, 2, 3])
  })

  it('deterministic', () => {
    const f = (_t: number, [x, y, z]: [number, number, number]): [number, number, number] => [y - x, x - z, z - y]
    const a = rk4Step3([1, 0.5, -0.5], 0, 0.01, f)
    const b = rk4Step3([1, 0.5, -0.5], 0, 0.01, f)
    expect(a).toEqual(b)
  })
})

// ── eulerStep ─────────────────────────────────────────────────────────────────

describe('eulerStep', () => {
  it('linear step — 1 + 0.1*(-1) = 0.9', () => {
    expect(eulerStep(1, 0, 0.1, () => -1)).toBeCloseTo(0.9, 5)
  })

  it('zero dt — returns state unchanged', () => {
    expect(eulerStep(5, 0, 0, (_t, x) => x * 100)).toBeCloseTo(5, 5)
  })

  it('deterministic', () => {
    const a = eulerStep(1, 0.5, 0.01, (t, s) => t - s)
    const b = eulerStep(1, 0.5, 0.01, (t, s) => t - s)
    expect(a).toBe(b)
  })
})

// ── lorenzStep ────────────────────────────────────────────────────────────────

describe('lorenzStep', () => {
  it('moves state', () => {
    const s0: [number, number, number] = [1, 1, 1]
    const s1 = lorenzStep(s0, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
    expect(s1).not.toEqual(s0)
  })

  it('zero dt — returns state unchanged', () => {
    const s0: [number, number, number] = [1, 2, 3]
    const s1 = lorenzStep(s0, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0)
    expect(s1[0]).toBeCloseTo(s0[0], 5)
    expect(s1[1]).toBeCloseTo(s0[1], 5)
    expect(s1[2]).toBeCloseTo(s0[2], 5)
  })

  it('bounded over 1000 steps', () => {
    const [x, y, z] = Array.from<null>({ length: 1000 }).reduce(
      ([s]: [[number, number, number]]): [[number, number, number]] => [
        lorenzStep(s, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01),
      ],
      [[1, 1, 1] as [number, number, number]],
    )[0]
    expect(Math.abs(x)).toBeLessThan(100)
    expect(Math.abs(y)).toBeLessThan(100)
    expect(Math.abs(z)).toBeLessThan(100)
  })

  it('deterministic', () => {
    const a = lorenzStep([1, 0, 0], LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
    const b = lorenzStep([1, 0, 0], LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01)
    expect(a).toEqual(b)
  })

  it('sensitive to initial conditions — 1e-3 perturbation diverges over 3000 steps', () => {
    const run = (x0: number) =>
      Array.from<null>({ length: 3000 }).reduce(
        ([s]: [[number, number, number]]): [[number, number, number]] => [
          lorenzStep(s, LORENZ_SIGMA, LORENZ_RHO, LORENZ_BETA, 0.01),
        ],
        [[x0, 1, 1] as [number, number, number]],
      )[0]
    const [x1] = run(1.0)
    const [x2] = run(1.001)
    expect(Math.abs(x1 - x2)).toBeGreaterThan(0.5)
  })
})

// ── rosslerStep ───────────────────────────────────────────────────────────────

describe('rosslerStep', () => {
  it('moves state', () => {
    const s0: [number, number, number] = [1, 0, 0]
    const s1 = rosslerStep(s0, 0.2, 0.2, 5.7, 0.01)
    expect(s1).not.toEqual(s0)
  })

  it('bounded over 1000 steps', () => {
    const [x, y, z] = Array.from<null>({ length: 1000 }).reduce(
      ([s]: [[number, number, number]]): [[number, number, number]] => [
        rosslerStep(s, 0.2, 0.2, 5.7, 0.01),
      ],
      [[1, 0, 0] as [number, number, number]],
    )[0]
    expect(Math.abs(x)).toBeLessThan(50)
    expect(Math.abs(y)).toBeLessThan(50)
    expect(Math.abs(z)).toBeLessThan(50)
  })

  it('deterministic', () => {
    const a = rosslerStep([1, 0, 0], 0.2, 0.2, 5.7, 0.01)
    const b = rosslerStep([1, 0, 0], 0.2, 0.2, 5.7, 0.01)
    expect(a).toEqual(b)
  })
})

// ── duffingStep ───────────────────────────────────────────────────────────────

describe('duffingStep', () => {
  const P = { delta: 0.3, alpha: -1, beta: 1, gamma: 0.37, omega: 1.2 }

  it('nonzero drive produces nonzero velocity from rest', () => {
    const [, v1] = duffingStep([0, 0], 0, P, 0.01)
    expect(Math.abs(v1)).toBeGreaterThan(0)
  })

  it('zero dt — returns state unchanged', () => {
    const [x1, v1] = duffingStep([1, 0.5], 0, P, 0)
    expect(x1).toBeCloseTo(1, 5)
    expect(v1).toBeCloseTo(0.5, 5)
  })

  it('deterministic', () => {
    const a = duffingStep([1, 0], 0.5, P, 0.01)
    const b = duffingStep([1, 0], 0.5, P, 0.01)
    expect(a).toEqual(b)
  })
})
