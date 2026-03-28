//! prime-noise — Noise functions: value noise, Perlin, FBM, Worley, Simplex, domain warp.
//!
//! All public functions are pure (LOAD + COMPUTE only). No `&mut`, no side effects,
//! no hidden state. Same inputs always produce the same output.
//!
//! # Temporal Assembly Model
//! Every function follows the temporal assembly thesis:
//! - **LOAD** — read function parameters
//! - **COMPUTE** — pure math
//!
//! STORE (`&mut`) and JUMP (mutating loops) do not exist here.

// ---------------------------------------------------------------------------
// Internal hash utilities
// ---------------------------------------------------------------------------

/// Mulberry32-variant hash: maps a `u32` to a pseudo-random `u32`.
///
/// Used internally to construct lattice hashes without external RNG crates.
#[inline(always)]
fn hash_u32(x: u32) -> u32 {
    let x = x.wrapping_add(0x6D2B79F5);
    let x = (x ^ (x >> 15)).wrapping_mul(x | 1);
    let x = x ^ x.wrapping_add((x ^ (x >> 7)).wrapping_mul(x | 61));
    x ^ (x >> 14)
}

/// Hash a 2-D integer lattice coordinate to a `f32` in [0, 1].
#[inline(always)]
fn hash_2d(xi: i32, yi: i32) -> f32 {
    let h = hash_u32(hash_u32(xi as u32).wrapping_add(yi as u32));
    (h as f32) / (u32::MAX as f32)
}

/// Hash a 2-D integer lattice coordinate with an additional seed word.
///
/// Used by `worley_2d` so that different seeds yield independent feature-point fields.
#[inline(always)]
fn hash_2d_seeded(xi: i32, yi: i32, seed: u32) -> f32 {
    let h = hash_u32(
        hash_u32(xi as u32)
            .wrapping_add(yi as u32)
            .wrapping_add(seed),
    );
    (h as f32) / (u32::MAX as f32)
}

// ---------------------------------------------------------------------------
// Interpolation helpers (LOAD + COMPUTE only)
// ---------------------------------------------------------------------------

/// Smoothstep fade curve: `t*t*(3 - 2*t)`.
///
/// Maps `t` in [0, 1] to [0, 1] with zero derivative at both endpoints.
#[inline(always)]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolation: `a + t * (b - a)`.
#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

// ---------------------------------------------------------------------------
// Gradient table (Perlin noise)
// ---------------------------------------------------------------------------

/// Eight unit gradient vectors evenly spaced around the circle.
///
/// Index is derived from a lattice hash, giving eight possible dot-product orientations.
const FRAC_1_SQRT_2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const GRADIENTS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (0.0, 1.0),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (-1.0, 0.0),
    (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
    (0.0, -1.0),
    (FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
];

/// Map a lattice hash value in [0, 1] to one of the eight gradient vectors.
#[inline(always)]
fn gradient(h: f32) -> (f32, f32) {
    let idx = (h * 8.0) as usize % 8;
    GRADIENTS[idx]
}

/// Dot product of gradient `g` with offset `(dx, dy)`.
#[inline(always)]
fn grad_dot(g: (f32, f32), dx: f32, dy: f32) -> f32 {
    g.0 * dx + g.1 * dy
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Smooth value noise at a 2-D point.
///
/// # Math
///
/// ```text
/// Given continuous point (x, y):
///   xi = floor(x),  yi = floor(y)
///   fx = fract(x),  fy = fract(y)
///   tx = smoothstep(fx),  ty = smoothstep(fy)
///
///   v00 = hash_2d(xi,   yi  )
///   v10 = hash_2d(xi+1, yi  )
///   v01 = hash_2d(xi,   yi+1)
///   v11 = hash_2d(xi+1, yi+1)
///
///   result = lerp(lerp(v00, v10, tx), lerp(v01, v11, tx), ty)
/// ```
///
/// # Arguments
/// * `x` — horizontal coordinate (any finite `f32`)
/// * `y` — vertical coordinate (any finite `f32`)
///
/// # Returns
/// A value in [0, 1].
///
/// # Edge cases
/// * At exact integer lattice points `fx = 0`, so the result equals `hash_2d(xi, yi)`.
/// * Large coordinates that overflow `i32` when cast from `f32` produce wrapping
///   integer values — the function remains deterministic but the visual period
///   compresses. Keep coordinates below ±2³¹ in practice.
///
/// # Example
/// ```rust
/// use prime_noise::value_noise_2d;
/// let v = value_noise_2d(1.5, 2.3);
/// assert!(v >= 0.0 && v <= 1.0);
/// ```
pub fn value_noise_2d(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    let tx = smoothstep(fx);
    let ty = smoothstep(fy);

    let v00 = hash_2d(xi, yi);
    let v10 = hash_2d(xi.wrapping_add(1), yi);
    let v01 = hash_2d(xi, yi.wrapping_add(1));
    let v11 = hash_2d(xi.wrapping_add(1), yi.wrapping_add(1));

    let bottom = lerp(v00, v10, tx);
    let top = lerp(v01, v11, tx);
    lerp(bottom, top, ty)
}

/// Classic Perlin gradient noise at a 2-D point.
///
/// # Math
///
/// ```text
/// Given continuous point (x, y):
///   xi = floor(x),  yi = floor(y)
///   fx = fract(x),  fy = fract(y)
///   tx = smoothstep(fx),  ty = smoothstep(fy)
///
///   For each of the four lattice corners (xi+dx, yi+dy), dx,dy ∈ {0,1}:
///     g = gradient(hash_2d(xi+dx, yi+dy))
///     n  = dot(g, (fx-dx, fy-dy))
///
///   result = bilinear_lerp(n00, n10, n01, n11, tx, ty)
/// ```
///
/// The gradient dot products produce values in approximately [-√2/2, √2/2] ≈ [-0.707, 0.707]
/// before blending, so the output is approximately in [-1, 1] but exact bounds are not
/// guaranteed.
///
/// # Arguments
/// * `x` — horizontal coordinate (any finite `f32`)
/// * `y` — vertical coordinate (any finite `f32`)
///
/// # Returns
/// A value approximately in [-1, 1]. Not clamped.
///
/// # Edge cases
/// * At exact integer lattice points all offset components are 0, so all four dot
///   products are 0 and the function returns 0.0.
/// * Coordinates beyond ±2³¹ produce wrapping behaviour — deterministic but visually
///   discontinuous.
///
/// # Example
/// ```rust
/// use prime_noise::perlin_2d;
/// let v = perlin_2d(0.5, 0.5);
/// assert!(v >= -1.0 && v <= 1.0);
///
/// // Returns exactly 0 at integer lattice points
/// assert_eq!(perlin_2d(2.0, 3.0), 0.0);
/// ```
pub fn perlin_2d(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    let tx = smoothstep(fx);
    let ty = smoothstep(fy);

    let g00 = gradient(hash_2d(xi, yi));
    let g10 = gradient(hash_2d(xi.wrapping_add(1), yi));
    let g01 = gradient(hash_2d(xi, yi.wrapping_add(1)));
    let g11 = gradient(hash_2d(xi.wrapping_add(1), yi.wrapping_add(1)));

    let n00 = grad_dot(g00, fx, fy);
    let n10 = grad_dot(g10, fx - 1.0, fy);
    let n01 = grad_dot(g01, fx, fy - 1.0);
    let n11 = grad_dot(g11, fx - 1.0, fy - 1.0);

    let bottom = lerp(n00, n10, tx);
    let top = lerp(n01, n11, tx);
    lerp(bottom, top, ty)
}

/// Fractional Brownian Motion: layered octaves of Perlin noise.
///
/// # Math
///
/// ```text
/// result = Σ_{i=0}^{octaves-1} amplitude_i * perlin_2d(x * frequency_i, y * frequency_i)
///
/// where:
///   frequency_0   = 1.0
///   amplitude_0   = 1.0
///   frequency_i   = frequency_{i-1} * lacunarity
///   amplitude_i   = amplitude_{i-1} * gain
/// ```
///
/// The sum is computed by folding over the octave range — no mutable loop variable.
///
/// # Arguments
/// * `x`          — horizontal coordinate
/// * `y`          — vertical coordinate
/// * `octaves`    — number of noise layers (0 returns 0.0; typical range 1–8)
/// * `lacunarity` — frequency multiplier per octave (typically 2.0)
/// * `gain`       — amplitude multiplier per octave (typically 0.5)
///
/// # Returns
/// Sum of all octave contributions. With `lacunarity=2.0` and `gain=0.5` (geometric series)
/// the theoretical maximum amplitude is `1 * (1 - 0.5^octaves) / (1 - 0.5)` ≈ 2.0 for
/// large `octaves`. The actual range depends on parameters and is not clamped.
///
/// # Edge cases
/// * `octaves = 0` → returns 0.0.
/// * `gain = 0.0` → only the first octave contributes (if octaves > 0).
/// * `lacunarity = 1.0` → all octaves sample the same frequency; result is
///   `perlin_2d(x, y) * geometric_sum(gain, octaves)`.
///
/// # Example
/// ```rust
/// use prime_noise::fbm_2d;
/// let v = fbm_2d(0.3, 0.7, 6, 2.0, 0.5);
/// // Range is roughly [-2, 2] for standard params
/// assert!(v.abs() < 3.0);
/// ```
pub fn fbm_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    // Fold over octave indices: state = (accumulated_value, frequency, amplitude)
    let (value, _, _) = (0..octaves).fold((0.0_f32, 1.0_f32, 1.0_f32), |(acc, freq, amp), _| {
        let contribution = amp * perlin_2d(x * freq, y * freq);
        (acc + contribution, freq * lacunarity, amp * gain)
    });
    value
}

/// Worley (cellular) noise at a 2-D point.
///
/// # Math
///
/// ```text
/// Given continuous point (x, y):
///   cell = (floor(x), floor(y))
///
///   For each of the 9 neighbouring cells (cell + (dx, dy)), dx,dy ∈ {-1, 0, 1}:
///     fx_cell = hash_2d_seeded(cell.x+dx, cell.y+dy, seed)         — feature x offset in [0,1]
///     fy_cell = hash_2d_seeded(cell.x+dx, cell.y+dy, seed + 1)     — feature y offset in [0,1]
///     feature = (cell.x+dx + fx_cell, cell.y+dy + fy_cell)
///     dist    = euclidean_distance((x, y), feature)
///
///   result = min(dist) over all 9 cells, clamped to [0, 1]
/// ```
///
/// Searching 9 cells guarantees the nearest feature point is always found, because a
/// feature point at distance > √2 from the query point cannot be the nearest.
///
/// # Arguments
/// * `x`    — horizontal coordinate (any finite `f32`)
/// * `y`    — vertical coordinate (any finite `f32`)
/// * `seed` — unsigned 32-bit seed; different seeds give independent feature fields.
///   Seeds `seed` and `seed+1` are used internally for x and y offsets.
///
/// # Returns
/// Distance to the nearest feature point, in [0, 1] (clamped).
/// The maximum possible distance before clamping is approximately √2 ≈ 1.414.
///
/// # Edge cases
/// * At exact integer lattice corners the nearest feature may be very close (distance → 0).
/// * `seed = u32::MAX` internally wraps to `seed.wrapping_add(1)` for the y-axis hash
///   — still deterministic.
///
/// # Example
/// ```rust
/// use prime_noise::worley_2d;
/// let d = worley_2d(0.5, 0.5, 42);
/// assert!(d >= 0.0 && d <= 1.0);
/// ```
pub fn worley_2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;

    // Fold over the 9 neighbouring cells to find the minimum distance.
    // Offsets: dx ∈ {-1, 0, 1}, dy ∈ {-1, 0, 1} → 9 combinations.
    let min_dist = [
        (-1_i32, -1_i32),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 0),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ]
    .iter()
    .fold(f32::MAX, |min_so_far, &(dx, dy)| {
        let cx = xi + dx;
        let cy = yi + dy;

        // Two independent hashes for x and y offsets within the cell.
        let fx = hash_2d_seeded(cx, cy, seed);
        let fy = hash_2d_seeded(cx, cy, seed.wrapping_add(1));

        let feat_x = cx as f32 + fx;
        let feat_y = cy as f32 + fy;

        let ddx = x - feat_x;
        let ddy = y - feat_y;
        let dist = (ddx * ddx + ddy * ddy).sqrt();

        if dist < min_so_far {
            dist
        } else {
            min_so_far
        }
    });

    min_dist.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// 3D internal helpers
// ---------------------------------------------------------------------------

/// Hash a 3-D integer lattice coordinate to a `f32` in [0, 1].
#[inline(always)]
fn hash_3d(xi: i32, yi: i32, zi: i32) -> f32 {
    let h = hash_u32(
        hash_u32(hash_u32(xi as u32).wrapping_add(yi as u32)).wrapping_add(zi as u32),
    );
    (h as f32) / (u32::MAX as f32)
}

/// Twelve gradient vectors: midpoints of the edges of a unit cube.
///
/// Used by `perlin_3d` and `simplex_3d`.
const GRADIENTS_3D: [(f32, f32, f32); 12] = [
    (1.0, 1.0, 0.0),
    (-1.0, 1.0, 0.0),
    (1.0, -1.0, 0.0),
    (-1.0, -1.0, 0.0),
    (1.0, 0.0, 1.0),
    (-1.0, 0.0, 1.0),
    (1.0, 0.0, -1.0),
    (-1.0, 0.0, -1.0),
    (0.0, 1.0, 1.0),
    (0.0, -1.0, 1.0),
    (0.0, 1.0, -1.0),
    (0.0, -1.0, -1.0),
];

/// Map a lattice hash in [0, 1] to one of the twelve 3-D gradient vectors.
#[inline(always)]
fn gradient_3d(h: f32) -> (f32, f32, f32) {
    GRADIENTS_3D[(h * 12.0) as usize % 12]
}

/// Dot product of a 3-D gradient with offset `(dx, dy, dz)`.
#[inline(always)]
fn grad_dot_3d(g: (f32, f32, f32), dx: f32, dy: f32, dz: f32) -> f32 {
    g.0 * dx + g.1 * dy + g.2 * dz
}

// ---------------------------------------------------------------------------
// 3-D value noise
// ---------------------------------------------------------------------------

/// Smooth value noise at a 3-D point.
///
/// # Math
///
/// ```text
/// xi = floor(x), yi = floor(y), zi = floor(z)
/// fx = fract(x), fy = fract(y), fz = fract(z)
/// tx = smoothstep(fx), ty = smoothstep(fy), tz = smoothstep(fz)
///
/// Trilinear interpolation over the 8 lattice corners.
/// result ∈ [0, 1]
/// ```
///
/// # Arguments
/// * `x` — x coordinate (any finite `f32`)
/// * `y` — y coordinate (any finite `f32`)
/// * `z` — z coordinate (any finite `f32`)
///
/// # Returns
/// A value in [0, 1].
///
/// # Example
/// ```rust
/// use prime_noise::value_noise_3d;
/// let v = value_noise_3d(1.5, 2.3, 0.7);
/// assert!(v >= 0.0 && v <= 1.0);
/// ```
pub fn value_noise_3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let fz = z - z.floor();

    let tx = smoothstep(fx);
    let ty = smoothstep(fy);
    let tz = smoothstep(fz);

    let xi1 = xi.wrapping_add(1);
    let yi1 = yi.wrapping_add(1);
    let zi1 = zi.wrapping_add(1);

    let v000 = hash_3d(xi,  yi,  zi );
    let v100 = hash_3d(xi1, yi,  zi );
    let v010 = hash_3d(xi,  yi1, zi );
    let v110 = hash_3d(xi1, yi1, zi );
    let v001 = hash_3d(xi,  yi,  zi1);
    let v101 = hash_3d(xi1, yi,  zi1);
    let v011 = hash_3d(xi,  yi1, zi1);
    let v111 = hash_3d(xi1, yi1, zi1);

    let bot = lerp(lerp(v000, v100, tx), lerp(v010, v110, tx), ty);
    let top = lerp(lerp(v001, v101, tx), lerp(v011, v111, tx), ty);
    lerp(bot, top, tz)
}

// ---------------------------------------------------------------------------
// 3-D Perlin noise
// ---------------------------------------------------------------------------

/// Classic Perlin gradient noise at a 3-D point.
///
/// # Math
///
/// ```text
/// 8-corner trilinear blend of gradient dot products.
/// result ∈ approximately [-1, 1] (exact bounds not guaranteed).
/// At integer lattice points: result = 0.
/// ```
///
/// # Arguments
/// * `x`, `y`, `z` — coordinates (any finite `f32`)
///
/// # Returns
/// Approximately in [-1, 1]. Not clamped.
///
/// # Edge cases
/// * At integer lattice points all offsets are 0, so all gradient dot products
///   are 0 and the result is 0.0.
///
/// # Example
/// ```rust
/// use prime_noise::perlin_3d;
/// let v = perlin_3d(0.5, 0.5, 0.5);
/// assert!(v >= -1.5 && v <= 1.5);
/// assert_eq!(perlin_3d(1.0, 2.0, 3.0), 0.0);
/// ```
pub fn perlin_3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let fz = z - z.floor();

    let tx = smoothstep(fx);
    let ty = smoothstep(fy);
    let tz = smoothstep(fz);

    let xi1 = xi.wrapping_add(1);
    let yi1 = yi.wrapping_add(1);
    let zi1 = zi.wrapping_add(1);

    let g000 = gradient_3d(hash_3d(xi,  yi,  zi ));
    let g100 = gradient_3d(hash_3d(xi1, yi,  zi ));
    let g010 = gradient_3d(hash_3d(xi,  yi1, zi ));
    let g110 = gradient_3d(hash_3d(xi1, yi1, zi ));
    let g001 = gradient_3d(hash_3d(xi,  yi,  zi1));
    let g101 = gradient_3d(hash_3d(xi1, yi,  zi1));
    let g011 = gradient_3d(hash_3d(xi,  yi1, zi1));
    let g111 = gradient_3d(hash_3d(xi1, yi1, zi1));

    let n000 = grad_dot_3d(g000, fx,       fy,       fz      );
    let n100 = grad_dot_3d(g100, fx - 1.0, fy,       fz      );
    let n010 = grad_dot_3d(g010, fx,       fy - 1.0, fz      );
    let n110 = grad_dot_3d(g110, fx - 1.0, fy - 1.0, fz      );
    let n001 = grad_dot_3d(g001, fx,       fy,       fz - 1.0);
    let n101 = grad_dot_3d(g101, fx - 1.0, fy,       fz - 1.0);
    let n011 = grad_dot_3d(g011, fx,       fy - 1.0, fz - 1.0);
    let n111 = grad_dot_3d(g111, fx - 1.0, fy - 1.0, fz - 1.0);

    let bot = lerp(lerp(n000, n100, tx), lerp(n010, n110, tx), ty);
    let top = lerp(lerp(n001, n101, tx), lerp(n011, n111, tx), ty);
    lerp(bot, top, tz)
}

/// Fractional Brownian Motion over 3-D Perlin noise.
///
/// # Math
///
/// ```text
/// result = Σ_{i=0}^{octaves-1} amplitude_i * perlin_3d(x*freq_i, y*freq_i, z*freq_i)
/// ```
///
/// # Arguments
/// * `x`, `y`, `z` — coordinates
/// * `octaves`     — number of noise layers (0 returns 0.0)
/// * `lacunarity`  — frequency multiplier per octave (typically 2.0)
/// * `gain`        — amplitude multiplier per octave (typically 0.5)
///
/// # Returns
/// Sum of octave contributions. Not clamped.
///
/// # Example
/// ```rust
/// use prime_noise::fbm_3d;
/// let v = fbm_3d(0.3, 0.7, 0.2, 6, 2.0, 0.5);
/// assert!(v.abs() < 3.0);
/// ```
pub fn fbm_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let (value, _, _) = (0..octaves).fold((0.0_f32, 1.0_f32, 1.0_f32), |(acc, freq, amp), _| {
        (acc + amp * perlin_3d(x * freq, y * freq, z * freq), freq * lacunarity, amp * gain)
    });
    value
}

// ---------------------------------------------------------------------------
// Simplex noise (2-D and 3-D)
// ---------------------------------------------------------------------------

// Skew / unskew constants (computed from known surds to avoid precision lint).
// F2 = (sqrt(3) - 1) / 2
const SIMPLEX_F2: f32 = 0.366_025_4; // (sqrt(3)-1)/2
// G2 = (3 - sqrt(3)) / 6
const SIMPLEX_G2: f32 = 0.211_324_87; // (3-sqrt(3))/6
// F3 = 1/3
const SIMPLEX_F3: f32 = 1.0 / 3.0;
// G3 = 1/6
const SIMPLEX_G3: f32 = 1.0 / 6.0;

/// Simplex noise at a 2-D point.
///
/// Uses triangular (simplex) instead of square lattice cells — no directional
/// artifacts and fewer corners to evaluate (3 vs. 4) compared to Perlin.
///
/// # Math
///
/// ```text
/// Skew (x,y) into a square lattice, locate the triangle (simplex) containing
/// the point, evaluate radial-falloff gradient contributions from the 3 corners,
/// scale so the result is approximately in [-1, 1].
/// ```
///
/// # Arguments
/// * `x`, `y` — coordinates (any finite `f32`)
///
/// # Returns
/// Approximately in [-1, 1]. Not clamped.
///
/// # Edge cases
/// * Returns 0.0 at the origin.
///
/// # Example
/// ```rust
/// use prime_noise::simplex_2d;
/// let v = simplex_2d(0.5, 0.5);
/// assert!(v >= -1.1 && v <= 1.1);
/// ```
pub fn simplex_2d(x: f32, y: f32) -> f32 {
    let s = (x + y) * SIMPLEX_F2;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;

    let t = (i + j) as f32 * SIMPLEX_G2;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);

    // Which triangle?
    let (i1, j1) = if x0 > y0 { (1_i32, 0_i32) } else { (0_i32, 1_i32) };

    let x1 = x0 - i1 as f32 + SIMPLEX_G2;
    let y1 = y0 - j1 as f32 + SIMPLEX_G2;
    let x2 = x0 - 1.0 + 2.0 * SIMPLEX_G2;
    let y2 = y0 - 1.0 + 2.0 * SIMPLEX_G2;

    let contrib = |xi: i32, yi: i32, dx: f32, dy: f32| -> f32 {
        let t = 0.5 - dx * dx - dy * dy;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            t2 * t2 * grad_dot(gradient(hash_2d(xi, yi)), dx, dy)
        }
    };

    70.0 * (contrib(i, j, x0, y0) + contrib(i + i1, j + j1, x1, y1) + contrib(i + 1, j + 1, x2, y2))
}

/// Simplex noise at a 3-D point.
///
/// Evaluates 4 tetrahedral corners instead of 8 cubic corners, reducing
/// the per-sample cost compared to 3-D Perlin while eliminating grid artifacts.
///
/// # Math
///
/// ```text
/// Skew (x,y,z) into a cubic lattice via F3=1/3 factor.
/// Locate which tetrahedron (simplex) contains the point.
/// Sum radial-falloff gradient contributions from the 4 corners.
/// Scale to approximately [-1, 1].
/// ```
///
/// # Arguments
/// * `x`, `y`, `z` — coordinates (any finite `f32`)
///
/// # Returns
/// Approximately in [-1, 1]. Not clamped.
///
/// # Example
/// ```rust
/// use prime_noise::simplex_3d;
/// let v = simplex_3d(0.5, 0.5, 0.5);
/// assert!(v >= -1.1 && v <= 1.1);
/// ```
pub fn simplex_3d(x: f32, y: f32, z: f32) -> f32 {
    let s = (x + y + z) * SIMPLEX_F3;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;

    let t = (i + j + k) as f32 * SIMPLEX_G3;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    // Which tetrahedron?
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0      { (1, 0, 0,  1, 1, 0) }
        else if x0 >= z0 { (1, 0, 0,  1, 0, 1) }
        else             { (0, 0, 1,  1, 0, 1) }
    } else if y0 < z0       { (0, 0, 1,  0, 1, 1) }
      else if x0 < z0  { (0, 1, 0,  0, 1, 1) }
      else             { (0, 1, 0,  1, 1, 0) };

    let x1 = x0 - i1 as f32 + SIMPLEX_G3;
    let y1 = y0 - j1 as f32 + SIMPLEX_G3;
    let z1 = z0 - k1 as f32 + SIMPLEX_G3;
    let x2 = x0 - i2 as f32 + 2.0 * SIMPLEX_G3;
    let y2 = y0 - j2 as f32 + 2.0 * SIMPLEX_G3;
    let z2 = z0 - k2 as f32 + 2.0 * SIMPLEX_G3;
    let x3 = x0 - 1.0 + 3.0 * SIMPLEX_G3;
    let y3 = y0 - 1.0 + 3.0 * SIMPLEX_G3;
    let z3 = z0 - 1.0 + 3.0 * SIMPLEX_G3;

    let contrib = |xi: i32, yi: i32, zi: i32, dx: f32, dy: f32, dz: f32| -> f32 {
        let t = 0.6 - dx * dx - dy * dy - dz * dz;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            t2 * t2 * grad_dot_3d(gradient_3d(hash_3d(xi, yi, zi)), dx, dy, dz)
        }
    };

    32.0 * (
        contrib(i,      j,      k,      x0, y0, z0)
      + contrib(i + i1, j + j1, k + k1, x1, y1, z1)
      + contrib(i + i2, j + j2, k + k2, x2, y2, z2)
      + contrib(i + 1,  j + 1,  k + 1,  x3, y3, z3)
    )
}

// ---------------------------------------------------------------------------
// Domain warping
// ---------------------------------------------------------------------------

/// Domain-warped FBM in 2-D.
///
/// Samples two independent FBM fields as a warp vector, then samples a third
/// FBM at the warped position. Produces swirling, turbulent shapes with rich
/// self-similar structure not achievable by plain FBM.
///
/// # Math
///
/// ```text
/// warp_x = fbm_2d(x + 0.0, y + 0.0, ...)
/// warp_y = fbm_2d(x + 5.2, y + 1.3, ...)   ← offset breaks correlation
/// result = fbm_2d(x + warp_scale * warp_x,
///                 y + warp_scale * warp_y, ...)
/// ```
///
/// # Arguments
/// * `x`, `y`      — input coordinates
/// * `octaves`     — FBM octave count
/// * `lacunarity`  — frequency multiplier per octave
/// * `gain`        — amplitude multiplier per octave
/// * `warp_scale`  — how far the domain is displaced (typically 1.0–2.0)
///
/// # Returns
/// An FBM-range value (not clamped).
///
/// # Example
/// ```rust
/// use prime_noise::domain_warp_2d;
/// let v = domain_warp_2d(0.3, 0.7, 6, 2.0, 0.5, 1.0);
/// assert!(v.abs() < 4.0);
/// ```
pub fn domain_warp_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32, warp_scale: f32) -> f32 {
    let wx = fbm_2d(x,       y,       octaves, lacunarity, gain);
    let wy = fbm_2d(x + 5.2, y + 1.3, octaves, lacunarity, gain);
    fbm_2d(x + warp_scale * wx, y + warp_scale * wy, octaves, lacunarity, gain)
}

/// Domain-warped FBM in 3-D.
///
/// Three independent FBM fields are sampled as a warp vector (with offsets to
/// break correlation), then a fourth FBM is evaluated at the warped position.
///
/// # Math
///
/// ```text
/// wx = fbm_3d(x+0.0, y+0.0, z+0.0, ...)
/// wy = fbm_3d(x+5.2, y+1.3, z+2.7, ...)
/// wz = fbm_3d(x+3.1, y+7.4, z+0.9, ...)
/// result = fbm_3d(x + warp_scale*wx, y + warp_scale*wy, z + warp_scale*wz, ...)
/// ```
///
/// # Arguments
/// * `x`, `y`, `z` — input coordinates
/// * `octaves`     — FBM octave count
/// * `lacunarity`  — frequency multiplier per octave
/// * `gain`        — amplitude multiplier per octave
/// * `warp_scale`  — domain displacement magnitude
///
/// # Returns
/// An FBM-range value (not clamped).
///
/// # Example
/// ```rust
/// use prime_noise::domain_warp_3d;
/// let v = domain_warp_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0);
/// assert!(v.abs() < 4.0);
/// ```
pub fn domain_warp_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32, warp_scale: f32) -> f32 {
    let wx = fbm_3d(x,       y,       z,       octaves, lacunarity, gain);
    let wy = fbm_3d(x + 5.2, y + 1.3, z + 2.7, octaves, lacunarity, gain);
    let wz = fbm_3d(x + 3.1, y + 7.4, z + 0.9, octaves, lacunarity, gain);
    fbm_3d(x + warp_scale * wx, y + warp_scale * wy, z + warp_scale * wz, octaves, lacunarity, gain)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    // --- value_noise_2d ---

    #[test]
    fn value_noise_range() {
        let points = [
            (0.0, 0.0),
            (0.5, 0.5),
            (1.234, 5.678),
            (-3.1, 2.9),
            (100.0, 200.0),
            (0.999, 0.001),
        ];
        for (x, y) in points {
            let v = value_noise_2d(x, y);
            assert!(
                v >= 0.0 && v <= 1.0,
                "value_noise_2d({x}, {y}) = {v} out of [0,1]"
            );
        }
    }

    #[test]
    fn value_noise_deterministic() {
        let a = value_noise_2d(3.14, 2.71);
        let b = value_noise_2d(3.14, 2.71);
        assert_eq!(a, b, "value_noise_2d must be deterministic");
    }

    #[test]
    fn value_noise_at_integer_lattice() {
        // At integer coordinates fx=0, fy=0, so smoothstep(0)=0 → result = hash_2d(xi, yi).
        let v = value_noise_2d(2.0, 3.0);
        let expected = hash_2d(2, 3);
        assert!(
            (v - expected).abs() < EPSILON,
            "at lattice point got {v}, expected {expected}"
        );
    }

    #[test]
    fn value_noise_different_coords_differ() {
        let a = value_noise_2d(0.5, 0.5);
        let b = value_noise_2d(1.5, 0.5);
        assert_ne!(a, b, "distinct coordinates should (almost certainly) differ");
    }

    // --- perlin_2d ---

    #[test]
    fn perlin_range_approx() {
        let points = [
            (0.5, 0.5),
            (1.234, 5.678),
            (-3.1, 2.9),
            (10.0, 20.0),
            (0.1, 0.9),
        ];
        for (x, y) in points {
            let v = perlin_2d(x, y);
            assert!(
                v >= -1.5 && v <= 1.5,
                "perlin_2d({x}, {y}) = {v} wildly out of range"
            );
        }
    }

    #[test]
    fn perlin_deterministic() {
        let a = perlin_2d(1.1, 2.2);
        let b = perlin_2d(1.1, 2.2);
        assert_eq!(a, b, "perlin_2d must be deterministic");
    }

    #[test]
    fn perlin_zero_at_integer_lattice() {
        // At exact integer lattice points all offset components are 0, so all
        // gradient dot products are 0 and the result is 0.
        for (xi, yi) in [(0, 0), (1, 2), (-3, 5), (10, -4)] {
            let v = perlin_2d(xi as f32, yi as f32);
            assert!(
                v.abs() < EPSILON,
                "perlin_2d({xi}, {yi}) = {v}, expected 0"
            );
        }
    }

    #[test]
    fn perlin_different_coords_differ() {
        let a = perlin_2d(0.3, 0.7);
        let b = perlin_2d(0.7, 0.3);
        // These are almost certainly different; symmetry of the hash makes equality
        // astronomically unlikely.
        assert_ne!(a, b);
    }

    // --- fbm_2d ---

    #[test]
    fn fbm_zero_octaves_returns_zero() {
        let v = fbm_2d(1.0, 2.0, 0, 2.0, 0.5);
        assert_eq!(v, 0.0, "fbm with 0 octaves must return 0");
    }

    #[test]
    fn fbm_one_octave_equals_perlin() {
        let x = 0.4;
        let y = 0.8;
        let fbm_val = fbm_2d(x, y, 1, 2.0, 0.5);
        let perlin_val = perlin_2d(x, y);
        assert!(
            (fbm_val - perlin_val).abs() < EPSILON,
            "fbm with 1 octave should equal perlin: fbm={fbm_val}, perlin={perlin_val}"
        );
    }

    #[test]
    fn fbm_deterministic() {
        let a = fbm_2d(0.5, 0.5, 6, 2.0, 0.5);
        let b = fbm_2d(0.5, 0.5, 6, 2.0, 0.5);
        assert_eq!(a, b, "fbm_2d must be deterministic");
    }

    #[test]
    fn fbm_standard_params_bounded() {
        // With lacunarity=2.0 and gain=0.5 the geometric series sums to < 2.0
        for (x, y) in [(0.1, 0.2), (5.5, 3.3), (-1.0, -2.0)] {
            let v = fbm_2d(x, y, 8, 2.0, 0.5);
            assert!(
                v.abs() < 3.0,
                "fbm_2d({x},{y},8,2.0,0.5) = {v} suspiciously large"
            );
        }
    }

    #[test]
    fn fbm_increases_with_octaves() {
        // More octaves generally add detail, so the absolute value should differ
        // from the single-octave result for typical coordinates.
        let x = 0.3;
        let y = 0.7;
        let one = fbm_2d(x, y, 1, 2.0, 0.5);
        let six = fbm_2d(x, y, 6, 2.0, 0.5);
        assert_ne!(one, six, "adding octaves should change the result");
    }

    // --- worley_2d ---

    #[test]
    fn worley_range() {
        let points = [
            (0.0, 0.0),
            (0.5, 0.5),
            (3.14, 2.71),
            (-1.5, 4.2),
            (100.0, 200.0),
        ];
        for (x, y) in points {
            let d = worley_2d(x, y, 42);
            assert!(
                d >= 0.0 && d <= 1.0,
                "worley_2d({x},{y},42) = {d} out of [0,1]"
            );
        }
    }

    #[test]
    fn worley_deterministic() {
        let a = worley_2d(1.23, 4.56, 99);
        let b = worley_2d(1.23, 4.56, 99);
        assert_eq!(a, b, "worley_2d must be deterministic");
    }

    #[test]
    fn worley_different_seeds_differ() {
        let a = worley_2d(0.5, 0.5, 0);
        let b = worley_2d(0.5, 0.5, 1);
        assert_ne!(a, b, "different seeds should produce different feature fields");
    }

    #[test]
    fn worley_non_negative() {
        // Distance is always non-negative; clamping to [0,1] keeps it so.
        let v = worley_2d(0.0, 0.0, 0);
        assert!(v >= 0.0);
    }

    #[test]
    fn worley_center_cell_plausible() {
        // At the centre of a cell the nearest feature is somewhere in the
        // surrounding 3x3 block — distance must be > 0.
        let d = worley_2d(0.5, 0.5, 7);
        assert!(d > 0.0, "distance at cell centre should be positive");
    }

    // --- value_noise_3d ---

    #[test]
    fn value_noise_3d_range() {
        for (x, y, z) in [(0.5, 0.5, 0.5), (1.234, 5.678, 2.345), (-3.1, 2.9, 1.1)] {
            let v = value_noise_3d(x, y, z);
            assert!(v >= 0.0 && v <= 1.0, "value_noise_3d({x},{y},{z})={v}");
        }
    }

    #[test]
    fn value_noise_3d_deterministic() {
        let a = value_noise_3d(1.1, 2.2, 3.3);
        let b = value_noise_3d(1.1, 2.2, 3.3);
        assert_eq!(a, b);
    }

    #[test]
    fn value_noise_3d_at_integer_lattice() {
        let v = value_noise_3d(2.0, 3.0, 4.0);
        let expected = hash_3d(2, 3, 4);
        assert!((v - expected).abs() < EPSILON, "v={v}, expected={expected}");
    }

    #[test]
    fn value_noise_3d_different_coords_differ() {
        assert_ne!(value_noise_3d(0.5, 0.5, 0.5), value_noise_3d(1.5, 0.5, 0.5));
    }

    // --- perlin_3d ---

    #[test]
    fn perlin_3d_range_approx() {
        for (x, y, z) in [(0.5, 0.5, 0.5), (1.234, 5.678, 2.3), (-3.1, 2.9, 0.7)] {
            let v = perlin_3d(x, y, z);
            assert!(v >= -1.5 && v <= 1.5, "perlin_3d({x},{y},{z})={v}");
        }
    }

    #[test]
    fn perlin_3d_deterministic() {
        let a = perlin_3d(1.1, 2.2, 3.3);
        let b = perlin_3d(1.1, 2.2, 3.3);
        assert_eq!(a, b);
    }

    #[test]
    fn perlin_3d_zero_at_integer_lattice() {
        for (x, y, z) in [(0, 0, 0), (1, 2, 3), (-1, 4, -2)] {
            let v = perlin_3d(x as f32, y as f32, z as f32);
            assert!(v.abs() < EPSILON, "perlin_3d({x},{y},{z})={v}");
        }
    }

    #[test]
    fn perlin_3d_different_coords_differ() {
        assert_ne!(perlin_3d(0.3, 0.7, 0.5), perlin_3d(0.7, 0.3, 0.5));
    }

    // --- fbm_3d ---

    #[test]
    fn fbm_3d_zero_octaves_returns_zero() {
        assert_eq!(fbm_3d(1.0, 2.0, 3.0, 0, 2.0, 0.5), 0.0);
    }

    #[test]
    fn fbm_3d_one_octave_equals_perlin() {
        let v_fbm = fbm_3d(0.4, 0.8, 0.2, 1, 2.0, 0.5);
        let v_perlin = perlin_3d(0.4, 0.8, 0.2);
        assert!((v_fbm - v_perlin).abs() < EPSILON);
    }

    #[test]
    fn fbm_3d_deterministic() {
        let a = fbm_3d(0.5, 0.5, 0.5, 6, 2.0, 0.5);
        let b = fbm_3d(0.5, 0.5, 0.5, 6, 2.0, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn fbm_3d_bounded() {
        let v = fbm_3d(0.3, 0.7, 0.2, 8, 2.0, 0.5);
        assert!(v.abs() < 3.0, "fbm_3d={v}");
    }

    // --- simplex_2d ---

    #[test]
    fn simplex_2d_range() {
        for (x, y) in [(0.5, 0.5), (1.2, 3.4), (-2.1, 0.7), (100.0, 200.0)] {
            let v = simplex_2d(x, y);
            assert!(v >= -1.1 && v <= 1.1, "simplex_2d({x},{y})={v}");
        }
    }

    #[test]
    fn simplex_2d_deterministic() {
        let a = simplex_2d(1.1, 2.2);
        let b = simplex_2d(1.1, 2.2);
        assert_eq!(a, b);
    }

    #[test]
    fn simplex_2d_different_coords_differ() {
        assert_ne!(simplex_2d(0.3, 0.7), simplex_2d(0.7, 0.3));
    }

    // --- simplex_3d ---

    #[test]
    fn simplex_3d_range() {
        for (x, y, z) in [(0.5, 0.5, 0.5), (1.2, 3.4, 2.1), (-2.1, 0.7, 1.3)] {
            let v = simplex_3d(x, y, z);
            assert!(v >= -1.1 && v <= 1.1, "simplex_3d({x},{y},{z})={v}");
        }
    }

    #[test]
    fn simplex_3d_deterministic() {
        let a = simplex_3d(1.1, 2.2, 3.3);
        let b = simplex_3d(1.1, 2.2, 3.3);
        assert_eq!(a, b);
    }

    #[test]
    fn simplex_3d_different_coords_differ() {
        assert_ne!(simplex_3d(0.3, 0.7, 0.2), simplex_3d(0.7, 0.3, 0.2));
    }

    // --- domain_warp_2d ---

    #[test]
    fn domain_warp_2d_deterministic() {
        let a = domain_warp_2d(0.3, 0.7, 4, 2.0, 0.5, 1.0);
        let b = domain_warp_2d(0.3, 0.7, 4, 2.0, 0.5, 1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn domain_warp_2d_differs_from_plain_fbm() {
        let warped = domain_warp_2d(0.3, 0.7, 4, 2.0, 0.5, 1.0);
        let plain = fbm_2d(0.3, 0.7, 4, 2.0, 0.5);
        assert_ne!(warped, plain);
    }

    #[test]
    fn domain_warp_2d_bounded() {
        let v = domain_warp_2d(0.3, 0.7, 6, 2.0, 0.5, 1.0);
        assert!(v.abs() < 4.0, "domain_warp_2d={v}");
    }

    // --- domain_warp_3d ---

    #[test]
    fn domain_warp_3d_deterministic() {
        let a = domain_warp_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0);
        let b = domain_warp_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn domain_warp_3d_differs_from_plain_fbm() {
        let warped = domain_warp_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0);
        let plain = fbm_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5);
        assert_ne!(warped, plain);
    }

    #[test]
    fn domain_warp_3d_bounded() {
        let v = domain_warp_3d(0.3, 0.7, 0.2, 4, 2.0, 0.5, 1.0);
        assert!(v.abs() < 4.0, "domain_warp_3d={v}");
    }

    // ── large / negative / zero inputs ───────────────────────────────────────

    #[test]
    fn value_noise_2d_large_inputs_finite() {
        assert!(value_noise_2d(1e10, 1e10).is_finite());
    }

    #[test]
    fn perlin_2d_negative_inputs_finite() {
        assert!(perlin_2d(-100.0, -200.0).is_finite());
    }

    #[test]
    fn simplex_2d_zero_input_finite() {
        assert!(simplex_2d(0.0, 0.0).is_finite());
    }

    #[test]
    fn simplex_3d_large_inputs_finite() {
        assert!(simplex_3d(1e6, 1e6, 1e6).is_finite());
    }

    #[test]
    fn fbm_2d_zero_octaves_returns_zero() {
        assert_eq!(fbm_2d(0.5, 0.5, 0, 2.0, 0.5), 0.0);
    }

    #[test]
    fn fbm_3d_zero_octaves_returns_zero_edge() {
        assert_eq!(fbm_3d(0.5, 0.5, 0.5, 0, 2.0, 0.5), 0.0);
    }

    #[test]
    fn value_noise_3d_negative_inputs_finite() {
        assert!(value_noise_3d(-50.0, -50.0, -50.0).is_finite());
    }

    #[test]
    fn perlin_3d_large_inputs_finite() {
        assert!(perlin_3d(1e8, 1e8, 1e8).is_finite());
    }
}
