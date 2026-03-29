//! prime-osc — Oscillators and envelopes.
//!
//! All public functions are LOAD + COMPUTE. No STORE. No JUMP.
//! State threads forward as an explicit parameter. Same inputs → same output.

use std::f32::consts::TAU;

// ── LFO shape functions ──────────────────────────────────────────────────────

/// Sine wave at normalised phase.
///
/// # Math
///   y = sin(phase × 2π)
///
/// # Arguments
/// * `phase` - Normalised phase in [0, 1). Wraps automatically.
///
/// # Returns
/// Value in [-1, 1].
///
/// # Example
/// ```rust
/// let y = prime_osc::lfo_sine(0.25); // ≈ 1.0 — quarter cycle
/// assert!((y - 1.0).abs() < 1e-5);
/// ```
pub fn lfo_sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

/// Triangle wave at normalised phase. Phase-aligned with lfo_sine (peak at 0.25).
///
/// # Math
///   p = frac(phase + 0.75)
///   y = 2 × |2p − 1| − 1
///
/// # Arguments
/// * `phase` - Normalised phase in [0, 1). Wraps automatically.
///
/// # Returns
/// Value in [-1, 1]. Zero at phase=0, peaks at phase=0.25, troughs at phase=0.75.
///
/// # Example
/// ```rust
/// let y = prime_osc::lfo_triangle(0.25); // 1.0 — peak
/// assert!((y - 1.0).abs() < 1e-5);
/// ```
pub fn lfo_triangle(phase: f32) -> f32 {
    let p = (phase + 0.75) - (phase + 0.75).floor();
    2.0 * (2.0 * p - 1.0).abs() - 1.0
}

/// Sawtooth wave at normalised phase (rising).
///
/// # Math
///   y = 2 × frac(phase) − 1
///
/// # Arguments
/// * `phase` - Normalised phase in [0, 1). Wraps automatically.
///
/// # Returns
/// Value in [-1, 1). Rises from -1 to +1, resets at each cycle.
///
/// # Example
/// ```rust
/// let y = prime_osc::lfo_sawtooth(0.0); // -1.0 — start of cycle
/// assert!((y + 1.0).abs() < 1e-5);
/// ```
pub fn lfo_sawtooth(phase: f32) -> f32 {
    let p = phase - phase.floor();
    2.0 * p - 1.0
}

/// Cosine LFO. `phase` in [0, 1] maps to one full cycle.
///
/// # Math
///   y = cos(phase × 2π)
///
/// # Example
/// ```rust
/// # use prime_osc::lfo_cosine;
/// assert!((lfo_cosine(0.0) - 1.0).abs() < 1e-5);
/// assert!((lfo_cosine(0.5) - (-1.0)).abs() < 1e-5);
/// ```
pub fn lfo_cosine(phase: f32) -> f32 {
    (phase * TAU).cos()
}

/// Square wave at normalised phase.
///
/// # Math
///   y = +1 if frac(phase) < width, else -1
///
/// # Arguments
/// * `phase` - Normalised phase in [0, 1).
/// * `width` - Duty cycle in (0, 1). 0.5 = 50% square. Clamped to (0.001, 0.999).
///
/// # Returns
/// Value in {-1, +1}.
///
/// # Edge cases
/// * `width=0.5` → symmetric square wave
///
/// # Example
/// ```rust
/// let y = prime_osc::lfo_square(0.1, 0.5); // 1.0 — first half of cycle
/// assert!((y - 1.0).abs() < 1e-5);
/// ```
pub fn lfo_square(phase: f32, width: f32) -> f32 {
    let p = phase - phase.floor();
    let w = width.clamp(0.001, 0.999);
    if p < w { 1.0 } else { -1.0 }
}

// ── Oscillator step ──────────────────────────────────────────────────────────

/// Advance oscillator phase by one sample and return (sample, new_phase).
///
/// # Math
///   new_phase = frac(phase + freq / sample_rate)
///   y = shape(new_phase)
///
/// # Arguments
/// * `phase` - Current phase in [0, 1).
/// * `freq` - Frequency in Hz.
/// * `sample_rate` - Sample rate in Hz (e.g. 44100.0).
/// * `shape` - Shape function — lfo_sine, lfo_triangle, etc.
///
/// # Returns
/// `(sample, new_phase)` — thread `new_phase` into the next call.
///
/// # Example
/// ```rust
/// let (y, next) = prime_osc::osc_step(0.0, 440.0, 44100.0, prime_osc::lfo_sine);
/// ```
pub fn osc_step(phase: f32, freq: f32, sample_rate: f32, shape: fn(f32) -> f32) -> (f32, f32) {
    let sr = sample_rate.max(1.0);
    let new_phase = (phase + freq / sr).fract();
    (shape(new_phase), new_phase)
}

// ── ADSR envelope ────────────────────────────────────────────────────────────

/// ADSR envelope parameters. All times in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdsrParams {
    /// Attack time (0 → peak in seconds)
    pub attack: f32,
    /// Decay time (peak → sustain in seconds)
    pub decay: f32,
    /// Sustain level [0, 1]
    pub sustain: f32,
    /// Release time (sustain → 0 in seconds)
    pub release: f32,
}

/// ADSR envelope stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdsrStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

/// ADSR envelope state. Thread forward with each call to `adsr_step`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdsrState {
    /// Current envelope stage.
    pub stage: AdsrStage,
    /// Current envelope value in [0, 1].
    pub value: f32,
    /// Time elapsed in the current stage (seconds).
    pub elapsed: f32,
}

impl AdsrState {
    /// Initial state — envelope at rest (gate off).
    pub const IDLE: Self = Self {
        stage: AdsrStage::Done,
        value: 0.0,
        elapsed: 0.0,
    };
}

/// Advance ADSR envelope by one time step — pure LOAD + COMPUTE.
///
/// # Math
///   Attack:  value += dt / attack
///   Decay:   value = lerp(1, sustain, elapsed / decay)
///   Sustain: value = sustain
///   Release: value = sustain_at_release × (1 − elapsed / release)
///   Done:    value = 0
///
/// # Arguments
/// * `state` - Current envelope state.
/// * `params` - ADSR parameters.
/// * `gate` - true = note on, false = note off (trigger release).
/// * `dt` - Delta time in seconds.
///
/// # Returns
/// `(value, new_state)` — thread `new_state` forward.
///
/// # Edge cases
/// * Gate goes false mid-attack → transitions immediately to release.
/// * Zero attack time → jumps directly to decay in one step.
///
/// # Example
/// ```rust
/// use prime_osc::{AdsrState, AdsrParams, adsr_step};
/// let params = AdsrParams { attack: 0.1, decay: 0.1, sustain: 0.7, release: 0.2 };
/// let (v, s1) = adsr_step(AdsrState::IDLE, &params, true, 0.016);
/// assert!(v > 0.0);
/// ```
pub fn adsr_step(state: AdsrState, params: &AdsrParams, gate: bool, dt: f32) -> (f32, AdsrState) {
    let attack = params.attack.max(1e-4);
    let decay = params.decay.max(1e-4);
    let release = params.release.max(1e-4);
    let sustain = params.sustain.clamp(0.0, 1.0);

    match (state.stage, gate) {
        // Gate off → begin or continue release from current level
        (AdsrStage::Done, false) => (0.0, AdsrState::IDLE),

        (_, false) if state.stage != AdsrStage::Release && state.stage != AdsrStage::Done => {
            // Transition into release — preserve current value as start point
            let new_state = AdsrState {
                stage: AdsrStage::Release,
                value: state.value,
                elapsed: 0.0,
            };
            (state.value, new_state)
        }

        (AdsrStage::Release, false) => {
            let t = (state.elapsed / release).min(1.0);
            let new_val = state.value * (1.0 - t);
            let new_elapsed = state.elapsed + dt;
            if new_elapsed >= release {
                (0.0, AdsrState::IDLE)
            } else {
                (new_val, AdsrState { stage: AdsrStage::Release, value: state.value, elapsed: new_elapsed })
            }
        }

        // Gate on
        (AdsrStage::Done, true) | (AdsrStage::Release, true) => {
            // (Re-)trigger: restart from attack
            let new_elapsed = state.elapsed + dt;
            let new_val = (new_elapsed / attack).min(1.0);
            if new_elapsed >= attack {
                (1.0, AdsrState { stage: AdsrStage::Decay, value: 1.0, elapsed: 0.0 })
            } else {
                (new_val, AdsrState { stage: AdsrStage::Attack, value: new_val, elapsed: new_elapsed })
            }
        }

        (AdsrStage::Attack, true) => {
            let new_elapsed = state.elapsed + dt;
            let new_val = (new_elapsed / attack).min(1.0);
            if new_elapsed >= attack {
                (1.0, AdsrState { stage: AdsrStage::Decay, value: 1.0, elapsed: 0.0 })
            } else {
                (new_val, AdsrState { stage: AdsrStage::Attack, value: new_val, elapsed: new_elapsed })
            }
        }

        (AdsrStage::Decay, true) => {
            let new_elapsed = state.elapsed + dt;
            let t = (new_elapsed / decay).min(1.0);
            let new_val = 1.0 + (sustain - 1.0) * t;
            if new_elapsed >= decay {
                (sustain, AdsrState { stage: AdsrStage::Sustain, value: sustain, elapsed: 0.0 })
            } else {
                (new_val, AdsrState { stage: AdsrStage::Decay, value: new_val, elapsed: new_elapsed })
            }
        }

        (AdsrStage::Sustain, true) => {
            (sustain, AdsrState { stage: AdsrStage::Sustain, value: sustain, elapsed: state.elapsed + dt })
        }

        // Catch-all (shouldn't be reachable)
        _ => (0.0, AdsrState::IDLE),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f32 = 1e-5;

    // lfo_sine
    #[test]
    fn sine_zero_phase() { assert!((lfo_sine(0.0)).abs() < EPS); }
    #[test]
    fn sine_quarter_phase() { assert!((lfo_sine(0.25) - 1.0).abs() < EPS); }
    #[test]
    fn sine_half_phase() { assert!((lfo_sine(0.5)).abs() < EPS); }
    #[test]
    fn sine_three_quarter_phase() { assert!((lfo_sine(0.75) + 1.0).abs() < EPS); }
    #[test]
    fn sine_range() {
        (0..1000).map(|i| lfo_sine(i as f32 / 1000.0))
            .for_each(|v| { assert!(v >= -1.0 - EPS && v <= 1.0 + EPS); });
    }

    // lfo_triangle
    #[test]
    fn triangle_zero() { assert!((lfo_triangle(0.0)).abs() < EPS); }
    #[test]
    fn triangle_quarter() { assert!((lfo_triangle(0.25) - 1.0).abs() < EPS); }
    #[test]
    fn triangle_half() { assert!((lfo_triangle(0.5)).abs() < EPS); }
    #[test]
    fn triangle_three_quarter() { assert!((lfo_triangle(0.75) + 1.0).abs() < EPS); }
    #[test]
    fn triangle_range() {
        (0..1000).map(|i| lfo_triangle(i as f32 / 1000.0))
            .for_each(|v| { assert!(v >= -1.0 - EPS && v <= 1.0 + EPS); });
    }

    // lfo_sawtooth
    #[test]
    fn sawtooth_zero() { assert!((lfo_sawtooth(0.0) + 1.0).abs() < EPS); }
    #[test]
    fn sawtooth_half() { assert!((lfo_sawtooth(0.5)).abs() < EPS); }
    #[test]
    fn sawtooth_range() {
        (0..1000).map(|i| lfo_sawtooth(i as f32 / 1000.0))
            .for_each(|v| { assert!(v >= -1.0 - EPS && v <= 1.0 + EPS); });
    }

    // lfo_cosine
    #[test]
    fn cosine_zero_phase() { assert!((lfo_cosine(0.0) - 1.0).abs() < EPS); }
    #[test]
    fn cosine_quarter_phase() { assert!((lfo_cosine(0.25)).abs() < EPS); }
    #[test]
    fn cosine_half_phase() { assert!((lfo_cosine(0.5) + 1.0).abs() < EPS); }
    #[test]
    fn cosine_three_quarter_phase() { assert!((lfo_cosine(0.75)).abs() < EPS); }
    #[test]
    fn cosine_range() {
        (0..1000).map(|i| lfo_cosine(i as f32 / 1000.0))
            .for_each(|v| { assert!(v >= -1.0 - EPS && v <= 1.0 + EPS); });
    }

    // lfo_square
    #[test]
    fn square_first_half() { assert!((lfo_square(0.1, 0.5) - 1.0).abs() < EPS); }
    #[test]
    fn square_second_half() { assert!((lfo_square(0.6, 0.5) + 1.0).abs() < EPS); }
    #[test]
    fn square_narrow_duty() {
        // 10% duty: phase 0.05 → high, phase 0.15 → low
        assert!((lfo_square(0.05, 0.1) - 1.0).abs() < EPS);
        assert!((lfo_square(0.15, 0.1) + 1.0).abs() < EPS);
    }

    // osc_step
    #[test]
    fn osc_phase_advances() {
        let (_, p1) = osc_step(0.0, 440.0, 44100.0, lfo_sine);
        assert!((p1 - 440.0 / 44100.0).abs() < EPS);
    }
    #[test]
    fn osc_phase_wraps() {
        let (_, p1) = osc_step(0.99, 440.0, 44100.0, lfo_sine);
        assert!(p1 < 1.0);
    }
    #[test]
    fn osc_deterministic() {
        let (y1, _) = osc_step(0.25, 440.0, 44100.0, lfo_sine);
        let (y2, _) = osc_step(0.25, 440.0, 44100.0, lfo_sine);
        assert_eq!(y1, y2);
    }

    // adsr_step
    #[test]
    fn adsr_starts_at_zero() {
        let params = AdsrParams { attack: 0.1, decay: 0.1, sustain: 0.7, release: 0.2 };
        let (v, _) = adsr_step(AdsrState::IDLE, &params, false, 0.016);
        assert!((v).abs() < EPS);
    }
    #[test]
    fn adsr_gate_on_rises() {
        let params = AdsrParams { attack: 0.1, decay: 0.1, sustain: 0.7, release: 0.2 };
        let (v, _) = adsr_step(AdsrState::IDLE, &params, true, 0.016);
        assert!(v > 0.0);
    }
    #[test]
    fn adsr_reaches_sustain() {
        let params = AdsrParams { attack: 0.01, decay: 0.01, sustain: 0.7, release: 0.2 };
        let final_state = (0..500).fold(
            (0.0_f32, AdsrState::IDLE),
            |(_, s), _| adsr_step(s, &params, true, 0.016),
        );
        assert!((final_state.0 - 0.7).abs() < 0.01);
    }
    #[test]
    fn adsr_release_decays_to_zero() {
        let params = AdsrParams { attack: 0.01, decay: 0.01, sustain: 0.7, release: 0.1 };
        // Reach sustain first
        let (_, sustain_state) = (0..200).fold(
            (0.0_f32, AdsrState::IDLE),
            |(_, s), _| adsr_step(s, &params, true, 0.016),
        );
        // Then release
        let (v, _) = (0..500).fold(
            (0.0_f32, sustain_state),
            |(_, s), _| adsr_step(s, &params, false, 0.016),
        );
        assert!(v.abs() < 0.01);
    }
}
