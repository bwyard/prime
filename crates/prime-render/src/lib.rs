//! `prime-render` — Pure sample-level scan loop.
//!
//! This is the ADVANCE evaluator for the temporal assembly thesis. A rendered
//! buffer is the result of folding a pure step function over time:
//!
//! ```text
//! output[n] = f(state_n, t_n)   where   state_{n+1} = step(state_n, t_n).sample_rate
//! ```
//!
//! `render` is the only place in PRIME where ADVANCE is the *explicit* design.
//! Everything else in PRIME is LOAD + COMPUTE (single-step). Here we fold N steps.
//!
//! # Temporal Assembly in PRIME
//!
//! ```text
//! LOAD    ← initial_state, sample_rate, num_samples, step fn
//! COMPUTE ← fold: (state, sample_index) → (sample, next_state)
//! APPEND  ← push each sample into the output buffer
//! ADVANCE ← repeat for every sample — this is the scan loop
//! ```
//!
//! The `step` function is always LOAD + COMPUTE only. It never calls `render`
//! recursively, never mutates, never reads the clock. Same inputs → same output.

/// Render a mono audio buffer by folding a pure step function over time.
///
/// This is the core ADVANCE operation for Score's temporal assembly thesis.
/// The output is a deterministic function of `initial_state`, `sample_rate`,
/// and `step` — no hidden state, no I/O, no side effects.
///
/// # Math
///
/// ```text
/// dt = 1 / sample_rate
/// output[n], state_{n+1} = step(state_n, n * dt)
/// ```
///
/// # Arguments
/// * `initial_state` — starting DSP state (e.g. oscillator phase, envelope stage)
/// * `sample_rate`   — samples per second (e.g. 44100)
/// * `num_samples`   — number of output samples to generate
/// * `step`          — pure function: `(state, time_in_seconds) → (sample, next_state)`
///
/// # Returns
/// `Vec<f32>` of exactly `num_samples` mono samples in [-1.0, 1.0].
///
/// # Edge cases
/// * `num_samples == 0` → returns empty `Vec`
/// * `sample_rate == 0` → `dt = infinity`; step receives `f64::INFINITY` for t > 0
///
/// # Example
/// ```rust
/// // Render 4 samples of a 440 Hz sine at 4 Hz (for clarity)
/// use prime_render::render;
/// use std::f64::consts::TAU;
///
/// let samples = render(0.0_f64, 4, 4, |phase, _t| {
///     let sample = phase.sin() as f32;
///     let next = (phase + TAU * 440.0 / 4.0) % TAU;
///     (sample, next)
/// });
/// assert_eq!(samples.len(), 4);
/// ```
pub fn render<S>(
    initial_state: S,
    sample_rate: u32,
    num_samples: usize,
    step: impl Fn(S, f64) -> (f32, S),
) -> Vec<f32> {
    let dt = 1.0 / sample_rate as f64;
    let (_, samples) = (0..num_samples).fold(
        (initial_state, Vec::with_capacity(num_samples)),
        |(state, mut buf), n| {
            let t = n as f64 * dt;
            let (sample, next_state) = step(state, t);
            buf.push(sample);
            (next_state, buf)
        },
    );
    samples
}

/// Render a stereo audio buffer by folding a pure step function over time.
///
/// Identical to [`render`] but the step function produces a `(left, right)`
/// sample pair per frame. Returns interleaved `[(L0, R0), (L1, R1), ...]`.
///
/// # Arguments
/// * `initial_state` — starting DSP state
/// * `sample_rate`   — samples per second
/// * `num_samples`   — number of stereo frames to generate
/// * `step`          — pure function: `(state, t) → ((left, right), next_state)`
///
/// # Returns
/// `Vec<(f32, f32)>` of `num_samples` stereo frames.
///
/// # Example
/// ```rust
/// use prime_render::render_stereo;
///
/// // Constant stereo frame — left=0.5, right=-0.5
/// let frames = render_stereo((), 44100, 3, |s, _t| ((0.5, -0.5), s));
/// assert_eq!(frames, vec![(0.5, -0.5), (0.5, -0.5), (0.5, -0.5)]);
/// ```
pub fn render_stereo<S>(
    initial_state: S,
    sample_rate: u32,
    num_samples: usize,
    step: impl Fn(S, f64) -> ((f32, f32), S),
) -> Vec<(f32, f32)> {
    let dt = 1.0 / sample_rate as f64;
    let (_, frames) = (0..num_samples).fold(
        (initial_state, Vec::with_capacity(num_samples)),
        |(state, mut buf), n| {
            let t = n as f64 * dt;
            let (frame, next_state) = step(state, t);
            buf.push(frame);
            (next_state, buf)
        },
    );
    frames
}

/// Render a buffer and reduce it to a scalar — useful for envelope integration
/// and level metering without allocating the full output.
///
/// Folds the same scan loop as [`render`] but accumulates into a single value
/// `A` instead of collecting samples. The `combine` function receives the
/// running accumulator and each new sample.
///
/// # Arguments
/// * `initial_state` — starting DSP state
/// * `initial_acc`   — starting accumulator value
/// * `sample_rate`   — samples per second
/// * `num_samples`   — number of samples to process
/// * `step`          — `(state, t) → (sample, next_state)`
/// * `combine`       — `(acc, sample) → next_acc`
///
/// # Returns
/// Final accumulated value of type `A`.
///
/// # Example
/// ```rust
/// use prime_render::render_fold;
///
/// // Sum all samples from a constant signal of 0.1
/// let total = render_fold((), 0.0_f32, 44100, 100, |s, _t| (0.1_f32, s), |acc, x| acc + x);
/// assert!((total - 10.0).abs() < 1e-4);
/// ```
pub fn render_fold<S, A>(
    initial_state: S,
    initial_acc: A,
    sample_rate: u32,
    num_samples: usize,
    step: impl Fn(S, f64) -> (f32, S),
    combine: impl Fn(A, f32) -> A,
) -> A {
    let dt = 1.0 / sample_rate as f64;
    let (_, acc) = (0..num_samples).fold(
        (initial_state, initial_acc),
        |(state, acc), n| {
            let t = n as f64 * dt;
            let (sample, next_state) = step(state, t);
            (next_state, combine(acc, sample))
        },
    );
    acc
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const EPSILON: f32 = 1e-5;

    // ── render ────────────────────────────────────────────────────────────────

    #[test]
    fn render_empty() {
        let out = render(0.0_f64, 44100, 0, |s, _t| (0.0, s));
        assert!(out.is_empty());
    }

    #[test]
    fn render_single_sample() {
        let out = render(0.0_f64, 44100, 1, |s, _t| (0.5_f32, s));
        assert_eq!(out.len(), 1);
        assert!((out[0] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn render_constant_signal() {
        let out = render((), 44100, 100, |s, _t| (0.25_f32, s));
        assert_eq!(out.len(), 100);
        assert!(out.iter().all(|&x| (x - 0.25).abs() < EPSILON));
    }

    #[test]
    fn render_correct_length() {
        let n = 512;
        let out = render(0_u32, 44100, n, |s, _t| (0.0, s + 1));
        assert_eq!(out.len(), n);
    }

    #[test]
    fn render_threads_state_forward() {
        // State should accumulate: output[n] should equal n as f32
        let out = render(0_u32, 44100, 5, |count, _t| (count as f32, count + 1));
        assert_eq!(out, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn render_time_argument_correct() {
        // At sr=10, dt=0.1: t[0]=0.0, t[1]=0.1, t[2]=0.2 ...
        let out = render((), 10, 4, |s, t| (t as f32, s));
        assert!((out[0] - 0.0).abs() < EPSILON);
        assert!((out[1] - 0.1).abs() < EPSILON);
        assert!((out[2] - 0.2).abs() < EPSILON);
        assert!((out[3] - 0.3).abs() < EPSILON);
    }

    #[test]
    fn render_deterministic() {
        let step = |phase: f64, _t: f64| {
            let s = phase.sin() as f32;
            let next = (phase + TAU * 440.0 / 44100.0) % TAU;
            (s, next)
        };
        let a = render(0.0_f64, 44100, 64, step);
        let b = render(0.0_f64, 44100, 64, step);
        assert_eq!(a, b);
    }

    #[test]
    fn render_sine_440hz_peak() {
        // 440 Hz at 44100 sr — peak should be close to 1.0 somewhere in first period
        let out = render(0.0_f64, 44100, 200, |phase, _t| {
            let s = phase.sin() as f32;
            let next = (phase + TAU * 440.0 / 44100.0) % TAU;
            (s, next)
        });
        let peak = out.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(peak > 0.99, "peak={}", peak);
    }

    #[test]
    fn render_no_mutation_of_input_state() {
        let initial = 42_u32;
        render(initial, 44100, 10, |s, _t| (0.0, s + 1));
        // initial is Copy — verify it was not mutated (it can't be in Rust, but
        // this confirms the API takes ownership and threads state correctly)
        assert_eq!(initial, 42);
    }

    // ── render_stereo ─────────────────────────────────────────────────────────

    #[test]
    fn render_stereo_empty() {
        let out = render_stereo((), 44100, 0, |s, _t| ((0.0, 0.0), s));
        assert!(out.is_empty());
    }

    #[test]
    fn render_stereo_constant() {
        let out = render_stereo((), 44100, 4, |s, _t| ((0.5_f32, -0.5_f32), s));
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|&(l, r)| (l - 0.5).abs() < EPSILON && (r + 0.5).abs() < EPSILON));
    }

    #[test]
    fn render_stereo_threads_state() {
        let out = render_stereo(0_i32, 44100, 3, |n, _t| ((n as f32, -(n as f32)), n + 1));
        assert_eq!(out, vec![(0.0, 0.0), (1.0, -1.0), (2.0, -2.0)]);
    }

    #[test]
    fn render_stereo_deterministic() {
        let step = |phase: f64, _t: f64| {
            let l = phase.sin() as f32;
            let r = (phase + std::f64::consts::FRAC_PI_2).sin() as f32;
            let next = (phase + TAU * 440.0 / 44100.0) % TAU;
            ((l, r), next)
        };
        let a = render_stereo(0.0_f64, 44100, 32, step);
        let b = render_stereo(0.0_f64, 44100, 32, step);
        assert_eq!(a, b);
    }

    // ── render_fold ───────────────────────────────────────────────────────────

    #[test]
    fn render_fold_sum() {
        // 100 samples of 0.1 → sum = 10.0
        let total = render_fold((), 0.0_f32, 44100, 100,
            |s, _t| (0.1_f32, s),
            |acc, x| acc + x,
        );
        assert!((total - 10.0).abs() < 1e-3, "total={}", total);
    }

    #[test]
    fn render_fold_max() {
        let step = |phase: f64, _t: f64| {
            let s = phase.sin() as f32;
            let next = (phase + TAU * 440.0 / 44100.0) % TAU;
            (s, next)
        };
        let peak = render_fold(0.0_f64, f32::NEG_INFINITY, 44100, 200, step,
            f32::max,
        );
        assert!(peak > 0.99, "peak={}", peak);
    }

    #[test]
    fn render_fold_count() {
        let count = render_fold((), 0_usize, 44100, 77,
            |s, _t| (0.0, s),
            |acc, _x| acc + 1,
        );
        assert_eq!(count, 77);
    }

    #[test]
    fn render_fold_deterministic() {
        let step = |phase: f64, _t: f64| {
            let s = phase.sin() as f32;
            let next = (phase + TAU / 10.0) % TAU;
            (s, next)
        };
        let a = render_fold(0.0_f64, 0.0_f32, 44100, 50, step, |acc, x| acc + x);
        let b = render_fold(0.0_f64, 0.0_f32, 44100, 50, step, |acc, x| acc + x);
        assert!((a - b).abs() < EPSILON);
    }

    // ── render + osc integration ──────────────────────────────────────────────

    #[test]
    fn render_integrates_with_tuple_state() {
        // Multi-field state as a tuple — (phase, amplitude)
        let out = render((0.0_f64, 1.0_f32), 44100, 4, |(phase, amp), _t| {
            let s = (phase.sin() as f32) * amp;
            let next_phase = (phase + TAU * 440.0 / 44100.0) % TAU;
            let next_amp = amp * 0.999;
            (s, (next_phase, next_amp))
        });
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|x| x.abs() <= 1.0));
    }
}
