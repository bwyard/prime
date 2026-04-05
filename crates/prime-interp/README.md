# prime-interp

Interpolation, easing, and remapping. Lerp, smoothstep, and a full easing function library — quad through elastic.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `lerp` / `lerp_clamped` / `inv_lerp` — linear interpolation
- `remap` — map a value from one range to another
- `smoothstep` / `smootherstep` — Hermite interpolation
- `repeat` / `pingpong` — wrapping and bouncing ranges
- Easing functions: `ease_in/out/in_out` × quad, cubic, quart, quint, sine, expo, circ, back, elastic, bounce

## Usage

```rust
use prime_interp::{lerp, remap, ease_out_cubic, smoothstep};

// Linear interpolation
let mid = lerp(0.0, 10.0, 0.5); // 5.0

// Remap from one range to another
let remapped = remap(0.5, 0.0, 1.0, 100.0, 200.0); // 150.0

// Eased animation progress
let t = ease_out_cubic(0.3); // decelerated

// Smooth threshold
let alpha = smoothstep(0.2, 0.8, 0.5);
```

## License

MIT
