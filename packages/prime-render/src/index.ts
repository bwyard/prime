/**
 * prime-render — Pure sample-level scan loop.
 *
 * A rendered buffer is the result of folding a pure step function over time:
 *
 *   output[n] = f(state_n, t_n)   where   state_{n+1} = step(state_n, t_n).sampleRate
 *
 * `render` is the only place in PRIME where ADVANCE is the *explicit* design.
 * Everything else in PRIME is LOAD + COMPUTE (single-step). Here we fold N steps.
 *
 * Pattern:
 *   LOAD    ← initialState, sampleRate, numSamples, step fn
 *   COMPUTE ← fold: (state, n) → (sample, nextState)
 *   APPEND  ← collect each sample into output
 *   ADVANCE ← repeat for every sample — this is the scan loop
 *
 * All exported functions are pure (LOAD + COMPUTE + ADVANCE only).
 * No mutation, no side effects, no hidden state. Zero `let`.
 */

/**
 * Render a mono audio buffer by folding a pure step function over time.
 *
 * The output is a deterministic function of `initialState`, `sampleRate`,
 * and `step` — no hidden state, no I/O, no side effects.
 *
 * Math:
 *   dt = 1 / sampleRate
 *   output[n], state_{n+1} = step(state_n, n * dt)
 *
 * @param initialState  - Starting DSP state (oscillator phase, envelope stage, etc.)
 * @param sampleRate    - Samples per second (e.g. 44100)
 * @param numSamples    - Number of output samples to generate
 * @param step          - Pure function: `(state, timeInSeconds) → [sample, nextState]`
 * @returns             - Array of `numSamples` mono samples
 *
 * @example
 * // Render 4 samples of a 440 Hz sine at 44100 sr
 * const TAU = Math.PI * 2
 * const samples = render(0, 44100, 4, (phase, _t) => [
 *   Math.sin(phase),
 *   (phase + TAU * 440 / 44100) % TAU,
 * ])
 */
export const render = <S>(
  initialState: S,
  sampleRate: number,
  numSamples: number,
  step: (state: S, t: number) => [number, S],
): number[] => {
  const dt = 1 / sampleRate
  const [, , samples] = Array.from<null>({ length: numSamples }).reduce(
    ([state, n, buf]: [S, number, number[]]): [S, number, number[]] => {
      const [sample, nextState] = step(state, n * dt)
      return [nextState, n + 1, [...buf, sample]]
    },
    [initialState, 0, []] as [S, number, number[]],
  )
  return samples
}

/**
 * Render a stereo audio buffer by folding a pure step function over time.
 *
 * Identical to `render` but the step function produces a `[left, right]`
 * sample pair per frame.
 *
 * @param initialState  - Starting DSP state
 * @param sampleRate    - Samples per second
 * @param numSamples    - Number of stereo frames to generate
 * @param step          - Pure function: `(state, t) → [[left, right], nextState]`
 * @returns             - Array of `[left, right]` stereo frame pairs
 *
 * @example
 * const frames = renderStereo(0, 44100, 3, (phase, _t) => [
 *   [Math.sin(phase), Math.cos(phase)],
 *   (phase + Math.PI * 2 * 440 / 44100) % (Math.PI * 2),
 * ])
 */
export const renderStereo = <S>(
  initialState: S,
  sampleRate: number,
  numSamples: number,
  step: (state: S, t: number) => [[number, number], S],
): [number, number][] => {
  const dt = 1 / sampleRate
  const [, , frames] = Array.from<null>({ length: numSamples }).reduce(
    ([state, n, buf]: [S, number, [number, number][]]): [S, number, [number, number][]] => {
      const [frame, nextState] = step(state, n * dt)
      return [nextState, n + 1, [...buf, frame]]
    },
    [initialState, 0, []] as [S, number, [number, number][]],
  )
  return frames
}

/**
 * Render a buffer and reduce it to a scalar — useful for envelope integration
 * and level metering without allocating the full output array.
 *
 * @param initialState  - Starting DSP state
 * @param initialAcc    - Starting accumulator value
 * @param sampleRate    - Samples per second
 * @param numSamples    - Number of samples to process
 * @param step          - `(state, t) → [sample, nextState]`
 * @param combine       - `(acc, sample) → nextAcc`
 * @returns             - Final accumulated value
 *
 * @example
 * // Sum all samples from a constant signal of 0.1
 * const total = renderFold(null, 0, 44100, 100,
 *   (_s, _t) => [0.1, null],
 *   (acc, x) => acc + x,
 * )
 * // total ≈ 10.0
 */
export const renderFold = <S, A>(
  initialState: S,
  initialAcc: A,
  sampleRate: number,
  numSamples: number,
  step: (state: S, t: number) => [number, S],
  combine: (acc: A, sample: number) => A,
): A => {
  const dt = 1 / sampleRate
  const [, , acc] = Array.from<null>({ length: numSamples }).reduce(
    ([state, n, acc]: [S, number, A]): [S, number, A] => {
      const [sample, nextState] = step(state, n * dt)
      return [nextState, n + 1, combine(acc, sample)]
    },
    [initialState, 0, initialAcc] as [S, number, A],
  )
  return acc
}
