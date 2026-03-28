# PRIME — Temporal Architecture Handoff

For Claude Code sessions on the PRIME repo.
Last updated: 2026-03-28

---

## What this is

PRIME is a pure math foundation for the FORM/SCORE/STAGE ecosystem. It is also — intentionally — a living proof of **Temporal Architecture**, a foundational theory of computation being developed alongside it. Every engineering decision in PRIME is grounded in that theory. This document explains the connection so Claude Code can make decisions consistent with both.

---

## The Four Primitives

The thesis argues these are the missing first-class citizens in computation:

### 1. Time

Every value exists at a time coordinate. Time must be a parameter, never a side effect. A function that calls `Date.now()`, `Math.random()`, or any clock internally is impure — it creates time rather than receiving it. Time comes from outside. Always.

```rust
// WRONG — creates time internally
pub fn next_id() -> u64 { SystemTime::now()... }

// RIGHT — time coordinate passed in
pub fn next_id(timestamp: u64, counter: u32) -> u64 { ... }
```

### 2. Causality

The ordering relation over events. Arrows go one direction only. Nothing causes itself. This appears in PRIME as:

- The dependency graph: `PRIME → SCORE/FORM → STAGE`. Never circular.
- The fold pattern: each step causally follows the previous.
- `prng_next(seed) → (value, next_seed)`: each seed causally precedes the next.
- No `&mut` in public APIs: the caller's state is never destroyed by a function call.

### 3. Information

What flows along causal chains. In PRIME this means:

- Function parameters are the complete information contract. No hidden inputs.
- Return values are the complete output contract. No hidden outputs.
- The Rustdoc `# Math` section IS the information specification — not documentation of the implementation, but the mathematical object the implementation expresses.
- Cross-language determinism: the same information must be extractable in Rust and TypeScript. If outputs differ, the information contract is broken.

### 4. Context

The receiver function that determines what information is extractable. In PRIME:

- TypeScript bindings are a context function over Rust implementations — same math, different receiver.
- WASM compilation is a context function — same causal structure, different execution target.
- naga shader transpilation (in FORM) is a context function — same math, different GPU target.
- The consumer (SCORE, FORM, STAGE) is a context function over PRIME's pure math.

---

## How the LOAD/COMPUTE/APPEND/ADVANCE pattern maps to the primitives

This is already in `CLAUDE.md`. Here is the explicit mapping:

```
LOAD    ← Information   — explicit parameters only, no hidden inputs
COMPUTE ← Causality     — pure transformation, same inputs always same outputs
APPEND  ← Time          — new state returned, old state untouched, causal chain preserved
ADVANCE ← Context       — fold/reduce moves the temporal thread forward
```

When a function follows LOAD/COMPUTE/APPEND/ADVANCE it is implementing the four primitives correctly. When it violates any of these it is violating the thesis.

---

## The RNG design is the clearest example

```javascript
// WRONG — Von Neumann model
Math.random() // mutable global, no time coordinate, no causal parent, irreproducible

// RIGHT — Temporal Architecture model
prngNext(seed) → [value, nextSeed]
// seed IS the time coordinate
// nextSeed is the causal successor
// same seed always produces same value
// the sequence is a causal chain of seeds
```

**"The seed IS the RNG"** — this line in CLAUDE.md is a statement that identity is temporal. The RNG is not an object. It is a causal chain of seeds. The current seed is the RNG's identity at this moment in time. The `nextSeed` is its identity at the next moment. There is no persistent object — only a causal trajectory.

---

## What "No `&mut` in public API" means theoretically

`&mut` is a mutable reference — permission to overwrite a value in the caller's memory. Banning it from public APIs is a formal statement that:

- No public operation destroys information in the caller's scope
- The caller's causal chain is never damaged by calling a function
- Every state transition produces a new value — the old value remains

This is the basketball proof encoded as a compiler constraint. The inflated and deflated basketball cannot occupy the same memory location. Every transition is a new element in the causal set.

---

## What "No `let` in TypeScript" means theoretically

`let` creates a mutable variable — a Von Neumann register. Banning it means:

- No information is destroyed by reassignment
- State must be threaded explicitly via function parameters and return values
- The fold pattern makes the causal chain structurally visible

```typescript
// WRONG — Von Neumann model, 200 states destroyed
let [pos, vel] = [0, 0]
for (let i = 0; i < 200; i++) [pos, vel] = step(pos, vel, dt)

// RIGHT — causal chain preserved, current state derived from fold
const [finalPos] = Array.from({ length: 200 }).reduce(
  ([p, v]) => step(p, v, dt),
  [0, 0]
)
```

The fold is the causal log being evaluated. The initial value is the genesis event. Each step is a pure transition. The result is current state derived from the full history.

---

## Cross-language determinism as the functor test

The thesis claims FP, WASM, and TypeScript are the same categorical structure at different abstraction levels — projections of the same causal chain through different receiver functions.

The cross-language determinism requirement is the test that this claim holds:

```
// Rust produces reference values
// TypeScript must match exactly
// If they differ, the functor doesn't hold
// The causal structure was not preserved across the compilation boundary
```

When writing cross-language tests, you are testing whether the category theory claim holds in practice. A test failure means the information was lost in translation — the receiver context changed the meaning, not just the form.

---

## Crate responsibilities mapped to primitives

| Crate | Primary primitive demonstrated |
|-------|-------------------------------|
| prime-random | **Time** — seed as temporal coordinate, prngNext as causal succession |
| prime-signal | **Causality** — smoothdamp, spring as causal transitions over time steps |
| prime-interp | **Information** — pure transformations preserving mathematical content |
| prime-sdf | **Causality** — CSG operations as functional composition of geometry |
| prime-noise | **Time** — deterministic noise as pure function of spatial + temporal coordinates |
| prime-spatial | **Context** — ray tests, frustum culling as receiver functions over geometry |
| prime-color | **Information** — color space transforms as information-preserving projections |
| prime-splines | **Time** — parametric curves as functions over continuous time coordinate |
| prime-osc | **Time** — LFO and ADSR as explicit functions of time coordinate |
| prime-dynamics | **Causality** — physics as ordered causal chain, each step causing the next |

---

## Decisions Claude Code should make consistently with the thesis

### When adding a new function:
- Time parameters (`t`, `dt`, `timestamp`) must be explicit parameters, never generated internally
- State returned as tuple `(new_state, output)` — never mutation
- The `# Math` section in Rustdoc is mandatory — the formula IS the specification

### When the seed/RNG is needed:
- Thread the seed explicitly: `fn thing(seed: u32) -> (output, u32)`
- Never store RNG state in a struct with mutation
- The returned `u32` is the next seed — the caller advances the temporal thread

### When handling errors:
- Return `Result<T, PrimeError>` — errors are values, not exceptions
- Throwing is a side effect that breaks the causal chain

### When writing TypeScript:
- `const` only — `let` is banned
- Fold over mutation — loops that reassign are wrong
- Tuple destructuring threads state: `const [a, nextSeed] = prngNext(seed)`

### When adding a dependency:
- Check the dependency arrow direction — PRIME must not import SCORE, FORM, or STAGE
- A dependency from PRIME on a game concept is a category error
- When in doubt, put it in PRIME — easier to move math up than untangle it later

---

## What PRIME still needs to fully prove the thesis

These are known gaps — flag them as architectural priorities, not bugs:

1. **Explicit time coordinates in non-random crates** — prime-signal's `dt` is correct. Other crates need explicit time parameters where time is relevant.

2. **Causal parent references** — the fold pattern preserves sequence but doesn't record causal ancestry explicitly. A `cause` field or causal log type may be needed for the thesis proof.

3. **The context primitive formalized** — cross-language tests are informal context testing. A formal `Receiver<T>` abstraction that makes the context function explicit would complete the model.

---

## One sentence for when you're unsure

**If a function creates information (timestamps, random values, IDs) rather than receiving it as a parameter, it is wrong.** Move the creation to the boundary and inject the result.

---

## Related reading

- `CLAUDE.md` in this repo — engineering conventions that implement the thesis
- `Cargo.toml` workspace — the dependency graph is the causal set
- Sorkin, "Causal Sets: Discrete Gravity" — arxiv.org/abs/gr-qc/0309009
- Shannon, "A Mathematical Theory of Communication" (1948)
- Landauer, "Irreversibility and Heat Generation in the Computing Process" (1961)
