//! `prime-color` — Color space math: sRGB, Oklab, HSL, and perceptual mixing.
//!
//! All functions are pure (LOAD + COMPUTE only). No mutation, no side effects,
//! no hidden state. Same inputs always produce the same output.

// Oklab matrix constants are from Björn Ottosson's original paper and are intentionally
// specified at full f64 precision for documentation accuracy, even though f32 truncates them.
#![allow(clippy::excessive_precision)]
//!
//! Colors are represented as bare `(f32, f32, f32)` tuples for simplicity and
//! FFI/WASM interop. No structs.
//!
//! # Color spaces
//! - **sRGB** — standard display color space, gamma-encoded, components in `[0, 1]`
//! - **Linear RGB** — gamma-decoded sRGB; linear light values used for math
//! - **Oklab** — perceptually uniform opponent color space by Björn Ottosson
//!   - `L` ∈ `[0, 1]` (lightness), `a` ∈ `[-0.5, 0.5]` (green↔red), `b` ∈ `[-0.5, 0.5]` (blue↔yellow)
//! - **HSL** — hue/saturation/lightness cylinder on sRGB
//!   - `h` ∈ `[0, 360)`, `s` ∈ `[0, 1]`, `l` ∈ `[0, 1]`

// ---------------------------------------------------------------------------
// sRGB ↔ Linear RGB
// ---------------------------------------------------------------------------

/// Converts a single sRGB component to linear light (gamma decode).
///
/// # Math
///   if c <= 0.04045: c / 12.92
///   else:            ((c + 0.055) / 1.055) ^ 2.4
#[inline]
fn srgb_channel_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts a single linear light component to sRGB (gamma encode).
///
/// # Math
///   if c <= 0.0031308: 12.92 * c
///   else:              1.055 * c^(1/2.4) - 0.055
#[inline]
fn linear_channel_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Converts an sRGB color (gamma-encoded) to linear RGB.
///
/// # Math
///   Per-channel:
///   if c <= 0.04045: c / 12.92
///   else:            ((c + 0.055) / 1.055) ^ 2.4
///
/// # Arguments
/// * `r` - red component in sRGB, typically `[0, 1]`
/// * `g` - green component in sRGB, typically `[0, 1]`
/// * `b` - blue component in sRGB, typically `[0, 1]`
///
/// # Returns
/// Linear RGB tuple `(r, g, b)`. Values outside `[0, 1]` are preserved (not clamped).
///
/// # Edge cases
/// * `0.0` → `0.0` (black stays black)
/// * `1.0` → `1.0` (white stays white)
/// * Values slightly above `0.04045` use the power branch; continuity is maintained by the IEC standard.
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::srgb_to_linear(0.5, 0.5, 0.5);
/// // approximately (0.2140, 0.2140, 0.2140)
/// assert!((r - 0.2140).abs() < 1e-3);
/// ```
pub fn srgb_to_linear(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        srgb_channel_to_linear(r),
        srgb_channel_to_linear(g),
        srgb_channel_to_linear(b),
    )
}

/// Converts a linear RGB color to sRGB (gamma-encoded).
///
/// # Math
///   Per-channel:
///   if c <= 0.0031308: 12.92 * c
///   else:              1.055 * c^(1/2.4) - 0.055
///
/// # Arguments
/// * `r` - red component in linear RGB, typically `[0, 1]`
/// * `g` - green component in linear RGB, typically `[0, 1]`
/// * `b` - blue component in linear RGB, typically `[0, 1]`
///
/// # Returns
/// sRGB tuple `(r, g, b)`. Values outside `[0, 1]` are preserved (not clamped).
///
/// # Edge cases
/// * `0.0` → `0.0`
/// * `1.0` → `1.0`
/// * Negative linear values produce negative sRGB (caller must clamp if needed).
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::linear_to_srgb(0.2140, 0.2140, 0.2140);
/// assert!((r - 0.5).abs() < 1e-3);
/// ```
pub fn linear_to_srgb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        linear_channel_to_srgb(r),
        linear_channel_to_srgb(g),
        linear_channel_to_srgb(b),
    )
}

// ---------------------------------------------------------------------------
// Oklab color space — Björn Ottosson, 2020
// Reference: https://bottosson.github.io/posts/oklab/
// No patents. Matrices used verbatim from the original post.
// ---------------------------------------------------------------------------

/// sRGB (linear) → LMS matrix (M1).
///
/// Rows are [l_coef, m_coef, s_coef] per output channel.
const M1: [[f32; 3]; 3] = [
    [0.4122214708, 0.5363325363, 0.0514459929],
    [0.2119034982, 0.6806995451, 0.1073969566],
    [0.0883024619, 0.2817188376, 0.6299787005],
];

/// LMS^(1/3) → Lab matrix (M2).
const M2: [[f32; 3]; 3] = [
    [ 0.2104542553,  0.7936177850, -0.0040720468],
    [ 1.9779984951, -2.4285922050,  0.4505937099],
    [ 0.0259040371,  0.7827717662, -0.8086757660],
];

/// Lab → LMS^(1/3) matrix (M2 inverse).
const M2_INV: [[f32; 3]; 3] = [
    [1.0,  0.3963377774,  0.2158037573],
    [1.0, -0.1055613458, -0.0638541728],
    [1.0, -0.0894841775, -1.2914855480],
];

/// LMS → sRGB (linear) matrix (M1 inverse).
const M1_INV: [[f32; 3]; 3] = [
    [ 4.0767416621, -3.3077115913,  0.2309699292],
    [-1.2684380046,  2.6097574011, -0.3413193965],
    [-0.0041960863, -0.7034186147,  1.7076147010],
];

/// Applies a 3×3 matrix to a column vector [x, y, z].
#[inline]
fn mat3_mul(m: &[[f32; 3]; 3], x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (
        m[0][0] * x + m[0][1] * y + m[0][2] * z,
        m[1][0] * x + m[1][1] * y + m[1][2] * z,
        m[2][0] * x + m[2][1] * y + m[2][2] * z,
    )
}

/// Signed cube root — preserves sign for negative values.
#[inline]
fn cbrt(x: f32) -> f32 {
    x.abs().powf(1.0 / 3.0).copysign(x)
}

/// Converts an sRGB color to Oklab.
///
/// # Math
///   1. Gamma-decode sRGB to linear RGB (per `srgb_to_linear`).
///   2. Multiply by M1 to get approximate LMS cone responses.
///   3. Apply cube root: l' = cbrt(l), m' = cbrt(m), s' = cbrt(s).
///   4. Multiply by M2 to get perceptual Lab coordinates.
///
/// # Arguments
/// * `r` - red component in sRGB, `[0, 1]`
/// * `g` - green component in sRGB, `[0, 1]`
/// * `b` - blue component in sRGB, `[0, 1]`
///
/// # Returns
/// `(L, a, b)` in Oklab:
/// * `L` ∈ `[0, 1]` — lightness
/// * `a` ∈ `[-0.5, 0.5]` approximately — green↔red axis
/// * `b` ∈ `[-0.5, 0.5]` approximately — blue↔yellow axis
///
/// # Edge cases
/// * Pure black `(0, 0, 0)` → `(0, 0, 0)`
/// * Pure white `(1, 1, 1)` → `(1, 0, 0)` (approximately)
/// * Out-of-gamut sRGB inputs produce valid but extrapolated Lab values.
///
/// # Example
/// ```rust
/// let (l, a, b) = prime_color::srgb_to_oklab(1.0, 0.0, 0.0);
/// // Red in Oklab: L ≈ 0.6280, a ≈ 0.2247, b ≈ 0.1260
/// assert!((l - 0.6280).abs() < 1e-3);
/// ```
pub fn srgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Step 1: gamma decode
    let (lr, lg, lb) = srgb_to_linear(r, g, b);

    // Step 2: linear RGB → LMS
    let (l, m, s) = mat3_mul(&M1, lr, lg, lb);

    // Step 3: cube root
    let (l_, m_, s_) = (cbrt(l), cbrt(m), cbrt(s));

    // Step 4: LMS^(1/3) → Lab
    mat3_mul(&M2, l_, m_, s_)
}

/// Converts an Oklab color back to sRGB.
///
/// # Math
///   1. Multiply by M2_INV to get LMS^(1/3).
///   2. Cube: l = l'^3, m = m'^3, s = s'^3.
///   3. Multiply by M1_INV to get linear RGB.
///   4. Gamma-encode to sRGB (per `linear_to_srgb`).
///   5. Clamp all components to `[0, 1]`.
///
/// # Arguments
/// * `l` - Oklab lightness, `[0, 1]`
/// * `a` - Oklab a axis (green↔red)
/// * `b` - Oklab b axis (blue↔yellow)
///
/// # Returns
/// sRGB tuple `(r, g, b)`, each clamped to `[0, 1]`.
///
/// # Edge cases
/// * Out-of-gamut Oklab values are clamped to displayable sRGB.
/// * `(0, 0, 0)` → `(0, 0, 0)`
/// * `(1, 0, 0)` → approximately `(1, 1, 1)`
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::oklab_to_srgb(0.6280, 0.2247, 0.1260);
/// assert!((r - 1.0).abs() < 1e-2); // approximately red
/// assert!(g < 0.1);
/// assert!(b < 0.1);
/// ```
pub fn oklab_to_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    // Step 1: Lab → LMS^(1/3)
    let (l_, m_, s_) = mat3_mul(&M2_INV, l, a, b);

    // Step 2: cube
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    // Step 3: LMS → linear RGB
    let (lr, lg, lb) = mat3_mul(&M1_INV, lc, mc, sc);

    // Step 4: gamma encode
    let (r, g, bv) = linear_to_srgb(lr, lg, lb);

    // Step 5: clamp to [0, 1]
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), bv.clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------
// HSL ↔ sRGB
// ---------------------------------------------------------------------------

/// Converts an sRGB color to HSL.
///
/// # Math
///   max = max(r, g, b), min = min(r, g, b), delta = max - min
///
///   L = (max + min) / 2
///
///   S = 0                         if delta == 0
///       delta / (1 - |2L - 1|)    otherwise
///
///   H = 60 * ((g - b) / delta mod 6)   if max == r
///       60 * ((b - r) / delta + 2)     if max == g
///       60 * ((r - g) / delta + 4)     if max == b
///       0                              if delta == 0
///
/// # Arguments
/// * `r` - red in sRGB, `[0, 1]`
/// * `g` - green in sRGB, `[0, 1]`
/// * `b` - blue in sRGB, `[0, 1]`
///
/// # Returns
/// `(h, s, l)` where:
/// * `h` ∈ `[0, 360)` — hue in degrees
/// * `s` ∈ `[0, 1]` — saturation
/// * `l` ∈ `[0, 1]` — lightness
///
/// # Edge cases
/// * Achromatic colors (r == g == b) → `h = 0`, `s = 0`
/// * Pure white `(1, 1, 1)` → `(0, 0, 1)`
/// * Pure black `(0, 0, 0)` → `(0, 0, 0)`
///
/// # Example
/// ```rust
/// let (h, s, l) = prime_color::srgb_to_hsl(1.0, 0.0, 0.0);
/// assert!((h - 0.0).abs() < 1e-4);
/// assert!((s - 1.0).abs() < 1e-4);
/// assert!((l - 0.5).abs() < 1e-4);
/// ```
pub fn srgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) * 0.5;

    let s = if delta < f32::EPSILON {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta < f32::EPSILON {
        0.0
    } else if max == r {
        let raw = (g - b) / delta;
        // fmod into [0, 6) then scale
        let sector = raw - 6.0 * (raw / 6.0).floor();
        60.0 * sector
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    // Ensure h is in [0, 360)
    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, l)
}

/// Helper used by `hsl_to_srgb`.
///
/// # Math
///   Standard HSL-to-RGB sector function.
///   p, q computed from l and s; k selects the hue sector.
#[inline]
fn hsl_component(p: f32, q: f32, t: f32) -> f32 {
    let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// Converts an HSL color to sRGB.
///
/// # Math
///   if s == 0: r = g = b = l  (achromatic)
///   else:
///     q = l + s - l*s          if l < 0.5
///         l*(1+s)               otherwise
///     p = 2*l - q
///     r = hsl_component(p, q, h/360 + 1/3)
///     g = hsl_component(p, q, h/360)
///     b = hsl_component(p, q, h/360 - 1/3)
///
/// # Arguments
/// * `h` - hue in degrees, `[0, 360)` (values outside are wrapped)
/// * `s` - saturation, `[0, 1]`
/// * `l` - lightness, `[0, 1]`
///
/// # Returns
/// sRGB tuple `(r, g, b)`, each in `[0, 1]`.
///
/// # Edge cases
/// * `s == 0` → achromatic gray at lightness `l`
/// * `l == 0` → `(0, 0, 0)` regardless of hue or saturation
/// * `l == 1` → `(1, 1, 1)` regardless of hue or saturation
/// * Hue wraps naturally via the modular arithmetic in `hsl_component`.
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::hsl_to_srgb(120.0, 1.0, 0.5);
/// // Pure green
/// assert!(r < 1e-4);
/// assert!((g - 1.0).abs() < 1e-4);
/// assert!(b < 1e-4);
/// ```
pub fn hsl_to_srgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < f32::EPSILON {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hk = h / 360.0;

    let r = hsl_component(p, q, hk + 1.0 / 3.0);
    let g = hsl_component(p, q, hk);
    let b = hsl_component(p, q, hk - 1.0 / 3.0);

    (r, g, b)
}

// ---------------------------------------------------------------------------
// Perceptual mix in Oklab
// ---------------------------------------------------------------------------

/// Linearly interpolates two values.
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Mixes two sRGB colors in Oklab space for perceptually uniform blending.
///
/// # Math
///   1. Convert `(r0, g0, b0)` and `(r1, g1, b1)` to Oklab.
///   2. Lerp each Lab component: L = lerp(L0, L1, t), a = lerp(a0, a1, t), b = lerp(b0, b1, t).
///   3. Convert blended Lab back to sRGB (clamped to `[0, 1]`).
///
///   lerp(a, b, t) = a + (b - a) * t
///
/// # Arguments
/// * `r0, g0, b0` - first color in sRGB, `[0, 1]`
/// * `r1, g1, b1` - second color in sRGB, `[0, 1]`
/// * `t` - blend factor; `0.0` returns the first color, `1.0` returns the second
///
/// # Returns
/// Blended color in sRGB, `(r, g, b)` clamped to `[0, 1]`.
///
/// # Edge cases
/// * `t = 0.0` → returns first color (approximately; subject to round-trip rounding)
/// * `t = 1.0` → returns second color (approximately)
/// * `t` outside `[0, 1]` extrapolates; output is still clamped.
///
/// # Example
/// ```rust
/// // Midpoint between red and blue
/// let (r, g, b) = prime_color::oklab_mix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5);
/// // Should be a perceptually even purple, not the flat sRGB average
/// assert!(r > 0.0 && b > 0.0);
/// ```
pub fn oklab_mix(
    r0: f32, g0: f32, b0: f32,
    r1: f32, g1: f32, b1: f32,
    t: f32,
) -> (f32, f32, f32) {
    let (l0, a0, bv0) = srgb_to_oklab(r0, g0, b0);
    let (l1, a1, bv1) = srgb_to_oklab(r1, g1, b1);

    let l = lerp(l0, l1, t);
    let a = lerp(a0, a1, t);
    let b = lerp(bv0, bv1, t);

    oklab_to_srgb(l, a, b)
}

// ---------------------------------------------------------------------------
// HSV ↔ sRGB
// ---------------------------------------------------------------------------

/// Convert sRGB to HSV. Returns `(h, s, v)` where `h` in `[0, 360)`, `s` and `v` in `[0, 1]`.
///
/// # Math
///   max = max(r, g, b), min = min(r, g, b), delta = max - min
///
///   V = max
///   S = 0              if max == 0
///       delta / max    otherwise
///   H = (same sector logic as HSL)
///
/// # Edge cases
/// * Achromatic inputs → `h = 0`, `s = 0`
/// * Pure black → `(0, 0, 0)`
/// * Pure white → `(0, 0, 1)`
///
/// # Example
/// ```rust
/// let (h, s, v) = prime_color::srgb_to_hsv(1.0, 0.0, 0.0);
/// assert!((h - 0.0).abs() < 1e-4);
/// assert!((s - 1.0).abs() < 1e-4);
/// assert!((v - 1.0).abs() < 1e-4);
/// ```
pub fn srgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Convert HSV to sRGB. `h` in `[0, 360)`, `s` and `v` in `[0, 1]`.
///
/// # Math
///   C = V * S
///   H' = H / 60
///   X = C * (1 - |H' mod 2 - 1|)
///   m = V - C
///   Select (R1, G1, B1) by sector, then add m.
///
/// # Edge cases
/// * `s == 0` → achromatic gray at value `v`
/// * `v == 0` → black regardless of h or s
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::hsv_to_srgb(120.0, 1.0, 1.0);
/// assert!(r < 1e-4);
/// assert!((g - 1.0).abs() < 1e-4);
/// assert!(b < 1e-4);
/// ```
pub fn hsv_to_srgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - ((h2 % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h2 < 1.0 {
        (c, x, 0.0)
    } else if h2 < 2.0 {
        (x, c, 0.0)
    } else if h2 < 3.0 {
        (0.0, c, x)
    } else if h2 < 4.0 {
        (0.0, x, c)
    } else if h2 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r + m, g + m, b + m)
}

// ---------------------------------------------------------------------------
// Luminance utilities
// ---------------------------------------------------------------------------

/// Relative luminance (ITU-R BT.709). Input is linear RGB.
///
/// # Math
///   Y = 0.2126 * R + 0.7152 * G + 0.0722 * B
///
/// # Example
/// ```rust
/// let y = prime_color::luminance(1.0, 1.0, 1.0);
/// assert!((y - 1.0).abs() < 1e-4);
/// ```
pub fn luminance(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio between two sRGB colors. Returns value >= 1.
///
/// # Math
///   Converts both colors to linear RGB, computes relative luminance for each,
///   then: ratio = (L_lighter + 0.05) / (L_darker + 0.05)
///
/// # Example
/// ```rust
/// // White on black has maximum contrast (~21:1).
/// let ratio = prime_color::contrast_ratio(1.0, 1.0, 1.0, 0.0, 0.0, 0.0);
/// assert!(ratio >= 20.9);
/// ```
pub fn contrast_ratio(r0: f32, g0: f32, b0: f32, r1: f32, g1: f32, b1: f32) -> f32 {
    let (lr0, lg0, lb0) = srgb_to_linear(r0, g0, b0);
    let (lr1, lg1, lb1) = srgb_to_linear(r1, g1, b1);
    let l0 = luminance(lr0, lg0, lb0);
    let l1 = luminance(lr1, lg1, lb1);
    let (lighter, darker) = if l0 > l1 { (l0, l1) } else { (l1, l0) };
    (lighter + 0.05) / (darker + 0.05)
}

// ---------------------------------------------------------------------------
// Palette generation
// ---------------------------------------------------------------------------

/// Complementary color — rotate hue by 180 degrees in HSL.
///
/// # Example
/// ```rust
/// let (r, g, b) = prime_color::palette_complementary(1.0, 0.0, 0.0);
/// // Red's complement is cyan
/// assert!(r < 0.1);
/// assert!((g - 1.0).abs() < 1e-3);
/// assert!((b - 1.0).abs() < 1e-3);
/// ```
pub fn palette_complementary(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (h, s, l) = srgb_to_hsl(r, g, b);
    hsl_to_srgb((h + 180.0) % 360.0, s, l)
}

/// Triadic palette — two colors at +120 and +240 degrees in HSL.
///
/// # Example
/// ```rust
/// let ((r1, g1, b1), (r2, g2, b2)) = prime_color::palette_triadic(1.0, 0.0, 0.0);
/// // Red → green-ish, blue-ish
/// assert!(g1 > 0.9);
/// assert!(b2 > 0.9);
/// ```
pub fn palette_triadic(r: f32, g: f32, b: f32) -> ((f32, f32, f32), (f32, f32, f32)) {
    let (h, s, l) = srgb_to_hsl(r, g, b);
    (
        hsl_to_srgb((h + 120.0) % 360.0, s, l),
        hsl_to_srgb((h + 240.0) % 360.0, s, l),
    )
}

/// Analogous palette — two colors at +30 and -30 degrees in HSL.
///
/// # Example
/// ```rust
/// let ((r1, g1, _b1), (r2, _g2, b2)) = prime_color::palette_analogous(1.0, 0.0, 0.0);
/// // Red ±30° → orange-ish and magenta-ish
/// assert!(r1 > 0.5);
/// assert!(r2 > 0.5);
/// ```
pub fn palette_analogous(r: f32, g: f32, b: f32) -> ((f32, f32, f32), (f32, f32, f32)) {
    let (h, s, l) = srgb_to_hsl(r, g, b);
    (
        hsl_to_srgb((h + 30.0) % 360.0, s, l),
        hsl_to_srgb((h + 330.0) % 360.0, s, l),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    // --- helpers ---

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn tuple_approx_eq(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
        approx_eq(a.0, b.0) && approx_eq(a.1, b.1) && approx_eq(a.2, b.2)
    }

    // --- sRGB ↔ Linear round-trips ---

    #[test]
    fn srgb_linear_roundtrip_mid_gray() {
        let original = (0.5_f32, 0.5_f32, 0.5_f32);
        let linear = srgb_to_linear(original.0, original.1, original.2);
        let back = linear_to_srgb(linear.0, linear.1, linear.2);
        assert!(tuple_approx_eq(back, original), "round-trip mid-gray failed: {:?}", back);
    }

    #[test]
    fn srgb_linear_roundtrip_black() {
        let linear = srgb_to_linear(0.0, 0.0, 0.0);
        let back = linear_to_srgb(linear.0, linear.1, linear.2);
        assert!(tuple_approx_eq(back, (0.0, 0.0, 0.0)));
    }

    #[test]
    fn srgb_linear_roundtrip_white() {
        let linear = srgb_to_linear(1.0, 1.0, 1.0);
        let back = linear_to_srgb(linear.0, linear.1, linear.2);
        assert!(tuple_approx_eq(back, (1.0, 1.0, 1.0)));
    }

    #[test]
    fn srgb_to_linear_known_value() {
        // 0.5 sRGB → ~0.2140 linear (standard reference)
        let (r, _, _) = srgb_to_linear(0.5, 0.5, 0.5);
        assert!((r - 0.2140).abs() < 1e-3, "expected ~0.2140, got {}", r);
    }

    #[test]
    fn linear_to_srgb_known_value() {
        let (r, _, _) = linear_to_srgb(0.2140, 0.2140, 0.2140);
        assert!((r - 0.5).abs() < 1e-3, "expected ~0.5, got {}", r);
    }

    // --- Oklab round-trips ---

    #[test]
    fn oklab_roundtrip_red() {
        let original = (1.0_f32, 0.0_f32, 0.0_f32);
        let lab = srgb_to_oklab(original.0, original.1, original.2);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, original), "round-trip red failed: {:?}", back);
    }

    #[test]
    fn oklab_roundtrip_green() {
        let original = (0.0_f32, 1.0_f32, 0.0_f32);
        let lab = srgb_to_oklab(original.0, original.1, original.2);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, original), "round-trip green failed: {:?}", back);
    }

    #[test]
    fn oklab_roundtrip_blue() {
        let original = (0.0_f32, 0.0_f32, 1.0_f32);
        let lab = srgb_to_oklab(original.0, original.1, original.2);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, original), "round-trip blue failed: {:?}", back);
    }

    #[test]
    fn oklab_roundtrip_black() {
        let lab = srgb_to_oklab(0.0, 0.0, 0.0);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, (0.0, 0.0, 0.0)), "{:?}", back);
    }

    #[test]
    fn oklab_roundtrip_white() {
        let lab = srgb_to_oklab(1.0, 1.0, 1.0);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, (1.0, 1.0, 1.0)), "{:?}", back);
    }

    #[test]
    fn oklab_roundtrip_arbitrary() {
        let original = (0.3_f32, 0.6_f32, 0.9_f32);
        let lab = srgb_to_oklab(original.0, original.1, original.2);
        let back = oklab_to_srgb(lab.0, lab.1, lab.2);
        assert!(tuple_approx_eq(back, original), "round-trip arbitrary failed: {:?}", back);
    }

    // --- Oklab known values (from reference implementation) ---

    #[test]
    fn oklab_known_red() {
        // sRGB red → Oklab reference values from bottosson.github.io
        let (l, a, b) = srgb_to_oklab(1.0, 0.0, 0.0);
        assert!((l - 0.6280).abs() < 1e-3, "L={}", l);
        assert!((a - 0.2247).abs() < 1e-3, "a={}", a);
        assert!((b - 0.1260).abs() < 1e-3, "b={}", b);
    }

    #[test]
    fn oklab_known_white() {
        let (l, a, b) = srgb_to_oklab(1.0, 1.0, 1.0);
        assert!((l - 1.0).abs() < 1e-3, "L={}", l);
        assert!(a.abs() < 1e-3, "a={}", a);
        assert!(b.abs() < 1e-3, "b={}", b);
    }

    #[test]
    fn oklab_known_black() {
        let (l, a, b) = srgb_to_oklab(0.0, 0.0, 0.0);
        assert!(l.abs() < EPS, "L={}", l);
        assert!(a.abs() < EPS, "a={}", a);
        assert!(b.abs() < EPS, "b={}", b);
    }

    // --- Oklab output range ---

    #[test]
    fn oklab_l_in_range_for_srgb_gamut() {
        // Spot-check that L stays in [0, 1] for corners of the sRGB cube
        let corners = [
            (0.0_f32, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0), (1.0, 1.0, 0.0), (0.0, 1.0, 1.0),
            (1.0, 0.0, 1.0), (1.0, 1.0, 1.0),
        ];
        for (r, g, b) in corners {
            let (l, _, _) = srgb_to_oklab(r, g, b);
            assert!(l >= -EPS && l <= 1.0 + EPS, "L out of range for ({},{},{}): L={}", r, g, b, l);
        }
    }

    // --- HSL round-trips ---

    #[test]
    fn hsl_roundtrip_red() {
        let original = (1.0_f32, 0.0_f32, 0.0_f32);
        let (h, s, l) = srgb_to_hsl(original.0, original.1, original.2);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, original), "round-trip red: {:?}", back);
    }

    #[test]
    fn hsl_roundtrip_green() {
        let original = (0.0_f32, 1.0_f32, 0.0_f32);
        let (h, s, l) = srgb_to_hsl(original.0, original.1, original.2);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, original), "round-trip green: {:?}", back);
    }

    #[test]
    fn hsl_roundtrip_blue() {
        let original = (0.0_f32, 0.0_f32, 1.0_f32);
        let (h, s, l) = srgb_to_hsl(original.0, original.1, original.2);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, original), "round-trip blue: {:?}", back);
    }

    #[test]
    fn hsl_roundtrip_arbitrary() {
        let original = (0.2_f32, 0.7_f32, 0.4_f32);
        let (h, s, l) = srgb_to_hsl(original.0, original.1, original.2);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, original), "round-trip arbitrary: {:?}", back);
    }

    #[test]
    fn hsl_roundtrip_black() {
        let (h, s, l) = srgb_to_hsl(0.0, 0.0, 0.0);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, (0.0, 0.0, 0.0)));
    }

    #[test]
    fn hsl_roundtrip_white() {
        let (h, s, l) = srgb_to_hsl(1.0, 1.0, 1.0);
        let back = hsl_to_srgb(h, s, l);
        assert!(tuple_approx_eq(back, (1.0, 1.0, 1.0)));
    }

    // --- HSL known values ---

    #[test]
    fn hsl_known_red() {
        let (h, s, l) = srgb_to_hsl(1.0, 0.0, 0.0);
        assert!(approx_eq(h, 0.0) || approx_eq(h, 360.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(l, 0.5), "l={}", l);
    }

    #[test]
    fn hsl_known_green() {
        let (h, s, l) = srgb_to_hsl(0.0, 1.0, 0.0);
        assert!(approx_eq(h, 120.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(l, 0.5), "l={}", l);
    }

    #[test]
    fn hsl_known_blue() {
        let (h, s, l) = srgb_to_hsl(0.0, 0.0, 1.0);
        assert!(approx_eq(h, 240.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(l, 0.5), "l={}", l);
    }

    #[test]
    fn hsl_known_cyan() {
        let (h, s, l) = srgb_to_hsl(0.0, 1.0, 1.0);
        assert!(approx_eq(h, 180.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(l, 0.5), "l={}", l);
    }

    #[test]
    fn hsl_known_mid_gray() {
        let (_, s, l) = srgb_to_hsl(0.5, 0.5, 0.5);
        assert!(approx_eq(s, 0.0), "s={}", s);
        assert!(approx_eq(l, 0.5), "l={}", l);
    }

    // --- HSL hue range ---

    #[test]
    fn hsl_hue_in_range() {
        let samples = [
            (1.0_f32, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0),
            (1.0, 1.0, 0.0), (0.0, 1.0, 1.0), (1.0, 0.0, 1.0),
            (0.3, 0.6, 0.9), (0.9, 0.1, 0.4),
        ];
        for (r, g, b) in samples {
            let (h, _, _) = srgb_to_hsl(r, g, b);
            assert!(h >= 0.0 && h < 360.0 + EPS, "hue out of range for ({},{},{}): h={}", r, g, b, h);
        }
    }

    // --- oklab_mix ---

    #[test]
    fn oklab_mix_at_zero_returns_first_color() {
        let (r, g, b) = oklab_mix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
        assert!(approx_eq(r, 1.0), "r={}", r);
        assert!(approx_eq(g, 0.0), "g={}", g);
        assert!(approx_eq(b, 0.0), "b={}", b);
    }

    #[test]
    fn oklab_mix_at_one_returns_second_color() {
        let (r, g, b) = oklab_mix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert!(approx_eq(r, 0.0), "r={}", r);
        assert!(approx_eq(g, 0.0), "g={}", g);
        assert!(approx_eq(b, 1.0), "b={}", b);
    }

    #[test]
    fn oklab_mix_midpoint_is_in_gamut() {
        let (r, g, b) = oklab_mix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5);
        assert!(r >= 0.0 && r <= 1.0, "r out of gamut: {}", r);
        assert!(g >= 0.0 && g <= 1.0, "g out of gamut: {}", g);
        assert!(b >= 0.0 && b <= 1.0, "b out of gamut: {}", b);
    }

    #[test]
    fn oklab_mix_symmetric() {
        // mix(a, b, 0.5) should equal mix(b, a, 0.5)
        let m1 = oklab_mix(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5);
        let m2 = oklab_mix(0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.5);
        assert!(tuple_approx_eq(m1, m2), "not symmetric: {:?} vs {:?}", m1, m2);
    }

    #[test]
    fn oklab_mix_black_white_midpoint() {
        // Midpoint between black and white in Oklab should be a medium gray
        let (r, g, b) = oklab_mix(0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5);
        // All channels should be equal (achromatic)
        assert!((r - g).abs() < EPS && (g - b).abs() < EPS, "not gray: {:?}", (r, g, b));
        // Should be roughly mid-gray in sRGB (not necessarily 0.5 exactly due to Oklab L)
        assert!(r > 0.1 && r < 0.9, "gray value out of expected range: {}", r);
    }

    // ── out-of-range [0,1] inputs ─────────────────────────────────────────────

    #[test]
    fn srgb_to_oklab_over_range_is_finite() {
        // sRGB > 1.0 (HDR-style) — must not produce NaN.
        let (l, a, b) = srgb_to_oklab(1.5, 0.5, 0.5);
        assert!(l.is_finite() && a.is_finite() && b.is_finite(),
            "srgb_to_oklab(1.5,...) produced non-finite: {:?}", (l, a, b));
    }

    #[test]
    fn srgb_to_linear_over_range_is_finite() {
        let (lr, lg, lb) = srgb_to_linear(2.0, 0.0, 0.0);
        assert!(lr.is_finite() && lg.is_finite() && lb.is_finite());
    }

    #[test]
    fn oklab_to_srgb_out_of_gamut_is_finite() {
        // Push an extreme Oklab value — output may be out of [0,1] but must be finite.
        let (r, g, b) = oklab_to_srgb(1.0, 0.5, 0.5);
        assert!(r.is_finite() && g.is_finite() && b.is_finite(),
            "oklab_to_srgb with large a/b produced non-finite: {:?}", (r, g, b));
    }

    #[test]
    fn hsl_to_srgb_hue_boundary_is_finite() {
        // h=0.0 and h=1.0 are both valid boundary values — must be finite.
        let at_0 = hsl_to_srgb(0.0, 1.0, 0.5);
        let at_1 = hsl_to_srgb(1.0, 1.0, 0.5);
        assert!(at_0.0.is_finite() && at_0.1.is_finite() && at_0.2.is_finite());
        assert!(at_1.0.is_finite() && at_1.1.is_finite() && at_1.2.is_finite());
        // Both endpoints should be red-family (high red channel)
        assert!(at_0.0 > 0.9 && at_1.0 > 0.9,
            "h=0 and h=1 should both be red-family; got {:?} and {:?}", at_0, at_1);
    }

    // --- HSV round-trips ---

    #[test]
    fn hsv_roundtrip_red() {
        let original = (1.0_f32, 0.0_f32, 0.0_f32);
        let (h, s, v) = srgb_to_hsv(original.0, original.1, original.2);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, original), "round-trip red: {:?}", back);
    }

    #[test]
    fn hsv_roundtrip_green() {
        let original = (0.0_f32, 1.0_f32, 0.0_f32);
        let (h, s, v) = srgb_to_hsv(original.0, original.1, original.2);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, original), "round-trip green: {:?}", back);
    }

    #[test]
    fn hsv_roundtrip_blue() {
        let original = (0.0_f32, 0.0_f32, 1.0_f32);
        let (h, s, v) = srgb_to_hsv(original.0, original.1, original.2);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, original), "round-trip blue: {:?}", back);
    }

    #[test]
    fn hsv_roundtrip_arbitrary() {
        let original = (0.3_f32, 0.6_f32, 0.9_f32);
        let (h, s, v) = srgb_to_hsv(original.0, original.1, original.2);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, original), "round-trip arbitrary: {:?}", back);
    }

    #[test]
    fn hsv_roundtrip_black() {
        let (h, s, v) = srgb_to_hsv(0.0, 0.0, 0.0);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, (0.0, 0.0, 0.0)));
    }

    #[test]
    fn hsv_roundtrip_white() {
        let (h, s, v) = srgb_to_hsv(1.0, 1.0, 1.0);
        let back = hsv_to_srgb(h, s, v);
        assert!(tuple_approx_eq(back, (1.0, 1.0, 1.0)));
    }

    #[test]
    fn hsv_known_red() {
        let (h, s, v) = srgb_to_hsv(1.0, 0.0, 0.0);
        assert!(approx_eq(h, 0.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(v, 1.0), "v={}", v);
    }

    #[test]
    fn hsv_known_green() {
        let (h, s, v) = srgb_to_hsv(0.0, 1.0, 0.0);
        assert!(approx_eq(h, 120.0), "h={}", h);
        assert!(approx_eq(s, 1.0), "s={}", s);
        assert!(approx_eq(v, 1.0), "v={}", v);
    }

    // --- Luminance ---

    #[test]
    fn luminance_white() {
        let y = luminance(1.0, 1.0, 1.0);
        assert!(approx_eq(y, 1.0), "luminance(white)={}", y);
    }

    #[test]
    fn luminance_black() {
        let y = luminance(0.0, 0.0, 0.0);
        assert!(approx_eq(y, 0.0), "luminance(black)={}", y);
    }

    #[test]
    fn luminance_green_dominates() {
        // Green has the largest coefficient (0.7152)
        let yr = luminance(1.0, 0.0, 0.0);
        let yg = luminance(0.0, 1.0, 0.0);
        let yb = luminance(0.0, 0.0, 1.0);
        assert!(yg > yr && yg > yb, "green should dominate: r={} g={} b={}", yr, yg, yb);
    }

    // --- Contrast ratio ---

    #[test]
    fn contrast_ratio_white_black() {
        let ratio = contrast_ratio(1.0, 1.0, 1.0, 0.0, 0.0, 0.0);
        assert!(ratio >= 21.0 - 0.1, "white/black contrast={}", ratio);
    }

    #[test]
    fn contrast_ratio_same_color() {
        let ratio = contrast_ratio(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);
        assert!(approx_eq(ratio, 1.0), "same color contrast={}", ratio);
    }

    #[test]
    fn contrast_ratio_symmetric() {
        let r1 = contrast_ratio(1.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let r2 = contrast_ratio(0.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        assert!(approx_eq(r1, r2), "not symmetric: {} vs {}", r1, r2);
    }

    // --- Palette ---

    #[test]
    fn palette_complementary_red_to_cyan() {
        let (r, g, b) = palette_complementary(1.0, 0.0, 0.0);
        // Red's complement in HSL is cyan (0, 1, 1)
        assert!(r < 0.1, "r={}", r);
        assert!((g - 1.0).abs() < 1e-3, "g={}", g);
        assert!((b - 1.0).abs() < 1e-3, "b={}", b);
    }

    #[test]
    fn palette_complementary_roundtrip() {
        // Complement of complement should return original
        let original = (0.8_f32, 0.2_f32, 0.4_f32);
        let comp = palette_complementary(original.0, original.1, original.2);
        let back = palette_complementary(comp.0, comp.1, comp.2);
        assert!(tuple_approx_eq(back, original),
            "double complement failed: {:?} vs {:?}", back, original);
    }

    #[test]
    fn palette_triadic_red() {
        let ((r1, g1, b1), (r2, g2, b2)) = palette_triadic(1.0, 0.0, 0.0);
        // Red + 120° = green, Red + 240° = blue
        assert!(g1 > 0.9, "first triadic should be green-ish: ({},{},{})", r1, g1, b1);
        assert!(b2 > 0.9, "second triadic should be blue-ish: ({},{},{})", r2, g2, b2);
    }

    #[test]
    fn palette_analogous_red() {
        let ((r1, _g1, _b1), (r2, _g2, _b2)) = palette_analogous(1.0, 0.0, 0.0);
        // Both analogous colors of red should still have significant red
        assert!(r1 > 0.5, "first analogous should be reddish: r={}", r1);
        assert!(r2 > 0.5, "second analogous should be reddish: r={}", r2);
    }
}
