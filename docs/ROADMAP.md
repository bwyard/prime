# PRIME Roadmap
Last updated: 2026-03-18

Pure math foundation for the FORM / SCORE / STAGE ecosystem.
Each phase ends with a PR into `dev`. Between each phase, a **review session**
walks through the code and PR before merging — intended to build Rust fluency.

---

## Branch strategy

```
main   ← major releases only (merged from dev)
dev    ← all PRs land here
feature/phase-N-<crate>  ← one branch per phase
```

---

## Crate inventory

### PRIME — pure math (no game concepts, no audio, no graphics pipeline)

| Crate | Description | Status |
|---|---|---|
| `prime-sdf` | SDF primitives, CSG, domain transforms | ✅ Done — 43 tests |
| `prime-random` | Seeded PCG64 RNG, Poisson disk sampling | stub |
| `prime-interp` | Easing functions, lerp, smoothstep, remap | stub |
| `prime-signal` | Smoothdamp, spring, low-pass filter, deadzone | stub |
| `prime-osc` | LFO waveform shapes, ADSR envelope | stub |
| `prime-noise` | Perlin, Simplex, FBM, Worley, curl noise | stub |
| `prime-color` | Oklab, sRGB, HSL/HSV, palette generation | stub |
| `prime-splines` | Bézier, Catmull-Rom, B-spline, slerp | stub |
| `prime-spatial` | Ray tests, AABB, frustum cull | stub |
| `prime-voronoi` | Voronoi, Delaunay, Lloyd relaxation | stub |
| `prime-diffusion` | Monte Carlo sampling, probability distributions, Gaussian kernels, stochastic integration | stub |
| `prime-dynamics` | ODE solvers (Euler, RK4), chaos attractors (Lorenz, Rössler), biological models (Lotka-Volterra, SIR), reaction-diffusion, L-systems | stub |

### STAGE — game domain systems (depends on PRIME)

| Crate | Description | Status |
|---|---|---|
| `stage-loop` | Fixed tick, accumulator, interpolation | stub |
| `stage-quest` | Quest graphs, topological sort | stub |
| `stage-combat` | Damage, stats, curves | stub |
| `stage-economy` | Loot tables, price curves | stub |
| `stage-ai` | Utility AI, FSM, influence maps | stub |
| `stage-proc` | Dungeon gen, BSP, biome assignment | stub |
| `stage-nav` | A*, flow fields, SDF navigation | stub |

### TypeScript packages (pure TS ports — WASM backend deferred)

One package per Rust crate. Pure TS implementation now; WASM swap-in later
without breaking the public API. Cross-language determinism tests required:
Rust output is the reference, TS must match.

| Package | Notes |
|---|---|
| `@prime/*` | Mirror of each prime-* crate |
| `@stage/*` | Mirror of each stage-* crate |
| `stage-world` | TS-only — live game state container (no Rust counterpart). Design deferred to Phase 6. |

---

## Dependency map

```
prime-random  ──► prime-diffusion, prime-noise, prime-dynamics (seeded sampling)
prime-interp  ──► SCORE (lerp/easing), stage-loop
prime-signal  ──► SCORE (smoothdamp), stage-ai
prime-osc     ──► SCORE (LFO/ADSR)
prime-sdf ✅  ──► FORM (already live)
prime-spatial ──► FORM, stage-nav
prime-splines ──► FORM, stage-nav
prime-noise   ──► FORM, stage-proc
prime-color   ──► FORM
prime-voronoi ──► stage-proc, stage-ai
prime-diffusion ──► FORM (soft shadows, AO), SCORE (granular synthesis), stage-proc
prime-dynamics  ──► SCORE (chaos math), FORM (reaction-diffusion), stage-ai, stage-proc
stage-loop    ──► all STAGE crates
stage-world   ──► idle-hero (TS state bridge)
stage-quest/combat/economy ──► idle-hero Phase 3+
stage-ai/proc/nav ──► idle-hero Phase 5+
```

---

## Phase plan

### Phase 0 — Scaffold + prime-sdf ✅
**Done.** Cargo workspace, pnpm workspace, 17 crate stubs, 21 TS package stubs.
`prime-sdf` fully implemented: 40 unit tests + 3 doc-tests.
`cargo test --workspace` green on MSVC toolchain.

---

### Phase 1 — Core determinism + interpolation
**Branch:** `feature/phase-1-random-interp`
**Target:** 2026-03-25
**Unlocks:** SCORE lerp/easing, seeded RNG for all future crates

#### `prime-random`
- Seeded PCG64 RNG (`Rng` struct with `seed(u64)`)
- `next_f32()`, `next_f64()`, `next_u32()`, `range_f32(min, max)`, `range_u32(min, max)`
- `shuffle(&mut [T])`, `choose(&[T]) -> Option<&T>`
- Poisson disk sampling 2D/3D
- Rustdoc on every public fn

#### `prime-interp`
- `lerp(a, b, t)`, `lerp_clamped(a, b, t)`
- `remap(v, in_min, in_max, out_min, out_max)`
- `smoothstep(edge0, edge1, x)`, `smootherstep(edge0, edge1, x)`
- Easing functions: `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`, `ease_in_out_cubic`, `ease_in_expo`, `ease_out_expo`, `ease_in_out_expo`, `ease_in_elastic`, `ease_out_elastic`, `ease_in_back`, `ease_out_back`, `ease_in_bounce`, `ease_out_bounce`
- `pingpong(t, length)`, `repeat(t, length)`
- Rustdoc on every public fn

**Review session after PR.**

---

### Phase 2 — Game feel + oscillation
**Branch:** `feature/phase-2-signal-osc`
**Target:** 2026-04-01
**Unlocks:** SCORE fully unblocked (all 4 PRIME imports available)

#### `prime-signal`
- `smooth_damp(current, target, velocity, smooth_time, dt)` — returns `(value, new_velocity)`
- `spring(current, target, velocity, stiffness, damping, dt)`
- `low_pass(current, target, alpha)` — exponential moving average
- `deadzone(value, threshold)`, `deadzone_remap(value, threshold, max)`
- All variants for `f32`, `Vec2`, `Vec3`

#### `prime-osc`
- LFO waveforms: `sine(t)`, `cosine(t)`, `triangle(t)`, `sawtooth(t)`, `square(t)`, `pulse(t, width)`
- `adsr(t, attack, decay, sustain, release, gate_open)` — returns envelope value 0..1
- `lfo(t, rate_hz, shape, phase_offset)` — normalized output

**Review session after PR.**

---

### SCORE wiring — import from PRIME
**Timing:** After Phase 2 (target weekend 2026-03-28)
Replace SCORE's internal easing/LFO/signal implementations with PRIME imports.
Validates PRIME API is ergonomic before building further. SCORE tests must stay green.

---

### Phase 3 — Space + curves
**Branch:** `feature/phase-3-spatial-splines`
**Target:** 2026-04-08
**Unlocks:** FORM ray marching, path following, navigation prep

#### `prime-spatial`
- Ray: `Ray { origin: Vec3, direction: Vec3 }` + `ray_at(t)`
- `ray_sphere(ray, center, radius)` → `Option<f32>` (t of hit)
- `ray_aabb(ray, min, max)` → `Option<f32>`
- `ray_plane(ray, normal, d)` → `Option<f32>`
- `Aabb { min: Vec3, max: Vec3 }` + `contains`, `intersects`, `expand`, `union`
- `frustum_cull(aabb, planes: &[Vec4; 6])` → `bool`

#### `prime-splines`
- `bezier_quadratic(p0, p1, p2, t)`, `bezier_cubic(p0, p1, p2, p3, t)`
- `bezier_tangent_cubic(p0, p1, p2, p3, t)`
- `catmull_rom(p0, p1, p2, p3, t, alpha)`
- `b_spline(control_points, t)` — uniform, open
- `slerp(a: Quat, b: Quat, t)`, `nlerp(a: Vec3, b: Vec3, t)`
- Arc-length parameterisation helper

**Review session after PR.**

---

### Phase 4 — Procedural + stochastic
**Branch:** `feature/phase-4-noise-voronoi-diffusion`
**Target:** 2026-04-22
**Unlocks:** FORM fully unblocked, STAGE procedural generation

#### `prime-noise`
- Value noise, Perlin noise (2D/3D)
- Simplex noise (2D/3D/4D)
- FBM (fractal Brownian motion) — configurable octaves, lacunarity, gain
- Worley / cellular noise (2D/3D)
- Curl noise (3D — divergence-free, for fluid/smoke)
- All seeded via `prime-random`

#### `prime-voronoi`
- Voronoi diagram (2D) — nearest site, distance
- Worley distance variants (F1, F2, F2-F1)
- Delaunay triangulation (2D)
- Lloyd relaxation (centroidal Voronoi)

#### `prime-diffusion`
- Probability distributions: uniform, normal (Box-Muller), exponential, beta, Bernoulli
- `sample_normal(mean, std_dev, rng)`, `sample_exponential(lambda, rng)`
- Monte Carlo integration: `integrate_mc(f, bounds, samples, rng)`
- Gaussian kernel: `gaussian(x, sigma)`, `gaussian_2d(x, y, sigma)`
- Importance sampling helper

**Review session after PR.**

---

### Phase 5 — Color
**Branch:** `feature/phase-5-color`
**Target:** 2026-04-28
**Unlocks:** FORM visual output, palette generation

#### `prime-color`
- `Srgb { r, g, b }`, `LinearRgb { r, g, b }`, `Oklab { L, a, b }`, `Hsl { h, s, l }`, `Hsv { h, s, v }`
- Conversions between all representations
- `oklab_lerp(a, b, t)` — perceptually uniform interpolation
- Palette generation: complementary, triadic, analogous, split-complementary
- `luma(rgb)`, `contrast_ratio(a, b)`, `luminance(rgb)`

**Review session after PR.**

---

### Phase 6 — Dynamics
**Branch:** `feature/phase-6-dynamics`
**Target:** 2026-05-05
**Unlocks:** SCORE chaos math from PRIME, FORM reaction-diffusion, STAGE AI variety

#### `prime-dynamics`
- ODE solvers: `euler_step(f, state, t, dt)`, `rk4_step(f, state, t, dt)`
- Lorenz attractor: `lorenz_step(state, sigma, rho, beta, dt)`
- Rössler attractor, Duffing oscillator, Van der Pol oscillator
- Logistic map: `logistic(x, r)`
- Lotka-Volterra (predator-prey): `lotka_volterra_step(prey, pred, alpha, beta, delta, gamma, dt)`
- SIR model: `sir_step(s, i, r, beta, gamma, dt)`
- L-systems: `LSystem { axiom, rules }` + `step(n)` → `String`
- Gray-Scott reaction-diffusion: `gray_scott_step(u, v, f, k, dt)`

**Review session after PR.**

---

### Phase 7 — STAGE foundation
**Branch:** `feature/phase-7-stage-loop`
**Target:** 2026-05-12
**Unlocks:** All other STAGE crates, idle-hero integration possible

#### `stage-loop`
- `FixedTick { tick: u64, dt: f32 }` — canonical game tick
- Fixed-timestep accumulator: `Accumulator::update(elapsed) -> Vec<FixedTick>`
- Interpolation alpha: `alpha(accumulator, fixed_dt)`
- `stage-input` (TS package) — input state snapshot, action mapping, digital/analog

**Review session after PR.**

---

### Phase 8 — STAGE game systems
**Branch:** `feature/phase-8-stage-systems`
**Target:** 2026-05-26
**Unlocks:** idle-hero Phase 3+ imports

#### `stage-quest`
- Quest node: `QuestNode { id, prerequisites: Vec<id>, state }`
- Topological sort: `topo_sort(nodes)` → ordered execution list
- Prerequisite validation, cycle detection

#### `stage-combat`
- `damage(base, armor, variance, rng)` — damage formula
- Stat curves: `stat_at_level(base, growth, level)`
- Hit chance: `hit(accuracy, evasion)`
- Critical: `critical(crit_chance, crit_mult, damage, rng)`

#### `stage-economy`
- Loot table: `LootTable { entries: Vec<(item_id, weight)> }` + `roll(rng)`
- Price curve: `price_at(base, supply, demand, curve_type)`
- Inflation model

**Review session after PR.**

---

### Phase 9 — STAGE AI + proc
**Branch:** `feature/phase-9-stage-ai-proc-nav`
**Target:** 2026-06-09
**Unlocks:** idle-hero Phase 5+ imports

#### `stage-ai`
- Utility AI: `score_action(considerations: &[f32]) -> f32` (geometric mean)
- FSM: `StateMachine<S, E>` + `transition(event)`
- Influence map: 2D grid + `propagate(sources, decay)`

#### `stage-proc`
- BSP dungeon: `bsp_split(room, min_size, rng)` → `Vec<Room>`
- Biome assignment: `biome(temperature, humidity)` → `BiomeId`
- Room connector, corridor generation

#### `stage-nav`
- A* pathfinding: `astar(grid, start, goal)` → `Option<Vec<Vec2>>`
- Flow field: `FlowField::build(grid, goal)` + `direction_at(pos)`
- SDF-aware navigation: `sdf_nav_step(pos, sdf, flow_field)`

**Review session after PR.**

---

### Phase 10 — TypeScript ports
**Branch:** `feature/phase-10-ts-ports`
**Target:** 2026-06-23
**All TS packages implemented.** Pure TS, matching Rust API.
Cross-language determinism tests for every crate.
WASM backend slot defined but not wired.

**Review session after PR.**

---

### Phase 11 — Publish
**Branch:** `feature/phase-11-publish`
**Target:** 2026-06-30
- `cargo publish` all PRIME crates in dependency order
- `pnpm publish` all TS packages
- `cargo publish` STAGE crates
- crates.io + npm README, examples, docs

---

## Key milestones

| Date | Milestone |
|---|---|
| 2026-03-25 | Phase 1 done — seeded RNG + interpolation available |
| 2026-03-28 | SCORE wiring — SCORE imports from PRIME |
| 2026-04-01 | Phase 2 done — SCORE fully unblocked |
| 2026-04-22 | Phase 4 done — FORM fully unblocked |
| 2026-05-12 | Phase 7 done — STAGE foundation, idle-hero integration possible |
| 2026-05-26 | Phase 8 done — idle-hero Phase 3+ systems available |
| 2026-06-09 | Phase 9 done — idle-hero Phase 5+ systems available |
| 2026-06-30 | Phase 11 done — public release on crates.io + npm |
