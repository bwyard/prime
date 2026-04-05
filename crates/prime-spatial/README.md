# prime-spatial

Spatial queries — ray casting, AABB operations, and frustum culling. Pure functions for collision detection and visibility testing.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `ray_aabb` / `ray_sphere` / `ray_plane` — ray intersection tests
- `aabb_overlaps` / `aabb_contains` / `aabb_union` / `aabb_closest_point` — AABB operations
- `frustum_cull_sphere` / `frustum_cull_aabb` — frustum visibility tests

## Usage

```rust
use prime_spatial::{ray_sphere, aabb_overlaps, frustum_cull_sphere};
use glam::Vec3;

// Ray-sphere intersection — returns hit distance if any
let hit = ray_sphere(ray_origin, ray_dir, sphere_center, sphere_radius);

// AABB overlap test
let overlapping = aabb_overlaps(min_a, max_a, min_b, max_b);

// Frustum culling
let visible = frustum_cull_sphere(&frustum_planes, sphere_center, sphere_radius);
```

## License

MIT
