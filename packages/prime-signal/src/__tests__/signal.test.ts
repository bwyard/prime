import { describe, it, expect } from 'vitest'
import {
  smoothdamp, spring, lowPass, highPass, deadzone,
  smoothdampVec2, smoothdampVec3, springVec2, springVec3,
} from '../index.js'
import type { Vec2, Vec3 } from '../index.js'

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

// ── smoothdampVec2 ───────────────────────────────────────────────────────────

describe('smoothdampVec2', () => {
  it('matches scalar smoothdamp component-wise', () => {
    const cur: Vec2 = [1, 2]
    const tgt: Vec2 = [10, 20]
    const vel: Vec2 = [0.5, -0.3]
    const [pos2, vel2] = smoothdampVec2(cur, tgt, vel, 0.3, 0.016)
    const [sx, svx] = smoothdamp(1, 10, 0.5, 0.3, 0.016)
    const [sy, svy] = smoothdamp(2, 20, -0.3, 0.3, 0.016)
    expect(pos2[0]).toBeCloseTo(sx, 4)
    expect(pos2[1]).toBeCloseTo(sy, 4)
    expect(vel2[0]).toBeCloseTo(svx, 4)
    expect(vel2[1]).toBeCloseTo(svy, 4)
  })

  it('approaches target', () => {
    const target: Vec2 = [10, 5]
    const [pos] = Array.from<null>({ length: 200 }).reduce(
      ([p, v]: [Vec2, Vec2]) => smoothdampVec2(p, target, v, 0.3, 0.016),
      [[0, 0], [0, 0]] as [Vec2, Vec2],
    )
    expect(Math.hypot(pos[0] - target[0], pos[1] - target[1])).toBeLessThan(0.01)
  })
})

// ── smoothdampVec3 ───────────────────────────────────────────────────────────

describe('smoothdampVec3', () => {
  it('matches scalar smoothdamp component-wise', () => {
    const cur: Vec3 = [1, 2, 3]
    const tgt: Vec3 = [10, 20, 30]
    const vel: Vec3 = [0.5, -0.3, 1]
    const [pos3, vel3] = smoothdampVec3(cur, tgt, vel, 0.3, 0.016)
    const [sx, svx] = smoothdamp(1, 10, 0.5, 0.3, 0.016)
    const [sy, svy] = smoothdamp(2, 20, -0.3, 0.3, 0.016)
    const [sz, svz] = smoothdamp(3, 30, 1, 0.3, 0.016)
    expect(pos3[0]).toBeCloseTo(sx, 4)
    expect(pos3[1]).toBeCloseTo(sy, 4)
    expect(pos3[2]).toBeCloseTo(sz, 4)
    expect(vel3[0]).toBeCloseTo(svx, 4)
    expect(vel3[1]).toBeCloseTo(svy, 4)
    expect(vel3[2]).toBeCloseTo(svz, 4)
  })

  it('approaches target', () => {
    const target: Vec3 = [10, 5, 3]
    const [pos] = Array.from<null>({ length: 200 }).reduce(
      ([p, v]: [Vec3, Vec3]) => smoothdampVec3(p, target, v, 0.3, 0.016),
      [[0, 0, 0], [0, 0, 0]] as [Vec3, Vec3],
    )
    expect(Math.hypot(pos[0] - target[0], pos[1] - target[1], pos[2] - target[2])).toBeLessThan(0.01)
  })
})

// ── springVec2 ───────────────────────────────────────────────────────────────

describe('springVec2', () => {
  it('matches scalar spring component-wise', () => {
    const pos: Vec2 = [1, 2]
    const vel: Vec2 = [0.5, -0.3]
    const tgt: Vec2 = [10, 20]
    const [p2, v2] = springVec2(pos, vel, tgt, 100, 20, 0.016)
    const [sx, svx] = spring(1, 0.5, 10, 100, 20, 0.016)
    const [sy, svy] = spring(2, -0.3, 20, 100, 20, 0.016)
    expect(p2[0]).toBeCloseTo(sx, 4)
    expect(p2[1]).toBeCloseTo(sy, 4)
    expect(v2[0]).toBeCloseTo(svx, 4)
    expect(v2[1]).toBeCloseTo(svy, 4)
  })

  it('approaches target', () => {
    const target: Vec2 = [10, 5]
    const [pos] = Array.from<null>({ length: 500 }).reduce(
      ([p, v]: [Vec2, Vec2]) => springVec2(p, v, target, 100, 20, 0.016),
      [[0, 0], [0, 0]] as [Vec2, Vec2],
    )
    expect(Math.hypot(pos[0] - target[0], pos[1] - target[1])).toBeLessThan(0.01)
  })
})

// ── springVec3 ───────────────────────────────────────────────────────────────

describe('springVec3', () => {
  it('matches scalar spring component-wise', () => {
    const pos: Vec3 = [1, 2, 3]
    const vel: Vec3 = [0.5, -0.3, 1]
    const tgt: Vec3 = [10, 20, 30]
    const [p3, v3] = springVec3(pos, vel, tgt, 100, 20, 0.016)
    const [sx, svx] = spring(1, 0.5, 10, 100, 20, 0.016)
    const [sy, svy] = spring(2, -0.3, 20, 100, 20, 0.016)
    const [sz, svz] = spring(3, 1, 30, 100, 20, 0.016)
    expect(p3[0]).toBeCloseTo(sx, 4)
    expect(p3[1]).toBeCloseTo(sy, 4)
    expect(p3[2]).toBeCloseTo(sz, 4)
    expect(v3[0]).toBeCloseTo(svx, 4)
    expect(v3[1]).toBeCloseTo(svy, 4)
    expect(v3[2]).toBeCloseTo(svz, 4)
  })

  it('approaches target', () => {
    const target: Vec3 = [10, 5, 3]
    const [pos] = Array.from<null>({ length: 500 }).reduce(
      ([p, v]: [Vec3, Vec3]) => springVec3(p, v, target, 100, 20, 0.016),
      [[0, 0, 0], [0, 0, 0]] as [Vec3, Vec3],
    )
    expect(Math.hypot(pos[0] - target[0], pos[1] - target[1], pos[2] - target[2])).toBeLessThan(0.01)
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
