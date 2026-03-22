#![allow(clippy::too_many_arguments)]
//! `prime-wasm` — WebAssembly bindings for the prime math library.
//!
//! Wraps all concrete (non-generic) functions from the prime crates with
//! `#[wasm_bindgen]` so they can be called from JavaScript/TypeScript.
//!
//! # Type mapping conventions
//!
//! | Rust type              | WASM/JS type                         |
//! |------------------------|--------------------------------------|
//! | `f32`                  | `number`                             |
//! | `(f32, f32)`           | `Float32Array` of length 2           |
//! | `(f32, f32, f32)`      | `Float32Array` of length 3           |
//! | `(f32, f32, f32, f32)` | `Float32Array` of length 4           |
//! | `Box<[f32]>`           | `Float32Array`                       |
//! | `Box<[f64]>`           | `Float64Array`                       |
//!
//! Tuple-valued functions return `Box<[f32]>` (maps to `Float32Array`).
//! Voronoi functions that take `&[(f32,f32)]` accept flat `&[f32]`.

use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// prime-noise
// ---------------------------------------------------------------------------

#[wasm_bindgen] pub fn value_noise_2d(x: f32, y: f32) -> f32 { prime_noise::value_noise_2d(x, y) }
#[wasm_bindgen] pub fn perlin_2d(x: f32, y: f32) -> f32 { prime_noise::perlin_2d(x, y) }
#[wasm_bindgen] pub fn fbm_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 { prime_noise::fbm_2d(x, y, octaves, lacunarity, gain) }
#[wasm_bindgen] pub fn worley_2d(x: f32, y: f32, seed: u32) -> f32 { prime_noise::worley_2d(x, y, seed) }
#[wasm_bindgen] pub fn value_noise_3d(x: f32, y: f32, z: f32) -> f32 { prime_noise::value_noise_3d(x, y, z) }
#[wasm_bindgen] pub fn perlin_3d(x: f32, y: f32, z: f32) -> f32 { prime_noise::perlin_3d(x, y, z) }
#[wasm_bindgen] pub fn fbm_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 { prime_noise::fbm_3d(x, y, z, octaves, lacunarity, gain) }
#[wasm_bindgen] pub fn simplex_2d(x: f32, y: f32) -> f32 { prime_noise::simplex_2d(x, y) }
#[wasm_bindgen] pub fn simplex_3d(x: f32, y: f32, z: f32) -> f32 { prime_noise::simplex_3d(x, y, z) }
#[wasm_bindgen] pub fn domain_warp_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32, warp_scale: f32) -> f32 { prime_noise::domain_warp_2d(x, y, octaves, lacunarity, gain, warp_scale) }
#[wasm_bindgen] pub fn domain_warp_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32, warp_scale: f32) -> f32 { prime_noise::domain_warp_3d(x, y, z, octaves, lacunarity, gain, warp_scale) }

// ---------------------------------------------------------------------------
// prime-interp
// ---------------------------------------------------------------------------

#[wasm_bindgen] pub fn lerp(a: f32, b: f32, t: f32) -> f32 { prime_interp::lerp(a, b, t) }
#[wasm_bindgen] pub fn inv_lerp(a: f32, b: f32, v: f32) -> f32 { prime_interp::inv_lerp(a, b, v) }
#[wasm_bindgen] pub fn remap(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 { prime_interp::remap(value, in_min, in_max, out_min, out_max) }
/// Smoothstep over [0, 1]. Convenience alias for smoothstep(0, 1, t).
#[wasm_bindgen] pub fn smoothstep(t: f32) -> f32 { prime_interp::smoothstep(0.0, 1.0, t) }
/// Smootherstep over [0, 1]. Convenience alias for smootherstep(0, 1, t).
#[wasm_bindgen] pub fn smootherstep(t: f32) -> f32 { prime_interp::smootherstep(0.0, 1.0, t) }
/// Smoothstep with explicit edge range.
#[wasm_bindgen] pub fn smoothstep_range(edge0: f32, edge1: f32, x: f32) -> f32 { prime_interp::smoothstep(edge0, edge1, x) }
/// Smootherstep with explicit edge range.
#[wasm_bindgen] pub fn smootherstep_range(edge0: f32, edge1: f32, x: f32) -> f32 { prime_interp::smootherstep(edge0, edge1, x) }
#[wasm_bindgen] pub fn clamp(value: f32, min: f32, max: f32) -> f32 { value.clamp(min, max) }
#[wasm_bindgen] pub fn ease_in_quad(t: f32) -> f32 { prime_interp::ease_in_quad(t) }
#[wasm_bindgen] pub fn ease_out_quad(t: f32) -> f32 { prime_interp::ease_out_quad(t) }
#[wasm_bindgen] pub fn ease_in_out_quad(t: f32) -> f32 { prime_interp::ease_in_out_quad(t) }
#[wasm_bindgen] pub fn ease_in_cubic(t: f32) -> f32 { prime_interp::ease_in_cubic(t) }
#[wasm_bindgen] pub fn ease_out_cubic(t: f32) -> f32 { prime_interp::ease_out_cubic(t) }
#[wasm_bindgen] pub fn ease_in_out_cubic(t: f32) -> f32 { prime_interp::ease_in_out_cubic(t) }
#[wasm_bindgen] pub fn ease_in_quart(t: f32) -> f32 { prime_interp::ease_in_quart(t) }
#[wasm_bindgen] pub fn ease_out_quart(t: f32) -> f32 { prime_interp::ease_out_quart(t) }
#[wasm_bindgen] pub fn ease_in_out_quart(t: f32) -> f32 { prime_interp::ease_in_out_quart(t) }
#[wasm_bindgen] pub fn ease_in_quint(t: f32) -> f32 { prime_interp::ease_in_quint(t) }
#[wasm_bindgen] pub fn ease_out_quint(t: f32) -> f32 { prime_interp::ease_out_quint(t) }
#[wasm_bindgen] pub fn ease_in_out_quint(t: f32) -> f32 { prime_interp::ease_in_out_quint(t) }
#[wasm_bindgen] pub fn ease_in_sine(t: f32) -> f32 { prime_interp::ease_in_sine(t) }
#[wasm_bindgen] pub fn ease_out_sine(t: f32) -> f32 { prime_interp::ease_out_sine(t) }
#[wasm_bindgen] pub fn ease_in_out_sine(t: f32) -> f32 { prime_interp::ease_in_out_sine(t) }
#[wasm_bindgen] pub fn ease_in_expo(t: f32) -> f32 { prime_interp::ease_in_expo(t) }
#[wasm_bindgen] pub fn ease_out_expo(t: f32) -> f32 { prime_interp::ease_out_expo(t) }
#[wasm_bindgen] pub fn ease_in_out_expo(t: f32) -> f32 { prime_interp::ease_in_out_expo(t) }
#[wasm_bindgen] pub fn ease_in_circ(t: f32) -> f32 { prime_interp::ease_in_circ(t) }
#[wasm_bindgen] pub fn ease_out_circ(t: f32) -> f32 { prime_interp::ease_out_circ(t) }
#[wasm_bindgen] pub fn ease_in_out_circ(t: f32) -> f32 { prime_interp::ease_in_out_circ(t) }
#[wasm_bindgen] pub fn ease_in_elastic(t: f32) -> f32 { prime_interp::ease_in_elastic(t) }
#[wasm_bindgen] pub fn ease_out_elastic(t: f32) -> f32 { prime_interp::ease_out_elastic(t) }
#[wasm_bindgen] pub fn ease_in_out_elastic(t: f32) -> f32 { prime_interp::ease_in_out_elastic(t) }
#[wasm_bindgen] pub fn ease_in_bounce(t: f32) -> f32 { prime_interp::ease_in_bounce(t) }
#[wasm_bindgen] pub fn ease_out_bounce(t: f32) -> f32 { prime_interp::ease_out_bounce(t) }
#[wasm_bindgen] pub fn ease_in_out_bounce(t: f32) -> f32 { prime_interp::ease_in_out_bounce(t) }

// ---------------------------------------------------------------------------
// prime-color  (Oklab, sRGB, HSL)
// ---------------------------------------------------------------------------

/// Convert sRGB `[r, g, b]` (0-1 each) to Oklab `[L, a, b]`.
#[wasm_bindgen]
pub fn srgb_to_oklab(r: f32, g: f32, b: f32) -> Box<[f32]> {
    let (l, a, b_) = prime_color::srgb_to_oklab(r, g, b);
    vec![l, a, b_].into_boxed_slice()
}

/// Convert Oklab `[L, a, b]` to sRGB `[r, g, b]`.
#[wasm_bindgen]
pub fn oklab_to_srgb(l: f32, a: f32, b: f32) -> Box<[f32]> {
    let (r, g, b_) = prime_color::oklab_to_srgb(l, a, b);
    vec![r, g, b_].into_boxed_slice()
}

/// Convert sRGB to linear RGB (gamma decode).
#[wasm_bindgen]
pub fn srgb_to_linear(r: f32, g: f32, b: f32) -> Box<[f32]> {
    let (lr, lg, lb) = prime_color::srgb_to_linear(r, g, b);
    vec![lr, lg, lb].into_boxed_slice()
}

/// Convert linear RGB to sRGB (gamma encode).
#[wasm_bindgen]
pub fn linear_to_srgb(r: f32, g: f32, b: f32) -> Box<[f32]> {
    let (sr, sg, sb) = prime_color::linear_to_srgb(r, g, b);
    vec![sr, sg, sb].into_boxed_slice()
}

/// Convert HSL `[h, s, l]` to sRGB `[r, g, b]`.
#[wasm_bindgen]
pub fn hsl_to_srgb(h: f32, s: f32, l: f32) -> Box<[f32]> {
    let (r, g, b) = prime_color::hsl_to_srgb(h, s, l);
    vec![r, g, b].into_boxed_slice()
}

/// Convert sRGB to HSL.
#[wasm_bindgen]
pub fn srgb_to_hsl(r: f32, g: f32, b: f32) -> Box<[f32]> {
    let (h, s, l) = prime_color::srgb_to_hsl(r, g, b);
    vec![h, s, l].into_boxed_slice()
}

/// Oklab perceptual interpolation between two sRGB colors. Returns interpolated sRGB `[r, g, b]`.
#[wasm_bindgen]
pub fn oklab_mix(r0: f32, g0: f32, b0: f32, r1: f32, g1: f32, b1: f32, t: f32) -> Box<[f32]> {
    let (r, g, b) = prime_color::oklab_mix(r0, g0, b0, r1, g1, b1, t);
    vec![r, g, b].into_boxed_slice()
}

// ---------------------------------------------------------------------------
// prime-sdf
// ---------------------------------------------------------------------------

/// Sphere SDF. `p` and `center` are world-space coords.
#[wasm_bindgen]
pub fn sdf_sphere(px: f32, py: f32, pz: f32, cx: f32, cy: f32, cz: f32, radius: f32) -> f32 {
    prime_sdf::sphere(glam::Vec3::new(px, py, pz), glam::Vec3::new(cx, cy, cz), radius)
}

/// Axis-aligned box SDF. `half_extents` is half-size on each axis.
#[wasm_bindgen]
pub fn sdf_box(px: f32, py: f32, pz: f32, cx: f32, cy: f32, cz: f32, hx: f32, hy: f32, hz: f32) -> f32 {
    prime_sdf::box_3d(glam::Vec3::new(px, py, pz), glam::Vec3::new(cx, cy, cz), glam::Vec3::new(hx, hy, hz))
}

/// Torus SDF. `major` = ring radius, `minor` = tube radius.
#[wasm_bindgen]
pub fn sdf_torus(px: f32, py: f32, pz: f32, cx: f32, cy: f32, cz: f32, major: f32, minor: f32) -> f32 {
    prime_sdf::torus(glam::Vec3::new(px, py, pz), glam::Vec3::new(cx, cy, cz), major, minor)
}

/// Capsule SDF. `a` and `b` are endpoint centers.
#[wasm_bindgen]
pub fn sdf_capsule(px: f32, py: f32, pz: f32, ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32, r: f32) -> f32 {
    prime_sdf::capsule_3d(glam::Vec3::new(px, py, pz), glam::Vec3::new(ax, ay, az), glam::Vec3::new(bx, by, bz), r)
}

/// Plane SDF. `normal` must be unit length.
#[wasm_bindgen]
pub fn sdf_plane(px: f32, py: f32, pz: f32, nx: f32, ny: f32, nz: f32, offset: f32) -> f32 {
    prime_sdf::plane(glam::Vec3::new(px, py, pz), glam::Vec3::new(nx, ny, nz), offset)
}

#[wasm_bindgen] pub fn sdf_union(a: f32, b: f32) -> f32 { prime_sdf::union(a, b) }
#[wasm_bindgen] pub fn sdf_subtract(a: f32, b: f32) -> f32 { prime_sdf::subtract(a, b) }
#[wasm_bindgen] pub fn sdf_intersection(a: f32, b: f32) -> f32 { prime_sdf::intersection(a, b) }
#[wasm_bindgen] pub fn sdf_smooth_union(a: f32, b: f32, k: f32) -> f32 { prime_sdf::smooth_union(a, b, k) }
#[wasm_bindgen] pub fn sdf_smooth_subtract(a: f32, b: f32, k: f32) -> f32 { prime_sdf::smooth_subtract(a, b, k) }
#[wasm_bindgen] pub fn sdf_smooth_intersection(a: f32, b: f32, k: f32) -> f32 { prime_sdf::smooth_intersection(a, b, k) }

// ---------------------------------------------------------------------------
// prime-signal
// ---------------------------------------------------------------------------

/// Smoothdamp step. Returns `[new_value, new_velocity]`.
#[wasm_bindgen]
pub fn smoothdamp(current: f32, target: f32, velocity: f32, smooth_time: f32, delta_time: f32) -> Box<[f32]> {
    let (v, vel) = prime_signal::smoothdamp(current, target, velocity, smooth_time, delta_time);
    vec![v, vel].into_boxed_slice()
}

/// Spring step. Returns `[new_value, new_velocity]`.
#[wasm_bindgen]
pub fn spring(value: f32, velocity: f32, target: f32, stiffness: f32, damping: f32, dt: f32) -> Box<[f32]> {
    let (v, vel) = prime_signal::spring(value, velocity, target, stiffness, damping, dt);
    vec![v, vel].into_boxed_slice()
}

/// One-pole low-pass filter. Returns new filtered value.
/// `time_constant` controls response speed in seconds.
#[wasm_bindgen]
pub fn low_pass(previous: f32, input: f32, time_constant: f32, dt: f32) -> f32 {
    prime_signal::low_pass(previous, input, time_constant, dt)
}

/// Deadzone + curve. `curve` = 1.0 for linear remapping past the deadzone.
#[wasm_bindgen]
pub fn deadzone(value: f32, deadzone_size: f32, curve: f32) -> f32 {
    prime_signal::deadzone(value, deadzone_size, curve)
}

// ---------------------------------------------------------------------------
// prime-random
// ---------------------------------------------------------------------------

/// Advance PCG seed and return `[sample_0_to_1, next_seed]` as f64 (JS number).
/// Seed must stay below 2^32.
#[wasm_bindgen]
pub fn prng_next(seed: f64) -> Box<[f64]> {
    let (v, next) = prime_random::prng_next(seed as u32);
    vec![v as f64, next as f64].into_boxed_slice()
}

/// Sample uniform f32 in `[min, max]`. Returns `[value, next_seed]`.
#[wasm_bindgen]
pub fn prng_range_f32(seed: f64, min: f32, max: f32) -> Box<[f64]> {
    let (v, next) = prime_random::prng_range_f32(seed as u32, min, max);
    vec![v as f64, next as f64].into_boxed_slice()
}

/// Sample uniform integer in `[0, n)`. Returns `[index, next_seed]`.
#[wasm_bindgen]
pub fn prng_range_usize(seed: f64, n: f64) -> Box<[f64]> {
    let (v, next) = prime_random::prng_range_usize(seed as u32, n as usize);
    vec![v as f64, next as f64].into_boxed_slice()
}

/// Bernoulli trial with probability `p`. Returns `[0 or 1, next_seed]`.
#[wasm_bindgen]
pub fn prng_bool(seed: f64, p: f32) -> Box<[f64]> {
    let (v, next) = prime_random::prng_bool(seed as u32, p);
    vec![if v { 1.0 } else { 0.0 }, next as f64].into_boxed_slice()
}

/// Poisson-disk 2D sampling. Returns flat `[x0, y0, x1, y1, ...]`.
#[wasm_bindgen]
pub fn poisson_disk_2d(seed: f64, width: f32, height: f32, min_dist: f32, max_attempts: f64) -> Box<[f32]> {
    let pts = prime_random::poisson_disk_2d(seed as u32, width, height, min_dist, max_attempts as usize);
    pts.into_iter().flat_map(|(x, y)| [x, y]).collect::<Vec<f32>>().into_boxed_slice()
}

// ---------------------------------------------------------------------------
// prime-osc
// ---------------------------------------------------------------------------

#[wasm_bindgen] pub fn lfo_sine(phase: f32) -> f32 { prime_osc::lfo_sine(phase) }
#[wasm_bindgen] pub fn lfo_triangle(phase: f32) -> f32 { prime_osc::lfo_triangle(phase) }
#[wasm_bindgen] pub fn lfo_sawtooth(phase: f32) -> f32 { prime_osc::lfo_sawtooth(phase) }
/// Square LFO with pulse width [0, 1]. Use 0.5 for 50% duty cycle.
#[wasm_bindgen] pub fn lfo_square(phase: f32, width: f32) -> f32 { prime_osc::lfo_square(phase, width) }

/// ADSR envelope step.
///
/// State is packed as `[stage, value, elapsed]` where stage is:
/// 0 = Done, 1 = Attack, 2 = Decay, 3 = Sustain, 4 = Release.
///
/// Returns `[level, new_stage, new_value, new_elapsed]`.
#[wasm_bindgen]
pub fn adsr_step(
    stage: u32, state_value: f32, elapsed: f32,
    attack: f32, decay: f32, sustain: f32, release: f32,
    gate: bool, dt: f32,
) -> Box<[f32]> {
    use prime_osc::{AdsrState, AdsrStage, AdsrParams};
    let adsr_stage = match stage {
        1 => AdsrStage::Attack,
        2 => AdsrStage::Decay,
        3 => AdsrStage::Sustain,
        4 => AdsrStage::Release,
        _ => AdsrStage::Done,
    };
    let state = AdsrState { stage: adsr_stage, value: state_value, elapsed };
    let params = AdsrParams { attack, decay, sustain, release };
    let (level, new_state) = prime_osc::adsr_step(state, &params, gate, dt);
    let new_stage = match new_state.stage {
        AdsrStage::Attack  => 1.0_f32,
        AdsrStage::Decay   => 2.0,
        AdsrStage::Sustain => 3.0,
        AdsrStage::Release => 4.0,
        AdsrStage::Done    => 0.0,
    };
    vec![level, new_stage, new_state.value, new_state.elapsed].into_boxed_slice()
}

// ---------------------------------------------------------------------------
// prime-splines
// ---------------------------------------------------------------------------

#[wasm_bindgen] pub fn bezier_quadratic(t: f32, p0: f32, p1: f32, p2: f32) -> f32 { prime_splines::bezier_quadratic(t, p0, p1, p2) }
#[wasm_bindgen] pub fn bezier_cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 { prime_splines::bezier_cubic(t, p0, p1, p2, p3) }
#[wasm_bindgen] pub fn hermite(t: f32, p0: f32, m0: f32, p1: f32, m1: f32) -> f32 { prime_splines::hermite(t, p0, m0, p1, m1) }
#[wasm_bindgen] pub fn catmull_rom(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 { prime_splines::catmull_rom(t, p0, p1, p2, p3) }
#[wasm_bindgen] pub fn b_spline_cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 { prime_splines::b_spline_cubic(t, p0, p1, p2, p3) }

/// Slerp for unit quaternions `(x, y, z, w)`. Returns `[x, y, z, w]`.
#[wasm_bindgen]
pub fn slerp(t: f32, q0x: f32, q0y: f32, q0z: f32, q0w: f32, q1x: f32, q1y: f32, q1z: f32, q1w: f32) -> Box<[f32]> {
    let (rx, ry, rz, rw) = prime_splines::slerp(t, (q0x, q0y, q0z, q0w), (q1x, q1y, q1z, q1w));
    vec![rx, ry, rz, rw].into_boxed_slice()
}

// ---------------------------------------------------------------------------
// prime-dynamics
// ---------------------------------------------------------------------------

/// Lorenz step. Returns `[x, y, z]`.
#[wasm_bindgen]
pub fn lorenz_step(x: f32, y: f32, z: f32, sigma: f32, rho: f32, beta: f32, dt: f32) -> Box<[f32]> {
    let (x1, y1, z1) = prime_dynamics::lorenz_step((x, y, z), sigma, rho, beta, dt);
    vec![x1, y1, z1].into_boxed_slice()
}

/// Rössler step. Returns `[x, y, z]`.
#[wasm_bindgen]
pub fn rossler_step(x: f32, y: f32, z: f32, a: f32, b: f32, c: f32, dt: f32) -> Box<[f32]> {
    let (x1, y1, z1) = prime_dynamics::rossler_step((x, y, z), a, b, c, dt);
    vec![x1, y1, z1].into_boxed_slice()
}

/// Duffing step. Returns `[x, v]`.
#[wasm_bindgen]
pub fn duffing_step(x: f32, v: f32, t: f32, delta: f32, alpha: f32, beta: f32, gamma: f32, omega: f32, dt: f32) -> Box<[f32]> {
    let (x1, v1) = prime_dynamics::duffing_step(
        (x, v), t,
        prime_dynamics::DuffingParams { delta, alpha, beta, gamma, omega },
        dt,
    );
    vec![x1, v1].into_boxed_slice()
}

/// Euler step for a linear ODE `dy/dt = k*y`. Returns new scalar state.
#[wasm_bindgen]
pub fn euler_step_linear(state: f32, t: f32, dt: f32, k: f32) -> f32 {
    prime_dynamics::euler_step(state, t, dt, |_t, s| k * s)
}

// ---------------------------------------------------------------------------
// prime-diffusion
// ---------------------------------------------------------------------------

#[wasm_bindgen] pub fn ou_step(x: f32, mu: f32, theta: f32, sigma: f32, dt: f32, w: f32) -> f32 { prime_diffusion::ou_step(x, mu, theta, sigma, dt, w) }
#[wasm_bindgen] pub fn gbm_step(x: f32, mu: f32, sigma: f32, dt: f32, w: f32) -> f32 { prime_diffusion::gbm_step(x, mu, sigma, dt, w) }

// ---------------------------------------------------------------------------
// prime-voronoi
// ---------------------------------------------------------------------------

/// Find nearest Voronoi seed. `seeds_flat` is `[x0, y0, x1, y1, ...]`.
/// Returns `[index, distance]` or `[-1, 0]` if empty.
#[wasm_bindgen]
pub fn voronoi_nearest_2d(qx: f32, qy: f32, seeds_flat: &[f32]) -> Box<[f32]> {
    let seeds: Vec<(f32, f32)> = seeds_flat.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    match prime_voronoi::voronoi_nearest_2d((qx, qy), &seeds) {
        Some((idx, dist)) => vec![idx as f32, dist].into_boxed_slice(),
        None => vec![-1.0, 0.0].into_boxed_slice(),
    }
}

/// F1 and F2 distances. `seeds_flat` is `[x0, y0, x1, y1, ...]`.
/// Returns `[f1, f2]` or `[-1, -1]` if empty.
#[wasm_bindgen]
pub fn voronoi_f1_f2_2d(qx: f32, qy: f32, seeds_flat: &[f32]) -> Box<[f32]> {
    let seeds: Vec<(f32, f32)> = seeds_flat.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    match prime_voronoi::voronoi_f1_f2_2d((qx, qy), &seeds) {
        Some((f1, f2)) => vec![f1, f2].into_boxed_slice(),
        None => vec![-1.0, -1.0].into_boxed_slice(),
    }
}

/// Lloyd relaxation step. Both `seeds_flat` and `samples_flat` are `[x0, y0, x1, y1, ...]`.
/// Returns new seeds as flat `[x0, y0, x1, y1, ...]`.
#[wasm_bindgen]
pub fn lloyd_relax_step_2d(seeds_flat: &[f32], samples_flat: &[f32]) -> Box<[f32]> {
    let seeds: Vec<(f32, f32)> = seeds_flat.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    let samples: Vec<(f32, f32)> = samples_flat.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    let relaxed = prime_voronoi::lloyd_relax_step_2d(&seeds, &samples);
    relaxed.into_iter().flat_map(|(x, y)| [x, y]).collect::<Vec<f32>>().into_boxed_slice()
}
