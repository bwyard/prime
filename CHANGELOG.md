# prime — Changelog

## 2026-03-29 (all commits)

### Features

- 64-bit PRNG tests + dynamics TS port updates (2257edc)
- **prime-random:** add SplitMix64 (2^64 period) + memoize_1d (969b5de)
- **prime-dynamics:** add numerical calculus + Van der Pol + benchmarks (fe0e102)
- add Criterion benchmarks for prime-dynamics and prime-voronoi (9cfa941)
- **prime-voronoi:** add Bowyer-Watson Delaunay triangulation (3dc27e4)
- **prime-dynamics:** add L-system string rewriting (b6b5060)
- Bridson im::Vector optimization, ADR-006/007, README update (af4c8b2)
- CausalStep<T> type + receiver model docs + ADR-005 (4c6b753)
- WASM bindings for Phase 2-6 + architecture decision records (17e96d9)
- **ts:** Phase 10 — complete batch 3 tests + diffusion PRNG unification (b5be424)
- **ts:** Phase 10 — update TS ports for prime-dynamics, prime-spatial, prime-splines (0f4a784)
- **ts:** Phase 10 — update TS ports for prime-signal, prime-osc, prime-interp (097fa43)
- **ts:** Phase 10 — update TS ports for prime-noise, prime-color, prime-voronoi (28ea695)
- additional Phase 3/5 completions from parallel agents (616d1e7)
- complete Phases 2-6 — add missing ROADMAP functions across 6 crates (c309abf)
- **prime-interp:** add missing Phase 1 functions, complete doc-tests (d1f2c37)
- **prime-random:** add distributions, quasi-random, Monte Carlo, fix Bridson (71dc0e9)
- **prime-wasm:** add WASM bindings for all prime crates + CI job (00a1a34)
- **prime-voronoi:** Voronoi nearest-neighbor, F1/F2 distances, Lloyd relaxation (5313880)
- **prime-diffusion:** Ornstein-Uhlenbeck and geometric Brownian motion (40c54b4)
- **prime-splines:** implement Bezier, Hermite, Catmull-Rom, B-spline, slerp (095db0c)
- **prime-noise:** add 3D noise, simplex, and domain warping (e0b3ec2)
- **prime-dynamics:** implement dynamical systems and numerical integration (fc14b94)
- **prime-render:** pure sample-level scan loop — ADVANCE evaluator (d2abac8)
- **ts-ports:** prime-noise, prime-color, prime-spatial TypeScript ports (fa4605e)
- **prime-spatial:** ray tests, AABB, frustum cull — unblocks Form (91df676)
- **prime-color:** Oklab, sRGB, HSL — pure color math, zero deps (623ff00)
- **prime-noise:** value noise, Perlin, FBM, Worley — pure, zero deps (af47955)
- **prime-osc:** LFO shapes + ADSR envelope — unblocks Score (9671d5f)
- **prime-signal:** smoothdamp, spring, lowpass, highpass, deadzone (563440e)
- **prime-interp:** easing + interpolation — Rust + TS, full test coverage (6f3fffd)
- **prime-random:** pure Mulberry32 — no class, APPEND/ADVANCE only (db0b1be)

### Bug Fixes

- use std::hint::black_box for criterion 0.8, fix clippy in prime-noise (b6f4570)
- **thesis:** self-contained eslint config — no file: dependency (b2420be)
- **prime-random:** noUncheckedIndexedAccess compatibility — non-null assertions + loose null check (cfa3922)
- **thesis:** remove mut params from private helpers hash_u32 and hsl_component (2e16950)
- **prime-voronoi:** fix typecheck error in voronoiF1F2_2d parity test (b35374b)
- **prime-splines:** remove --passWithNoTests from test script (9b024ac)
- vitest --passWithNoTests for stub packages (sdf, splines, voronoi) (537dcc1)
- rng.test.ts prngBool reduce return type annotation (f1ec24a)
- remaining clippy + TS type annotation issues (dddc4a8)
- noise fbm2d callback return type annotation for TS strict overload resolution (14c40fe)
- CI — clippy excessive_precision + map_or + TS Array.from<null> type inference (a7a93a5)

### Performance

- revert im::Vector — Vec clone faster at Bridson grid sizes (68b2bc7)
- **prime-random:** thesis-pure Bridson, add stratified MC integration (b2d8eae)

### Refactoring

- **prime-diffusion:** unify PRNG with prime-random Mulberry32 (1422f59)
- **prime-random:** replace let/while with successors utility in TS Bridson (2af35dd)

### Tests

- add 7 rigorous statistical tests for academic/production standards (75bcbe2)
- add 8 statistical validation tests for thesis proof (517ac99)
- add receiver model golden fixture tests (0f15287)

### Documentation

- **audit:** TS bindings audit + CLAUDE.md ADVANCE-EXCEPTION clarification (d03d8ab)
- update ROADMAP — all PRIME phases complete (9d1d00e)
- add README for release (da11035)
- math reference for prime-splines (cac902d)
- complete math reference for all remaining crates (471d94a)
- math reference for prime-interp, prime-signal, prime-osc (0204555)
- update prime-random benchmarks with post-merge numbers (006630a)
- write prime-random mathematical reference for thesis (16607b8)
- **prime:** document batch publish policy + GitHub dep strategy for consumers (adccecb)

### Chores

- upgrade glam 0.27 → 0.32, criterion 0.5 → 0.8 (998e6b3)
- trim remaining rustdoc bloat on MC and Bridson functions (f08d516)
- trim rustdoc bloat, remove dead deps, update CLAUDE.md (79c9711)
- scrub research language from comments + add NOTICE file (a995f1c)
- migrate thesis enforcement to shared ESLint config (8ce575d)
- add thesis-check CI gate (TS packages) (2e5c8da)
- **polish:** implement prime-sdf TS, add cross-language parity tests, fill coverage gaps (fc6b105)
- full coverage pass — edge cases, WASM gaps, overflow fix (98bbc1d)
- update pnpm lockfile for new packages (fbea8c4)
- gitignore THESIS.md — local only, not ready to publish (701da3e)
- remove stage-* TypeScript packages from prime workspace (fc168b4)
- remove stage-* crate dirs from prime workspace (444f84f)
- temporal assembly vocabulary, workspace tsconfig + pnpm setup (95760c8)
- roadmap, prime-diffusion + prime-dynamics stubs, remove form packages (47600b4)
- initial scaffold — workspace, all crate stubs, TS packages (3003184)

### CI

- trigger CI after retarget to dev (7f08ab5)
- add GitHub Actions workflow — Rust + TypeScript (75e010f)
