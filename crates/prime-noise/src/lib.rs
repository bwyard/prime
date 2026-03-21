//! prime-noise — Noise functions: value noise, Perlin, FBM, Worley.
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
fn hash_u32(mut x: u32) -> u32 {
    x = x.wrapping_add(0x6D2B79F5);
    x = (x ^ (x >> 15)).wrapping_mul(x | 1);
    x = x ^ (x.wrapping_add((x ^ (x >> 7)).wrapping_mul(x | 61)));
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
const GRADIENTS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (0.7071068, 0.7071068),
    (0.0, 1.0),
    (-0.7071068, 0.7071068),
    (-1.0, 0.0),
    (-0.7071068, -0.7071068),
    (0.0, -1.0),
    (0.7071068, -0.7071068),
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
    let v10 = hash_2d(xi + 1, yi);
    let v01 = hash_2d(xi, yi + 1);
    let v11 = hash_2d(xi + 1, yi + 1);

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
    let g10 = gradient(hash_2d(xi + 1, yi));
    let g01 = gradient(hash_2d(xi, yi + 1));
    let g11 = gradient(hash_2d(xi + 1, yi + 1));

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
///            Seeds `seed` and `seed+1` are used internally for x and y offsets.
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
}
