import { describe, it, expect } from 'vitest'
import { smoothdamp, spring, lowPass, highPass, deadzone } from '../index.js'

const EPS = 1e-4

describe('smoothdamp', () => {
  it('approaches target over time', () => {
    const [pos] = Array.from<null>({ length: 200 }).reduce(
      ([p, v]: [number, number]) => smoothdamp(p, 10, v, 0.3, 0.016),
      [0, 0] as [number, number],
    )
    expect(Math.abs(pos - 10)).toBeLessThan(0.01)
  })

  it('does not overshoot', () => {
    const { maxPos } = Array.from<null>({ length: 300 }).reduce(
      ({ pos, vel, maxPos }: { pos: number; vel: number; maxPos: number }) => {
        const [p, v] = smoothdamp(pos, 10, vel, 0.3, 0.016)
        return { pos: p, vel: v, maxPos: Math.max(maxPos, p) }
      },
      { pos: 0, vel: 0, maxPos: 0 },
    )
    expect(maxPos).toBeLessThanOrEqual(10.001)
  })

  it('returns current when dt=0', () => {
    const [val] = smoothdamp(3, 10, 5, 0.3, 0)
    expect(val).toBeCloseTo(3, 4)
  })

  it('velocity decays to zero', () => {
    const [, vel] = Array.from<null>({ length: 500 }).reduce(
      ([p, v]: [number, number]) => smoothdamp(p, 10, v, 0.3, 0.016),
      [0, 0] as [number, number],
    )
    expect(Math.abs(vel)).toBeLessThan(0.001)
  })

  it('handles negative smooth time without throwing', () => {
    expect(() => smoothdamp(0, 10, 0, -1, 0.016)).not.toThrow()
  })
})

describe('spring', () => {
  it('approaches target', () => {
    const [pos] = Array.from<null>({ length: 500 }).reduce(
      ([p, v]: [number, number]) => spring(p, v, 10, 100, 20, 0.016),
      [0, 0] as [number, number],
    )
    expect(Math.abs(pos - 10)).toBeLessThan(0.01)
  })

  it('no change when dt=0', () => {
    const [pos, vel] = spring(3, 5, 10, 100, 20, 0)
    expect(pos).toBeCloseTo(3, 4)
    expect(vel).toBeCloseTo(5, 4)
  })

  it('underdamped spring overshoots', () => {
    const { max } = Array.from<null>({ length: 100 }).reduce(
      ({ pos, vel, max }: { pos: number; vel: number; max: number }) => {
        const [p, v] = spring(pos, vel, 10, 200, 5, 0.016)
        return { pos: p, vel: v, max: Math.max(max, p) }
      },
      { pos: 0, vel: 0, max: 0 },
    )
    expect(max).toBeGreaterThan(10)
  })
})

describe('lowPass', () => {
  it('converges to input', () => {
    const f = Array.from<null>({ length: 300 }).reduce(
      (prev: number) => lowPass(prev, 1, 0.1, 0.016),
      0,
    )
    expect(Math.abs(f - 1)).toBeLessThan(0.01)
  })

  it('returns previous when dt=0', () => {
    expect(lowPass(0.5, 1, 0.1, 0)).toBeCloseTo(0.5, 4)
  })

  it('large time constant responds slowly', () => {
    expect(lowPass(0, 1, 10, 0.016)).toBeLessThan(0.01)
  })

  it('small time constant responds fast', () => {
    expect(lowPass(0, 1, 0.001, 0.016)).toBeGreaterThan(0.9)
  })
})

describe('highPass', () => {
  it('DC signal approaches zero', () => {
    const [out] = Array.from<null>({ length: 300 }).reduce(
      ([, lp]: [number, number]) => highPass(lp, 1, 0.05, 0.016),
      [0, 0] as [number, number],
    )
    expect(Math.abs(out)).toBeLessThan(0.01)
  })

  it('step produces non-zero initial output', () => {
    const [out] = highPass(0, 1, 0.1, 0.016)
    expect(out).toBeGreaterThan(0)
  })
})

describe('deadzone', () => {
  it('returns 0 inside deadzone', () => {
    expect(deadzone(0.05, 0.1)).toBe(0)
    expect(deadzone(-0.05, 0.1)).toBe(0)
    expect(deadzone(0, 0.1)).toBe(0)
  })

  it('returns ±1 at full deflection', () => {
    expect(deadzone(1, 0.1)).toBeCloseTo(1, 4)
    expect(deadzone(-1, 0.1)).toBeCloseTo(-1, 4)
  })

  it('preserves sign', () => {
    expect(deadzone(0.5, 0.1)).toBeGreaterThan(0)
    expect(deadzone(-0.5, 0.1)).toBeLessThan(0)
  })

  it('value at exact threshold returns 0', () => {
    expect(deadzone(0.1, 0.1)).toBe(0)
  })

  it('quadratic curve smaller than linear near threshold', () => {
    expect(deadzone(0.55, 0.1, 2)).toBeLessThan(deadzone(0.55, 0.1, 1))
  })
})

// ── Cross-language parity (values verified against Rust prime-signal) ─────────

describe('cross-language parity', () => {
  it('lowPass(0, 1, 1, 0.1) matches Rust ≈ 0.09516', () =>
    expect(lowPass(0, 1, 1, 0.1)).toBeCloseTo(0.09516258, 5))
  it('deadzone(0.5, 0.3) matches Rust ≈ 0.2857', () =>
    expect(deadzone(0.5, 0.3)).toBeCloseTo(0.2857142857, 4))
  it('deadzone(0.0, 0.3) matches Rust = 0.0', () =>
    expect(deadzone(0.0, 0.3)).toBe(0))
  it('smoothdamp is deterministic — same inputs each call', () => {
    const [p1, v1] = smoothdamp(0, 10, 0, 0.3, 0.016)
    const [p2, v2] = smoothdamp(0, 10, 0, 0.3, 0.016)
    expect(p1).toBe(p2)
    expect(v1).toBe(v2)
  })
  it('spring is deterministic — same inputs each call', () => {
    const [p1, v1] = spring(0, 10, 0, 100, 10, 0.016)
    const [p2, v2] = spring(0, 10, 0, 100, 10, 0.016)
    expect(p1).toBe(p2)
    expect(v1).toBe(v2)
  })
})
