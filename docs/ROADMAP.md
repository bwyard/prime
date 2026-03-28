# PRIME Roadmap
Last updated: 2026-03-28

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
| `prime-random` | Mulberry32 RNG, distributions, Bridson, Monte Carlo, Halton | ✅ Done — 60+ tests, benchmarked |
| `prime-interp` | Easing functions, lerp, smoothstep, remap, pingpong | ✅ Done — 48 tests |
| `prime-signal` | Smoothdamp, spring, low/high pass, deadzone (f32/Vec2/Vec3) | ✅ Done — 36 tests |
| `prime-osc` | LFO waveforms (sine/cos/tri/saw/square), ADSR envelope | ✅ Done — 34 tests |
| `prime-noise` | Perlin, Simplex, FBM, Worley, curl noise (2D/3D) | ✅ Done — 74 tests |
| `prime-color` | Oklab, sRGB, HSL/HSV, luminance, contrast, palette | ✅ Done — 70 tests |
| `prime-splines` | Bézier, Catmull-Rom, B-spline, slerp, arc-length | ✅ Done — 56 tests |
| `prime-spatial` | Ray tests, AABB, frustum cull (sphere + AABB) | ✅ Done — 39 tests |
| `prime-voronoi` | Voronoi nearest, F1/F2, Lloyd relaxation | ✅ Done — 16 tests |
| `prime-diffusion` | Ornstein-Uhlenbeck, GBM (uses prime-random Mulberry32) | ✅ Done — 23 tests |
| `prime-dynamics` | Euler/RK4, Lorenz, Rössler, Duffing, Lotka-Volterra, SIR, Gray-Scott | ✅ Done — 45 tests |

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

### Phase 1 — Core determinism + interpolation ✅
**Done.** PRs #22, #24, #25 merged.

#### `prime-random` ✅
- Mulberry32 PRNG: `prng_next(seed) -> (f32, u32)` — pure seed threading
- Distributions: Gaussian (Box-Muller), exponential, disk/annulus uniform
- Bridson's Poisson disk sampling with area-uniform annulus correction
- Monte Carlo integration (plain + stratified), Welford variance
- Quasi-random: van der Corput, Halton 2D/3D
- CausalStep<T> for backward traversal of deterministic sequences
- 15 statistical validation tests (chi-square, KS, Jarque-Bera, Anderson-Darling)
- Criterion benchmarks (8 benchmark groups)
- 60+ unit tests, 23 doc tests

#### `prime-interp` ✅
- lerp, lerp_clamped, inv_lerp, remap, repeat, pingpong
- smoothstep, smootherstep
- 10 easing families: quad, cubic, quart, quint, sine, expo, circ, elastic, bounce, back
- 48 tests

---

### Phase 2 — Game feel + oscillation ✅
**Done.** PR #25 merged.

#### `prime-signal` ✅
- smoothdamp, spring, low_pass, high_pass, deadzone
- Vec2/Vec3 variants for smoothdamp and spring
- 36 tests

#### `prime-osc` ✅
- LFO: sine, cosine, triangle, sawtooth, square (with duty cycle)
- ADSR envelope state machine
- 34 tests

---

### SCORE wiring — import from PRIME
**Status:** Ready. PRIME APIs are stable. SCORE can import from `dev` branch.

---

### Phase 3 — Space + curves ✅
**Done.** PR #25 merged.

#### `prime-spatial` ✅
- ray_sphere, ray_aabb, ray_plane
- aabb_overlaps, aabb_contains, aabb_union, aabb_closest_point
- frustum_cull_sphere, frustum_cull_aabb
- 39 tests

#### `prime-splines` ✅
- bezier_quadratic/cubic (1D + 3D), hermite, catmull_rom, b_spline_cubic, slerp
- Arc-length: bezier_cubic_arc_length, bezier_cubic_t_at_length (1D + 3D)
- 56 tests

---

### Phase 4 — Procedural + stochastic ✅
**Done.** PR #25 merged.

#### `prime-noise` ✅
- value_noise, perlin, simplex (2D/3D), fbm (2D/3D), worley, domain_warp (2D/3D)
- curl_2d, curl_3d (divergence-free vector fields)
- 74 tests

#### `prime-voronoi` ✅
- voronoi_nearest_2d, voronoi_f1_f2_2d, lloyd_relax_step_2d
- Delaunay triangulation deferred (Bowyer-Watson complexity)
- 16 tests

#### `prime-diffusion` ✅
- ou_step, ou_step_seeded, gbm_step, gbm_step_seeded
- Uses prime-random Mulberry32 (unified PRNG, u32 seeds)
- 23 tests

---

### Phase 5 — Color ✅
**Done.** PR #25 merged.

#### `prime-color` ✅
- sRGB ↔ linear, sRGB ↔ Oklab, sRGB ↔ HSL, sRGB ↔ HSV
- oklab_mix (perceptual interpolation)
- luminance (BT.709), contrast_ratio (WCAG)
- palette_complementary, palette_triadic, palette_analogous
- 70 tests

---

### Phase 6 — Dynamics ✅
**Done.** PR #25 merged.

#### `prime-dynamics` ✅
- ODE solvers: euler_step, rk4_step, rk4_step3
- Chaos: lorenz_step, rossler_step, duffing_step
- Biological: lotka_volterra_step, sir_step
- Discrete: logistic map
- Reaction-diffusion: gray_scott_step
- L-systems deferred (string allocation paradigm mismatch)
- 45 tests

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

### Phase 10 — TypeScript ports ✅
**Done.** PR #25 merged.

All 10 TS packages updated to match Rust APIs. Cross-language tests for every crate.
WASM bindings complete (60+ functions). `successors` utility encapsulates the only
`let`/`while` in the TS codebase (ADVANCE-EXCEPTION, see ADR-007).

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

| Date | Milestone | Status |
|---|---|---|
| 2026-03-17 | Phase 0 done — scaffold + prime-sdf | ✅ |
| 2026-03-28 | Phases 1-6 done — all PRIME crates implemented | ✅ |
| 2026-03-28 | Phase 10 done — all TS ports complete | ✅ |
| 2026-03-28 | Math docs complete — 11 files with formulas/derivations | ✅ |
| 2026-03-28 | ADRs complete — 7 architecture decision records | ✅ |
| 2026-03-28 | Benchmarks — Criterion 0.8, statistical validation | ✅ |
| — | Phase 7 — STAGE foundation (stage-loop) | Pending (separate repo) |
| — | Phase 8 — STAGE game systems | Pending (separate repo) |
| — | Phase 9 — STAGE AI + proc | Pending (separate repo) |
| — | Phase 11 — Publish to crates.io + npm | **Next** |

## Deferred items

| Item | Reason |
|---|---|
| Delaunay triangulation | Bowyer-Watson algorithm complexity (~200 lines); deferred to post-release |
| L-systems | String allocation paradigm mismatch with pure numeric functions |
| Van der Pol oscillator | Low priority; Duffing covers the same ODE category |
| Simplex 4D | Not needed by current consumers (SCORE/FORM) |
