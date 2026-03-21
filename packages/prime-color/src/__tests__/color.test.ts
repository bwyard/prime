/**
 * prime-color — unit tests.
 *
 * Cross-language reference: values verified against the Rust implementation at
 * `prime/crates/prime-color/src/lib.rs`. If the Rust side changes its
 * matrices or formulas, update the known-value tests here to match.
 *
 * Rules:
 * - `const` only — no `let` anywhere.
 * - No `for` loops — use `.map` / `.every` / destructuring.
 * - All pure: every assertion is a closed expression with no side effects.
 */

import { describe, it, expect } from "vitest";
import {
  srgbToLinear,
  linearToSrgb,
  srgbToOklab,
  oklabToSrgb,
  srgbToHsl,
  hslToSrgb,
  oklabMix,
} from "../index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const EPS = 1e-4;

const approxEq = (a: number, b: number, eps = EPS): boolean =>
  Math.abs(a - b) < eps;

const tupleApproxEq = (
  a: [number, number, number],
  b: [number, number, number],
  eps = EPS,
): boolean =>
  approxEq(a[0], b[0], eps) &&
  approxEq(a[1], b[1], eps) &&
  approxEq(a[2], b[2], eps);

// ---------------------------------------------------------------------------
// sRGB ↔ Linear RGB — known values
// ---------------------------------------------------------------------------

describe("srgbToLinear", () => {
  it("black stays black", () => {
    const result = srgbToLinear(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("white stays white", () => {
    const result = srgbToLinear(1.0, 1.0, 1.0);
    expect(tupleApproxEq(result, [1.0, 1.0, 1.0])).toBe(true);
  });

  it("mid-gray 0.5 → approximately 0.2140", () => {
    const [r, g, b] = srgbToLinear(0.5, 0.5, 0.5);
    expect(approxEq(r, 0.2140, 1e-3)).toBe(true);
    expect(approxEq(g, 0.2140, 1e-3)).toBe(true);
    expect(approxEq(b, 0.2140, 1e-3)).toBe(true);
  });

  it("uses linear segment below 0.04045", () => {
    // 0.04 / 12.92 ≈ 0.003096
    const [r] = srgbToLinear(0.04, 0.0, 0.0);
    expect(approxEq(r, 0.04 / 12.92, EPS)).toBe(true);
  });

  it("uses power segment above 0.04045", () => {
    const [r] = srgbToLinear(0.5, 0.0, 0.0);
    const expected = Math.pow((0.5 + 0.055) / 1.055, 2.4);
    expect(approxEq(r, expected, EPS)).toBe(true);
  });
});

describe("linearToSrgb", () => {
  it("black stays black", () => {
    const result = linearToSrgb(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("white stays white", () => {
    const result = linearToSrgb(1.0, 1.0, 1.0);
    expect(tupleApproxEq(result, [1.0, 1.0, 1.0])).toBe(true);
  });

  it("0.2140 → approximately 0.5", () => {
    const [r, g, b] = linearToSrgb(0.2140, 0.2140, 0.2140);
    expect(approxEq(r, 0.5, 1e-3)).toBe(true);
    expect(approxEq(g, 0.5, 1e-3)).toBe(true);
    expect(approxEq(b, 0.5, 1e-3)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// sRGB ↔ Linear RGB — round-trips
// ---------------------------------------------------------------------------

describe("sRGB ↔ Linear RGB round-trips", () => {
  const colors: Array<[string, [number, number, number]]> = [
    ["black",   [0.0, 0.0, 0.0]],
    ["white",   [1.0, 1.0, 1.0]],
    ["red",     [1.0, 0.0, 0.0]],
    ["green",   [0.0, 1.0, 0.0]],
    ["blue",    [0.0, 0.0, 1.0]],
    ["gray50",  [0.5, 0.5, 0.5]],
    ["coral",   [0.8, 0.4, 0.2]],
  ];

  colors.forEach(([name, [r, g, b]]) => {
    it(`sRGB → linear → sRGB round-trip: ${name}`, () => {
      const linear = srgbToLinear(r, g, b);
      const back = linearToSrgb(linear[0], linear[1], linear[2]);
      expect(tupleApproxEq(back, [r, g, b])).toBe(true);
    });
  });
});

// ---------------------------------------------------------------------------
// sRGB ↔ Oklab — known values (cross-language reference from Rust)
// ---------------------------------------------------------------------------

describe("srgbToOklab — known values", () => {
  it("black (0,0,0) → (0,0,0)", () => {
    const result = srgbToOklab(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("white (1,1,1) → approximately (1, 0, 0)", () => {
    const [l, a, b] = srgbToOklab(1.0, 1.0, 1.0);
    expect(approxEq(l, 1.0, 1e-3)).toBe(true);
    expect(approxEq(a, 0.0, 1e-3)).toBe(true);
    expect(approxEq(b, 0.0, 1e-3)).toBe(true);
  });

  it("red (1,0,0) → L ≈ 0.6280, a ≈ 0.2247, b ≈ 0.1260", () => {
    const [l, a, b] = srgbToOklab(1.0, 0.0, 0.0);
    expect(approxEq(l, 0.6280, 1e-3)).toBe(true);
    expect(approxEq(a, 0.2247, 1e-3)).toBe(true);
    expect(approxEq(b, 0.1260, 1e-3)).toBe(true);
  });

  it("green (0,1,0) → L ≈ 0.8664, a ≈ -0.2338, b ≈ 0.1794", () => {
    const [l, a, b] = srgbToOklab(0.0, 1.0, 0.0);
    expect(approxEq(l, 0.8664, 1e-3)).toBe(true);
    expect(approxEq(a, -0.2338, 1e-3)).toBe(true);
    expect(approxEq(b, 0.1794, 1e-3)).toBe(true);
  });

  it("blue (0,0,1) → L ≈ 0.4520, a ≈ -0.0325, b ≈ -0.3117", () => {
    const [l, a, b] = srgbToOklab(0.0, 0.0, 1.0);
    expect(approxEq(l, 0.4520, 1e-3)).toBe(true);
    expect(approxEq(a, -0.0325, 1e-3)).toBe(true);
    expect(approxEq(b, -0.3117, 1e-3)).toBe(true);
  });
});

describe("oklabToSrgb — known values", () => {
  it("(0,0,0) → black", () => {
    const result = oklabToSrgb(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("(1,0,0) → approximately white", () => {
    const [r, g, b] = oklabToSrgb(1.0, 0.0, 0.0);
    expect(approxEq(r, 1.0, 1e-3)).toBe(true);
    expect(approxEq(g, 1.0, 1e-3)).toBe(true);
    expect(approxEq(b, 1.0, 1e-3)).toBe(true);
  });

  it("red oklab → approximately (1,0,0)", () => {
    const [r, g, b] = oklabToSrgb(0.6280, 0.2247, 0.1260);
    expect(approxEq(r, 1.0, 1e-2)).toBe(true);
    expect(b < 0.1).toBe(true);
  });

  it("output is always clamped to [0,1] for out-of-gamut inputs", () => {
    const [r, g, b] = oklabToSrgb(0.5, 0.5, 0.5); // likely out of gamut
    expect(r >= 0.0 && r <= 1.0).toBe(true);
    expect(g >= 0.0 && g <= 1.0).toBe(true);
    expect(b >= 0.0 && b <= 1.0).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// sRGB ↔ Oklab — round-trips
// ---------------------------------------------------------------------------

describe("sRGB ↔ Oklab round-trips", () => {
  const colors: Array<[string, [number, number, number]]> = [
    ["black",  [0.0, 0.0, 0.0]],
    ["white",  [1.0, 1.0, 1.0]],
    ["red",    [1.0, 0.0, 0.0]],
    ["green",  [0.0, 1.0, 0.0]],
    ["blue",   [0.0, 0.0, 1.0]],
  ];

  colors.forEach(([name, [r, g, b]]) => {
    it(`sRGB → Oklab → sRGB round-trip: ${name}`, () => {
      const [l, a, bv] = srgbToOklab(r, g, b);
      const back = oklabToSrgb(l, a, bv);
      expect(tupleApproxEq(back, [r, g, b], 1e-3)).toBe(true);
    });
  });
});

// ---------------------------------------------------------------------------
// sRGB ↔ HSL — known values
// ---------------------------------------------------------------------------

describe("srgbToHsl — known values", () => {
  it("black (0,0,0) → (0, 0, 0)", () => {
    const result = srgbToHsl(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("white (1,1,1) → (0, 0, 1)", () => {
    const result = srgbToHsl(1.0, 1.0, 1.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 1.0])).toBe(true);
  });

  it("red (1,0,0) → (0, 1, 0.5)", () => {
    const result = srgbToHsl(1.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 1.0, 0.5])).toBe(true);
  });

  it("green (0,1,0) → (120, 1, 0.5)", () => {
    const result = srgbToHsl(0.0, 1.0, 0.0);
    expect(tupleApproxEq(result, [120.0, 1.0, 0.5])).toBe(true);
  });

  it("blue (0,0,1) → (240, 1, 0.5)", () => {
    const result = srgbToHsl(0.0, 0.0, 1.0);
    expect(tupleApproxEq(result, [240.0, 1.0, 0.5])).toBe(true);
  });

  it("hue is always in [0, 360)", () => {
    // cyan (0,1,1) has h=180
    const [h] = srgbToHsl(0.0, 1.0, 1.0);
    expect(h >= 0.0 && h < 360.0).toBe(true);
    expect(approxEq(h, 180.0)).toBe(true);
  });

  it("achromatic mid-gray → s = 0", () => {
    const [_h, s] = srgbToHsl(0.5, 0.5, 0.5);
    expect(approxEq(s, 0.0)).toBe(true);
  });
});

describe("hslToSrgb — known values", () => {
  it("(0, 0, 0) → black", () => {
    const result = hslToSrgb(0.0, 0.0, 0.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 0.0])).toBe(true);
  });

  it("achromatic (0, 0, 1) → white", () => {
    const result = hslToSrgb(0.0, 0.0, 1.0);
    expect(tupleApproxEq(result, [1.0, 1.0, 1.0])).toBe(true);
  });

  it("(0, 1, 0.5) → red", () => {
    const result = hslToSrgb(0.0, 1.0, 0.5);
    expect(tupleApproxEq(result, [1.0, 0.0, 0.0])).toBe(true);
  });

  it("(120, 1, 0.5) → green", () => {
    const result = hslToSrgb(120.0, 1.0, 0.5);
    expect(tupleApproxEq(result, [0.0, 1.0, 0.0])).toBe(true);
  });

  it("(240, 1, 0.5) → blue", () => {
    const result = hslToSrgb(240.0, 1.0, 0.5);
    expect(tupleApproxEq(result, [0.0, 0.0, 1.0])).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// sRGB ↔ HSL — round-trips
// ---------------------------------------------------------------------------

describe("sRGB ↔ HSL round-trips", () => {
  const colors: Array<[string, [number, number, number]]> = [
    ["black",  [0.0, 0.0, 0.0]],
    ["white",  [1.0, 1.0, 1.0]],
    ["red",    [1.0, 0.0, 0.0]],
    ["green",  [0.0, 1.0, 0.0]],
    ["blue",   [0.0, 0.0, 1.0]],
  ];

  colors.forEach(([name, [r, g, b]]) => {
    it(`sRGB → HSL → sRGB round-trip: ${name}`, () => {
      const [h, s, l] = srgbToHsl(r, g, b);
      const back = hslToSrgb(h, s, l);
      expect(tupleApproxEq(back, [r, g, b])).toBe(true);
    });
  });
});

// ---------------------------------------------------------------------------
// oklabMix
// ---------------------------------------------------------------------------

describe("oklabMix", () => {
  it("t=0 returns first color (red)", () => {
    const result = oklabMix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
    expect(tupleApproxEq(result, [1.0, 0.0, 0.0], 1e-3)).toBe(true);
  });

  it("t=1 returns second color (blue)", () => {
    const result = oklabMix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
    expect(tupleApproxEq(result, [0.0, 0.0, 1.0], 1e-3)).toBe(true);
  });

  it("t=0.5 between red and blue produces a perceptual purple (r>0, b>0)", () => {
    const [r, g, b] = oklabMix(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5);
    expect(r > 0.0).toBe(true);
    expect(b > 0.0).toBe(true);
    // g should be small — purple has little green
    expect(g < 0.5).toBe(true);
  });

  it("t=0.5 between black and white → mid gray", () => {
    const [r, g, b] = oklabMix(0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5);
    // all channels equal (neutral gray)
    expect(approxEq(r, g, 1e-3)).toBe(true);
    expect(approxEq(g, b, 1e-3)).toBe(true);
    // lightness roughly mid
    expect(r > 0.1 && r < 0.9).toBe(true);
  });

  it("output is clamped to [0,1]", () => {
    const [r, g, b] = oklabMix(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5);
    expect(r >= 0.0 && r <= 1.0).toBe(true);
    expect(g >= 0.0 && g <= 1.0).toBe(true);
    expect(b >= 0.0 && b <= 1.0).toBe(true);
  });

  it("mix with itself at any t returns the same color", () => {
    const [r, g, b] = oklabMix(0.8, 0.2, 0.4, 0.8, 0.2, 0.4, 0.5);
    expect(tupleApproxEq([r, g, b], [0.8, 0.2, 0.4], 1e-3)).toBe(true);
  });
});
