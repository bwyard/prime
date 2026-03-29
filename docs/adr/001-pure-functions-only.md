# ADR-001: Pure Functions Only

**Status:** Accepted
**Date:** 2026-03-17

## Context
PRIME is the math foundation for SCORE, FORM, and STAGE. Game math libraries typically use mutable structs and `&mut self` methods. This creates hidden state coupling, non-determinism, and prevents safe parallelism.

## Decision
All public functions in PRIME are pure: LOAD inputs, COMPUTE result, return new state. No `&mut` in any public signature. No side effects. No hidden state.

## Consequences
- **Positive:** Deterministic by construction. Same inputs = same outputs, always.
- **Positive:** Thread-safe without locks. No shared mutable state.
- **Positive:** Testable with a single function call. No setup, no teardown.
- **Positive:** Cross-language portable. Pure math works identically in Rust and TypeScript.
- **Negative:** Spatial algorithms (Bridson) pay ~4x overhead from state reconstruction.
- **Negative:** Cannot use in-place mutation for performance-critical inner loops.
- **Mitigation:** The 4x overhead is constant-factor and sub-frame. Persistent data structures could close the gap without changing the API.
