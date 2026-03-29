# ADR-003: Four Temporal Operations (LOAD, COMPUTE, APPEND, ADVANCE)

**Status:** Accepted
**Date:** 2026-03-17

## Context
Traditional programming uses six operations: READ, WRITE, COMPUTE, BRANCH, LOOP, DELETE. WRITE (mutation) and DELETE introduce non-determinism. BRANCH (exceptions/goto) creates hidden control flow.

## Decision
PRIME uses only four operations:
- **LOAD** — read inputs (function parameters)
- **COMPUTE** — pure math (function body)
- **APPEND** — write new fact (return new state as tuple)
- **ADVANCE** — move forward in time (fold/reduce over steps)

STORE (mutation), JUMP (exceptions), and DELETE are excluded by design.

## Consequences
- **Positive:** Every intermediate state is a value that can be inspected, logged, serialized.
- **Positive:** Time is explicit. Each ADVANCE step takes previous state and returns next state.
- **Positive:** No hidden control flow. No exceptions, no goto, no early returns from mutation.
- **Negative:** Algorithms with data-dependent termination (Bridson) need the ADVANCE-EXCEPTION pattern.
- **Mitigation:** The `successors` utility encapsulates the only `let`/`while` in the codebase.
