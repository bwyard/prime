/**
 * Tests for `prime-sdf`.
 *
 * Cross-language reference values are taken from the Rust implementation at
 * `crates/prime-sdf/src/`. If the Rust implementation changes, update the
 * expected values here to match.
 *
 * All identifiers are `const` only — no `let` anywhere in this file.
 */

import { describe, it, expect } from 'vitest'
import {
  circle, box2d, roundedBox, capsule2d, lineSegment, triangle, ring,
  sphere, box3d, capsule3d, cylinder, torus, plane,
  union, intersection, subtract, xor,
  smoothUnion, smoothIntersection, smoothSubtract,
  translate, rotate2d, scale2d, repeat2d, mirrorX, mirrorY, elongate,
} from '../index'

const EPS = 1e-5
const approx = (a: number, b: number) => Math.abs(a - b) < EPS

// ── 2D Primitives ─────────────────────────────────────────────────────────────

describe('circle', () => {
  it('outside', () => expect(approx(circle([3, 0], [0, 0], 1), 2)).toBe(true))
  it('inside', () => expect(approx(circle([0, 0], [0, 0], 2), -2)).toBe(true))
  it('on surface', () => expect(approx(circle([1, 0], [0, 0], 1), 0)).toBe(true))
  it('cross-language parity — matches Rust circle([1,0], [0,0], 2) = -1', () => {
    expect(approx(circle([1, 0], [0, 0], 2), -1)).toBe(true)
  })
})

describe('box2d', () => {
  it('outside', () => expect(approx(box2d([3, 0], [0, 0], [1, 1]), 2)).toBe(true))
  it('inside', () => expect(box2d([0.5, 0], [0, 0], [1, 1]) < 0).toBe(true))
  it('on surface', () => expect(approx(box2d([1, 0], [0, 0], [1, 1]), 0)).toBe(true))
  it('cross-language parity — matches Rust box_2d([3,0], [0,0], [1,1]) = 2', () => {
    expect(approx(box2d([3, 0], [0, 0], [1, 1]), 2)).toBe(true)
  })
})

describe('roundedBox', () => {
  it('outside is box minus radius', () =>
    expect(approx(roundedBox([3, 0], [0, 0], [1, 1], 0.5), 1.5)).toBe(true))
  it('inside', () => expect(roundedBox([0, 0], [0, 0], [2, 2], 0.5) < 0).toBe(true))
})

describe('capsule2d', () => {
  it('midpoint perpendicular', () =>
    expect(approx(capsule2d([0, 1], [-1, 0], [1, 0], 0.5), 0.5)).toBe(true))
  it('at endpoint', () =>
    expect(approx(capsule2d([2, 0], [-1, 0], [1, 0], 0.5), 0.5)).toBe(true))
  it('cross-language parity — matches Rust capsule_2d([0,1],[-1,0],[1,0],0.5) = 0.5', () => {
    expect(approx(capsule2d([0, 1], [-1, 0], [1, 0], 0.5), 0.5)).toBe(true)
  })
})

describe('lineSegment', () => {
  it('same as capsule2d', () =>
    expect(lineSegment([0, 1], [-1, 0], [1, 0], 0.5)).toBe(capsule2d([0, 1], [-1, 0], [1, 0], 0.5)))
})

describe('triangle', () => {
  it('inside', () =>
    expect(approx(triangle([0.5, 0.5], [0, 0], [2, 0], [0, 2]), -0.5)).toBe(true))
  it('outside', () =>
    expect(approx(triangle([3, 0], [0, 0], [2, 0], [0, 2]), 1)).toBe(true))
  it('on surface', () =>
    expect(approx(triangle([1, 0], [0, 0], [2, 0], [0, 2]), 0)).toBe(true))
  it('cross-language parity — inside matches Rust triangle([0.5,0.5],[0,0],[2,0],[0,2]) = -0.5', () => {
    expect(approx(triangle([0.5, 0.5], [0, 0], [2, 0], [0, 2]), -0.5)).toBe(true)
  })
})

describe('ring', () => {
  it('on inner surface', () =>
    expect(approx(ring([1, 0], [0, 0], 2, 1), 0)).toBe(true))
  it('on outer surface', () =>
    expect(approx(ring([2, 0], [0, 0], 2, 1), 0)).toBe(true))
  it('outside', () =>
    expect(ring([3, 0], [0, 0], 2, 1) > 0).toBe(true))
})

// ── 3D Primitives ─────────────────────────────────────────────────────────────

describe('sphere', () => {
  it('outside', () => expect(approx(sphere([2, 0, 0], [0, 0, 0], 1), 1)).toBe(true))
  it('inside', () => expect(approx(sphere([0, 0, 0], [0, 0, 0], 2), -2)).toBe(true))
  it('on surface', () => expect(approx(sphere([1, 0, 0], [0, 0, 0], 1), 0)).toBe(true))
  it('cross-language parity — matches Rust sphere([2,0,0], [0,0,0], 1) = 1', () => {
    expect(approx(sphere([2, 0, 0], [0, 0, 0], 1), 1)).toBe(true)
  })
})

describe('box3d', () => {
  it('outside', () => expect(approx(box3d([3, 0, 0], [0, 0, 0], [1, 1, 1]), 2)).toBe(true))
  it('inside', () => expect(box3d([0, 0, 0], [0, 0, 0], [2, 2, 2]) < 0).toBe(true))
  it('cross-language parity — matches Rust box_3d([3,0,0],[0,0,0],[1,1,1]) = 2', () => {
    expect(approx(box3d([3, 0, 0], [0, 0, 0], [1, 1, 1]), 2)).toBe(true)
  })
})

describe('capsule3d', () => {
  it('midpoint perpendicular', () =>
    expect(approx(capsule3d([1, 1, 0], [0, 0, 0], [0, 2, 0], 0.5), 0.5)).toBe(true))
  it('at endpoint', () =>
    expect(approx(capsule3d([1, 0, 0], [0, 0, 0], [0, 2, 0], 0.5), 0.5)).toBe(true))
  it('on surface', () =>
    expect(approx(capsule3d([0.5, 1, 0], [0, 0, 0], [0, 2, 0], 0.5), 0)).toBe(true))
})

describe('cylinder', () => {
  it('outside', () => expect(approx(cylinder([3, 0, 0], [0, 0, 0], 2, 1), 2)).toBe(true))
  it('inside', () => expect(approx(cylinder([0, 0, 0], [0, 0, 0], 2, 1), -1)).toBe(true))
  it('on curved surface', () =>
    expect(approx(cylinder([1, 0, 0], [0, 0, 0], 2, 1), 0)).toBe(true))
  it('cross-language parity — matches Rust cylinder([3,0,0],[0,0,0],2,1) = 2', () => {
    expect(approx(cylinder([3, 0, 0], [0, 0, 0], 2, 1), 2)).toBe(true)
  })
})

describe('torus', () => {
  it('outside', () => expect(approx(torus([6, 0, 0], [0, 0, 0], 3, 1), 2)).toBe(true))
  it('inside tube', () => expect(approx(torus([3, 0.5, 0], [0, 0, 0], 3, 1), -0.5)).toBe(true))
  it('on surface', () => expect(approx(torus([4, 0, 0], [0, 0, 0], 3, 1), 0)).toBe(true))
  it('cross-language parity — matches Rust torus([6,0,0],[0,0,0],3,1) = 2', () => {
    expect(approx(torus([6, 0, 0], [0, 0, 0], 3, 1), 2)).toBe(true)
  })
})

describe('plane', () => {
  it('above', () => expect(approx(plane([0, 5, 0], [0, 1, 0], 0), 5)).toBe(true))
  it('below', () => expect(approx(plane([0, -3, 0], [0, 1, 0], 0), -3)).toBe(true))
  it('on surface', () => expect(approx(plane([0, 2, 0], [0, 1, 0], 2), 0)).toBe(true))
})

// ── Boolean Ops ───────────────────────────────────────────────────────────────

describe('union', () => {
  it('takes min', () => expect(union(1, 2)).toBe(1))
  it('negative wins', () => expect(union(-1, 2)).toBe(-1))
})

describe('intersection', () => {
  it('takes max', () => expect(intersection(1, 2)).toBe(2))
})

describe('subtract', () => {
  it('removes second shape from first', () => expect(subtract(1, -2) > 0).toBe(true))
  it('inside both → outside result', () => expect(subtract(-1, -2) > 0).toBe(true))
})

describe('xor', () => {
  it('both inside → outside', () => expect(xor(-1, -2) > 0).toBe(true))
  it('one inside → inside result', () => expect(xor(1, -2) < 0).toBe(true))
})

// ── Smooth Ops ────────────────────────────────────────────────────────────────

describe('smoothUnion', () => {
  it('approaches regular union as k→0', () => {
    expect(Math.abs(smoothUnion(1, 2, 0.001) - Math.min(1, 2)) < 0.01).toBe(true)
  })
  it('blends below min at equal distances', () =>
    expect(smoothUnion(1, 1, 0.5) < 1).toBe(true))
  it('cross-language parity — matches Rust smooth_union(1.0, 2.0, 0.001) ≈ 1.0', () => {
    expect(Math.abs(smoothUnion(1, 2, 0.001) - 1.0) < 0.01).toBe(true)
  })
})

describe('smoothIntersection', () => {
  it('approaches regular intersection as k→0', () => {
    expect(Math.abs(smoothIntersection(1, 2, 0.001) - Math.max(1, 2)) < 0.01).toBe(true)
  })
})

describe('smoothSubtract', () => {
  it('approaches regular subtract as k→0', () => {
    expect(Math.abs(smoothSubtract(1, 2, 0.001) - Math.max(1, -2)) < 0.01).toBe(true)
  })
})

// ── Domain Transforms ─────────────────────────────────────────────────────────

describe('translate', () => {
  it('moves point by offset', () => {
    const [x, y] = translate([3, 0], [1, 0])
    expect(approx(x, 2)).toBe(true)
    expect(approx(y, 0)).toBe(true)
  })
  it('cross-language parity — matches Rust translate([3,0], [1,0]) = [2,0]', () => {
    const [x] = translate([3, 0], [1, 0])
    expect(approx(x, 2)).toBe(true)
  })
})

describe('rotate2d', () => {
  it('90 degrees rotates [1,0] to [0,-1]', () => {
    const [x, y] = rotate2d([1, 0], Math.PI / 2)
    expect(approx(x, 0)).toBe(true)
    expect(approx(y, -1)).toBe(true)
  })
  it('0 degrees is identity', () => {
    const [x, y] = rotate2d([3, 4], 0)
    expect(approx(x, 3)).toBe(true)
    expect(approx(y, 4)).toBe(true)
  })
})

describe('scale2d', () => {
  it('halves coordinates', () => {
    const [x, y] = scale2d([4, 6], 2)
    expect(approx(x, 2)).toBe(true)
    expect(approx(y, 3)).toBe(true)
  })
  it('identity at factor=1', () => {
    const [x, y] = scale2d([3, 5], 1)
    expect(approx(x, 3) && approx(y, 5)).toBe(true)
  })
})

describe('repeat2d', () => {
  it('period-apart points map to same value', () => {
    const [x1] = repeat2d([0, 0], [4, 4])
    const [x2] = repeat2d([4, 0], [4, 4])
    expect(approx(x1, x2)).toBe(true)
  })
})

describe('mirrorX', () => {
  it('negatives fold to positive x', () => {
    const [x, y] = mirrorX([-2, 3])
    expect(approx(x, 2)).toBe(true)
    expect(approx(y, 3)).toBe(true)
  })
})

describe('mirrorY', () => {
  it('negatives fold to positive y', () => {
    const [x, y] = mirrorY([2, -3])
    expect(approx(x, 2)).toBe(true)
    expect(approx(y, 3)).toBe(true)
  })
})

describe('elongate', () => {
  it('zero h is identity', () => {
    const [x, y] = elongate([3, 2], [0, 0])
    expect(approx(x, 3) && approx(y, 2)).toBe(true)
  })
  it('point within band collapses to zero', () => {
    const [x, y] = elongate([0.5, 0], [2, 1])
    expect(Math.sqrt(x * x + y * y) < EPS).toBe(true)
  })
  it('point beyond band offset', () => {
    const [x, y] = elongate([5, 0], [2, 1])
    expect(approx(x, 3) && approx(y, 0)).toBe(true)
  })
})
