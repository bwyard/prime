# PRIME

Pure math foundation for the FORM/SCORE/STAGE ecosystem. No game concepts, no audio, no graphics pipeline — just math expressed as pure functions.

```
PRIME   pure math foundation  ← this library
SCORE   audio framework       → imports prime-osc, prime-interp, prime-signal, prime-random
FORM    graphics framework    → imports prime-sdf, prime-noise, prime-color, prime-spatial
STAGE   game systems          → imports PRIME + FORM + SCORE
```

## Thesis

PRIME is built on four primitives that underlie all computation:

- **Time** — computation unfolds as a sequence of discrete steps
- **Causality** — each step depends only on the previous step's output
- **Information** — state is created (appended), never destroyed
- **Context** — every function receives its full context as parameters

These map to four operations that every PRIME function follows:

- **LOAD** — read inputs (function parameters) — *Context*
- **COMPUTE** — pure math (function body) — *Information*
- **APPEND** — return new state as tuple — *Information + Causality*
- **ADVANCE** — fold over time steps — *Time*

**The Causal Irrecoverability Lemma** ([ADR-006](docs/adr/006-causal-irrecoverability.md)): mutable state is lossy compression of time. When you overwrite a value, the previous state is irrecoverably lost. PRIME avoids this by returning new state instead of mutating — every intermediate value remains recoverable.

No mutation. No side effects. No `&mut` in any public API. Same inputs always produce the same outputs. See [ADR-001](docs/adr/001-pure-functions-only.md), [ADR-003](docs/adr/003-four-temporal-operations.md).

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
| **prime-random** | Mulberry32 PRNG, distributions, Bridson, Monte Carlo, Halton | 23 | 75 |
| **prime-interp** | Lerp, smoothstep, 10 easing families, repeat, pingpong | 37 | 48 |
| **prime-signal** | Smoothdamp, spring, low/high pass, deadzone (f32/Vec2/Vec3) | 9 | 40 |
| **prime-osc** | LFO waveforms (sine/cos/tri/saw/square), ADSR envelope | 7 | 35 |
| **prime-noise** | Perlin, simplex, FBM, Worley, curl noise (2D/3D) | 13 | 75 |
| **prime-color** | Oklab, sRGB, HSL/HSV, luminance, contrast, palette | 14 | 68 |
| **prime-splines** | Bezier, Catmull-Rom, B-spline, slerp, arc-length | 15 | 59 |
| **prime-spatial** | Ray-sphere/AABB/plane, frustum culling, AABB ops | 9 | 60 |
| **prime-voronoi** | Voronoi nearest, F1/F2, Lloyd relaxation | 3 | 16 |
| **prime-diffusion** | Ornstein-Uhlenbeck, geometric Brownian motion | 4 | 23 |
| **prime-dynamics** | Euler/RK4, Lorenz, Lotka-Volterra, SIR, Gray-Scott | 10 | 45 |
| **prime-sdf** | SDF primitives, CSG, smooth ops, domain transforms | 27 | 43 |

**608 Rust tests. 500+ TypeScript tests. Zero `&mut` violations.**

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

## Documentation

- **`docs/math/`** — formulas and derivations for every algorithm (11 files, one per crate)
- **`docs/adr/`** — architecture decision records (see below)
- The handoff document is private and not included in this repository

## Architecture

```
docs/
├── math/           # LaTeX formulas and derivations for every algorithm
│   ├── prime-random.md
│   ├── prime-interp.md
│   └── ... (11 files)
├── adr/            # Architecture decision records
│   ├── 001-pure-functions-only.md
│   ├── 002-seed-as-thread.md
│   ├── 003-four-temporal-operations.md
│   ├── 004-cross-language-determinism.md
│   ├── 005-receiver-model.md
│   ├── 006-causal-irrecoverability.md
│   └── 007-advance-exception.md
└── ROADMAP.md      # Phase-by-phase delivery plan
```

**Design decisions:**

- **No `&mut`** — state flows through return tuples ([ADR-001](docs/adr/001-pure-functions-only.md))
- **Seed = capability** — who holds the seed controls the sequence ([ADR-002](docs/adr/002-seed-as-thread.md))
- **LOAD/COMPUTE/APPEND/ADVANCE** — the only four operations needed ([ADR-003](docs/adr/003-four-temporal-operations.md))
- **Cross-language determinism** — Rust and TypeScript produce identical outputs ([ADR-004](docs/adr/004-cross-language-determinism.md))
- **Receiver model** — pure functions receive context, never reach for it ([ADR-005](docs/adr/005-receiver-model.md))
- **Causal irrecoverability** — mutable state is lossy compression of time ([ADR-006](docs/adr/006-causal-irrecoverability.md))
- **ADVANCE-EXCEPTION** — controlled `while` loops for data-dependent termination ([ADR-007](docs/adr/007-advance-exception.md))

## Building

```bash
# Build all crates
cargo build --workspace

# Test all (608 tests)
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
