import { describe, it, expect } from 'vitest'
import {
  lfoSine,
  lfoTriangle,
  lfoSawtooth,
  lfoSquare,
  oscStep,
  adsrStep,
  ADSR_IDLE,
} from '../index.js'

const EPS = 1e-5

// ── lfoSine ───────────────────────────────────────────────────────────────────

describe('lfoSine', () => {
  it('zero phase → 0', () => expect(Math.abs(lfoSine(0))).toBeLessThan(EPS))
  it('quarter phase → 1', () => expect(Math.abs(lfoSine(0.25) - 1)).toBeLessThan(EPS))
  it('half phase → 0', () => expect(Math.abs(lfoSine(0.5))).toBeLessThan(EPS))
  it('three-quarter phase → -1', () => expect(Math.abs(lfoSine(0.75) + 1)).toBeLessThan(EPS))
  it('range [-1, 1]', () =>
    Array.from({ length: 1000 }, (_, i) => lfoSine(i / 1000))
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(-1 - EPS)
        expect(v).toBeLessThanOrEqual(1 + EPS)
      }))
})

// ── lfoTriangle ───────────────────────────────────────────────────────────────

describe('lfoTriangle', () => {
  it('zero phase → 0', () => expect(Math.abs(lfoTriangle(0))).toBeLessThan(EPS))
  it('quarter phase → 1', () => expect(Math.abs(lfoTriangle(0.25) - 1)).toBeLessThan(EPS))
  it('half phase → 0', () => expect(Math.abs(lfoTriangle(0.5))).toBeLessThan(EPS))
  it('three-quarter phase → -1', () => expect(Math.abs(lfoTriangle(0.75) + 1)).toBeLessThan(EPS))
  it('range [-1, 1]', () =>
    Array.from({ length: 1000 }, (_, i) => lfoTriangle(i / 1000))
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(-1 - EPS)
        expect(v).toBeLessThanOrEqual(1 + EPS)
      }))
})

// ── lfoSawtooth ───────────────────────────────────────────────────────────────

describe('lfoSawtooth', () => {
  it('zero phase → -1', () => expect(Math.abs(lfoSawtooth(0) + 1)).toBeLessThan(EPS))
  it('half phase → 0', () => expect(Math.abs(lfoSawtooth(0.5))).toBeLessThan(EPS))
  it('range [-1, 1]', () =>
    Array.from({ length: 1000 }, (_, i) => lfoSawtooth(i / 1000))
      .forEach(v => {
        expect(v).toBeGreaterThanOrEqual(-1 - EPS)
        expect(v).toBeLessThanOrEqual(1 + EPS)
      }))
  it('monotonically increasing within cycle', () =>
    Array.from({ length: 99 }, (_, i) => [lfoSawtooth(i / 100), lfoSawtooth((i + 1) / 100)] as [number, number])
      .forEach(([a, b]) => expect(b).toBeGreaterThan(a - EPS)))
})

// ── lfoSquare ─────────────────────────────────────────────────────────────────

describe('lfoSquare', () => {
  it('first half → +1 (50% duty)', () => expect(lfoSquare(0.1, 0.5)).toBe(1))
  it('second half → -1 (50% duty)', () => expect(lfoSquare(0.6, 0.5)).toBe(-1))
  it('narrow duty: low at 0.15 when duty=0.1', () => expect(lfoSquare(0.15, 0.1)).toBe(-1))
  it('only +1 or -1', () =>
    Array.from({ length: 100 }, (_, i) => lfoSquare(i / 100))
      .forEach(v => expect(Math.abs(Math.abs(v) - 1)).toBeLessThan(EPS)))
})

// ── oscStep ───────────────────────────────────────────────────────────────────

describe('oscStep', () => {
  it('phase advances by freq/sampleRate', () => {
    const [, p1] = oscStep(0, 440, 44100, lfoSine)
    expect(Math.abs(p1 - 440 / 44100)).toBeLessThan(EPS)
  })
  it('phase stays in [0, 1)', () => {
    const [, p1] = oscStep(0.99, 440, 44100, lfoSine)
    expect(p1).toBeGreaterThanOrEqual(0)
    expect(p1).toBeLessThan(1)
  })
  it('deterministic — same inputs same output', () => {
    const [y1] = oscStep(0.25, 440, 44100, lfoSine)
    const [y2] = oscStep(0.25, 440, 44100, lfoSine)
    expect(y1).toBe(y2)
  })
  it('threads forward — generates deterministic sequence', () => {
    const seqA = Array.from({ length: 10 }).reduce(
      ([samples, p]: [number[], number]) => {
        const [y, next] = oscStep(p, 440, 44100, lfoSine)
        return [[...samples, y], next]
      },
      [[], 0] as [number[], number],
    )[0]
    const seqB = Array.from({ length: 10 }).reduce(
      ([samples, p]: [number[], number]) => {
        const [y, next] = oscStep(p, 440, 44100, lfoSine)
        return [[...samples, y], next]
      },
      [[], 0] as [number[], number],
    )[0]
    expect(seqA).toEqual(seqB)
  })
})

// ── adsrStep ──────────────────────────────────────────────────────────────────

describe('adsrStep', () => {
  const params = { attack: 0.1, decay: 0.1, sustain: 0.7, release: 0.2 }

  it('idle with gate off → 0', () => {
    const [v] = adsrStep(ADSR_IDLE, params, false, 0.016)
    expect(v).toBe(0)
  })

  it('gate on from idle → value rises', () => {
    const [v] = adsrStep(ADSR_IDLE, params, true, 0.016)
    expect(v).toBeGreaterThan(0)
  })

  it('reaches sustain level after attack + decay', () => {
    const fastParams = { attack: 0.01, decay: 0.01, sustain: 0.7, release: 0.2 }
    const [v] = Array.from({ length: 500 }).reduce(
      ([, s]: [number, typeof ADSR_IDLE]) => adsrStep(s, fastParams, true, 0.016),
      [0, ADSR_IDLE] as [number, typeof ADSR_IDLE],
    )
    expect(Math.abs(v - 0.7)).toBeLessThan(0.01)
  })

  it('release decays to zero after gate off', () => {
    const fastParams = { attack: 0.01, decay: 0.01, sustain: 0.7, release: 0.1 }
    const [, sustainState] = Array.from({ length: 200 }).reduce(
      ([, s]: [number, typeof ADSR_IDLE]) => adsrStep(s, fastParams, true, 0.016),
      [0, ADSR_IDLE] as [number, typeof ADSR_IDLE],
    )
    const [v] = Array.from({ length: 500 }).reduce(
      ([, s]: [number, typeof ADSR_IDLE]) => adsrStep(s, fastParams, false, 0.016),
      [0, sustainState] as [number, typeof ADSR_IDLE],
    )
    expect(v).toBeLessThan(0.01)
  })

  it('does not mutate params or state', () => {
    const state = { ...ADSR_IDLE }
    const p = { ...params }
    adsrStep(state, p, true, 0.016)
    expect(state).toEqual(ADSR_IDLE)
    expect(p).toEqual(params)
  })

  it('deterministic — same inputs same output', () => {
    const [v1, s1] = adsrStep(ADSR_IDLE, params, true, 0.016)
    const [v2, s2] = adsrStep(ADSR_IDLE, params, true, 0.016)
    expect(v1).toBe(v2)
    expect(s1).toEqual(s2)
  })
})
