# PRIME — Claude Code Project Guide
Last updated: 2026-03-17

Read claude-resources/CLAUDE.md first, then this file.

---

## What is PRIME?

Pure math foundation for the FORM/SCORE/STAGE ecosystem. No game concepts, no audio, no graphics pipeline.

```
PRIME   pure math foundation  ← this repo
SCORE   audio framework       → imports prime-osc, prime-interp, prime-signal, prime-random
FORM    graphics framework    → imports prime-sdf, prime-noise, prime-color, prime-spatial
STAGE   game systems          → imports PRIME + FORM + SCORE
```

**Dependency rule:** arrows go downward only. PRIME knows nothing about SCORE, FORM, or STAGE. No circular dependencies ever.

**The rule that overrides everything:**
- "distance from point to sphere" → PRIME (pure geometry)
- "damage reduced by armor rating" → STAGE (requires game concept of armor)
- When in doubt, put it in PRIME. Easier to move math up into STAGE than to untangle game concepts from a math library.

---

## Workspace structure

```
prime/
├── Cargo.toml              # Cargo workspace — all Rust crates
├── package.json            # pnpm workspace root
├── pnpm-workspace.yaml
├── crates/
│   ├── prime-sdf/          # SDF primitives + CSG + domain transforms  ← ACTIVE
│   ├── prime-noise/        # Perlin, Simplex, FBM, Worley, curl        ← stub
│   ├── prime-color/        # Oklab, sRGB, HSL/HSV, palette             ← stub
│   ├── prime-splines/      # Bezier, Catmull-Rom, B-spline, slerp      ← stub
│   ├── prime-signal/       # smoothdamp, spring, low_pass, deadzone    ← stub
│   ├── prime-random/       # seeded PCG64 RNG, Poisson disk            ← stub
│   ├── prime-interp/       # easing, lerp, smoothstep                  ← stub
│   ├── prime-osc/          # LFO shapes, ADSR envelope                 ← stub
│   ├── prime-spatial/      # ray tests, AABB, frustum cull             ← stub
│   ├── prime-voronoi/      # Voronoi, Delaunay, Lloyd relaxation       ← stub
│   ├── stage-nav/          # A*, flow fields, SDF navigation           ← stub
│   ├── stage-combat/       # damage, stats, curves                     ← stub
│   ├── stage-economy/      # loot tables, price curves                 ← stub
│   ├── stage-quest/        # quest graphs, topological sort            ← stub
│   ├── stage-ai/           # utility AI, FSM, influence maps           ← stub
│   ├── stage-proc/         # dungeon gen, BSP, biome assignment        ← stub
│   └── stage-loop/         # fixed tick, accumulator, interpolation    ← stub
├── packages/               # TypeScript packages (WASM wrappers + pure TS)
│   └── (see pnpm-workspace.yaml)
└── docs/
    └── math/               # one .md per crate — formulas, derivations
```

---

## Implementation priority order

1. `prime-random` — everything depends on determinism
2. `prime-interp` — easing/lerp (extracted from SCORE too)
3. `prime-signal` — smoothdamp + spring (game feel foundation)
4. `prime-sdf` — already implemented, migrate done ✓
5. `prime-noise`, `prime-color`, `prime-splines`, `prime-spatial`, `prime-osc`, `prime-voronoi`
6. `stage-loop` — unblocks all other STAGE crates
7. `stage-input` — pure TypeScript, 300 lines
8. `stage-quest`, `stage-combat`, `stage-economy`, `stage-ai`, `stage-proc`, `stage-nav`

---

## Code standards

### Rustdoc — MANDATORY on every public function

```rust
/// One-line description.
///
/// # Math
///   formula in plain ASCII math
///
/// # Arguments
/// * `arg` - what it represents, valid range
///
/// # Returns
/// What the return value means.
///
/// # Edge cases
/// * edge → behavior
///
/// # Example
/// ```rust
/// // Happy path — concrete input/output pair
/// ```
```

No exceptions. The math documentation IS the product.

### Testing — every public function needs:
- outside test
- inside test
- on-surface test (where applicable)
- edge case

Use `const EPSILON: f32 = 1e-5` for float comparisons.

### Cross-language determinism
For functions in both Rust and TypeScript, add a cross-language test in the TS package.
Values from Rust implementation are the reference. If Rust changes, update TS test values.

---

## What NOT to do without being asked

- No serde on any crate until API is stable
- No async/tokio — all math is synchronous pure functions
- No wasm-pack yet — native Rust only until core is stable
- No publishing to crates.io or npm yet
- No stage-nav before prime-sdf and prime-spatial are complete
- No web server, HTTP, or networking in any crate

---

## Commands

```bash
export PATH="$PATH:/c/Users/bwyar/.cargo/bin"

# Build all
cargo build --workspace

# Test all
cargo test --workspace

# Test one crate
cargo test -p prime-sdf

# Clippy
cargo clippy --workspace -- -D warnings

# Docs
cargo doc -p prime-sdf --open
```

---

## Relation to FORM

`form/crates/form-sdf` is now a thin re-export of `prime-sdf`:
```rust
pub use prime_sdf::*;
```
All SDF logic lives here. FORM imports prime-sdf as a path dependency during development,
will switch to crates.io dependency when published.
