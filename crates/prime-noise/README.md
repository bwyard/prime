# prime-noise

Noise functions — Value, Perlin, Simplex, FBM, Worley, domain warping, and curl noise. 2D and 3D variants throughout.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `value_noise_2d` / `value_noise_3d` — lattice value noise
- `perlin_2d` / `perlin_3d` — gradient noise
- `simplex_2d` / `simplex_3d` — simplex noise
- `fbm_2d` / `fbm_3d` — fractal Brownian motion (octaves, lacunarity, gain)
- `worley_2d` — cellular / Worley noise
- `domain_warp_2d` / `domain_warp_3d` — FBM-based domain warping
- `curl_2d` / `curl_3d` — divergence-free curl noise (fluid simulation)

## Usage

```rust
use prime_noise::{perlin_2d, fbm_2d, curl_2d};

// Basic Perlin noise
let n = perlin_2d(x, y); // [-1, 1]

// Layered FBM terrain
let height = fbm_2d(x, y, 6, 2.0, 0.5);

// Curl noise for fluid-like motion
let (vx, vy) = curl_2d(x, y, 0.01);
```

## License

MIT
