//! prime-interp — Interpolation, easing, and smoothstep functions.
//!
//! # Modules
//! - Basic: [`lerp`], [`inv_lerp`], [`remap`]
//! - Smooth: [`smoothstep`], [`smootherstep`]
//! - Easing: quad, cubic, quart, quint, sine, expo, circ, elastic, bounce
//!
//! # Naming convention
//! - `ease_in_*` — slow start, fast end
//! - `ease_out_*` — fast start, slow end
//! - `ease_in_out_*` — slow start, slow end, fast middle
//!
//! All easing functions take `t` in [0, 1] and return a value in [0, 1]
//! (elastic and bounce may briefly exceed this range intentionally).

use std::f32::consts::PI;

// ── Basic ─────────────────────────────────────────────────────────────────

/// Linear interpolation between `a` and `b`.
///
/// # Math
///   lerp(a, b, t) = a + t × (b − a) = a × (1−t) + b × t
///
/// # Arguments
/// * `a` - Start value (returned when t = 0.0)
/// * `b` - End value (returned when t = 1.0)
/// * `t` - Interpolation factor. Not clamped — extrapolates outside [0,1].
///
/// # Example
/// ```rust
/// use prime_interp::lerp;
/// assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-5);
/// assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-5);
/// assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-5);
/// ```
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Lerp with t clamped to [0, 1].
/// ```rust
/// # use prime_interp::lerp_clamped;
/// assert!((lerp_clamped(0.0, 10.0, 1.5) - 10.0).abs() < 1e-5);
/// ```
pub fn lerp_clamped(a: f32, b: f32, t: f32) -> f32 {
    lerp(a, b, t.clamp(0.0, 1.0))
}

/// Inverse lerp — the t that produces `v` between `a` and `b`.
///
/// # Math
///   inv_lerp(a, b, v) = (v − a) / (b − a)
///
/// # Arguments
/// * `a` - Start of range
/// * `b` - End of range. Must not equal `a`.
/// * `v` - Value to find t for
///
/// # Returns
/// t such that lerp(a, b, t) = v. Not clamped.
///
/// # Edge cases
/// * `a == b` → returns 0.0 (undefined, avoid)
///
/// # Example
/// ```rust
/// use prime_interp::inv_lerp;
/// assert!((inv_lerp(0.0, 10.0, 5.0) - 0.5).abs() < 1e-5);
/// ```
pub fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON { return 0.0; }
    (v - a) / (b - a)
}

/// Remap a value from one range to another.
///
/// # Math
///   remap(v, in_min, in_max, out_min, out_max) =
///     lerp(out_min, out_max, inv_lerp(in_min, in_max, v))
///
/// # Example
/// ```rust
/// use prime_interp::remap;
/// // Map 5 from [0,10] → [0,100]
/// assert!((remap(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 1e-3);
/// ```
pub fn remap(v: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    lerp(out_min, out_max, inv_lerp(in_min, in_max, v))
}

/// Repeat: wraps t into [0, length).
///
/// `repeat(2.5, 1.0) = 0.5`
/// ```rust
/// # use prime_interp::repeat;
/// assert!((repeat(2.5, 1.0) - 0.5).abs() < 1e-5);
/// assert!((repeat(-0.3, 1.0) - 0.7).abs() < 1e-5);
/// ```
pub fn repeat(t: f32, length: f32) -> f32 {
    if length == 0.0 { return 0.0; }
    t - (t / length).floor() * length
}

/// Ping-pong: t bounces between 0 and length.
///
/// `pingpong(2.5, 1.0) = 0.5` — t=2.5 wraps to 0.5 on the return stroke.
/// ```rust
/// # use prime_interp::pingpong;
/// assert!((pingpong(2.5, 1.0) - 0.5).abs() < 1e-5);
/// assert!((pingpong(1.5, 1.0) - 0.5).abs() < 1e-5);
/// ```
pub fn pingpong(t: f32, length: f32) -> f32 {
    if length == 0.0 { return 0.0; }
    let t = repeat(t, length * 2.0);
    length - (t - length).abs()
}

// ── Smooth ────────────────────────────────────────────────────────────────

/// Hermite smoothstep — S-curve from 0 to 1.
///
/// # Math
///   t = clamp((x − e0) / (e1 − e0), 0, 1)
///   return t² × (3 − 2t)
///
/// Zero first derivative at both endpoints.
///
/// # Arguments
/// * `edge0` - Lower edge (returns 0 below this)
/// * `edge1` - Upper edge (returns 1 above this)
/// * `x` - Input value
///
/// # Edge cases
/// * `x <= edge0` → 0.0
/// * `x >= edge1` → 1.0
///
/// # Example
/// ```rust
/// use prime_interp::smoothstep;
/// assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-5);
/// assert!((smoothstep(0.0, 1.0, 0.0) - 0.0).abs() < 1e-5);
/// assert!((smoothstep(0.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
/// ```
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ken Perlin's smootherstep — C2 continuity (zero 1st AND 2nd derivative at edges).
///
/// # Math
///   t = clamp((x − e0) / (e1 − e0), 0, 1)
///   return t³ × (t × (6t − 15) + 10)
///
/// Smoother than smoothstep at the cost of slightly more computation.
/// Preferred for noise functions and terrain blending.
///
/// # Example
/// ```rust
/// use prime_interp::smootherstep;
/// assert!((smootherstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-5);
/// assert!((smootherstep(0.0, 1.0, 0.0) - 0.0).abs() < 1e-5);
/// assert!((smootherstep(0.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
/// ```
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (6.0 * t - 15.0) + 10.0)
}

// ── Easing — Quad ─────────────────────────────────────────────────────────

/// Quadratic ease-in. t² — accelerates from zero.
///
/// # Math
///   f(t) = t²
pub fn ease_in_quad(t: f32) -> f32 { t * t }

/// Quadratic ease-out. 1−(1−t)² — decelerates to zero.
///
/// # Math
///   f(t) = 1 − (1−t)² = t × (2−t)
pub fn ease_out_quad(t: f32) -> f32 { t * (2.0 - t) }

/// Quadratic ease-in-out. S-curve with quadratic segments.
///
/// # Math
///   t < 0.5 → 2t²
///   t ≥ 0.5 → 1 − (−2t+2)² / 2
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
}

// ── Easing — Cubic ────────────────────────────────────────────────────────

/// Cubic ease-in. t³
pub fn ease_in_cubic(t: f32) -> f32 { t * t * t }

/// Cubic ease-out. 1−(1−t)³
pub fn ease_out_cubic(t: f32) -> f32 { 1.0 - (1.0 - t).powi(3) }

/// Cubic ease-in-out.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

// ── Easing — Quart ────────────────────────────────────────────────────────

/// Quartic ease-in. t⁴
pub fn ease_in_quart(t: f32) -> f32 { t * t * t * t }

/// Quartic ease-out. 1−(1−t)⁴
pub fn ease_out_quart(t: f32) -> f32 { 1.0 - (1.0 - t).powi(4) }

/// Quartic ease-in-out.
pub fn ease_in_out_quart(t: f32) -> f32 {
    if t < 0.5 { 8.0 * t * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(4) / 2.0 }
}

// ── Easing — Quint ────────────────────────────────────────────────────────

/// Quintic ease-in. t⁵
pub fn ease_in_quint(t: f32) -> f32 { t * t * t * t * t }

/// Quintic ease-out. 1−(1−t)⁵
pub fn ease_out_quint(t: f32) -> f32 { 1.0 - (1.0 - t).powi(5) }

/// Quintic ease-in-out.
pub fn ease_in_out_quint(t: f32) -> f32 {
    if t < 0.5 { 16.0 * t * t * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(5) / 2.0 }
}

// ── Easing — Sine ─────────────────────────────────────────────────────────

/// Sine ease-in.
///
/// # Math
///   f(t) = 1 − cos(t × π/2)
pub fn ease_in_sine(t: f32) -> f32 { 1.0 - (t * PI / 2.0).cos() }

/// Sine ease-out.
///
/// # Math
///   f(t) = sin(t × π/2)
pub fn ease_out_sine(t: f32) -> f32 { (t * PI / 2.0).sin() }

/// Sine ease-in-out.
///
/// # Math
///   f(t) = −(cos(πt) − 1) / 2
pub fn ease_in_out_sine(t: f32) -> f32 { -((PI * t).cos() - 1.0) / 2.0 }

// ── Easing — Expo ─────────────────────────────────────────────────────────

/// Exponential ease-in.
///
/// # Math
///   t = 0 → 0.0
///   t > 0 → 2^(10t − 10)
pub fn ease_in_expo(t: f32) -> f32 {
    if t == 0.0 { 0.0 } else { (2.0_f32).powf(10.0 * t - 10.0) }
}

/// Exponential ease-out.
///
/// # Math
///   t = 1 → 1.0
///   t < 1 → 1 − 2^(−10t)
pub fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 { 1.0 } else { 1.0 - (2.0_f32).powf(-10.0 * t) }
}

/// Exponential ease-in-out.
pub fn ease_in_out_expo(t: f32) -> f32 {
    if t == 0.0 { return 0.0; }
    if t == 1.0 { return 1.0; }
    if t < 0.5 { (2.0_f32).powf(20.0 * t - 10.0) / 2.0 }
    else { (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0 }
}

// ── Easing — Circ ─────────────────────────────────────────────────────────

/// Circular ease-in. Based on quarter circle arc.
///
/// # Math
///   f(t) = 1 − √(1 − t²)
pub fn ease_in_circ(t: f32) -> f32 { 1.0 - (1.0 - t * t).sqrt() }

/// Circular ease-out.
///
/// # Math
///   f(t) = √(1 − (t−1)²)
pub fn ease_out_circ(t: f32) -> f32 { (1.0 - (t - 1.0).powi(2)).sqrt() }

/// Circular ease-in-out.
pub fn ease_in_out_circ(t: f32) -> f32 {
    if t < 0.5 { (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0 }
    else { ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0 }
}

// ── Easing — Elastic ──────────────────────────────────────────────────────

/// Elastic ease-in — spring past zero then snap to target.
///
/// # Math
///   c4 = 2π / 3
///   t = 0 → 0; t = 1 → 1
///   else → −2^(10t−10) × sin((10t−10.75) × c4)
///
/// # Returns
/// May return values outside [0, 1] — the elastic overshoot is intentional.
pub fn ease_in_elastic(t: f32) -> f32 {
    if t == 0.0 { return 0.0; }
    if t == 1.0 { return 1.0; }
    let c4 = 2.0 * PI / 3.0;
    -(2.0_f32).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * c4).sin()
}

/// Elastic ease-out — overshoot then settle.
///
/// # Math
///   c4 = 2π / 3
///   t = 0 → 0; t = 1 → 1
///   else → 2^(−10t) × sin((10t−0.75) × c4) + 1
pub fn ease_out_elastic(t: f32) -> f32 {
    if t == 0.0 { return 0.0; }
    if t == 1.0 { return 1.0; }
    let c4 = 2.0 * PI / 3.0;
    (2.0_f32).powf(-10.0 * t) * ((10.0 * t - 0.75) * c4).sin() + 1.0
}

/// Elastic ease-in-out.
pub fn ease_in_out_elastic(t: f32) -> f32 {
    if t == 0.0 { return 0.0; }
    if t == 1.0 { return 1.0; }
    let c5 = 2.0 * PI / 4.5;
    if t < 0.5 {
        -(2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0
    } else {
        (2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0 + 1.0
    }
}

// ── Easing — Bounce ───────────────────────────────────────────────────────

/// Bounce ease-out — bounces at the end, settling to target.
///
/// # Math
///
/// Piecewise polynomial approximating a bouncing ball:
///   n1 = 7.5625, d1 = 2.75
///   if t < 1/d1:             n1 × t²
///   elif t < 2/d1:   t -= 1.5/d1;  n1×t² + 0.75
///   elif t < 2.5/d1: t -= 2.25/d1; n1×t² + 0.9375
///   else:            t -= 2.625/d1; n1×t² + 0.984375
pub fn ease_out_bounce(t: f32) -> f32 {
    let n1 = 7.5625_f32;
    let d1 = 2.75_f32;
    let mut t = t;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        t -= 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        t -= 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        t -= 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Bounce ease-in — bounces at the start.
///
/// # Math
///   f(t) = 1 − ease_out_bounce(1 − t)
pub fn ease_in_bounce(t: f32) -> f32 { 1.0 - ease_out_bounce(1.0 - t) }

/// Bounce ease-in-out.
pub fn ease_in_out_bounce(t: f32) -> f32 {
    if t < 0.5 { (1.0 - ease_out_bounce(1.0 - 2.0 * t)) / 2.0 }
    else { (1.0 + ease_out_bounce(2.0 * t - 1.0)) / 2.0 }
}

// ── Easing — Back ─────────────────────────────────────────────────────────

/// Ease in with overshoot (back). `s = 1.70158`.
/// ```rust
/// # use prime_interp::ease_in_back;
/// assert!((ease_in_back(0.0)).abs() < 1e-5);
/// assert!((ease_in_back(1.0) - 1.0).abs() < 1e-5);
/// ```
pub fn ease_in_back(t: f32) -> f32 {
    let s = 1.70158_f32;
    t * t * ((s + 1.0) * t - s)
}

/// Ease out with overshoot (back). `s = 1.70158`.
/// ```rust
/// # use prime_interp::ease_out_back;
/// assert!((ease_out_back(0.0)).abs() < 1e-5);
/// assert!((ease_out_back(1.0) - 1.0).abs() < 1e-5);
/// ```
pub fn ease_out_back(t: f32) -> f32 {
    let s = 1.70158_f32;
    let t = t - 1.0;
    t * t * ((s + 1.0) * t + s) + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    // ── lerp ──

    #[test]
    fn lerp_at_t0() { assert!((lerp(3.0, 7.0, 0.0) - 3.0).abs() < EPSILON); }
    #[test]
    fn lerp_at_t1() { assert!((lerp(3.0, 7.0, 1.0) - 7.0).abs() < EPSILON); }
    #[test]
    fn lerp_at_midpoint() { assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < EPSILON); }
    #[test]
    fn lerp_extrapolates_below() { assert!((lerp(0.0, 10.0, -0.5) - (-5.0)).abs() < EPSILON); }
    #[test]
    fn lerp_extrapolates_above() { assert!((lerp(0.0, 10.0, 1.5) - 15.0).abs() < EPSILON); }

    // ── inv_lerp ──

    #[test]
    fn inv_lerp_at_start() { assert!((inv_lerp(0.0, 10.0, 0.0) - 0.0).abs() < EPSILON); }
    #[test]
    fn inv_lerp_at_end() { assert!((inv_lerp(0.0, 10.0, 10.0) - 1.0).abs() < EPSILON); }
    #[test]
    fn inv_lerp_at_midpoint() { assert!((inv_lerp(0.0, 10.0, 5.0) - 0.5).abs() < EPSILON); }
    #[test]
    fn inv_lerp_equal_range_returns_zero() { assert_eq!(inv_lerp(5.0, 5.0, 5.0), 0.0); }

    // ── remap ──

    #[test]
    fn remap_mid_value() {
        assert!((remap(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < EPSILON);
    }
    #[test]
    fn remap_start_value() {
        assert!((remap(0.0, 0.0, 10.0, -1.0, 1.0) - (-1.0)).abs() < EPSILON);
    }
    #[test]
    fn remap_end_value() {
        assert!((remap(10.0, 0.0, 10.0, -1.0, 1.0) - 1.0).abs() < EPSILON);
    }

    // ── smoothstep ──

    #[test]
    fn smoothstep_at_edge0() { assert!((smoothstep(0.0, 1.0, 0.0) - 0.0).abs() < EPSILON); }
    #[test]
    fn smoothstep_at_edge1() { assert!((smoothstep(0.0, 1.0, 1.0) - 1.0).abs() < EPSILON); }
    #[test]
    fn smoothstep_at_midpoint() { assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < EPSILON); }
    #[test]
    fn smoothstep_clamped_below() { assert!((smoothstep(0.0, 1.0, -1.0) - 0.0).abs() < EPSILON); }
    #[test]
    fn smoothstep_clamped_above() { assert!((smoothstep(0.0, 1.0, 2.0) - 1.0).abs() < EPSILON); }

    // ── smootherstep ──

    #[test]
    fn smootherstep_at_edge0() { assert!((smootherstep(0.0, 1.0, 0.0) - 0.0).abs() < EPSILON); }
    #[test]
    fn smootherstep_at_edge1() { assert!((smootherstep(0.0, 1.0, 1.0) - 1.0).abs() < EPSILON); }
    #[test]
    fn smootherstep_at_midpoint() { assert!((smootherstep(0.0, 1.0, 0.5) - 0.5).abs() < EPSILON); }

    // ── Easing — boundary conditions ──

    fn check_ease(f: &dyn Fn(f32) -> f32, name: &str) {
        assert!((f(0.0) - 0.0).abs() < 0.01, "{name}(0) should be ~0, got {}", f(0.0));
        assert!((f(1.0) - 1.0).abs() < 0.01, "{name}(1) should be ~1, got {}", f(1.0));
    }

    #[test]
    fn all_ease_in_functions_boundary() {
        check_ease(&ease_in_quad, "ease_in_quad");
        check_ease(&ease_in_cubic, "ease_in_cubic");
        check_ease(&ease_in_quart, "ease_in_quart");
        check_ease(&ease_in_quint, "ease_in_quint");
        check_ease(&ease_in_sine, "ease_in_sine");
        check_ease(&ease_in_expo, "ease_in_expo");
        check_ease(&ease_in_circ, "ease_in_circ");
        check_ease(&ease_in_elastic, "ease_in_elastic");
        check_ease(&ease_in_bounce, "ease_in_bounce");
        check_ease(&ease_in_back, "ease_in_back");
    }

    #[test]
    fn all_ease_out_functions_boundary() {
        check_ease(&ease_out_quad, "ease_out_quad");
        check_ease(&ease_out_cubic, "ease_out_cubic");
        check_ease(&ease_out_quart, "ease_out_quart");
        check_ease(&ease_out_quint, "ease_out_quint");
        check_ease(&ease_out_sine, "ease_out_sine");
        check_ease(&ease_out_expo, "ease_out_expo");
        check_ease(&ease_out_circ, "ease_out_circ");
        check_ease(&ease_out_elastic, "ease_out_elastic");
        check_ease(&ease_out_bounce, "ease_out_bounce");
        check_ease(&ease_out_back, "ease_out_back");
    }

    #[test]
    fn all_ease_in_out_functions_boundary() {
        check_ease(&ease_in_out_quad, "ease_in_out_quad");
        check_ease(&ease_in_out_cubic, "ease_in_out_cubic");
        check_ease(&ease_in_out_quart, "ease_in_out_quart");
        check_ease(&ease_in_out_quint, "ease_in_out_quint");
        check_ease(&ease_in_out_sine, "ease_in_out_sine");
        check_ease(&ease_in_out_expo, "ease_in_out_expo");
        check_ease(&ease_in_out_circ, "ease_in_out_circ");
        check_ease(&ease_in_out_elastic, "ease_in_out_elastic");
        check_ease(&ease_in_out_bounce, "ease_in_out_bounce");
    }

    // ── ease_in is monotonically non-decreasing ──

    #[test]
    fn ease_in_cubic_is_monotone() {
        let mut prev = 0.0f32;
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let v = ease_in_cubic(t);
            assert!(v >= prev - EPSILON, "ease_in_cubic not monotone at t={t}");
            prev = v;
        }
    }

    #[test]
    fn ease_out_cubic_is_monotone() {
        let mut prev = 0.0f32;
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let v = ease_out_cubic(t);
            assert!(v >= prev - EPSILON, "ease_out_cubic not monotone at t={t}");
            prev = v;
        }
    }

    // ── bounce is monotone and bounded ──

    #[test]
    fn ease_out_bounce_bounded() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = ease_out_bounce(t);
            assert!(v >= -0.01 && v <= 1.01, "ease_out_bounce({t}) = {v} out of bounds");
        }
    }

    // ── lerp_clamped ──────────────────────────────────────────────────────────

    #[test]
    fn lerp_clamped_within_range() {
        assert!((lerp_clamped(0.0, 10.0, 0.5) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn lerp_clamped_clamps_above() {
        assert!((lerp_clamped(0.0, 10.0, 1.5) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn lerp_clamped_clamps_below() {
        assert!((lerp_clamped(0.0, 10.0, -0.5) - 0.0).abs() < EPSILON);
    }

    // ── repeat ────────────────────────────────────────────────────────────────

    #[test]
    fn repeat_wraps_positive() {
        assert!((repeat(2.5, 1.0) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn repeat_wraps_negative() {
        assert!((repeat(-0.3, 1.0) - 0.7).abs() < EPSILON);
    }

    #[test]
    fn repeat_zero_length() {
        assert_eq!(repeat(5.0, 0.0), 0.0);
    }

    // ── pingpong ──────────────────────────────────────────────────────────────

    #[test]
    fn pingpong_bounces() {
        assert!((pingpong(0.5, 1.0) - 0.5).abs() < EPSILON);
        assert!((pingpong(1.5, 1.0) - 0.5).abs() < EPSILON);
        assert!((pingpong(2.5, 1.0) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn pingpong_at_boundaries() {
        assert!((pingpong(0.0, 1.0)).abs() < EPSILON);
        assert!((pingpong(1.0, 1.0) - 1.0).abs() < EPSILON);
        assert!((pingpong(2.0, 1.0)).abs() < EPSILON);
    }

    // ── ease_in_back / ease_out_back ──────────────────────────────────────────

    #[test]
    fn ease_in_back_boundaries() {
        assert!((ease_in_back(0.0)).abs() < EPSILON);
        assert!((ease_in_back(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn ease_in_back_undershoots() {
        // Back easing goes negative before t=0.5
        assert!(ease_in_back(0.2) < 0.0);
    }

    #[test]
    fn ease_out_back_boundaries() {
        assert!((ease_out_back(0.0)).abs() < EPSILON);
        assert!((ease_out_back(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn ease_out_back_overshoots() {
        // Back easing overshoots past 1.0 before settling
        assert!(ease_out_back(0.8) > 1.0);
    }
}
