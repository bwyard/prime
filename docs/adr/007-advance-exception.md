# ADR-007: ADVANCE-EXCEPTION Pattern

**Status:** Accepted
**Date:** 2026-03-28

## Context
PRIME mandates `const`-only TypeScript and fold-based iteration (ADR-003). However, certain algorithms have data-dependent termination — the number of iterations is not known in advance. Bridson's Poisson disk sampling, for example, runs until a spatial queue is exhausted. Expressing this as a fixed-length fold would either require an arbitrary upper bound (incorrect) or recursive calls (stack overflow risk for large inputs).

## Decision
Algorithms with data-dependent termination may use `while` loops with `let` bindings, provided they are marked with the `// ADVANCE-EXCEPTION` comment. The exception applies only to the iteration mechanism — the body of each step must still be pure (no side effects, no external mutation).

In Rust, these algorithms use `loop` or `while` with local mutability only. The public API remains pure: immutable parameters in, new values out.

In TypeScript, these are the only places where `let` is permitted in production code.

## Constraints
- The `// ADVANCE-EXCEPTION` comment must appear on the line above the `while`/`let`.
- The loop body must not mutate anything outside the loop's local scope.
- The public function signature must remain pure: no callbacks that mutate, no shared references.
- Each ADVANCE-EXCEPTION must be justified by data-dependent termination (not convenience).

## Consequences
- **Positive:** Bridson, Lloyd relaxation, and similar algorithms can terminate correctly without artificial bounds.
- **Positive:** The exception is visible and auditable — grep for `ADVANCE-EXCEPTION` to find every instance.
- **Negative:** Introduces a controlled deviation from the `const`-only rule.
- **Mitigation:** The pattern is scoped to iteration control only. The step function inside each loop is still pure.
