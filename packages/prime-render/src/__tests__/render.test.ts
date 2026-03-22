import { describe, it, expect } from 'vitest'
import { render, renderStereo, renderFold } from '../index.js'

const TAU = Math.PI * 2
const EPS = 1e-5

// ── render ────────────────────────────────────────────────────────────────────

describe('render', () => {
  it('empty — numSamples=0 returns []', () => {
    expect(render(0, 44100, 0, (s, _t) => [0, s])).toEqual([])
  })

  it('single sample', () => {
    const [x] = render(0, 44100, 1, (_s, _t) => [0.5, 0])
    expect(x).toBeCloseTo(0.5, 5)
  })

  it('correct length', () => {
    expect(render(0, 44100, 512, (s, _t) => [0, s])).toHaveLength(512)
  })

  it('constant signal — all samples equal', () => {
    const out = render(0, 44100, 100, (s, _t) => [0.25, s])
    expect(out.every(x => Math.abs(x - 0.25) < EPS)).toBe(true)
  })

  it('threads state forward', () => {
    // output[n] = n (state counts up)
    const out = render(0, 44100, 5, (count, _t) => [count, count + 1])
    expect(out).toEqual([0, 1, 2, 3, 4])
  })

  it('time argument is correct — t[n] = n / sampleRate', () => {
    const out = render(0, 10, 4, (_s, t) => [t, 0])
    expect(out[0]).toBeCloseTo(0.0, 5)
    expect(out[1]).toBeCloseTo(0.1, 5)
    expect(out[2]).toBeCloseTo(0.2, 5)
    expect(out[3]).toBeCloseTo(0.3, 5)
  })

  it('deterministic — same inputs produce same output', () => {
    const step = (phase: number, _t: number): [number, number] => [
      Math.sin(phase),
      (phase + TAU * 440 / 44100) % TAU,
    ]
    const a = render(0, 44100, 64, step)
    const b = render(0, 44100, 64, step)
    expect(a).toEqual(b)
  })

  it('440 Hz sine reaches peak near 1.0', () => {
    const out = render(0, 44100, 200, (phase, _t) => [
      Math.sin(phase),
      (phase + TAU * 440 / 44100) % TAU,
    ])
    const peak = Math.max(...out)
    expect(peak).toBeGreaterThan(0.99)
  })

  it('tuple state — (phase, amplitude)', () => {
    const out = render([0, 1] as [number, number], 44100, 4, ([phase, amp], _t) => [
      Math.sin(phase) * amp,
      [(phase + TAU * 440 / 44100) % TAU, amp * 0.999] as [number, number],
    ])
    expect(out).toHaveLength(4)
    expect(out.every(x => Math.abs(x) <= 1)).toBe(true)
  })

  it('cross-language parity — matches Rust render output', () => {
    // Rust: render(0.0_f64, 10, 4, |s, _t| (s as f32, s + 1.0))
    // output: [0.0, 1.0, 2.0, 3.0]
    const out = render(0, 10, 4, (s, _t) => [s, s + 1])
    expect(out[0]).toBeCloseTo(0, 5)
    expect(out[1]).toBeCloseTo(1, 5)
    expect(out[2]).toBeCloseTo(2, 5)
    expect(out[3]).toBeCloseTo(3, 5)
  })
})

// ── renderStereo ──────────────────────────────────────────────────────────────

describe('renderStereo', () => {
  it('empty — returns []', () => {
    expect(renderStereo(0, 44100, 0, (s, _t) => [[0, 0], s])).toEqual([])
  })

  it('correct length', () => {
    expect(renderStereo(0, 44100, 8, (s, _t) => [[0, 0], s])).toHaveLength(8)
  })

  it('constant stereo — all frames equal', () => {
    const frames = renderStereo(0, 44100, 4, (s, _t) => [[0.5, -0.5], s])
    expect(frames.every(([l, r]) => Math.abs(l - 0.5) < EPS && Math.abs(r + 0.5) < EPS)).toBe(true)
  })

  it('threads state forward', () => {
    const frames = renderStereo(0, 44100, 3, (n, _t): [[number, number], number] => [[n, -n], n + 1])
    expect(frames[0][0]).toBeCloseTo(0, 5)
    expect(frames[0][1]).toBeCloseTo(0, 5)
    expect(frames[1]).toEqual([1, -1])
    expect(frames[2]).toEqual([2, -2])
  })

  it('deterministic', () => {
    const step = (phase: number, _t: number): [[number, number], number] => [
      [Math.sin(phase), Math.cos(phase)],
      (phase + TAU * 440 / 44100) % TAU,
    ]
    const a = renderStereo(0, 44100, 32, step)
    const b = renderStereo(0, 44100, 32, step)
    expect(a).toEqual(b)
  })

  it('left and right channels are independent', () => {
    const frames = renderStereo(0, 44100, 4, (n, _t): [[number, number], number] => [
      [n * 0.1, n * -0.2],
      n + 1,
    ])
    const lefts = frames.map(([l]) => l)
    const rights = frames.map(([, r]) => r)
    expect(lefts[0]).toBeCloseTo(0, 5)
    expect(lefts[1]).toBeCloseTo(0.1, 5)
    expect(lefts[2]).toBeCloseTo(0.2, 5)
    expect(lefts[3]).toBeCloseTo(0.3, 5)
    expect(rights[0]).toBeCloseTo(0, 5)
    expect(rights[1]).toBeCloseTo(-0.2, 5)
  })
})

// ── renderFold ────────────────────────────────────────────────────────────────

describe('renderFold', () => {
  it('sum of constant signal', () => {
    const total = renderFold(0, 0, 44100, 100,
      (s, _t) => [0.1, s],
      (acc, x) => acc + x,
    )
    expect(total).toBeCloseTo(10.0, 2)
  })

  it('peak of sine wave', () => {
    const peak = renderFold(0, -Infinity, 44100, 200,
      (phase, _t) => [Math.sin(phase), (phase + TAU * 440 / 44100) % TAU],
      (acc, x) => Math.max(acc, x),
    )
    expect(peak).toBeGreaterThan(0.99)
  })

  it('count equals numSamples', () => {
    const count = renderFold(0, 0, 44100, 77,
      (s, _t) => [0, s],
      (acc) => acc + 1,
    )
    expect(count).toBe(77)
  })

  it('deterministic', () => {
    const step = (phase: number, _t: number): [number, number] => [
      Math.sin(phase),
      (phase + TAU / 10) % TAU,
    ]
    const a = renderFold(0, 0, 44100, 50, step, (acc, x) => acc + x)
    const b = renderFold(0, 0, 44100, 50, step, (acc, x) => acc + x)
    expect(Math.abs(a - b)).toBeLessThan(EPS)
  })

  it('max-fold matches max of render output', () => {
    const step = (phase: number, _t: number): [number, number] => [
      Math.sin(phase),
      (phase + TAU * 220 / 44100) % TAU,
    ]
    const foldMax = renderFold(0, -Infinity, 44100, 100, step, Math.max)
    const renderMax = Math.max(...render(0, 44100, 100, step))
    expect(Math.abs(foldMax - renderMax)).toBeLessThan(EPS)
  })
})
