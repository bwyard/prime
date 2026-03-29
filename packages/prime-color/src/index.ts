/**
 * `prime-color` — Color space math: sRGB, Oklab, HSL, and perceptual mixing.
 *
 * All functions are pure (LOAD + COMPUTE only). No mutation, no side effects,
 * no hidden state. Same inputs always produce the same output.
 *
 * Colors are represented as plain `number` tuples for simplicity and interop.
 * No classes, no objects with methods.
 *
 * @remarks
 * Color spaces supported:
 * - **sRGB** — standard display color space, gamma-encoded, components in `[0, 1]`
 * - **Linear RGB** — gamma-decoded sRGB; linear light values used for matrix math
 * - **Oklab** — perceptually uniform opponent color space by Björn Ottosson (2020)
 *   - `L` ∈ `[0, 1]` (lightness), `a` ∈ `[-0.5, 0.5]` (green↔red), `b` ∈ `[-0.5, 0.5]` (blue↔yellow)
 * - **HSL** — hue/saturation/lightness cylinder mapped onto sRGB
 *   - `h` ∈ `[0, 360)`, `s` ∈ `[0, 1]`, `l` ∈ `[0, 1]`
 *
 * @module prime-color
 */

// ---------------------------------------------------------------------------
// Internal helpers — not exported
// ---------------------------------------------------------------------------

/**
 * Converts a single sRGB channel to linear light (gamma decode).
 *
 * @remarks
 * Math:
 * ```
 * if c <= 0.04045:  c / 12.92
 * else:             ((c + 0.055) / 1.055) ^ 2.4
 * ```
 */
const srgbChannelToLinear = (c: number): number =>
  c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);

/**
 * Converts a single linear light channel to sRGB (gamma encode).
 *
 * @remarks
 * Math:
 * ```
 * if c <= 0.0031308:  12.92 * c
 * else:               1.055 * c^(1/2.4) - 0.055
 * ```
 */
const linearChannelToSrgb = (c: number): number =>
  c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1.0 / 2.4) - 0.055;

// ---------------------------------------------------------------------------
// Oklab matrices — verbatim from Björn Ottosson's reference implementation.
// Reference: https://bottosson.github.io/posts/oklab/
// No patents. Matrices used verbatim from the original post.
// ---------------------------------------------------------------------------

/** sRGB (linear) → LMS matrix (M1). Rows are [l_coef, m_coef, s_coef] per output channel. */
const M1: readonly [
  readonly [number, number, number],
  readonly [number, number, number],
  readonly [number, number, number],
] = [
  [0.4122214708, 0.5363325363, 0.0514459929],
  [0.2119034982, 0.6806995451, 0.1073969566],
  [0.0883024619, 0.2817188376, 0.6299787005],
];

/** LMS^(1/3) → Lab matrix (M2). */
const M2: readonly [
  readonly [number, number, number],
  readonly [number, number, number],
  readonly [number, number, number],
] = [
  [ 0.2104542553,  0.7936177850, -0.0040720468],
  [ 1.9779984951, -2.4285922050,  0.4505937099],
  [ 0.0259040371,  0.7827717662, -0.8086757660],
];

/** Lab → LMS^(1/3) matrix (M2 inverse). */
const M2_INV: readonly [
  readonly [number, number, number],
  readonly [number, number, number],
  readonly [number, number, number],
] = [
  [1.0,  0.3963377774,  0.2158037573],
  [1.0, -0.1055613458, -0.0638541728],
  [1.0, -0.0894841775, -1.2914855480],
];

/** LMS → sRGB (linear) matrix (M1 inverse). */
const M1_INV: readonly [
  readonly [number, number, number],
  readonly [number, number, number],
  readonly [number, number, number],
] = [
  [ 4.0767416621, -3.3077115913,  0.2309699292],
  [-1.2684380046,  2.6097574011, -0.3413193965],
  [-0.0041960863, -0.7034186147,  1.7076147010],
];

/**
 * Applies a 3×3 matrix to a column vector [x, y, z].
 */
const mat3Mul = (
  m: readonly [
    readonly [number, number, number],
    readonly [number, number, number],
    readonly [number, number, number],
  ],
  x: number,
  y: number,
  z: number,
): [number, number, number] => [
  m[0][0] * x + m[0][1] * y + m[0][2] * z,
  m[1][0] * x + m[1][1] * y + m[1][2] * z,
  m[2][0] * x + m[2][1] * y + m[2][2] * z,
];

/**
 * Signed cube root — preserves sign for negative values.
 *
 * @remarks
 * Math: `sign(x) * |x|^(1/3)`
 */
const cbrt = (x: number): number =>
  x < 0 ? -Math.pow(-x, 1.0 / 3.0) : Math.pow(x, 1.0 / 3.0);

/**
 * Linearly interpolates two values.
 *
 * @remarks
 * Math: `a + (b - a) * t`
 */
const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/**
 * HSL helper — computes one sRGB channel from HSL `p`, `q`, and offset `t`.
 *
 * @remarks
 * Standard HSL-to-RGB sector function. `p` and `q` are derived from `l` and
 * `s`; `t` is the normalized hue shifted by ±1/3 for the R, G, B channels.
 *
 * Math:
 * ```
 * Wrap t into [0, 1].
 * if t < 1/6:  p + (q - p) * 6 * t
 * if t < 1/2:  q
 * if t < 2/3:  p + (q - p) * (2/3 - t) * 6
 * else:         p
 * ```
 */
const hslComponent = (p: number, q: number, tRaw: number): number => {
  const t = tRaw < 0 ? tRaw + 1.0 : tRaw > 1.0 ? tRaw - 1.0 : tRaw;
  if (t < 1.0 / 6.0) return p + (q - p) * 6.0 * t;
  if (t < 1.0 / 2.0) return q;
  if (t < 2.0 / 3.0) return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
  return p;
};

// ---------------------------------------------------------------------------
// Public API — sRGB ↔ Linear RGB
// ---------------------------------------------------------------------------

/**
 * Converts an sRGB color (gamma-encoded) to linear RGB.
 *
 * @remarks
 * Per-channel gamma decode (IEC 61966-2-1):
 * ```
 * if c <= 0.04045:  c / 12.92
 * else:             ((c + 0.055) / 1.055) ^ 2.4
 * ```
 * Values outside `[0, 1]` are preserved (not clamped). Continuity at the
 * threshold is guaranteed by the IEC standard.
 *
 * @param r - red component in sRGB, typically `[0, 1]`
 * @param g - green component in sRGB, typically `[0, 1]`
 * @param b - blue component in sRGB, typically `[0, 1]`
 * @returns Linear RGB tuple `[r, g, b]`. Out-of-range values are not clamped.
 *
 * @example
 * ```ts
 * const [r, g, b] = srgbToLinear(0.5, 0.5, 0.5);
 * // approximately [0.2140, 0.2140, 0.2140]
 * ```
 */
export const srgbToLinear = (
  r: number,
  g: number,
  b: number,
): [number, number, number] => [
  srgbChannelToLinear(r),
  srgbChannelToLinear(g),
  srgbChannelToLinear(b),
];

/**
 * Converts a linear RGB color to sRGB (gamma-encoded).
 *
 * @remarks
 * Per-channel gamma encode (IEC 61966-2-1):
 * ```
 * if c <= 0.0031308:  12.92 * c
 * else:               1.055 * c^(1/2.4) - 0.055
 * ```
 * Values outside `[0, 1]` are preserved (not clamped). Callers must clamp
 * if display-safe output is required.
 *
 * @param r - red component in linear RGB, typically `[0, 1]`
 * @param g - green component in linear RGB, typically `[0, 1]`
 * @param b - blue component in linear RGB, typically `[0, 1]`
 * @returns sRGB tuple `[r, g, b]`. Out-of-range values are not clamped.
 *
 * @example
 * ```ts
 * const [r, g, b] = linearToSrgb(0.2140, 0.2140, 0.2140);
 * // approximately [0.5, 0.5, 0.5]
 * ```
 */
export const linearToSrgb = (
  r: number,
  g: number,
  b: number,
): [number, number, number] => [
  linearChannelToSrgb(r),
  linearChannelToSrgb(g),
  linearChannelToSrgb(b),
];

// ---------------------------------------------------------------------------
// Public API — sRGB ↔ Oklab
// ---------------------------------------------------------------------------

/**
 * Converts an sRGB color to Oklab.
 *
 * @remarks
 * Four-step pipeline (Björn Ottosson, 2020):
 * ```
 * 1. Gamma-decode sRGB → linear RGB  (srgbToLinear)
 * 2. Linear RGB × M1  → LMS cone responses
 * 3. Per-channel cube root:  l' = cbrt(l),  m' = cbrt(m),  s' = cbrt(s)
 * 4. [l', m', s'] × M2  → [L, a, b]
 * ```
 * Matrix values are taken verbatim from the reference post.
 *
 * @param r - red component in sRGB, `[0, 1]`
 * @param g - green component in sRGB, `[0, 1]`
 * @param b - blue component in sRGB, `[0, 1]`
 * @returns `[L, a, b]` in Oklab:
 *   - `L` ∈ `[0, 1]` — lightness
 *   - `a` ∈ `[-0.5, 0.5]` approximately — green↔red axis
 *   - `b` ∈ `[-0.5, 0.5]` approximately — blue↔yellow axis
 *
 * @example
 * ```ts
 * const [l, a, b] = srgbToOklab(1.0, 0.0, 0.0);
 * // Red in Oklab: L ≈ 0.6280, a ≈ 0.2247, b ≈ 0.1260
 * ```
 */
export const srgbToOklab = (
  r: number,
  g: number,
  b: number,
): [number, number, number] => {
  // Step 1: gamma decode
  const [lr, lg, lb] = srgbToLinear(r, g, b);

  // Step 2: linear RGB → LMS
  const [l, m, s] = mat3Mul(M1, lr, lg, lb);

  // Step 3: cube root
  const lp = cbrt(l);
  const mp = cbrt(m);
  const sp = cbrt(s);

  // Step 4: LMS^(1/3) → Lab
  return mat3Mul(M2, lp, mp, sp);
};

/**
 * Converts an Oklab color back to sRGB.
 *
 * @remarks
 * Five-step pipeline (Björn Ottosson, 2020):
 * ```
 * 1. [L, a, b] × M2_INV  → [l', m', s']  (LMS^(1/3))
 * 2. Per-channel cube:  l = l'^3,  m = m'^3,  s = s'^3
 * 3. [l, m, s] × M1_INV  → linear RGB
 * 4. Gamma-encode linear RGB → sRGB  (linearToSrgb)
 * 5. Clamp all components to [0, 1]
 * ```
 *
 * @param l - Oklab lightness, `[0, 1]`
 * @param a - Oklab a axis (green↔red)
 * @param b - Oklab b axis (blue↔yellow)
 * @returns sRGB tuple `[r, g, b]`, each clamped to `[0, 1]`.
 *
 * @example
 * ```ts
 * const [r, g, b] = oklabToSrgb(0.6280, 0.2247, 0.1260);
 * // approximately red: r ≈ 1.0, g ≈ 0, b ≈ 0
 * ```
 */
export const oklabToSrgb = (
  l: number,
  a: number,
  b: number,
): [number, number, number] => {
  // Step 1: Lab → LMS^(1/3)
  const [lp, mp, sp] = mat3Mul(M2_INV, l, a, b);

  // Step 2: cube
  const lc = lp * lp * lp;
  const mc = mp * mp * mp;
  const sc = sp * sp * sp;

  // Step 3: LMS → linear RGB
  const [lr, lg, lb] = mat3Mul(M1_INV, lc, mc, sc);

  // Step 4: gamma encode
  const [r, g, bv] = linearToSrgb(lr, lg, lb);

  // Step 5: clamp to [0, 1]
  return [
    Math.max(0.0, Math.min(1.0, r)),
    Math.max(0.0, Math.min(1.0, g)),
    Math.max(0.0, Math.min(1.0, bv)),
  ];
};

// ---------------------------------------------------------------------------
// Public API — sRGB ↔ HSL
// ---------------------------------------------------------------------------

/**
 * Converts an sRGB color to HSL.
 *
 * @remarks
 * Standard HSL derivation from RGB min/max:
 * ```
 * max = max(r, g, b),  min = min(r, g, b),  delta = max - min
 *
 * L = (max + min) / 2
 *
 * S = 0                              if delta == 0
 *     delta / (1 - |2L - 1|)        otherwise
 *
 * H = 60 * ((g - b) / delta mod 6)  if max == r
 *     60 * ((b - r) / delta + 2)    if max == g
 *     60 * ((r - g) / delta + 4)    if max == b
 *     0                              if delta == 0
 * ```
 * The raw hue sector for the red branch uses a floating-point modulo to keep
 * the result in `[0, 6)` before scaling to degrees. A final `+360` guard maps
 * any remaining negative hue into `[0, 360)`.
 *
 * @param r - red in sRGB, `[0, 1]`
 * @param g - green in sRGB, `[0, 1]`
 * @param b - blue in sRGB, `[0, 1]`
 * @returns `[h, s, l]` where:
 *   - `h` ∈ `[0, 360)` — hue in degrees
 *   - `s` ∈ `[0, 1]` — saturation
 *   - `l` ∈ `[0, 1]` — lightness
 *
 * @example
 * ```ts
 * const [h, s, l] = srgbToHsl(1.0, 0.0, 0.0);
 * // [0, 1, 0.5]  — pure red
 * ```
 */
export const srgbToHsl = (
  r: number,
  g: number,
  b: number,
): [number, number, number] => {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;

  const l = (max + min) * 0.5;

  const s = delta < Number.EPSILON ? 0.0 : delta / (1.0 - Math.abs(2.0 * l - 1.0));

  const hRaw = delta < Number.EPSILON
    ? 0.0
    : max === r
      ? (() => {
          const raw = (g - b) / delta;
          // fmod into [0, 6) then scale to degrees
          const sector = raw - 6.0 * Math.floor(raw / 6.0);
          return 60.0 * sector;
        })()
      : max === g
        ? 60.0 * ((b - r) / delta + 2.0)
        : 60.0 * ((r - g) / delta + 4.0);

  const h = hRaw < 0.0 ? hRaw + 360.0 : hRaw;

  return [h, s, l];
};

/**
 * Converts an HSL color to sRGB.
 *
 * @remarks
 * Standard HSL-to-RGB sector expansion:
 * ```
 * if s == 0:  r = g = b = l  (achromatic)
 * else:
 *   q = l * (1 + s)           if l < 0.5
 *       l + s - l * s         otherwise
 *   p = 2 * l - q
 *   r = hslComponent(p, q, h/360 + 1/3)
 *   g = hslComponent(p, q, h/360)
 *   b = hslComponent(p, q, h/360 - 1/3)
 * ```
 * Hue values outside `[0, 360)` are wrapped naturally by the modular
 * arithmetic inside `hslComponent`.
 *
 * @param h - hue in degrees, `[0, 360)` (out-of-range values are wrapped)
 * @param s - saturation, `[0, 1]`
 * @param l - lightness, `[0, 1]`
 * @returns sRGB tuple `[r, g, b]`, each in `[0, 1]`.
 *
 * @example
 * ```ts
 * const [r, g, b] = hslToSrgb(120.0, 1.0, 0.5);
 * // pure green: [0, 1, 0]
 * ```
 */
export const hslToSrgb = (
  h: number,
  s: number,
  l: number,
): [number, number, number] => {
  if (s < Number.EPSILON) {
    return [l, l, l];
  }

  const q = l < 0.5 ? l * (1.0 + s) : l + s - l * s;
  const p = 2.0 * l - q;
  const hk = h / 360.0;

  return [
    hslComponent(p, q, hk + 1.0 / 3.0),
    hslComponent(p, q, hk),
    hslComponent(p, q, hk - 1.0 / 3.0),
  ];
};

// ---------------------------------------------------------------------------
// Public API — Perceptual mix in Oklab
// ---------------------------------------------------------------------------

/**
 * Mixes two sRGB colors in Oklab space for perceptually uniform blending.
 *
 * @remarks
 * Three-step pipeline:
 * ```
 * 1. Convert both sRGB inputs to Oklab.
 * 2. Lerp each Lab component independently:
 *      L = lerp(L0, L1, t)
 *      a = lerp(a0, a1, t)
 *      b = lerp(b0, b1, t)
 *    where lerp(a, b, t) = a + (b - a) * t
 * 3. Convert blended Lab back to sRGB (clamped to [0, 1]).
 * ```
 * Mixing in Oklab produces perceptually uniform transitions — a midpoint
 * blend at `t=0.5` has equal perceptual distance from both endpoints,
 * unlike sRGB lerp which over-brightens or darkens the midpoint.
 *
 * @param r0 - red of first color in sRGB, `[0, 1]`
 * @param g0 - green of first color in sRGB, `[0, 1]`
 * @param b0 - blue of first color in sRGB, `[0, 1]`
 * @param r1 - red of second color in sRGB, `[0, 1]`
 * @param g1 - green of second color in sRGB, `[0, 1]`
 * @param b1 - blue of second color in sRGB, `[0, 1]`
 * @param t - blend factor; `0.0` → first color, `1.0` → second color
 * @returns Blended sRGB tuple `[r, g, b]`, clamped to `[0, 1]`.
 *
 * @example
 * ```ts
 * // Midpoint between red and blue — perceptually uniform purple
 * const [r, g, b] = oklabMix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5);
 * // r > 0, b > 0, perceptual midpoint (not flat sRGB average)
 * ```
 */
export const oklabMix = (
  r0: number,
  g0: number,
  b0: number,
  r1: number,
  g1: number,
  b1: number,
  t: number,
): [number, number, number] => {
  const [l0, a0, bv0] = srgbToOklab(r0, g0, b0);
  const [l1, a1, bv1] = srgbToOklab(r1, g1, b1);

  const lMix = lerp(l0, l1, t);
  const aMix = lerp(a0, a1, t);
  const bMix = lerp(bv0, bv1, t);

  return oklabToSrgb(lMix, aMix, bMix);
};

// ---------------------------------------------------------------------------
// Public API — sRGB ↔ HSV
// ---------------------------------------------------------------------------

/**
 * Converts an sRGB color to HSV.
 *
 * @param r - red in sRGB, `[0, 1]`
 * @param g - green in sRGB, `[0, 1]`
 * @param b - blue in sRGB, `[0, 1]`
 * @returns `[h, s, v]` where h in `[0, 360)`, s and v in `[0, 1]`
 *
 * @example
 * ```ts
 * const [h, s, v] = srgbToHsv(1.0, 0.0, 0.0);
 * // [0, 1, 1]  — pure red
 * ```
 */
export const srgbToHsv = (
  r: number,
  g: number,
  b: number,
): [number, number, number] => {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  const v = max;
  const s = max === 0 ? 0 : delta / max;
  const hRaw = delta === 0
    ? 0
    : max === r
      ? 60.0 * (((g - b) / delta) % 6.0)
      : max === g
        ? 60.0 * ((b - r) / delta + 2.0)
        : 60.0 * ((r - g) / delta + 4.0);
  const h = hRaw < 0 ? hRaw + 360.0 : hRaw;
  return [h, s, v];
};

/**
 * Converts an HSV color to sRGB.
 *
 * @param h - hue in degrees, `[0, 360)`
 * @param s - saturation, `[0, 1]`
 * @param v - value, `[0, 1]`
 * @returns sRGB tuple `[r, g, b]`, each in `[0, 1]`
 *
 * @example
 * ```ts
 * const [r, g, b] = hsvToSrgb(120.0, 1.0, 1.0);
 * // [0, 1, 0]  — pure green
 * ```
 */
export const hsvToSrgb = (
  h: number,
  s: number,
  v: number,
): [number, number, number] => {
  const c = v * s;
  const h2 = h / 60.0;
  const x = c * (1.0 - Math.abs((h2 % 2.0) - 1.0));
  const m = v - c;
  const [r1, g1, b1] = h2 < 1 ? [c, x, 0]
    : h2 < 2 ? [x, c, 0]
    : h2 < 3 ? [0, c, x]
    : h2 < 4 ? [0, x, c]
    : h2 < 5 ? [x, 0, c]
    : [c, 0, x];
  return [r1 + m, g1 + m, b1 + m];
};

// ---------------------------------------------------------------------------
// Public API — Luminance utilities
// ---------------------------------------------------------------------------

/**
 * Relative luminance (ITU-R BT.709). Input is linear RGB.
 *
 * Math: `Y = 0.2126 * R + 0.7152 * G + 0.0722 * B`
 *
 * @param r - red in linear RGB
 * @param g - green in linear RGB
 * @param b - blue in linear RGB
 * @returns Relative luminance in `[0, 1]`
 *
 * @example
 * ```ts
 * luminance(1.0, 1.0, 1.0) // 1.0
 * ```
 */
export const luminance = (r: number, g: number, b: number): number =>
  0.2126 * r + 0.7152 * g + 0.0722 * b;

/**
 * WCAG contrast ratio between two sRGB colors. Returns value >= 1.
 *
 * Converts both to linear RGB, computes relative luminance, then:
 * `ratio = (L_lighter + 0.05) / (L_darker + 0.05)`
 *
 * @param r0 - red of first color in sRGB
 * @param g0 - green of first color in sRGB
 * @param b0 - blue of first color in sRGB
 * @param r1 - red of second color in sRGB
 * @param g1 - green of second color in sRGB
 * @param b1 - blue of second color in sRGB
 * @returns Contrast ratio >= 1
 *
 * @example
 * ```ts
 * contrastRatio(1, 1, 1, 0, 0, 0) // ~21
 * ```
 */
export const contrastRatio = (
  r0: number, g0: number, b0: number,
  r1: number, g1: number, b1: number,
): number => {
  const [lr0, lg0, lb0] = srgbToLinear(r0, g0, b0);
  const [lr1, lg1, lb1] = srgbToLinear(r1, g1, b1);
  const l0 = luminance(lr0, lg0, lb0);
  const l1 = luminance(lr1, lg1, lb1);
  const [lighter, darker] = l0 > l1 ? [l0, l1] : [l1, l0];
  return (lighter + 0.05) / (darker + 0.05);
};

// ---------------------------------------------------------------------------
// Public API — Palette generation
// ---------------------------------------------------------------------------

/**
 * Complementary color — rotate hue by 180 degrees in HSL.
 *
 * @param r - red in sRGB, `[0, 1]`
 * @param g - green in sRGB, `[0, 1]`
 * @param b - blue in sRGB, `[0, 1]`
 * @returns sRGB tuple of the complementary color
 *
 * @example
 * ```ts
 * paletteComplementary(1, 0, 0) // cyan
 * ```
 */
export const paletteComplementary = (
  r: number, g: number, b: number,
): [number, number, number] => {
  const [h, s, l] = srgbToHsl(r, g, b);
  return hslToSrgb((h + 180.0) % 360.0, s, l);
};

/**
 * Triadic palette — two colors at +120 and +240 degrees in HSL.
 *
 * @param r - red in sRGB, `[0, 1]`
 * @param g - green in sRGB, `[0, 1]`
 * @param b - blue in sRGB, `[0, 1]`
 * @returns `[[r1, g1, b1], [r2, g2, b2]]`
 *
 * @example
 * ```ts
 * const [c1, c2] = paletteTriadic(1, 0, 0)
 * ```
 */
export const paletteTriadic = (
  r: number, g: number, b: number,
): [[number, number, number], [number, number, number]] => {
  const [h, s, l] = srgbToHsl(r, g, b);
  return [
    hslToSrgb((h + 120.0) % 360.0, s, l),
    hslToSrgb((h + 240.0) % 360.0, s, l),
  ];
};

/**
 * Analogous palette — two colors at +30 and -30 degrees in HSL.
 *
 * @param r - red in sRGB, `[0, 1]`
 * @param g - green in sRGB, `[0, 1]`
 * @param b - blue in sRGB, `[0, 1]`
 * @returns `[[r1, g1, b1], [r2, g2, b2]]`
 *
 * @example
 * ```ts
 * const [c1, c2] = paletteAnalogous(1, 0, 0)
 * ```
 */
export const paletteAnalogous = (
  r: number, g: number, b: number,
): [[number, number, number], [number, number, number]] => {
  const [h, s, l] = srgbToHsl(r, g, b);
  return [
    hslToSrgb((h + 30.0) % 360.0, s, l),
    hslToSrgb((h + 330.0) % 360.0, s, l),
  ];
};
