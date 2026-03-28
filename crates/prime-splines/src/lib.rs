//! `prime-splines` — Curve interpolation: Bezier, Hermite, Catmull-Rom, B-spline, slerp.
//!
//! All public functions are pure (LOAD + COMPUTE only). No `&mut`, no side effects,
//! no hidden state. Same inputs always produce the same output.
//!
//! # Temporal Assembly Model
//! - **LOAD** — read function parameters
//! - **COMPUTE** — pure math
//!
//! STORE and JUMP do not exist here.
//!
//! # Included
//! - `bezier_quadratic` / `bezier_quadratic_3d` — quadratic Bezier (3 control points)
//! - `bezier_cubic` / `bezier_cubic_3d` — cubic Bezier (4 control points)
//! - `hermite` / `hermite_3d` — cubic Hermite (position + tangent at endpoints)
//! - `catmull_rom` / `catmull_rom_3d` — Catmull-Rom (smooth through control points)
//! - `b_spline_cubic` / `b_spline_cubic_3d` — uniform cubic B-spline segment
//! - `slerp` — spherical linear interpolation for unit quaternions
//! - `bezier_cubic_arc_length` / `bezier_cubic_arc_length_3d` — approximate arc length
//! - `bezier_cubic_t_at_length` / `bezier_cubic_t_at_length_3d` — inverse arc-length parameterisation

// ── Quadratic Bezier ──────────────────────────────────────────────────────────

/// Quadratic Bezier interpolation (3 control points).
///
/// # Math
///
/// ```text
/// u = 1 - t
/// B(t) = u²·p0 + 2u·t·p1 + t²·p2
/// ```
///
/// # Arguments
/// * `t`  — parameter in [0, 1] (0 = p0, 1 = p2)
/// * `p0` — start point
/// * `p1` — control point
/// * `p2` — end point
///
/// # Returns
/// Interpolated value.
///
/// # Edge cases
/// * `t = 0` → `p0`
/// * `t = 1` → `p2`
///
/// # Example
/// ```rust
/// use prime_splines::bezier_quadratic;
/// let mid = bezier_quadratic(0.5, 0.0, 1.0, 0.0);
/// assert!((mid - 0.5).abs() < 1e-5);
/// ```
pub fn bezier_quadratic(t: f32, p0: f32, p1: f32, p2: f32) -> f32 {
    let u = 1.0 - t;
    u * u * p0 + 2.0 * u * t * p1 + t * t * p2
}

/// Quadratic Bezier interpolation on `(x, y, z)` tuples.
///
/// Applies [`bezier_quadratic`] independently to each component.
///
/// # Example
/// ```rust
/// use prime_splines::bezier_quadratic_3d;
/// let (x, _, _) = bezier_quadratic_3d(0.5, (0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (2.0, 0.0, 0.0));
/// assert!((x - 1.0).abs() < 1e-5);
/// ```
pub fn bezier_quadratic_3d(
    t: f32,
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        bezier_quadratic(t, p0.0, p1.0, p2.0),
        bezier_quadratic(t, p0.1, p1.1, p2.1),
        bezier_quadratic(t, p0.2, p1.2, p2.2),
    )
}

// ── Cubic Bezier ──────────────────────────────────────────────────────────────

/// Cubic Bezier interpolation (4 control points).
///
/// # Math
///
/// ```text
/// u = 1 - t
/// B(t) = u³·p0 + 3u²t·p1 + 3ut²·p2 + t³·p3
/// ```
///
/// # Arguments
/// * `t`  — parameter in [0, 1]
/// * `p0` — start point
/// * `p1` — first control point
/// * `p2` — second control point
/// * `p3` — end point
///
/// # Returns
/// Interpolated value.
///
/// # Edge cases
/// * `t = 0` → `p0`
/// * `t = 1` → `p3`
///
/// # Example
/// ```rust
/// use prime_splines::bezier_cubic;
/// assert!((bezier_cubic(0.0, 1.0, 2.0, 3.0, 4.0) - 1.0).abs() < 1e-5);
/// assert!((bezier_cubic(1.0, 1.0, 2.0, 3.0, 4.0) - 4.0).abs() < 1e-5);
/// ```
pub fn bezier_cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let u = 1.0 - t;
    u * u * u * p0
        + 3.0 * u * u * t * p1
        + 3.0 * u * t * t * p2
        + t * t * t * p3
}

/// Cubic Bezier interpolation on `(x, y, z)` tuples.
///
/// Applies [`bezier_cubic`] independently to each component.
///
/// # Example
/// ```rust
/// use prime_splines::bezier_cubic_3d;
/// let (x, _, _) = bezier_cubic_3d(0.0, (1.0, 0.0, 0.0), (2.0, 1.0, 0.0), (3.0, 1.0, 0.0), (4.0, 0.0, 0.0));
/// assert!((x - 1.0).abs() < 1e-5);
/// ```
pub fn bezier_cubic_3d(
    t: f32,
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    p3: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        bezier_cubic(t, p0.0, p1.0, p2.0, p3.0),
        bezier_cubic(t, p0.1, p1.1, p2.1, p3.1),
        bezier_cubic(t, p0.2, p1.2, p2.2, p3.2),
    )
}

// ── Hermite cubic ─────────────────────────────────────────────────────────────

/// Cubic Hermite interpolation: endpoints and tangents.
///
/// # Math
///
/// ```text
/// h00(t) =  2t³ - 3t² + 1
/// h10(t) =   t³ - 2t² + t
/// h01(t) = -2t³ + 3t²
/// h11(t) =   t³ -  t²
///
/// h(t) = h00·p0 + h10·m0 + h01·p1 + h11·m1
/// ```
///
/// # Arguments
/// * `t`  — parameter in [0, 1]
/// * `p0` — value at t=0
/// * `m0` — tangent (derivative) at t=0
/// * `p1` — value at t=1
/// * `m1` — tangent (derivative) at t=1
///
/// # Returns
/// Interpolated value.
///
/// # Edge cases
/// * `t = 0` → `p0`
/// * `t = 1` → `p1`
///
/// # Example
/// ```rust
/// use prime_splines::hermite;
/// // Straight line — tangents chosen so it passes linearly
/// let v = hermite(0.5, 0.0, 1.0, 1.0, 1.0);
/// assert!((v - 0.5).abs() < 1e-5);
/// ```
pub fn hermite(t: f32, p0: f32, m0: f32, p1: f32, m1: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
}

/// Cubic Hermite interpolation on `(x, y, z)` tuples.
///
/// # Example
/// ```rust
/// use prime_splines::hermite_3d;
/// let (x, _, _) = hermite_3d(0.0, (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 0.0, 0.0));
/// assert!(x.abs() < 1e-5);
/// ```
pub fn hermite_3d(
    t: f32,
    p0: (f32, f32, f32),
    m0: (f32, f32, f32),
    p1: (f32, f32, f32),
    m1: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        hermite(t, p0.0, m0.0, p1.0, m1.0),
        hermite(t, p0.1, m0.1, p1.1, m1.1),
        hermite(t, p0.2, m0.2, p1.2, m1.2),
    )
}

// ── Catmull-Rom ───────────────────────────────────────────────────────────────

/// Uniform Catmull-Rom spline segment.
///
/// Interpolates between `p1` and `p2`, using `p0` and `p3` as phantom
/// neighbours to compute tangents. The curve passes through `p1` at `t=0`
/// and `p2` at `t=1`.
///
/// # Math
///
/// ```text
/// tangent at p1: m0 = (p2 - p0) / 2
/// tangent at p2: m1 = (p3 - p1) / 2
/// result = hermite(t, p1, m0, p2, m1)
///
/// Equivalently (matrix form):
/// result = 0.5 * ( 2p1
///                + (-p0 + p2) * t
///                + (2p0 - 5p1 + 4p2 - p3) * t²
///                + (-p0 + 3p1 - 3p2 + p3) * t³ )
/// ```
///
/// # Arguments
/// * `t`      — parameter in [0, 1]
/// * `p0..p3` — four consecutive control points; curve segment is p1→p2
///
/// # Returns
/// Interpolated value on the p1→p2 segment.
///
/// # Edge cases
/// * `t = 0` → `p1`
/// * `t = 1` → `p2`
///
/// # Example
/// ```rust
/// use prime_splines::catmull_rom;
/// let v = catmull_rom(0.5, 0.0, 1.0, 2.0, 3.0);
/// assert!((v - 1.5).abs() < 1e-5);
/// ```
pub fn catmull_rom(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Catmull-Rom spline segment on `(x, y, z)` tuples.
///
/// # Example
/// ```rust
/// use prime_splines::catmull_rom_3d;
/// let p = catmull_rom_3d(0.0, (0.0,0.0,0.0), (1.0,0.0,0.0), (2.0,0.0,0.0), (3.0,0.0,0.0));
/// assert!((p.0 - 1.0).abs() < 1e-5);
/// ```
pub fn catmull_rom_3d(
    t: f32,
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    p3: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        catmull_rom(t, p0.0, p1.0, p2.0, p3.0),
        catmull_rom(t, p0.1, p1.1, p2.1, p3.1),
        catmull_rom(t, p0.2, p1.2, p2.2, p3.2),
    )
}

// ── Uniform cubic B-spline ────────────────────────────────────────────────────

/// Uniform cubic B-spline segment.
///
/// Evaluates one segment of a uniform cubic B-spline. The curve does NOT
/// pass through the control points (unlike Catmull-Rom). It is contained
/// within the convex hull of the four control points.
///
/// # Math
///
/// ```text
/// Basis matrix (1/6 factor applied):
///
/// [-1  3 -3  1]   [p0]
/// [ 3 -6  3  0] * [p1] * (1/6)
/// [-3  0  3  0]   [p2]
/// [ 1  4  1  0]   [p3]
///
/// B(t) = (1/6) * ((-t³+3t²-3t+1)·p0
///                +(3t³-6t²+4)·p1
///                +(-3t³+3t²+3t+1)·p2
///                + t³·p3)
/// ```
///
/// # Arguments
/// * `t`      — parameter in [0, 1]
/// * `p0..p3` — four consecutive control points
///
/// # Returns
/// Interpolated B-spline value.
///
/// # Edge cases
/// * `t = 0` → `(p0 + 4·p1 + p2) / 6` (knot value at start of segment)
/// * `t = 1` → `(p1 + 4·p2 + p3) / 6` (knot value at end of segment)
///
/// # Example
/// ```rust
/// use prime_splines::b_spline_cubic;
/// // Uniform points on a line → B-spline stays on that line
/// let v = b_spline_cubic(0.5, 0.0, 1.0, 2.0, 3.0);
/// assert!((v - 1.5).abs() < 1e-5);
/// ```
pub fn b_spline_cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    ((-t3 + 3.0 * t2 - 3.0 * t + 1.0) * p0
        + (3.0 * t3 - 6.0 * t2 + 4.0) * p1
        + (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) * p2
        + t3 * p3)
        / 6.0
}

/// Uniform cubic B-spline segment on `(x, y, z)` tuples.
///
/// # Example
/// ```rust
/// use prime_splines::b_spline_cubic_3d;
/// let p = b_spline_cubic_3d(0.5, (0.0,0.0,0.0), (1.0,0.0,0.0), (2.0,0.0,0.0), (3.0,0.0,0.0));
/// assert!((p.0 - 1.5).abs() < 1e-5);
/// ```
pub fn b_spline_cubic_3d(
    t: f32,
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    p3: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        b_spline_cubic(t, p0.0, p1.0, p2.0, p3.0),
        b_spline_cubic(t, p0.1, p1.1, p2.1, p3.1),
        b_spline_cubic(t, p0.2, p1.2, p2.2, p3.2),
    )
}

// ── Slerp ─────────────────────────────────────────────────────────────────────

/// Spherical linear interpolation between two unit quaternions.
///
/// Takes the shorter arc (negates `q1` if `dot < 0`). Falls back to linear
/// interpolation when the quaternions are nearly identical (avoids division
/// by near-zero `sin(θ)`).
///
/// # Math
///
/// ```text
/// dot = q0·q1
/// if dot < 0: q1 = -q1, dot = -dot   ← shorter arc
/// if dot > 0.9995: lerp and normalise ← near-identical quaternions
///
/// θ = acos(dot)
/// result = sin((1-t)·θ)/sin(θ) · q0 + sin(t·θ)/sin(θ) · q1
/// ```
///
/// # Arguments
/// * `t`  — parameter in [0, 1]
/// * `q0` — start quaternion `(x, y, z, w)`, assumed unit length
/// * `q1` — end quaternion `(x, y, z, w)`, assumed unit length
///
/// # Returns
/// Unit quaternion interpolated along the shorter arc.
///
/// # Edge cases
/// * `t = 0` → `q0`
/// * `t = 1` → `q1` (or `-q1` if the shorter arc required negation)
/// * Nearly identical quaternions → normalised linear interpolation
///
/// # Example
/// ```rust
/// use prime_splines::slerp;
/// let q = slerp(0.0, (0.0, 0.0, 0.0, 1.0), (0.0, 0.0, 1.0, 0.0));
/// let (x, y, z, w) = q;
/// assert!((x*x + y*y + z*z + w*w - 1.0).abs() < 1e-5);
/// ```
pub fn slerp(
    t: f32,
    q0: (f32, f32, f32, f32),
    q1: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    let dot_raw = q0.0 * q1.0 + q0.1 * q1.1 + q0.2 * q1.2 + q0.3 * q1.3;

    // Shorter arc: negate q1 if dot is negative
    let (q1, dot) = if dot_raw < 0.0 {
        ((-q1.0, -q1.1, -q1.2, -q1.3), -dot_raw)
    } else {
        (q1, dot_raw)
    };

    // Near-identical quaternions → normalised linear interpolation
    if dot > 0.9995 {
        let rx = q0.0 + t * (q1.0 - q0.0);
        let ry = q0.1 + t * (q1.1 - q0.1);
        let rz = q0.2 + t * (q1.2 - q0.2);
        let rw = q0.3 + t * (q1.3 - q0.3);
        let len = (rx * rx + ry * ry + rz * rz + rw * rw).sqrt();
        return (rx / len, ry / len, rz / len, rw / len);
    }

    let theta = dot.acos();
    let sin_theta = theta.sin();
    let w0 = ((1.0 - t) * theta).sin() / sin_theta;
    let w1 = (t * theta).sin() / sin_theta;

    (
        w0 * q0.0 + w1 * q1.0,
        w0 * q0.1 + w1 * q1.1,
        w0 * q0.2 + w1 * q1.2,
        w0 * q0.3 + w1 * q1.3,
    )
}

// ── Arc-length parameterisation ───────────────────────────────────────────────

/// Approximate arc length of a 1-D cubic Bezier by subdividing into `steps` linear segments.
///
/// # Math
///
/// ```text
/// L ≈ Σ |B(t_i) - B(t_{i-1})|   for i = 1..steps, t_i = i / steps
/// ```
///
/// # Example
/// ```rust
/// # use prime_splines::bezier_cubic_arc_length;
/// let len = bezier_cubic_arc_length(0.0, 1.0, 2.0, 3.0, 100);
/// assert!((len - 3.0).abs() < 0.01); // straight line p0=0, p1=1, p2=2, p3=3
/// ```
pub fn bezier_cubic_arc_length(p0: f32, p1: f32, p2: f32, p3: f32, steps: usize) -> f32 {
    (1..=steps)
        .fold((0.0_f32, p0), |(acc, prev), i| {
            let t = i as f32 / steps as f32;
            let curr = bezier_cubic(t, p0, p1, p2, p3);
            (acc + (curr - prev).abs(), curr)
        })
        .0
}

/// Approximate arc length of a 3-D cubic Bezier by subdividing into `steps` linear segments.
///
/// # Math
///
/// ```text
/// L ≈ Σ |B(t_i) - B(t_{i-1})|   (Euclidean distance in 3-D)
/// ```
///
/// # Example
/// ```rust
/// # use prime_splines::bezier_cubic_arc_length_3d;
/// // Straight line from origin to (3, 0, 0)
/// let len = bezier_cubic_arc_length_3d(
///     (0.0, 0.0, 0.0), (1.0, 0.0, 0.0),
///     (2.0, 0.0, 0.0), (3.0, 0.0, 0.0), 100,
/// );
/// assert!((len - 3.0).abs() < 0.01);
/// ```
pub fn bezier_cubic_arc_length_3d(
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    p3: (f32, f32, f32),
    steps: usize,
) -> f32 {
    (1..=steps)
        .fold((0.0_f32, p0), |(acc, prev), i| {
            let t = i as f32 / steps as f32;
            let curr = bezier_cubic_3d(t, p0, p1, p2, p3);
            let dx = curr.0 - prev.0;
            let dy = curr.1 - prev.1;
            let dz = curr.2 - prev.2;
            (acc + (dx * dx + dy * dy + dz * dz).sqrt(), curr)
        })
        .0
}

/// Find the parameter `t` corresponding to a target arc length along a 1-D cubic Bezier.
///
/// Uses binary search over `t` in `[0, 1]` to find the `t` where the arc length
/// from `t=0` equals `target_length`. Returns `t` within tolerance after `iterations`
/// bisection steps.
///
/// # Arguments
/// * `p0..p3`        — control points
/// * `target_length` — desired arc length from `t=0`
/// * `steps`         — subdivision count for each arc-length measurement
/// * `iterations`    — number of binary-search bisection steps
///
/// # Example
/// ```rust
/// # use prime_splines::bezier_cubic_t_at_length;
/// // Straight line 0→3: half the length should be at t≈0.5
/// let t = bezier_cubic_t_at_length(0.0, 1.0, 2.0, 3.0, 1.5, 100, 20);
/// assert!((t - 0.5).abs() < 0.01);
/// ```
pub fn bezier_cubic_t_at_length(
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    target_length: f32,
    steps: usize,
    iterations: usize,
) -> f32 {
    // Binary search helper: compute arc length from 0 to t_max
    let arc_length_to = |t_max: f32| -> f32 {
        if t_max <= 0.0 {
            return 0.0;
        }
        (1..=steps)
            .fold((0.0_f32, p0), |(acc, prev), i| {
                let t = t_max * i as f32 / steps as f32;
                let curr = bezier_cubic(t, p0, p1, p2, p3);
                (acc + (curr - prev).abs(), curr)
            })
            .0
    };

    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..iterations {
        let mid = (lo + hi) * 0.5;
        if arc_length_to(mid) < target_length {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) * 0.5
}

/// Find the parameter `t` corresponding to a target arc length along a 3-D cubic Bezier.
///
/// Uses binary search over `t` in `[0, 1]`.
///
/// # Example
/// ```rust
/// # use prime_splines::bezier_cubic_t_at_length_3d;
/// let t = bezier_cubic_t_at_length_3d(
///     (0.0, 0.0, 0.0), (1.0, 0.0, 0.0),
///     (2.0, 0.0, 0.0), (3.0, 0.0, 0.0),
///     1.5, 100, 20,
/// );
/// assert!((t - 0.5).abs() < 0.01);
/// ```
pub fn bezier_cubic_t_at_length_3d(
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    p3: (f32, f32, f32),
    target_length: f32,
    steps: usize,
    iterations: usize,
) -> f32 {
    let arc_length_to = |t_max: f32| -> f32 {
        if t_max <= 0.0 {
            return 0.0;
        }
        (1..=steps)
            .fold((0.0_f32, p0), |(acc, prev), i| {
                let t = t_max * i as f32 / steps as f32;
                let curr = bezier_cubic_3d(t, p0, p1, p2, p3);
                let dx = curr.0 - prev.0;
                let dy = curr.1 - prev.1;
                let dz = curr.2 - prev.2;
                (acc + (dx * dx + dy * dy + dz * dz).sqrt(), curr)
            })
            .0
    };

    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..iterations {
        let mid = (lo + hi) * 0.5;
        if arc_length_to(mid) < target_length {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) * 0.5
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    // ── bezier_quadratic ──────────────────────────────────────────────────────

    #[test]
    fn bezier_quadratic_at_endpoints() {
        assert!((bezier_quadratic(0.0, 1.0, 2.0, 3.0) - 1.0).abs() < EPSILON);
        assert!((bezier_quadratic(1.0, 1.0, 2.0, 3.0) - 3.0).abs() < EPSILON);
    }

    #[test]
    fn bezier_quadratic_midpoint() {
        // Symmetric case: p0=0, p1=1, p2=0 → peak at t=0.5
        let peak = bezier_quadratic(0.5, 0.0, 1.0, 0.0);
        assert!((peak - 0.5).abs() < EPSILON);
    }

    #[test]
    fn bezier_quadratic_deterministic() {
        let a = bezier_quadratic(0.3, 0.0, 1.0, 2.0);
        let b = bezier_quadratic(0.3, 0.0, 1.0, 2.0);
        assert_eq!(a, b);
    }

    #[test]
    fn bezier_quadratic_3d_endpoints() {
        let p0 = (0.0, 0.0, 0.0);
        let p1 = (1.0, 1.0, 1.0);
        let p2 = (2.0, 0.0, 0.0);
        let start = bezier_quadratic_3d(0.0, p0, p1, p2);
        let end = bezier_quadratic_3d(1.0, p0, p1, p2);
        assert!((start.0 - 0.0).abs() < EPSILON);
        assert!((end.0 - 2.0).abs() < EPSILON);
    }

    // ── bezier_cubic ──────────────────────────────────────────────────────────

    #[test]
    fn bezier_cubic_at_endpoints() {
        assert!((bezier_cubic(0.0, 1.0, 2.0, 3.0, 4.0) - 1.0).abs() < EPSILON);
        assert!((bezier_cubic(1.0, 1.0, 2.0, 3.0, 4.0) - 4.0).abs() < EPSILON);
    }

    #[test]
    fn bezier_cubic_collinear_is_linear() {
        // When all four points are collinear (0, 1, 2, 3), the curve is linear
        let v = bezier_cubic(0.5, 0.0, 1.0, 2.0, 3.0);
        assert!((v - 1.5).abs() < EPSILON, "v={v}");
    }

    #[test]
    fn bezier_cubic_deterministic() {
        let a = bezier_cubic(0.4, 0.0, 1.0, 1.0, 0.0);
        let b = bezier_cubic(0.4, 0.0, 1.0, 1.0, 0.0);
        assert_eq!(a, b);
    }

    #[test]
    fn bezier_cubic_3d_endpoints() {
        let (p0, p1, p2, p3) = ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0), (3.0, 0.0, 0.0));
        assert!((bezier_cubic_3d(0.0, p0, p1, p2, p3).0).abs() < EPSILON);
        assert!((bezier_cubic_3d(1.0, p0, p1, p2, p3).0 - 3.0).abs() < EPSILON);
    }

    // ── hermite ───────────────────────────────────────────────────────────────

    #[test]
    fn hermite_at_endpoints() {
        assert!((hermite(0.0, 1.0, 2.0, 5.0, 2.0) - 1.0).abs() < EPSILON);
        assert!((hermite(1.0, 1.0, 2.0, 5.0, 2.0) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn hermite_linear_case() {
        // p0=0, m0=1, p1=1, m1=1 → straight line
        let v = hermite(0.5, 0.0, 1.0, 1.0, 1.0);
        assert!((v - 0.5).abs() < EPSILON, "v={v}");
    }

    #[test]
    fn hermite_deterministic() {
        let a = hermite(0.3, 0.0, 1.0, 1.0, 0.0);
        let b = hermite(0.3, 0.0, 1.0, 1.0, 0.0);
        assert_eq!(a, b);
    }

    #[test]
    fn hermite_3d_endpoints() {
        let p0 = (0.0_f32, 0.0, 0.0);
        let m0 = (1.0, 0.0, 0.0);
        let p1 = (1.0, 1.0, 0.0);
        let m1 = (1.0, 0.0, 0.0);
        let start = hermite_3d(0.0, p0, m0, p1, m1);
        assert!((start.0 - p0.0).abs() < EPSILON);
        assert!((start.1 - p0.1).abs() < EPSILON);
    }

    // ── catmull_rom ───────────────────────────────────────────────────────────

    #[test]
    fn catmull_rom_passes_through_inner_points() {
        // t=0 → p1, t=1 → p2
        assert!((catmull_rom(0.0, 0.0, 1.0, 2.0, 3.0) - 1.0).abs() < EPSILON);
        assert!((catmull_rom(1.0, 0.0, 1.0, 2.0, 3.0) - 2.0).abs() < EPSILON);
    }

    #[test]
    fn catmull_rom_midpoint_collinear() {
        // Uniform spacing: p0=0, p1=1, p2=2, p3=3 → linear
        let v = catmull_rom(0.5, 0.0, 1.0, 2.0, 3.0);
        assert!((v - 1.5).abs() < EPSILON, "v={v}");
    }

    #[test]
    fn catmull_rom_deterministic() {
        let a = catmull_rom(0.4, 0.0, 1.0, 0.5, 0.0);
        let b = catmull_rom(0.4, 0.0, 1.0, 0.5, 0.0);
        assert_eq!(a, b);
    }

    #[test]
    fn catmull_rom_3d_passes_through_p1() {
        let p = catmull_rom_3d(0.0, (0.0, 0.0, 0.0), (1.0, 2.0, 3.0), (3.0, 1.0, 0.0), (5.0, 0.0, 0.0));
        assert!((p.0 - 1.0).abs() < EPSILON);
        assert!((p.1 - 2.0).abs() < EPSILON);
    }

    // ── b_spline_cubic ────────────────────────────────────────────────────────

    #[test]
    fn b_spline_cubic_collinear_midpoint() {
        // Uniform collinear points → midpoint on the line
        let v = b_spline_cubic(0.5, 0.0, 1.0, 2.0, 3.0);
        assert!((v - 1.5).abs() < EPSILON, "v={v}");
    }

    #[test]
    fn b_spline_cubic_start_knot() {
        // At t=0: (p0 + 4*p1 + p2) / 6
        let expected = (0.0 + 4.0 * 1.0 + 2.0) / 6.0;
        let v = b_spline_cubic(0.0, 0.0, 1.0, 2.0, 3.0);
        assert!((v - expected).abs() < EPSILON, "v={v}, expected={expected}");
    }

    #[test]
    fn b_spline_cubic_end_knot() {
        // At t=1: (p1 + 4*p2 + p3) / 6
        let expected = (1.0 + 4.0 * 2.0 + 3.0) / 6.0;
        let v = b_spline_cubic(1.0, 0.0, 1.0, 2.0, 3.0);
        assert!((v - expected).abs() < EPSILON, "v={v}, expected={expected}");
    }

    #[test]
    fn b_spline_cubic_deterministic() {
        let a = b_spline_cubic(0.3, 0.0, 1.0, 2.0, 3.0);
        let b = b_spline_cubic(0.3, 0.0, 1.0, 2.0, 3.0);
        assert_eq!(a, b);
    }

    #[test]
    fn b_spline_cubic_3d_midpoint() {
        let p = b_spline_cubic_3d(0.5, (0.0,0.0,0.0), (1.0,0.0,0.0), (2.0,0.0,0.0), (3.0,0.0,0.0));
        assert!((p.0 - 1.5).abs() < EPSILON);
    }

    // ── slerp ─────────────────────────────────────────────────────────────────

    #[test]
    fn slerp_t0_returns_q0() {
        let q0 = (0.0, 0.0, 0.0, 1.0_f32);
        let q1 = (0.0, 0.0, 1.0, 0.0_f32);
        let r = slerp(0.0, q0, q1);
        assert!((r.0 - q0.0).abs() < EPSILON);
        assert!((r.3 - q0.3).abs() < EPSILON);
    }

    #[test]
    fn slerp_t1_returns_q1() {
        let q0 = (0.0, 0.0, 0.0, 1.0_f32);
        let q1 = (0.0, 0.0, 1.0, 0.0_f32);
        let r = slerp(1.0, q0, q1);
        assert!((r.2 - q1.2).abs() < EPSILON);
        assert!((r.3 - q1.3).abs() < EPSILON);
    }

    #[test]
    fn slerp_preserves_unit_length() {
        let q0 = (0.0, 0.0, 0.0, 1.0_f32);
        let q1 = (0.0, 1.0, 0.0, 0.0_f32);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let r = slerp(t, q0, q1);
            let len_sq = r.0 * r.0 + r.1 * r.1 + r.2 * r.2 + r.3 * r.3;
            assert!((len_sq - 1.0).abs() < 1e-3, "t={t} len_sq={len_sq}");
        }
    }

    #[test]
    fn slerp_deterministic() {
        let q0 = (0.0_f32, 0.0, 0.0, 1.0);
        let q1 = (0.0_f32, 0.0, 1.0, 0.0);
        let a = slerp(0.5, q0, q1);
        let b = slerp(0.5, q0, q1);
        assert_eq!(a, b);
    }

    #[test]
    fn slerp_halfway_is_equidistant() {
        // Identity → 90° rotation: midpoint should be a 45° rotation
        let q0 = (0.0_f32, 0.0, 0.0, 1.0); // identity
        let q1 = (0.0_f32, 0.0, 1.0, 0.0); // 180° around z
        let mid = slerp(0.5, q0, q1);
        // Should be 90° around z: (0, 0, sin(45°), cos(45°))
        let expected_w = std::f32::consts::FRAC_1_SQRT_2;
        assert!((mid.3.abs() - expected_w).abs() < 1e-3, "w={}", mid.3);
    }

    // ── degenerate control points (all equal) ─────────────────────────────────

    #[test]
    fn bezier_quadratic_degenerate_returns_constant() {
        assert!((bezier_quadratic(0.5, 3.0, 3.0, 3.0) - 3.0).abs() < EPSILON);
    }

    #[test]
    fn bezier_cubic_degenerate_returns_constant() {
        assert!((bezier_cubic(0.5, 2.0, 2.0, 2.0, 2.0) - 2.0).abs() < EPSILON);
    }

    #[test]
    fn catmull_rom_degenerate_returns_constant() {
        assert!((catmull_rom(0.5, 5.0, 5.0, 5.0, 5.0) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn hermite_degenerate_zero_tangents_returns_endpoints() {
        // p0=p1, m0=m1=0 → flat line
        assert!((hermite(0.0, 1.0, 0.0, 1.0, 0.0) - 1.0).abs() < EPSILON);
        assert!((hermite(1.0, 1.0, 0.0, 1.0, 0.0) - 1.0).abs() < EPSILON);
        assert!((hermite(0.5, 1.0, 0.0, 1.0, 0.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn b_spline_cubic_degenerate_returns_constant() {
        assert!((b_spline_cubic(0.5, 4.0, 4.0, 4.0, 4.0) - 4.0).abs() < EPSILON);
    }

    // ── t outside [0, 1] ──────────────────────────────────────────────────────

    #[test]
    fn bezier_cubic_extrapolates_beyond_t1() {
        // t > 1 → extrapolation, result should be finite
        assert!(bezier_cubic(1.5, 0.0, 0.3, 0.7, 1.0).is_finite());
    }

    #[test]
    fn catmull_rom_extrapolates_below_t0() {
        assert!(catmull_rom(-0.5, 0.0, 1.0, 2.0, 3.0).is_finite());
    }

    // ── bezier_cubic_arc_length ──────────────────────────────────────────────

    #[test]
    fn arc_length_straight_line() {
        // Collinear points 0→3: arc length should equal 3.0
        let len = bezier_cubic_arc_length(0.0, 1.0, 2.0, 3.0, 200);
        assert!((len - 3.0).abs() < 0.01, "len={len}");
    }

    #[test]
    fn arc_length_curve_exceeds_chord() {
        // Curved bezier: arc length should be greater than chord length |p3 - p0| = 1.0
        let len = bezier_cubic_arc_length(0.0, 5.0, -5.0, 1.0, 200);
        assert!(len > 1.0, "curve arc length {len} should exceed chord 1.0");
    }

    #[test]
    fn arc_length_degenerate_zero() {
        // All points the same → arc length = 0
        let len = bezier_cubic_arc_length(2.0, 2.0, 2.0, 2.0, 100);
        assert!(len.abs() < EPSILON, "len={len}");
    }

    #[test]
    fn arc_length_3d_straight_line() {
        let len = bezier_cubic_arc_length_3d(
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
            200,
        );
        assert!((len - 3.0).abs() < 0.01, "len={len}");
    }

    #[test]
    fn arc_length_3d_curve_exceeds_chord() {
        let p0 = (0.0, 0.0, 0.0);
        let p1 = (0.0, 5.0, 0.0);
        let p2 = (0.0, -5.0, 0.0);
        let p3 = (1.0, 0.0, 0.0);
        let len = bezier_cubic_arc_length_3d(p0, p1, p2, p3, 200);
        let chord = ((p3.0 - p0.0).powi(2) + (p3.1 - p0.1).powi(2) + (p3.2 - p0.2).powi(2)).sqrt();
        assert!(len > chord, "arc {len} should exceed chord {chord}");
    }

    #[test]
    fn arc_length_3d_diagonal() {
        // Straight line from (0,0,0) to (1,1,1): length = sqrt(3)
        let len = bezier_cubic_arc_length_3d(
            (0.0, 0.0, 0.0),
            (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
            (2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0),
            (1.0, 1.0, 1.0),
            200,
        );
        let expected = 3.0_f32.sqrt();
        assert!((len - expected).abs() < 0.01, "len={len}, expected={expected}");
    }

    // ── bezier_cubic_t_at_length ─────────────────────────────────────────────

    #[test]
    fn t_at_length_midpoint_straight_line() {
        // Straight line 0→3: half-length (1.5) at t≈0.5
        let t = bezier_cubic_t_at_length(0.0, 1.0, 2.0, 3.0, 1.5, 100, 30);
        assert!((t - 0.5).abs() < 0.01, "t={t}");
    }

    #[test]
    fn t_at_length_zero_returns_zero() {
        let t = bezier_cubic_t_at_length(0.0, 1.0, 2.0, 3.0, 0.0, 100, 30);
        assert!(t < 0.01, "t={t}");
    }

    #[test]
    fn t_at_length_full_returns_one() {
        let total = bezier_cubic_arc_length(0.0, 1.0, 2.0, 3.0, 200);
        let t = bezier_cubic_t_at_length(0.0, 1.0, 2.0, 3.0, total, 100, 30);
        assert!((t - 1.0).abs() < 0.01, "t={t}");
    }

    #[test]
    fn t_at_length_3d_midpoint() {
        let t = bezier_cubic_t_at_length_3d(
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
            1.5,
            100,
            30,
        );
        assert!((t - 0.5).abs() < 0.01, "t={t}");
    }

    #[test]
    fn t_at_length_3d_deterministic() {
        let a = bezier_cubic_t_at_length_3d(
            (0.0, 0.0, 0.0),
            (0.0, 5.0, 0.0),
            (5.0, 0.0, 0.0),
            (5.0, 5.0, 0.0),
            3.0,
            100,
            30,
        );
        let b = bezier_cubic_t_at_length_3d(
            (0.0, 0.0, 0.0),
            (0.0, 5.0, 0.0),
            (5.0, 0.0, 0.0),
            (5.0, 5.0, 0.0),
            3.0,
            100,
            30,
        );
        assert_eq!(a, b);
    }
}
