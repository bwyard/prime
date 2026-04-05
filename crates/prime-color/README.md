# prime-color

Color math — perceptual color spaces (Oklab), sRGB, HSL, HSV, palette generation, luminance, and contrast ratio.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `srgb_to_oklab` / `oklab_to_srgb` — perceptual color space conversion
- `srgb_to_linear` / `linear_to_srgb` — gamma correction
- `srgb_to_hsl` / `hsl_to_srgb` — HSL conversion
- `srgb_to_hsv` / `hsv_to_srgb` — HSV conversion
- `oklab_mix` — perceptually uniform color interpolation
- `luminance` / `contrast_ratio` — WCAG accessibility math
- `palette_complementary` / `palette_triadic` / `palette_analogous` — palette generation

## Usage

```rust
use prime_color::{srgb_to_oklab, oklab_mix, contrast_ratio};

// Convert to Oklab for perceptual operations
let (l, a, b) = srgb_to_oklab(0.8, 0.2, 0.4);

// Perceptually uniform mix — avoids muddy grays in the middle
let (r, g, b) = oklab_mix(0.9, 0.1, 0.1,  // red
                           0.1, 0.1, 0.9,  // blue
                           0.5);           // t

// WCAG contrast ratio
let ratio = contrast_ratio(0.0, 0.0, 0.0,  // black
                           1.0, 1.0, 1.0); // white — 21.0
```

## License

MIT
