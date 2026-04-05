# prime-voronoi

Voronoi and Delaunay geometry — nearest-cell queries, F1/F2 distances, Lloyd relaxation, and Bowyer-Watson triangulation.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `voronoi_nearest_2d` — find nearest seed and distance for a query point
- `voronoi_f1_f2_2d` — F1 (nearest) and F2 (second-nearest) distances for cellular noise
- `lloyd_relax_step_2d` — one step of Lloyd relaxation (centroidal Voronoi)
- `delaunay_2d` — Delaunay triangulation, returns triangle indices

## Usage

```rust
use prime_voronoi::{voronoi_nearest_2d, delaunay_2d, lloyd_relax_step_2d};

let seeds = vec![(0.0, 0.0), (1.0, 0.5), (0.5, 1.0)];

// Nearest cell
let (cell_index, distance) = voronoi_nearest_2d((0.4, 0.4), &seeds).unwrap();

// Delaunay triangulation
let triangles = delaunay_2d(&seeds); // Vec<(usize, usize, usize)>

// Centroidal relaxation
let samples: Vec<(f32, f32)> = /* dense point set */;
let relaxed = lloyd_relax_step_2d(&seeds, &samples);
```

## License

MIT
