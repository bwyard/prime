/**
 * prime-osc — Oscillators and envelopes.
 *
 * All public functions are LOAD + COMPUTE. No STORE. No JUMP.
 * Phase threads forward as an explicit value — same inputs → same output.
 */

// ── LFO shape functions ──────────────────────────────────────────────────────

/**
 * Sine wave at normalised phase.
 *
 * @param phase - Normalised phase in [0, 1). Wraps automatically.
 * @returns Value in [-1, 1].
 *
 * @example
 * lfoSine(0.25) // ≈ 1.0 — quarter cycle
 */
export const lfoSine = (phase: number): number =>
  Math.sin(phase * Math.PI * 2)

/**
 * Triangle wave at normalised phase.
 *
 * @remarks
 *   p = frac(phase),  y = 2 × |2p − 1| − 1
 *
 * @param phase - Normalised phase in [0, 1).
 * @returns Value in [-1, 1]. Peaks at phase=0.25, troughs at phase=0.75.
 *
 * @example
 * lfoTriangle(0.25) // 1.0 — peak
 */
export const lfoTriangle = (phase: number): number => {
  const p = (phase + 0.75) - Math.floor(phase + 0.75)
  return 2 * Math.abs(2 * p - 1) - 1
}

/**
 * Sawtooth wave (rising) at normalised phase.
 *
 * @remarks
 *   y = 2 × frac(phase) − 1
 *
 * @param phase - Normalised phase in [0, 1).
 * @returns Value in [-1, 1).
 *
 * @example
 * lfoSawtooth(0.0) // -1.0 — start of cycle
 */
export const lfoSawtooth = (phase: number): number => {
  const p = phase - Math.floor(phase)
  return 2 * p - 1
}

/**
 * Square wave at normalised phase.
 *
 * @remarks
 *   y = +1 if frac(phase) < width, else -1
 *
 * @param phase - Normalised phase in [0, 1).
 * @param width - Duty cycle in (0, 1). Default 0.5.
 * @returns Value in {-1, +1}.
 *
 * @example
 * lfoSquare(0.1, 0.5) // 1.0 — first half of cycle
 */
export const lfoSquare = (phase: number, width = 0.5): number => {
  const p = phase - Math.floor(phase)
  const w = Math.min(0.999, Math.max(0.001, width))
  return p < w ? 1 : -1
}

// ── Oscillator step ──────────────────────────────────────────────────────────

/**
 * Advance oscillator phase by one sample — APPEND pattern.
 *
 * @remarks
 *   new_phase = frac(phase + freq / sampleRate)
 *   y = shape(new_phase)
 *
 * @param phase - Current phase in [0, 1).
 * @param freq - Frequency in Hz.
 * @param sampleRate - Sample rate in Hz.
 * @param shape - Shape function: lfoSine, lfoTriangle, etc.
 * @returns `[sample, newPhase]` — thread `newPhase` into the next call.
 *
 * @example
 * const [y, p1] = oscStep(0, 440, 44100, lfoSine)
 * const [y2, p2] = oscStep(p1, 440, 44100, lfoSine)
 */
export const oscStep = (
  phase: number,
  freq: number,
  sampleRate: number,
  shape: (phase: number) => number,
): [number, number] => {
  const sr = Math.max(sampleRate, 1)
  const newPhase = (phase + freq / sr) % 1
  return [shape(newPhase), newPhase]
}

// ── ADSR envelope ────────────────────────────────────────────────────────────

/** ADSR stage identifier. */
export type AdsrStage = 'attack' | 'decay' | 'sustain' | 'release' | 'done'

/** ADSR envelope parameters. All times in seconds. */
export type AdsrParams = {
  /** Attack time: 0 → 1 */
  readonly attack: number
  /** Decay time: 1 → sustain */
  readonly decay: number
  /** Sustain level [0, 1] */
  readonly sustain: number
  /** Release time: sustain → 0 */
  readonly release: number
}

/** ADSR envelope state. Thread this forward with each call to adsrStep. */
export type AdsrState = {
  readonly stage: AdsrStage
  /** Current envelope value in [0, 1]. */
  readonly value: number
  /** Time elapsed in current stage (seconds). */
  readonly elapsed: number
}

/** Initial envelope state — at rest. */
export const ADSR_IDLE: AdsrState = { stage: 'done', value: 0, elapsed: 0 }

/**
 * Advance ADSR envelope by one time step — pure LOAD + COMPUTE.
 *
 * @remarks
 *   Attack:  value rises 0 → 1 over attack seconds
 *   Decay:   value falls 1 → sustain over decay seconds
 *   Sustain: value holds at sustain while gate is on
 *   Release: value falls from current → 0 over release seconds
 *   Done:    value = 0
 *
 * @param state - Current envelope state.
 * @param params - ADSR parameters.
 * @param gate - true = note on, false = note off (trigger release).
 * @param dt - Delta time in seconds.
 * @returns `[value, newState]` — thread `newState` forward.
 *
 * @example
 * const params = { attack: 0.1, decay: 0.1, sustain: 0.7, release: 0.2 }
 * const [v, s1] = adsrStep(ADSR_IDLE, params, true, 0.016)
 */
export const adsrStep = (
  state: AdsrState,
  params: AdsrParams,
  gate: boolean,
  dt: number,
): [number, AdsrState] => {
  const attack = Math.max(params.attack, 1e-4)
  const decay = Math.max(params.decay, 1e-4)
  const release = Math.max(params.release, 1e-4)
  const sustain = Math.min(1, Math.max(0, params.sustain))

  if (!gate) {
    if (state.stage === 'done') return [0, ADSR_IDLE]
    if (state.stage === 'release') {
      const t = Math.min(1, state.elapsed / release)
      const newElapsed = state.elapsed + dt
      if (newElapsed >= release) return [0, ADSR_IDLE]
      return [state.value * (1 - t), { stage: 'release', value: state.value, elapsed: newElapsed }]
    }
    return [state.value, { stage: 'release', value: state.value, elapsed: 0 }]
  }

  if (state.stage === 'done' || state.stage === 'release') {
    const newElapsed = dt
    const newVal = Math.min(1, newElapsed / attack)
    if (newElapsed >= attack) return [1, { stage: 'decay', value: 1, elapsed: 0 }]
    return [newVal, { stage: 'attack', value: newVal, elapsed: newElapsed }]
  }

  if (state.stage === 'attack') {
    const newElapsed = state.elapsed + dt
    const newVal = Math.min(1, newElapsed / attack)
    if (newElapsed >= attack) return [1, { stage: 'decay', value: 1, elapsed: 0 }]
    return [newVal, { stage: 'attack', value: newVal, elapsed: newElapsed }]
  }

  if (state.stage === 'decay') {
    const newElapsed = state.elapsed + dt
    const t = Math.min(1, newElapsed / decay)
    const newVal = 1 + (sustain - 1) * t
    if (newElapsed >= decay) return [sustain, { stage: 'sustain', value: sustain, elapsed: 0 }]
    return [newVal, { stage: 'decay', value: newVal, elapsed: newElapsed }]
  }

  return [sustain, { stage: 'sustain', value: sustain, elapsed: state.elapsed + dt }]
}
