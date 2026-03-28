# PRIME

Pure math foundation for the FORM/SCORE/STAGE ecosystem. No game concepts, no audio, no graphics pipeline — just math expressed as pure functions.

```
PRIME   pure math foundation  ← this library
SCORE   audio framework       → imports prime-osc, prime-interp, prime-signal, prime-random
FORM    graphics framework    → imports prime-sdf, prime-noise, prime-color, prime-spatial
STAGE   game systems          → imports PRIME + FORM + SCORE
```

## Thesis

Every function in PRIME follows the **Functional Temporal Architecture**:

- **LOAD** — read inputs (function parameters)
- **COMPUTE** — pure math (function body)
- **APPEND** — return new state as tuple
- **ADVANCE** — fold over time steps

No mutation. No side effects. No `&mut` in any public API. Same inputs always produce the same outputs.

```rust
// The seed IS the RNG. No class, no write head, no hidden state.
let (value, next_seed) = prng_next(seed);

// Animation is a fold, not a mutation loop.
let (pos, vel) = smoothdamp(current, target, velocity, smooth_time, dt);

// Composition replaces inheritance.
let d = smooth_union(sphere(p, center, r), box_3d(p, c, half), k);
```

## Crates

| Crate | Description | Functions | Tests |
|-------|-------------|-----------|-------|
| **prime-random** | Mulberry32 PRNG, distributions, Bridson, Monte Carlo, Halton | 19 | 69 |
| **prime-interp** | Lerp, smoothstep, 10 easing families, repeat, pingpong | 37 | 48 |
| **prime-signal** | Smoothdamp, spring, low/high pass, deadzone (f32/Vec2/Vec3) | 9 | 36 |
| **prime-osc** | LFO waveforms (sine/cos/tri/saw/square), ADSR envelope | 9 | 34 |
| **prime-noise** | Perlin, simplex, FBM, Worley, curl noise (2D/3D) | 13 | 74 |
| **prime-color** | Oklab, sRGB, HSL/HSV, luminance, contrast, palette | 14 | 70 |
| **prime-splines** | Bezier, Catmull-Rom, B-spline, slerp, arc-length | 16 | 56 |
| **prime-spatial** | Ray-sphere/AABB/plane, frustum culling, AABB ops | 9 | 39 |
| **prime-voronoi** | Voronoi nearest, F1/F2, Lloyd relaxation | 3 | 16 |
| **prime-diffusion** | Ornstein-Uhlenbeck, geometric Brownian motion | 4 | 23 |
| **prime-dynamics** | Euler/RK4, Lorenz, Lotka-Volterra, SIR, Gray-Scott | 13 | 45 |
| **prime-sdf** | SDF primitives, CSG, smooth ops, domain transforms | 15+ | 43 |

**602 Rust tests. 500+ TypeScript tests. Zero `&mut` violations.**

## Quick Start

```toml
# Cargo.toml (pre-publish, use git dependency)
[dependencies]
prime-random = { git = "https://github.com/bwyard/prime", branch = "dev" }
prime-interp = { git = "https://github.com/bwyard/prime", branch = "dev" }
```

```rust
use prime_random::{prng_next, prng_gaussian, poisson_disk_2d, monte_carlo_1d};
use prime_interp::{lerp, ease_out_elastic, smoothstep};

// Deterministic RNG — same seed, same result, always
let (value, s1) = prng_next(42);
let (gaussian, s2) = prng_gaussian(s1);

// Poisson disk sampling with corrected area-uniform annulus
let (points, _seed) = poisson_disk_2d(42, 100.0, 100.0, 10.0, 30);

// Monte Carlo integration — pure fold, no mutation
let (integral, _seed) = monte_carlo_1d(42, |x| x.sin(), 0.0, std::f32::consts::PI, 10000);
// integral ≈ 2.0

// Easing and interpolation
let t = ease_out_elastic(0.7);
let blended = lerp(0.0, 100.0, smoothstep(0.0, 1.0, t));
```

## TypeScript

```json
{
  "dependencies": {
    "@prime/prime-random": "github:bwyard/prime#dev"
  }
}
```

```typescript
import { prngNext, prngGaussian, poissonDisk2d, monteCarlo1d } from '@prime/prime-random'

// Identical Mulberry32 algorithm — same seed, same output as Rust
const [value, s1] = prngNext(42)
const [gaussian, s2] = prngGaussian(s1)

// Bridson with area-uniform annulus sampling
const [points, seed] = poissonDisk2d(42, 100, 100, 10)

// Monte Carlo — closures work natively in TS
const [integral, _] = monteCarlo1d(42, Math.sin, 0, Math.PI, 10000)
```

## Benchmarks

Measured with Criterion 0.8 (`cargo bench -p prime-random`):

| Operation | Time | vs Mutable |
|-----------|------|------------|
| prng_next | 1.3 ns | 1.3× |
| prng_gaussian | 0.4 ns/chain | 1.0× |
| Monte Carlo n=10K | 88 µs | ~1.1× |
| Bridson 100×100 | 1.19 ms | ~4× |
| weighted_choice n=100 | 102 ns | ~1.0× |

Pure functions have zero overhead for point operations. Spatial algorithms (Bridson) pay ~4× from immutable state reconstruction — still sub-frame at 60fps.

## Architecture

```
docs/
├── math/           # LaTeX formulas and derivations for every algorithm
│   ├── prime-random.md
│   ├── prime-interp.md
│   └── ... (10 files)
├── adr/            # Architecture decision records
│   ├── 001-pure-functions-only.md
│   ├── 002-seed-as-thread.md
│   ├── 003-four-temporal-operations.md
│   └── 004-cross-language-determinism.md
└── ROADMAP.md      # Phase-by-phase delivery plan
```

**Design decisions:**

- **No `&mut`** — state flows through return tuples ([ADR-001](docs/adr/001-pure-functions-only.md))
- **Seed = capability** — who holds the seed controls the sequence ([ADR-002](docs/adr/002-seed-as-thread.md))
- **LOAD/COMPUTE/APPEND/ADVANCE** — the only four operations needed ([ADR-003](docs/adr/003-four-temporal-operations.md))
- **Cross-language determinism** — Rust and TypeScript produce identical outputs ([ADR-004](docs/adr/004-cross-language-determinism.md))

## Building

```bash
# Build all crates
cargo build --workspace

# Test all (602 tests)
cargo test --workspace

# Benchmark prime-random
cargo bench -p prime-random

# Clippy (CI-level strictness)
cargo clippy --workspace -- -D warnings

# TypeScript tests
npx vitest run
```

## License

MIT
