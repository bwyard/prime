# prime-wasm

WebAssembly bindings for the prime math ecosystem. Exposes color, SDF, noise, and signal functions to JavaScript/TypeScript via wasm-bindgen.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

Flat WASM-friendly API (arrays returned as `Box<[f32]>`) covering:

- **Color:** `srgb_to_oklab`, `oklab_to_srgb`, `srgb_to_linear`, `linear_to_srgb`, `hsl_to_srgb`, `srgb_to_hsl`, `srgb_to_hsv`, `hsv_to_srgb`, `oklab_mix`, `luminance`, `contrast_ratio`, `palette_complementary`, `palette_triadic`, `palette_analogous`
- **SDF:** `sdf_sphere`, `sdf_box`, `sdf_torus`, `sdf_capsule`
- **Noise:** `curl_2d`, `curl_3d`

## Usage

Build with `wasm-pack`:

```bash
wasm-pack build --target web
```

Then in JavaScript/TypeScript:

```js
import init, { srgb_to_oklab, sdf_sphere } from './pkg/prime_wasm.js';

await init();

const [l, a, b] = srgb_to_oklab(0.8, 0.2, 0.4);
const dist = sdf_sphere(1, 0, 0,  // point
                        0, 0, 0,  // center
                        0.5);     // radius
```

## License

MIT
