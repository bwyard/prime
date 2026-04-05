# prime-sdf

Signed distance functions — 2D and 3D primitives, boolean CSG operations, smooth blending, and domain transforms.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

**2D primitives:** `circle`, `box_2d`, `rounded_box`, `capsule_2d`, `line_segment`, `triangle`, `ring`

**3D primitives:** `sphere`, `box_3d`, `capsule_3d`, `cylinder`, `torus`

**Boolean ops:** `union`, `intersection`, `subtract`, `xor`

**Smooth ops:** `smooth_union`, `smooth_intersection`, `smooth_subtract`

**Domain transforms:** `translate`, `rotate_2d`, `scale`, `repeat`, `mirror_x`, `mirror_y`, `elongate`

## Usage

```rust
use prime_sdf::{sphere, box_3d, smooth_union, translate};
use glam::{Vec2, Vec3};

// Distance from point to sphere
let d = sphere(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0.5);

// CSG: smoothly blend two shapes
let d_sphere = sphere(p, center_a, 0.5);
let d_box = box_3d(p, center_b, Vec3::splat(0.3));
let blended = smooth_union(d_sphere, d_box, 0.1);

// Repeated tiling
let p_tiled = repeat(p.truncate(), Vec2::splat(2.0));
```

## License

MIT
