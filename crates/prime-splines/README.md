# prime-splines

Spline math — Bezier (quadratic, cubic), Hermite, Catmull-Rom, B-spline, slerp, arc length, and uniform-speed parameterization. Scalar and 3D variants.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `bezier_quadratic` / `bezier_cubic` — Bezier curves (scalar + 3D)
- `hermite` / `hermite_3d` — Hermite interpolation with tangents
- `catmull_rom` / `catmull_rom_3d` — smooth pass-through splines
- `b_spline_cubic` / `b_spline_cubic_3d` — B-spline curves
- `slerp` — spherical linear interpolation for rotations
- `bezier_cubic_arc_length` — arc length approximation
- `bezier_cubic_t_at_length` — uniform-speed parameterization

## Usage

```rust
use prime_splines::{catmull_rom_3d, slerp, bezier_cubic_t_at_length_3d};
use glam::Vec3;

// Smooth path through control points
let pos = catmull_rom_3d(t, p0, p1, p2, p3);

// Spherical interpolation between directions
let dir = slerp(from, to, t);

// Move at constant speed along a bezier
let uniform_t = bezier_cubic_t_at_length_3d(p0, p1, p2, p3, target_length, 64);
```

## License

MIT
