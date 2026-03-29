import { describe, it, expect } from 'vitest'
import {
  lerp, lerpClamped, invLerp, remap,
  smoothstep, smootherstep,
  easeInQuad, easeOutQuad, easeInOutQuad,
  easeInCubic, easeOutCubic, easeInOutCubic,
  easeInSine, easeOutSine, easeInOutSine,
  easeInExpo, easeOutExpo, easeInOutExpo,
  easeInElastic, easeOutElastic,
  easeInBounce, easeOutBounce, easeInOutBounce,
  easeInBack, easeOutBack,
  repeat, pingpong,
} from '../index.js'

const EPS = 1e-5

describe('lerp', () => {
  it('returns a at t=0', () => expect(lerp(3, 7, 0)).toBeCloseTo(3, 5))
  it('returns b at t=1', () => expect(lerp(3, 7, 1)).toBeCloseTo(7, 5))
  it('returns midpoint at t=0.5', () => expect(lerp(0, 10, 0.5)).toBeCloseTo(5, 5))
  it('extrapolates below 0', () => expect(lerp(0, 10, -0.5)).toBeCloseTo(-5, 5))
  it('extrapolates above 1', () => expect(lerp(0, 10, 1.5)).toBeCloseTo(15, 5))
})

describe('invLerp', () => {
  it('returns 0 at start', () => expect(invLerp(0, 10, 0)).toBeCloseTo(0, 5))
  it('returns 1 at end', () => expect(invLerp(0, 10, 10)).toBeCloseTo(1, 5))
  it('returns 0.5 at midpoint', () => expect(invLerp(0, 10, 5)).toBeCloseTo(0.5, 5))
  it('returns 0 when range is zero', () => expect(invLerp(5, 5, 5)).toBe(0))
})

describe('remap', () => {
  it('maps midpoint correctly', () => expect(remap(5, 0, 10, 0, 100)).toBeCloseTo(50, 3))
  it('maps start value', () => expect(remap(0, 0, 10, -1, 1)).toBeCloseTo(-1, 5))
  it('maps end value', () => expect(remap(10, 0, 10, -1, 1)).toBeCloseTo(1, 5))
})

describe('smoothstep', () => {
  it('returns 0 at edge0', () => expect(smoothstep(0, 1, 0)).toBeCloseTo(0, 5))
  it('returns 1 at edge1', () => expect(smoothstep(0, 1, 1)).toBeCloseTo(1, 5))
  it('returns 0.5 at midpoint', () => expect(smoothstep(0, 1, 0.5)).toBeCloseTo(0.5, 5))
  it('clamps below edge0', () => expect(smoothstep(0, 1, -1)).toBeCloseTo(0, 5))
  it('clamps above edge1', () => expect(smoothstep(0, 1, 2)).toBeCloseTo(1, 5))
})

describe('smootherstep', () => {
  it('returns 0 at edge0', () => expect(smootherstep(0, 1, 0)).toBeCloseTo(0, 5))
  it('returns 1 at edge1', () => expect(smootherstep(0, 1, 1)).toBeCloseTo(1, 5))
  it('returns 0.5 at midpoint', () => expect(smootherstep(0, 1, 0.5)).toBeCloseTo(0.5, 5))
})

// All easing functions must return ~0 at t=0 and ~1 at t=1
const easings: [string, (t: number) => number][] = [
  ['easeInQuad', easeInQuad],
  ['easeOutQuad', easeOutQuad],
  ['easeInOutQuad', easeInOutQuad],
  ['easeInCubic', easeInCubic],
  ['easeOutCubic', easeOutCubic],
  ['easeInOutCubic', easeInOutCubic],
  ['easeInSine', easeInSine],
  ['easeOutSine', easeOutSine],
  ['easeInOutSine', easeInOutSine],
  ['easeInExpo', easeInExpo],
  ['easeOutExpo', easeOutExpo],
  ['easeInOutExpo', easeInOutExpo],
  ['easeInElastic', easeInElastic],
  ['easeOutElastic', easeOutElastic],
  ['easeInBounce', easeInBounce],
  ['easeOutBounce', easeOutBounce],
  ['easeInOutBounce', easeInOutBounce],
  ['easeInBack', easeInBack],
  ['easeOutBack', easeOutBack],
]

describe('easing boundary conditions', () => {
  easings.forEach(([name, fn]) => {
    it(`${name}(0) ≈ 0`, () => expect(fn(0)).toBeCloseTo(0, 1))
    it(`${name}(1) ≈ 1`, () => expect(fn(1)).toBeCloseTo(1, 1))
  })
})

describe('ease_out_cubic is monotone', () => {
  it('never decreases', () => {
    Array.from({ length: 100 }, (_, i) => easeOutCubic((i + 1) / 100))
      .reduce((prev, v) => {
        expect(v).toBeGreaterThanOrEqual(prev - EPS)
        return v
      }, 0)
  })
})

describe('easeOutBounce', () => {
  it('stays within [-0.01, 1.01]', () => {
    Array.from({ length: 101 }, (_, i) => easeOutBounce(i / 100))
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(-0.01)
        expect(v).toBeLessThanOrEqual(1.01)
      })
  })
})

// ── lerpClamped ──────────────────────────────────────────────────────────────

describe('lerpClamped', () => {
  it('returns midpoint at t=0.5', () => expect(lerpClamped(0, 10, 0.5)).toBeCloseTo(5, 5))
  it('clamps t above 1', () => expect(lerpClamped(0, 10, 1.5)).toBeCloseTo(10, 5))
  it('clamps t below 0', () => expect(lerpClamped(0, 10, -0.5)).toBeCloseTo(0, 5))
})

// ── repeat ───────────────────────────────────────────────────────────────────

describe('repeat', () => {
  it('wraps positive', () => expect(repeat(2.5, 1)).toBeCloseTo(0.5, 5))
  it('wraps negative', () => expect(repeat(-0.3, 1)).toBeCloseTo(0.7, 5))
  it('zero length returns 0', () => expect(repeat(5, 0)).toBe(0))
})

// ── pingpong ─────────────────────────────────────────────────────────────────

describe('pingpong', () => {
  it('bounces at 0.5', () => expect(pingpong(0.5, 1)).toBeCloseTo(0.5, 5))
  it('bounces at 1.5', () => expect(pingpong(1.5, 1)).toBeCloseTo(0.5, 5))
  it('bounces at 2.5', () => expect(pingpong(2.5, 1)).toBeCloseTo(0.5, 5))
  it('at boundaries', () => {
    expect(pingpong(0, 1)).toBeCloseTo(0, 5)
    expect(pingpong(1, 1)).toBeCloseTo(1, 5)
    expect(pingpong(2, 1)).toBeCloseTo(0, 5)
  })
})

// ── easeInBack / easeOutBack ─────────────────────────────────────────────────

describe('easeInBack', () => {
  it('boundary: f(0) = 0', () => expect(easeInBack(0)).toBeCloseTo(0, 5))
  it('boundary: f(1) = 1', () => expect(easeInBack(1)).toBeCloseTo(1, 5))
  it('undershoots (goes negative)', () => expect(easeInBack(0.2)).toBeLessThan(0))
})

describe('easeOutBack', () => {
  it('boundary: f(0) = 0', () => expect(easeOutBack(0)).toBeCloseTo(0, 5))
  it('boundary: f(1) = 1', () => expect(easeOutBack(1)).toBeCloseTo(1, 5))
  it('overshoots (exceeds 1)', () => expect(easeOutBack(0.8)).toBeGreaterThan(1))
})

// ── Cross-language parity (values verified against Rust prime-interp) ─────────

describe('cross-language parity', () => {
  it('lerp(0, 10, 0.3) matches Rust = 3.0', () =>
    expect(lerp(0, 10, 0.3)).toBeCloseTo(3.0, 5))
  it('invLerp(0, 10, 3) matches Rust = 0.3', () =>
    expect(invLerp(0, 10, 3)).toBeCloseTo(0.3, 5))
  it('smoothstep(0, 1, 0.5) matches Rust = 0.5', () =>
    expect(smoothstep(0, 1, 0.5)).toBeCloseTo(0.5, 5))
  it('smootherstep(0, 1, 0.5) matches Rust = 0.5', () =>
    expect(smootherstep(0, 1, 0.5)).toBeCloseTo(0.5, 5))
  it('easeInQuad(0.5) matches Rust = 0.25', () =>
    expect(easeInQuad(0.5)).toBeCloseTo(0.25, 5))
  it('easeOutQuad(0.5) matches Rust = 0.75', () =>
    expect(easeOutQuad(0.5)).toBeCloseTo(0.75, 5))
  it('easeInCubic(0.5) matches Rust = 0.125', () =>
    expect(easeInCubic(0.5)).toBeCloseTo(0.125, 5))
  it('easeOutCubic(0.5) matches Rust = 0.875', () =>
    expect(easeOutCubic(0.5)).toBeCloseTo(0.875, 5))
})
