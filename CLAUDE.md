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
│   └── prime-voronoi/      # Voronoi, Delaunay, Lloyd relaxation       ← stub
│   # stage-* crates moved to development/stage/ (separate workspace)
│   # STAGE imports prime as a path dependency
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

## Temporal Assembly Model — MANDATORY

PRIME implements the temporal assembly thesis in code. Every public function is LOAD + COMPUTE only.

```
Temporal assembly:       What it means in PRIME:
LOAD    ← read input     function parameters
COMPUTE ← pure math      function body
APPEND  ← write new fact return new state as tuple
ADVANCE ← move forward   reduce/fold over time steps
```

**STORE and JUMP do not exist in PRIME.** They are poverty-era compromises.
STORE was invented because RAM was scarce in 1945. JUMP because mutating the instruction pointer
was cheaper than logging. Neither constraint applies here.

### Rules

1. **No STORE — no `&mut` in public function signatures.** Functions take values in, return values out.
   - STORE (wrong): `pub fn smoothdamp(velocity: &mut f32) -> f32`
   - APPEND (right): `pub fn smoothdamp(velocity: f32) -> (f32, f32)` — new state returned, old untouched

2. **No STORE — no mutation of external state.** Never write to caller-owned memory.

3. **Deterministic.** Same inputs always produce the same output. No hidden state.

4. **No side effects.** No printing, no I/O, no global state, no clocks.

5. **No exceptions.** prime-random has no STORE, no JUMP, no classes.
   The seed IS the RNG. `prngNext(seed) → [value, nextSeed]` threads state forward.
   No class, no write head, no mutation anywhere.

6. **TypeScript: no `let` — use `const` only.** Production code AND tests.
   - ADVANCE pattern: `Array.from({ length: N }).reduce((state) => step(state, dt), init)`
   - `reduce` is ADVANCE — it moves the state forward through time steps without a mutable pointer.
   - Tuple destructuring `const [a, b] = fn()` threads state explicitly between steps.
   - Exceptions: `while` loops in Bridson-class algorithms (stack overflow risk) — mark with `// ADVANCE-EXCEPTION`.

### Flag these patterns — they are STORE or JUMP violations

```rust
// ❌ STORE — mutates caller's state
pub fn smoothdamp(vel: &mut f32) -> f32 { ... }

// ❌ STORE — mutates slice in place
pub fn shuffle(slice: &mut [T]) { ... }

// ✅ APPEND — returns new state as tuple
pub fn smoothdamp(vel: f32) -> (f32, f32) { ... }

// ✅ APPEND — returns new collection
pub fn shuffled<T: Clone>(slice: &[T]) -> Vec<T> { ... }
```

```typescript
// ❌ STORE + JUMP
let [pos, vel] = [0, 0]
for (let i = 0; i < 200; i++) [pos, vel] = smoothdamp(pos, 10, vel, 0.3, 0.016)

// ✅ APPEND + ADVANCE
const [pos] = Array.from({ length: 200 }).reduce(
  ([p, v]: [number, number]) => smoothdamp(p, 10, v, 0.3, 0.016),
  [0, 0] as [number, number],
)
```

When reviewing code, call out any STORE or JUMP as a violation of the temporal assembly model.

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
- No publishing to crates.io or npm yet — **all packages publish together as a batch, never individually**
- No stage-nav before prime-sdf and prime-spatial are complete
- No web server, HTTP, or networking in any crate

## Consumer dependency strategy (pre-publish)

Until the batch publish happens, other repos (SCORE, FORM, STAGE, idle-hero) consume prime via
**GitHub path dependencies** pointing directly at this repo. Do not wire consumers via npm.

**Rust crates** — use git dependency in consumer Cargo.toml:
```toml
prime-random = { git = "https://github.com/bwyard/prime", branch = "dev" }
```

**TypeScript packages** — use GitHub URL dependency:
```json
"@prime/prime-random": "github:bwyard/prime#dev"
```
This tells pnpm/npm to pull directly from the prime repo's `dev` branch.
No npm account, no publish step, no version bumping — just point at the repo.

When all packages are stable and API-frozen, publish the whole batch to npm/crates.io at once.

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
