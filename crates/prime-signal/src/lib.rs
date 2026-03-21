//! prime-signal — Signal processing for game feel and real-time systems.
//!
//! # Modules
//! - [`smoothdamp`] — critically damped spring (camera follow, UI animation)
//! - [`spring`] — Hooke spring with damping (oscillating follow)
//! - [`low_pass`] — exponential low-pass filter (smooth noisy input)
//! - [`high_pass`] — exponential high-pass filter (isolate fast changes)
//! - [`deadzone`] — axis deadzone and response curve (gamepad input)

/// Smoothly damps a value toward a target using a critically damped spring.
///
/// This is the standard "camera follow" function — smooth, no overshoot,
/// arrives at target with zero velocity.
///
/// # Math
///
/// Critically damped spring approximation (Runge-Kutta inspired polynomial):
///
///   omega = 2 / smooth_time
///   k = 1 + omega·dt + 0.48·omega²·dt² + 0.235·omega³·dt³
///   change = current − target
///   temp = (velocity + omega·change) · dt
///   new_velocity = (velocity − omega·temp) / k
///   new_value = target + (change + temp) / k
///
/// The polynomial denominator `k` approximates the exact exponential decay.
/// Error < 0.1% for typical dt values (< 0.1s).
///
/// # Arguments
/// * `current` - Current value
/// * `target` - Target value to approach
/// * `velocity` - Current velocity (from previous frame)
/// * `smooth_time` - Approximate time to reach target (seconds). Must be > 0.
/// * `dt` - Delta time since last call (seconds)
///
/// # Returns
/// `(new_value, new_velocity)` — store both for the next frame.
///
/// # Edge cases
/// * `smooth_time <= 0` → clamps to 1e-4 to avoid division by zero
/// * `dt == 0` → returns `(current, velocity)` unchanged
///
/// # Example
/// ```rust
/// use prime_signal::smoothdamp;
/// let mut vel = 0.0f32;
/// let mut pos = 0.0f32;
/// for _ in 0..100 {
///     (pos, vel) = smoothdamp(pos, 10.0, vel, 0.3, 0.016);
/// }
/// // After 100 frames at 60fps, should be very close to 10.0
/// assert!((pos - 10.0).abs() < 0.01);
/// ```
pub fn smoothdamp(
    current: f32,
    target: f32,
    velocity: f32,
    smooth_time: f32,
    dt: f32,
) -> (f32, f32) {
    if dt <= 0.0 { return (current, velocity); }
    let smooth_time = smooth_time.max(1e-4);
    let omega = 2.0 / smooth_time;
    let x = omega * dt;
    let k = 1.0 + x + 0.48 * x * x + 0.235 * x * x * x;
    let change = current - target;
    let temp = (velocity + omega * change) * dt;
    let new_vel = (velocity - omega * temp) / k;
    let new_val = target + (change + temp) / k;
    (new_val, new_vel)
}

/// Spring simulation — Hooke's law with damping. Allows overshoot.
///
/// Unlike smoothdamp (no overshoot), spring oscillates around the target
/// based on stiffness and damping. Use for bouncy, physical-feeling motion.
///
/// # Math
///
/// Symplectic Euler integration of Hooke's law with damping:
///   force = −stiffness × (position − target) − damping × velocity
///   velocity += force × dt
///   position += velocity × dt
///
/// Critical damping ratio: damping = 2 × √(stiffness × mass)
/// For mass=1: damping = 2√stiffness → no oscillation, fastest settle.
/// Lower damping → more oscillation. Higher → slower but no bounce.
///
/// # Arguments
/// * `position` - Current position
/// * `velocity` - Current velocity
/// * `target` - Target position
/// * `stiffness` - Spring constant (higher = snappier). Typical: 50–500.
/// * `damping` - Damping coefficient. Typical: 2×√stiffness for critical.
/// * `dt` - Delta time (seconds)
///
/// # Returns
/// `(new_position, new_velocity)` — store both for the next frame.
///
/// # Edge cases
/// * `dt == 0` → returns `(position, velocity)` unchanged
///
/// # Example
/// ```rust
/// use prime_signal::spring;
/// let mut pos = 0.0f32;
/// let mut vel = 0.0f32;
/// for _ in 0..200 {
///     (pos, vel) = spring(pos, vel, 10.0, 100.0, 20.0, 0.016);
/// }
/// assert!((pos - 10.0).abs() < 0.01);
/// ```
pub fn spring(
    position: f32,
    velocity: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    dt: f32,
) -> (f32, f32) {
    if dt <= 0.0 { return (position, velocity); }
    let force = -stiffness * (position - target) - damping * velocity;
    let new_vel = velocity + force * dt;
    let new_pos = position + new_vel * dt;
    (new_pos, new_vel)
}

/// Exponential low-pass filter — smooths noisy or rapidly changing input.
///
/// # Math
///
/// One-pole IIR low-pass filter:
///   alpha = 1 − e^(−dt / time_constant)
///   output = previous + alpha × (input − previous)
///         = lerp(previous, input, alpha)
///
/// alpha → 0: heavy smoothing (slow response)
/// alpha → 1: no smoothing (instant response)
///
/// time_constant is the RC constant: time for output to reach ~63% of
/// a step input. Higher time_constant → more smoothing.
///
/// # Arguments
/// * `previous` - Last filtered output (store between calls)
/// * `input` - New raw input sample
/// * `time_constant` - Smoothing time constant (seconds). Must be > 0.
/// * `dt` - Delta time (seconds)
///
/// # Returns
/// New filtered value. Store this as `previous` for the next call.
///
/// # Edge cases
/// * `time_constant <= 0` → clamps to 1e-6
/// * `dt == 0` → returns `previous` unchanged
///
/// # Example
/// ```rust
/// use prime_signal::low_pass;
/// let mut filtered = 0.0f32;
/// for _ in 0..200 {
///     filtered = low_pass(filtered, 1.0, 0.1, 0.016);
/// }
/// assert!((filtered - 1.0).abs() < 0.01);
/// ```
pub fn low_pass(previous: f32, input: f32, time_constant: f32, dt: f32) -> f32 {
    if dt <= 0.0 { return previous; }
    let tc = time_constant.max(1e-6);
    let alpha = 1.0 - (-dt / tc).exp();
    previous + alpha * (input - previous)
}

/// Exponential high-pass filter — isolates fast changes, removes DC offset.
///
/// # Math
///
/// Derived from low-pass complement:
///   lp = low_pass(previous_lp, input, time_constant, dt)
///   output = input − lp
///
/// Passes high-frequency changes, blocks slow drift.
/// Use for: detecting sudden inputs, removing gravity from accelerometer,
/// isolating transients from a signal.
///
/// # Arguments
/// * `previous_lp` - Last low-pass state (from previous frame)
/// * `input` - New raw input sample
/// * `time_constant` - Cutoff time constant (seconds)
/// * `dt` - Delta time (seconds)
///
/// # Returns
/// `(output, new_lp_state)` — store `new_lp_state` for the next frame.
///
/// # Example
/// ```rust
/// use prime_signal::high_pass;
/// let mut lp_state = 0.0f32;
/// // DC signal — high pass should output ~0 after settling
/// let mut out = 0.0f32;
/// for _ in 0..200 {
///     (out, lp_state) = high_pass(lp_state, 1.0, 0.05, 0.016);
/// }
/// assert!(out.abs() < 0.01);
/// ```
pub fn high_pass(previous_lp: f32, input: f32, time_constant: f32, dt: f32) -> (f32, f32) {
    let new_lp = low_pass(previous_lp, input, time_constant, dt);
    (input - new_lp, new_lp)
}

/// Apply deadzone and response curve to a raw axis value.
///
/// # Math
///
/// Given raw value r in [−1, 1]:
///   if |r| < deadzone  → output = 0  (stick at rest, ignore noise)
///   else               → remap |r| from [deadzone, 1] → [0, 1],
///                        preserve sign,
///                        apply power curve: output = sign × t^curve
///
/// curve = 1.0 → linear response after deadzone
/// curve = 2.0 → quadratic (slow near deadzone, fast at max)
/// curve = 0.5 → square root (fast near deadzone, leveling off)
///
/// # Arguments
/// * `value` - Raw axis value in [−1, 1]
/// * `deadzone` - Deadzone threshold in [0, 1). Values below this → 0.
/// * `curve` - Response curve exponent. 1.0 = linear, 2.0 = quadratic.
///
/// # Returns
/// Processed value in [−1, 1] with deadzone applied and curve shaped.
///
/// # Edge cases
/// * `value` outside [−1, 1] → clamped
/// * `deadzone >= 1.0` → always returns 0
/// * `curve <= 0` → clamped to 0.01 to avoid undefined behavior
///
/// # Example
/// ```rust
/// use prime_signal::deadzone;
/// assert_eq!(deadzone(0.05, 0.1, 1.0), 0.0); // inside deadzone
/// assert!((deadzone(1.0, 0.1, 1.0) - 1.0).abs() < 1e-5); // full deflection
/// assert_eq!(deadzone(-0.05, 0.1, 1.0), 0.0); // negative inside deadzone
/// ```
pub fn deadzone(value: f32, deadzone: f32, curve: f32) -> f32 {
    let v = value.clamp(-1.0, 1.0);
    let dz = deadzone.clamp(0.0, 0.9999);
    let curve = curve.max(0.01);
    let abs = v.abs();
    if abs < dz { return 0.0; }
    let t = (abs - dz) / (1.0 - dz);
    let shaped = t.powf(curve);
    v.signum() * shaped
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    // ── smoothdamp ──

    #[test]
    fn smoothdamp_approaches_target() {
        let (mut pos, mut vel) = (0.0f32, 0.0f32);
        for _ in 0..200 {
            (pos, vel) = smoothdamp(pos, 10.0, vel, 0.3, 0.016);
        }
        assert!((pos - 10.0).abs() < 0.01, "pos={pos}");
    }

    #[test]
    fn smoothdamp_no_overshoot() {
        let (mut pos, mut vel) = (0.0f32, 0.0f32);
        let mut max_pos = 0.0f32;
        for _ in 0..300 {
            (pos, vel) = smoothdamp(pos, 10.0, vel, 0.3, 0.016);
            max_pos = max_pos.max(pos);
        }
        assert!(max_pos <= 10.0 + 0.001, "smoothdamp overshot: max={max_pos}");
    }

    #[test]
    fn smoothdamp_dt_zero_returns_current() {
        let (pos, _) = smoothdamp(3.0, 10.0, 5.0, 0.3, 0.0);
        assert!((pos - 3.0).abs() < EPSILON);
    }

    #[test]
    fn smoothdamp_negative_smooth_time_clamped() {
        // Should not panic
        let _ = smoothdamp(0.0, 10.0, 0.0, -1.0, 0.016);
    }

    #[test]
    fn smoothdamp_velocity_decays_to_zero() {
        let (mut pos, mut vel) = (0.0f32, 0.0f32);
        for _ in 0..500 {
            (pos, vel) = smoothdamp(pos, 10.0, vel, 0.3, 0.016);
        }
        assert!(vel.abs() < 0.001, "velocity should decay to ~0, got {vel}");
    }

    // ── spring ──

    #[test]
    fn spring_approaches_target() {
        let (mut pos, mut vel) = (0.0f32, 0.0f32);
        for _ in 0..500 {
            (pos, vel) = spring(pos, vel, 10.0, 100.0, 20.0, 0.016);
        }
        assert!((pos - 10.0).abs() < 0.01, "pos={pos}");
    }

    #[test]
    fn spring_dt_zero_no_change() {
        let (pos, vel) = spring(3.0, 5.0, 10.0, 100.0, 20.0, 0.0);
        assert!((pos - 3.0).abs() < EPSILON);
        assert!((vel - 5.0).abs() < EPSILON);
    }

    #[test]
    fn spring_underdamped_overshoots() {
        let (mut pos, mut vel) = (0.0f32, 0.0f32);
        let mut max_pos = 0.0f32;
        for _ in 0..100 {
            (pos, vel) = spring(pos, vel, 10.0, 200.0, 5.0, 0.016);
            max_pos = max_pos.max(pos);
        }
        assert!(max_pos > 10.0, "underdamped spring should overshoot, max={max_pos}");
    }

    // ── low_pass ──

    #[test]
    fn low_pass_converges_to_input() {
        let mut filtered = 0.0f32;
        for _ in 0..300 {
            filtered = low_pass(filtered, 1.0, 0.1, 0.016);
        }
        assert!((filtered - 1.0).abs() < 0.01, "filtered={filtered}");
    }

    #[test]
    fn low_pass_dt_zero_returns_previous() {
        let result = low_pass(0.5, 1.0, 0.1, 0.0);
        assert!((result - 0.5).abs() < EPSILON);
    }

    #[test]
    fn low_pass_large_time_constant_is_slow() {
        let filtered = low_pass(0.0, 1.0, 10.0, 0.016);
        assert!(filtered < 0.01, "large TC should respond slowly: {filtered}");
    }

    #[test]
    fn low_pass_small_time_constant_is_fast() {
        let filtered = low_pass(0.0, 1.0, 0.001, 0.016);
        assert!(filtered > 0.9, "small TC should respond fast: {filtered}");
    }

    // ── high_pass ──

    #[test]
    fn high_pass_dc_signal_approaches_zero() {
        let (mut out, mut lp) = (0.0f32, 0.0f32);
        for _ in 0..300 {
            (out, lp) = high_pass(lp, 1.0, 0.05, 0.016);
        }
        assert!(out.abs() < 0.01, "HP of DC signal should → 0, got {out}");
    }

    #[test]
    fn high_pass_step_produces_initial_output() {
        let (out, _) = high_pass(0.0, 1.0, 0.1, 0.016);
        assert!(out > 0.0, "HP of step should produce positive output");
    }

    // ── deadzone ──

    #[test]
    fn deadzone_inside_returns_zero() {
        assert_eq!(deadzone(0.05, 0.1, 1.0), 0.0);
        assert_eq!(deadzone(-0.05, 0.1, 1.0), 0.0);
        assert_eq!(deadzone(0.0, 0.1, 1.0), 0.0);
    }

    #[test]
    fn deadzone_full_deflection_returns_one() {
        assert!((deadzone(1.0, 0.1, 1.0) - 1.0).abs() < EPSILON);
        assert!((deadzone(-1.0, 0.1, 1.0) - (-1.0)).abs() < EPSILON);
    }

    #[test]
    fn deadzone_preserves_sign() {
        assert!(deadzone(0.5, 0.1, 1.0) > 0.0);
        assert!(deadzone(-0.5, 0.1, 1.0) < 0.0);
    }

    #[test]
    fn deadzone_at_boundary() {
        // Exactly at deadzone threshold → 0
        assert_eq!(deadzone(0.1, 0.1, 1.0), 0.0);
    }

    #[test]
    fn deadzone_clamps_input() {
        // Value > 1.0 should be clamped
        let v = deadzone(2.0, 0.1, 1.0);
        assert!((v - 1.0).abs() < EPSILON);
    }

    #[test]
    fn deadzone_quadratic_curve() {
        // curve=2 should give smaller values near deadzone than linear
        let linear = deadzone(0.55, 0.1, 1.0);
        let quad = deadzone(0.55, 0.1, 2.0);
        assert!(quad < linear, "quadratic curve should be smaller near deadzone");
    }
}
