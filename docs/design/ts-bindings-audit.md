# PRIME TypeScript Bindings Audit
Session: prime s004 | Date: 2026-03-29 | Todo: t227

Audit of all TypeScript packages against thesis compliance:
zero `let`, zero classes, pure functions, factory pattern, immutable by default.

---

## Verdict by Package

| Package | Status | Notes |
|---|---|---|
| prime-random | ✅ COMPLIANT | |
| prime-sdf | ✅ COMPLIANT | |
| prime-interp | ✅ COMPLIANT | |
| prime-voronoi | ✅ COMPLIANT | |
| prime-dynamics | ✅ COMPLIANT | `let` in ADVANCE-EXCEPTION blocks only — all marked and justified |

---

## Detail: prime-random

**Pattern:** `(seed, ...params) => [value, nextSeed]`
All state threads forward through return values. No hidden RNG state.

**ADVANCE-EXCEPTION:** `successors()` helper encapsulates the one `let` + `while` needed
for Bridson-class algorithms. The exception rationale is documented inline:
data-dependent termination, stack overflow risk, encapsulated mutation.

All other functions are pure `const` → tuple. `vanDerCorput` has a `while` loop
for digit extraction, also marked ADVANCE-EXCEPTION.

**No issues.**

---

## Detail: prime-sdf

Pure coordinate → scalar functions. No state. No tuples needed (geometry is stateless).

**No issues.**

---

## Detail: prime-interp

Pure `(t) => number` easing functions. All `const`. No state.

Minor gap: most easing functions have one-liner comments only. Full TSDoc with
`@param` and `@example` present only on the core functions (`lerp`, `smoothstep`,
`remap`). Not a thesis violation — a doc quality note.

**No issues.**

---

## Detail: prime-voronoi

Pure functions over `readonly [number, number][]` point arrays.
Uses `reduce` throughout — no loops, no mutation.

Functions audited: `voronoiNearest2d`, `voronoiF1F2_2d`, Lloyd relaxation,
Bowyer-Watson Delaunay triangulation.

**No issues.**

---

## Detail: prime-dynamics

Pure step functions: `(state, ...params) => newState`.
State is a scalar or `[number, number, number]` tuple.

**ADVANCE-EXCEPTION blocks (all justified and marked):**

| Function | Let vars | Reason |
|---|---|---|
| `rk45Adaptive` | `y, t, dt, steps` | Adaptive step ODE — termination is data-dependent, step count unknown a priori |
| `newtonRaphson` | `x` | Convergence loop — bounded by `maxIter` |
| `bisection` | `lo, hi, fLo` | Convergence loop — bounded by `maxIter` |
| `integrateAdaptive` | none | Uses recursion (bounded by `maxDepth`) — no `let` |

The CLAUDE.md ADVANCE-EXCEPTION rule was originally scoped to Bridson-class algorithms.
The dynamics convergence loops extend the scope. This extension is valid — the rationale
(data-dependent termination, mutation encapsulated within the function, same inputs →
same output) applies equally to convergence loops.

**Recommendation:** Update CLAUDE.md to extend ADVANCE-EXCEPTION to bounded convergence
loops, not just Bridson-class. Proposed wording:
> "ADVANCE-EXCEPTION: `while`/`for` loops where termination is data-dependent
> (Bridson-class algorithms, adaptive ODE solvers, convergence loops) — mark with
> `// ADVANCE-EXCEPTION`. Mutation must be local to the function."

**TSDoc gap:** `duffingStep` is exported without a block docstring. Minor — fix in same PR.

---

## Chainable Vectors — Closed

The coordination audit specifically asked about chainable vectors.

**Decision: the current tuple approach is correct. Chainable vectors are a thesis violation.**

A chainable vector (`new Vec2(1,2).add(Vec2(3,4)).scale(2)`) requires a class with
methods — zero classes is a hard rule. The tuple approach (`[x, y]` + pure functions)
is the correct thesis-aligned pattern. It is more composable, not less: any `reduce`,
`map`, or destructuring works directly.

FORM and STAGE may introduce `ChainableShape` or `ChainableEntity` builder objects in
their DSL layers. Those live above PRIME in the stack. PRIME stays pure math: tuples +
pure functions only.

**No chainable vector API should be added to any prime package.**

---

## Factory Function Pattern

The coordination audit asked about factory functions.

PRIME doesn't need `createX` factories because there are no stateful objects to
construct. Every package exports pure functions. A "factory" in PRIME would return
a closure (e.g., `memoize1d` returns a lookup function — this is the correct pattern).

The `createX` factory pattern applies to Score/Form/Stage where stateful or configurable
objects are needed. PRIME has no such objects.

**Verdict: factory functions are not needed. `memoize1d`-style closures are the correct
pattern when a pre-computation step is needed.**

---

## Issues Added as Todos

Three open GitHub issues were added to the todo list (t331–t333):

| Todo | Issue | Description |
|---|---|---|
| t331 | #31 | Adaptive Simpson's quadrature — Rust crate implementation |
| t332 | #32 | Dormand-Prince RK45 adaptive ODE solver — Rust crate implementation |
| t333 | #33 | Newton-Raphson + bisection root finding — Rust crate implementation |

Note: all three are **already implemented in prime-dynamics TS**. The issues track the
corresponding Rust crate implementations in `prime-dynamics/src/`.

---

## Gaps: Packages with No TS Bindings Yet

These packages have Rust crates but no TS `index.ts`:

| Package | Rust status | TS status | Priority |
|---|---|---|---|
| prime-noise | stub | none | Medium (FORM needs this) |
| prime-color | stub | none | Medium (FORM needs this) |
| prime-splines | stub | none | Low |
| prime-signal | stub | none | High (SCORE + STAGE need smoothdamp/spring) |
| prime-osc | stub | none | Medium (SCORE needs LFO/ADSR) |
| prime-spatial | stub | none | Low |

`prime-signal` is highest priority because `smoothdamp` and `spring` are needed by
both SCORE (parameter smoothing) and STAGE (game feel). These should be pure TS
(no WASM needed — they're simple numerical functions).

---

## CLI Scope

The coordination notes listed CLI scoping as part of t227. Proposed scope:

**Purpose:** Development tooling for PRIME consumers. Not a user-facing product.

**Commands:**
```
prime sample <package> <function> [args]   # evaluate a function, print result
prime bench <crate>                        # run Criterion benchmarks
prime xtest <package>                      # cross-language determinism test (TS vs Rust)
prime lint                                 # run ESLint + cargo clippy together
```

**Implementation:** Pure Node.js script (`packages/prime-cli/src/index.ts`).
No external CLI framework — just `process.argv` + direct function calls.

**Priority:** Low. Not needed until SCORE/FORM start consuming PRIME packages.
Add as a separate todo when actually needed.

---

## Recommended Actions

### Immediate (this session)

1. Update CLAUDE.md to extend ADVANCE-EXCEPTION to bounded convergence loops (not just Bridson)
2. Add TSDoc to `duffingStep`

### Next session

3. Implement `prime-signal` TS bindings: `smoothdamp`, `spring`, `deadzone` — pure TS, no WASM
4. Implement `prime-osc` TS bindings: `lfoSine`, `lfoSquare`, `adsr` — pure TS
5. After Rust crates are implemented (t331–t333): add TS counterparts to `prime-dynamics`

### Planning sessions now unblocked

- **STAGE DSL planning** (t228) — PRIME audit complete
- **FORM DSL planning** (t230) — PRIME audit complete
- **Idle Hero PRIME wiring** — PRIME audit complete; `prime-signal` TS needed first

---

## Phase Plan Sketch

| Phase | Scope | Blocks |
|---|---|---|
| P1 (current) | Audit complete, CLAUDE.md update, duffingStep doc | Nothing |
| P2 | prime-signal TS bindings | Idle Hero PRIME wiring |
| P3 | prime-osc TS bindings | SCORE LFO/ADSR wiring |
| P4 | Rust implementations: #31 #32 #33 | Cross-language determinism tests |
| P5 | prime-noise + prime-color TS bindings | FORM shader pipeline |
| P6 | CLI tooling | Developer ergonomics |
| P7 | Batch publish to npm + crates.io | All consumers switch to registry deps |
