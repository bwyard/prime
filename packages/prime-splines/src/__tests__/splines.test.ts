import { describe, it, expect } from 'vitest'
import {
  bezierQuadratic, bezierQuadratic3d,
  bezierCubic, bezierCubic3d,
  bezierCubicArcLength, bezierCubicArcLength3d,
  bezierCubicTAtLength, bezierCubicTAtLength3d,
  hermite, hermite3d,
  catmullRom, catmullRom3d,
  bSplineCubic, bSplineCubic3d,
  slerp,
} from '../index.js'

const EPS = 1e-4

// ── bezierQuadratic ───────────────────────────────────────────────────────────

describe('bezierQuadratic', () => {
  it('t=0 → p0', () => expect(bezierQuadratic(0, 1, 2, 3)).toBeCloseTo(1, 5))
  it('t=1 → p2', () => expect(bezierQuadratic(1, 1, 2, 3)).toBeCloseTo(3, 5))

  it('symmetric peak at midpoint', () => {
    expect(bezierQuadratic(0.5, 0, 1, 0)).toBeCloseTo(0.5, 5)
  })

  it('deterministic', () => {
    expect(bezierQuadratic(0.3, 0, 1, 2)).toBe(bezierQuadratic(0.3, 0, 1, 2))
  })

  it('cross-language parity — matches Rust bezier_quadratic(0.5, 0.0, 1.0, 0.0)', () => {
    expect(bezierQuadratic(0.5, 0, 1, 0)).toBeCloseTo(0.5, 5)
  })
})

describe('bezierQuadratic3d', () => {
  it('t=0 → p0', () => {
    const [x] = bezierQuadratic3d(0, [1, 0, 0], [2, 0, 0], [3, 0, 0])
    expect(x).toBeCloseTo(1, 5)
  })

  it('t=1 → p2', () => {
    const [x] = bezierQuadratic3d(1, [1, 0, 0], [2, 0, 0], [3, 0, 0])
    expect(x).toBeCloseTo(3, 5)
  })
})

// ── bezierCubic ───────────────────────────────────────────────────────────────

describe('bezierCubic', () => {
  it('t=0 → p0', () => expect(bezierCubic(0, 1, 2, 3, 4)).toBeCloseTo(1, 5))
  it('t=1 → p3', () => expect(bezierCubic(1, 1, 2, 3, 4)).toBeCloseTo(4, 5))

  it('collinear control points → linear', () => {
    expect(bezierCubic(0.5, 0, 1, 2, 3)).toBeCloseTo(1.5, 4)
  })

  it('deterministic', () => {
    expect(bezierCubic(0.4, 0, 1, 1, 0)).toBe(bezierCubic(0.4, 0, 1, 1, 0))
  })
})

describe('bezierCubic3d', () => {
  it('t=0 → p0', () => {
    const [x] = bezierCubic3d(0, [0,0,0], [1,0,0], [2,0,0], [3,0,0])
    expect(x).toBeCloseTo(0, 5)
  })

  it('t=1 → p3', () => {
    const [x] = bezierCubic3d(1, [0,0,0], [1,0,0], [2,0,0], [3,0,0])
    expect(x).toBeCloseTo(3, 5)
  })
})

// ── hermite ───────────────────────────────────────────────────────────────────

describe('hermite', () => {
  it('t=0 → p0', () => expect(hermite(0, 1, 2, 5, 2)).toBeCloseTo(1, 5))
  it('t=1 → p1', () => expect(hermite(1, 1, 2, 5, 2)).toBeCloseTo(5, 5))

  it('linear case — tangents = 1, straight line', () => {
    expect(hermite(0.5, 0, 1, 1, 1)).toBeCloseTo(0.5, 4)
  })

  it('deterministic', () => {
    expect(hermite(0.3, 0, 1, 1, 0)).toBe(hermite(0.3, 0, 1, 1, 0))
  })
})

describe('hermite3d', () => {
  it('t=0 → p0', () => {
    const [x, y] = hermite3d(0, [0,0,0], [1,0,0], [1,1,0], [1,0,0])
    expect(x).toBeCloseTo(0, 5)
    expect(y).toBeCloseTo(0, 5)
  })
})

// ── catmullRom ────────────────────────────────────────────────────────────────

describe('catmullRom', () => {
  it('t=0 → p1', () => expect(catmullRom(0, 0, 1, 2, 3)).toBeCloseTo(1, 5))
  it('t=1 → p2', () => expect(catmullRom(1, 0, 1, 2, 3)).toBeCloseTo(2, 5))

  it('midpoint collinear → linear', () => {
    expect(catmullRom(0.5, 0, 1, 2, 3)).toBeCloseTo(1.5, 4)
  })

  it('deterministic', () => {
    expect(catmullRom(0.4, 0, 1, 0.5, 0)).toBe(catmullRom(0.4, 0, 1, 0.5, 0))
  })
})

describe('catmullRom3d', () => {
  it('t=0 → p1', () => {
    const [x, y] = catmullRom3d(0, [0,0,0], [1,2,3], [3,1,0], [5,0,0])
    expect(x).toBeCloseTo(1, 4)
    expect(y).toBeCloseTo(2, 4)
  })
})

// ── bSplineCubic ──────────────────────────────────────────────────────────────

describe('bSplineCubic', () => {
  it('collinear midpoint → 1.5', () => {
    expect(bSplineCubic(0.5, 0, 1, 2, 3)).toBeCloseTo(1.5, 4)
  })

  it('start knot value = (p0 + 4p1 + p2) / 6', () => {
    const expected = (0 + 4 * 1 + 2) / 6
    expect(bSplineCubic(0, 0, 1, 2, 3)).toBeCloseTo(expected, 4)
  })

  it('end knot value = (p1 + 4p2 + p3) / 6', () => {
    const expected = (1 + 4 * 2 + 3) / 6
    expect(bSplineCubic(1, 0, 1, 2, 3)).toBeCloseTo(expected, 4)
  })

  it('deterministic', () => {
    expect(bSplineCubic(0.3, 0, 1, 2, 3)).toBe(bSplineCubic(0.3, 0, 1, 2, 3))
  })

  it('cross-language parity — collinear midpoint matches Rust', () => {
    expect(bSplineCubic(0.5, 0, 1, 2, 3)).toBeCloseTo(1.5, 4)
  })
})

describe('bSplineCubic3d', () => {
  it('collinear midpoint', () => {
    const [x] = bSplineCubic3d(0.5, [0,0,0], [1,0,0], [2,0,0], [3,0,0])
    expect(x).toBeCloseTo(1.5, 4)
  })
})

// ── bezierCubicArcLength ─────────────────────────────────────────────────────

describe('bezierCubicArcLength', () => {
  it('straight line 0→3 has arc length ≈ 3', () => {
    expect(bezierCubicArcLength(0, 1, 2, 3, 100)).toBeCloseTo(3, 1)
  })

  it('zero-length curve (all points equal)', () => {
    expect(bezierCubicArcLength(1, 1, 1, 1, 100)).toBeCloseTo(0, 5)
  })

  it('deterministic', () => {
    expect(bezierCubicArcLength(0, 1, 2, 3, 50)).toBe(bezierCubicArcLength(0, 1, 2, 3, 50))
  })
})

describe('bezierCubicArcLength3d', () => {
  it('straight line along x-axis has arc length ≈ 3', () => {
    expect(bezierCubicArcLength3d(
      [0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0], 100,
    )).toBeCloseTo(3, 1)
  })

  it('zero-length curve', () => {
    expect(bezierCubicArcLength3d(
      [1, 1, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1], 100,
    )).toBeCloseTo(0, 5)
  })
})

// ── bezierCubicTAtLength ─────────────────────────────────────────────────────

describe('bezierCubicTAtLength', () => {
  it('half length of straight line 0→3 at t≈0.5', () => {
    const t = bezierCubicTAtLength(0, 1, 2, 3, 1.5, 100, 20)
    expect(Math.abs(t - 0.5)).toBeLessThan(0.01)
  })

  it('zero target length → t≈0', () => {
    const t = bezierCubicTAtLength(0, 1, 2, 3, 0, 100, 20)
    expect(t).toBeLessThan(0.01)
  })

  it('full length → t≈1', () => {
    const fullLen = bezierCubicArcLength(0, 1, 2, 3, 100)
    const t = bezierCubicTAtLength(0, 1, 2, 3, fullLen, 100, 20)
    expect(Math.abs(t - 1)).toBeLessThan(0.01)
  })

  it('deterministic', () => {
    expect(bezierCubicTAtLength(0, 1, 2, 3, 1.5, 100, 20))
      .toBe(bezierCubicTAtLength(0, 1, 2, 3, 1.5, 100, 20))
  })
})

describe('bezierCubicTAtLength3d', () => {
  it('half length of straight line along x-axis at t≈0.5', () => {
    const t = bezierCubicTAtLength3d(
      [0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0], 1.5, 100, 20,
    )
    expect(Math.abs(t - 0.5)).toBeLessThan(0.01)
  })

  it('full length → t≈1', () => {
    const fullLen = bezierCubicArcLength3d(
      [0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0], 100,
    )
    const t = bezierCubicTAtLength3d(
      [0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0], fullLen, 100, 20,
    )
    expect(Math.abs(t - 1)).toBeLessThan(0.01)
  })
})

// ── slerp ─────────────────────────────────────────────────────────────────────

describe('slerp', () => {
  const identity: [number, number, number, number] = [0, 0, 0, 1]
  const rotZ90: [number, number, number, number] = [0, 0, 1, 0] // 180° around z (not 90°)

  it('t=0 → q0', () => {
    const r = slerp(0, identity, rotZ90)
    expect(r[3]).toBeCloseTo(1, 4)
  })

  it('t=1 → q1', () => {
    const r = slerp(1, identity, rotZ90)
    expect(r[2]).toBeCloseTo(1, 4)
    expect(r[3]).toBeCloseTo(0, 4)
  })

  it('preserves unit length throughout', () => {
    Array.from<null>({ length: 11 }).forEach((_, i) => {
      const t = i / 10
      const r = slerp(t, identity, rotZ90)
      const lenSq = r[0] ** 2 + r[1] ** 2 + r[2] ** 2 + r[3] ** 2
      expect(Math.abs(lenSq - 1)).toBeLessThan(EPS)
    })
  })

  it('deterministic', () => {
    const a = slerp(0.5, identity, rotZ90)
    const b = slerp(0.5, identity, rotZ90)
    expect(a).toEqual(b)
  })

  it('halfway is equidistant — 90° rotation', () => {
    const FRAC_1_SQRT_2 = 1 / Math.sqrt(2)
    const mid = slerp(0.5, identity, rotZ90)
    expect(Math.abs(mid[3])).toBeCloseTo(FRAC_1_SQRT_2, 3)
  })
})
