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

use glam::{Vec2, Vec3};

/// Smoothdamp for Vec2. Returns (new_value, new_velocity).
///
/// Component-wise application of [`smoothdamp`].
///
/// # Example
/// ```rust
/// # use prime_signal::smoothdamp_vec2;
/// # use glam::Vec2;
/// let (pos, _vel) = smoothdamp_vec2(
///     Vec2::ZERO, Vec2::new(10.0, 5.0), Vec2::ZERO, 0.3, 0.016,
/// );
/// assert!(pos.x > 0.0);
/// ```
pub fn smoothdamp_vec2(current: Vec2, target: Vec2, velocity: Vec2, smooth_time: f32, dt: f32) -> (Vec2, Vec2) {
    let (x, vx) = smoothdamp(current.x, target.x, velocity.x, smooth_time, dt);
    let (y, vy) = smoothdamp(current.y, target.y, velocity.y, smooth_time, dt);
    (Vec2::new(x, y), Vec2::new(vx, vy))
}

/// Smoothdamp for Vec3. Returns (new_value, new_velocity).
///
/// Component-wise application of [`smoothdamp`].
///
/// # Example
/// ```rust
/// # use prime_signal::smoothdamp_vec3;
/// # use glam::Vec3;
/// let (pos, _vel) = smoothdamp_vec3(
///     Vec3::ZERO, Vec3::new(10.0, 5.0, 3.0), Vec3::ZERO, 0.3, 0.016,
/// );
/// assert!(pos.x > 0.0);
/// ```
pub fn smoothdamp_vec3(current: Vec3, target: Vec3, velocity: Vec3, smooth_time: f32, dt: f32) -> (Vec3, Vec3) {
    let (x, vx) = smoothdamp(current.x, target.x, velocity.x, smooth_time, dt);
    let (y, vy) = smoothdamp(current.y, target.y, velocity.y, smooth_time, dt);
    let (z, vz) = smoothdamp(current.z, target.z, velocity.z, smooth_time, dt);
    (Vec3::new(x, y, z), Vec3::new(vx, vy, vz))
}

/// Spring for Vec2. Returns (new_position, new_velocity).
///
/// Component-wise application of [`spring`].
///
/// # Example
/// ```rust
/// # use prime_signal::spring_vec2;
/// # use glam::Vec2;
/// let (pos, _vel) = spring_vec2(
///     Vec2::ZERO, Vec2::ZERO, Vec2::new(10.0, 5.0), 100.0, 20.0, 0.016,
/// );
/// assert!(pos.x > 0.0);
/// ```
pub fn spring_vec2(pos: Vec2, vel: Vec2, target: Vec2, stiffness: f32, damping: f32, dt: f32) -> (Vec2, Vec2) {
    let (x, vx) = spring(pos.x, vel.x, target.x, stiffness, damping, dt);
    let (y, vy) = spring(pos.y, vel.y, target.y, stiffness, damping, dt);
    (Vec2::new(x, y), Vec2::new(vx, vy))
}

/// Spring for Vec3. Returns (new_position, new_velocity).
///
/// Component-wise application of [`spring`].
///
/// # Example
/// ```rust
/// # use prime_signal::spring_vec3;
/// # use glam::Vec3;
/// let (pos, _vel) = spring_vec3(
///     Vec3::ZERO, Vec3::ZERO, Vec3::new(10.0, 5.0, 3.0), 100.0, 20.0, 0.016,
/// );
/// assert!(pos.x > 0.0);
/// ```
pub fn spring_vec3(pos: Vec3, vel: Vec3, target: Vec3, stiffness: f32, damping: f32, dt: f32) -> (Vec3, Vec3) {
    let (x, vx) = spring(pos.x, vel.x, target.x, stiffness, damping, dt);
    let (y, vy) = spring(pos.y, vel.y, target.y, stiffness, damping, dt);
    let (z, vz) = spring(pos.z, vel.z, target.z, stiffness, damping, dt);
    (Vec3::new(x, y, z), Vec3::new(vx, vy, vz))
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

    // ── spring edge cases ─────────────────────────────────────────────────────

    #[test]
    fn spring_zero_stiffness_no_nan() {
        let (v1, vel1) = spring(1.0, 0.5, 0.0, 0.0, 0.5, 0.016);
        assert!(v1.is_finite(), "value must be finite with stiffness=0; got {v1}");
        assert!(vel1.is_finite(), "velocity must be finite with stiffness=0; got {vel1}");
    }

    #[test]
    fn spring_zero_damping_accelerates_toward_target() {
        let (_, vel) = spring(0.0, 0.0, 1.0, 100.0, 0.0, 0.016);
        assert!(vel > 0.0, "undamped spring should accelerate toward target; vel={vel}");
    }

    #[test]
    fn smoothdamp_zero_smooth_time_no_nan() {
        let (v, vel) = smoothdamp(0.0, 1.0, 0.0, 0.0, 0.016);
        assert!(v.is_finite() && vel.is_finite(),
            "smooth_time=0 must not produce NaN/Inf; v={v} vel={vel}");
    }

    // ── smoothdamp_vec2 ──

    #[test]
    fn smoothdamp_vec2_matches_scalar() {
        let cur = Vec2::new(1.0, 2.0);
        let tgt = Vec2::new(10.0, 20.0);
        let vel = Vec2::new(0.5, -0.3);
        let (pos2, vel2) = smoothdamp_vec2(cur, tgt, vel, 0.3, 0.016);
        let (sx, svx) = smoothdamp(1.0, 10.0, 0.5, 0.3, 0.016);
        let (sy, svy) = smoothdamp(2.0, 20.0, -0.3, 0.3, 0.016);
        assert!((pos2.x - sx).abs() < EPSILON);
        assert!((pos2.y - sy).abs() < EPSILON);
        assert!((vel2.x - svx).abs() < EPSILON);
        assert!((vel2.y - svy).abs() < EPSILON);
    }

    #[test]
    fn smoothdamp_vec2_approaches_target() {
        let mut pos = Vec2::ZERO;
        let mut vel = Vec2::ZERO;
        let target = Vec2::new(10.0, 5.0);
        for _ in 0..200 {
            (pos, vel) = smoothdamp_vec2(pos, target, vel, 0.3, 0.016);
        }
        assert!((pos - target).length() < 0.01);
    }

    // ── smoothdamp_vec3 ──

    #[test]
    fn smoothdamp_vec3_matches_scalar() {
        let cur = Vec3::new(1.0, 2.0, 3.0);
        let tgt = Vec3::new(10.0, 20.0, 30.0);
        let vel = Vec3::new(0.5, -0.3, 1.0);
        let (pos3, vel3) = smoothdamp_vec3(cur, tgt, vel, 0.3, 0.016);
        let (sx, svx) = smoothdamp(1.0, 10.0, 0.5, 0.3, 0.016);
        let (sy, svy) = smoothdamp(2.0, 20.0, -0.3, 0.3, 0.016);
        let (sz, svz) = smoothdamp(3.0, 30.0, 1.0, 0.3, 0.016);
        assert!((pos3.x - sx).abs() < EPSILON);
        assert!((pos3.y - sy).abs() < EPSILON);
        assert!((pos3.z - sz).abs() < EPSILON);
        assert!((vel3.x - svx).abs() < EPSILON);
        assert!((vel3.y - svy).abs() < EPSILON);
        assert!((vel3.z - svz).abs() < EPSILON);
    }

    #[test]
    fn smoothdamp_vec3_approaches_target() {
        let mut pos = Vec3::ZERO;
        let mut vel = Vec3::ZERO;
        let target = Vec3::new(10.0, 5.0, 3.0);
        for _ in 0..200 {
            (pos, vel) = smoothdamp_vec3(pos, target, vel, 0.3, 0.016);
        }
        assert!((pos - target).length() < 0.01);
    }

    // ── spring_vec2 ──

    #[test]
    fn spring_vec2_matches_scalar() {
        let pos = Vec2::new(1.0, 2.0);
        let vel = Vec2::new(0.5, -0.3);
        let tgt = Vec2::new(10.0, 20.0);
        let (p2, v2) = spring_vec2(pos, vel, tgt, 100.0, 20.0, 0.016);
        let (sx, svx) = spring(1.0, 0.5, 10.0, 100.0, 20.0, 0.016);
        let (sy, svy) = spring(2.0, -0.3, 20.0, 100.0, 20.0, 0.016);
        assert!((p2.x - sx).abs() < EPSILON);
        assert!((p2.y - sy).abs() < EPSILON);
        assert!((v2.x - svx).abs() < EPSILON);
        assert!((v2.y - svy).abs() < EPSILON);
    }

    #[test]
    fn spring_vec2_approaches_target() {
        let mut pos = Vec2::ZERO;
        let mut vel = Vec2::ZERO;
        let target = Vec2::new(10.0, 5.0);
        for _ in 0..500 {
            (pos, vel) = spring_vec2(pos, vel, target, 100.0, 20.0, 0.016);
        }
        assert!((pos - target).length() < 0.01);
    }

    // ── spring_vec3 ──

    #[test]
    fn spring_vec3_matches_scalar() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let vel = Vec3::new(0.5, -0.3, 1.0);
        let tgt = Vec3::new(10.0, 20.0, 30.0);
        let (p3, v3) = spring_vec3(pos, vel, tgt, 100.0, 20.0, 0.016);
        let (sx, svx) = spring(1.0, 0.5, 10.0, 100.0, 20.0, 0.016);
        let (sy, svy) = spring(2.0, -0.3, 20.0, 100.0, 20.0, 0.016);
        let (sz, svz) = spring(3.0, 1.0, 30.0, 100.0, 20.0, 0.016);
        assert!((p3.x - sx).abs() < EPSILON);
        assert!((p3.y - sy).abs() < EPSILON);
        assert!((p3.z - sz).abs() < EPSILON);
        assert!((v3.x - svx).abs() < EPSILON);
        assert!((v3.y - svy).abs() < EPSILON);
        assert!((v3.z - svz).abs() < EPSILON);
    }

    #[test]
    fn spring_vec3_approaches_target() {
        let mut pos = Vec3::ZERO;
        let mut vel = Vec3::ZERO;
        let target = Vec3::new(10.0, 5.0, 3.0);
        for _ in 0..500 {
            (pos, vel) = spring_vec3(pos, vel, target, 100.0, 20.0, 0.016);
        }
        assert!((pos - target).length() < 0.01);
    }
}
