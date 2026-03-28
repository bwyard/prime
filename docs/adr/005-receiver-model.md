# ADR-005: Receiver Model (Context Primitive)

**Status:** Accepted
**Date:** 2026-03-28

## Context

Shannon's information theory measures bits but brackets meaning. The same data has different significance to different receivers. A 32-bit seed can be a uniform float, a Gaussian sample, a boolean, or a spatial coordinate — depending on which function reads it.

The thesis extends Shannon: `I(data, receiver)` rather than `I(data)`. Information is relational.

## Decision

Every function that extracts a typed value from a seed is documented as a **receiver** — a context function over the causal datum. The receiver table in `prime-random`'s module docs enumerates all interpretations of the u32 seed.

No formal `Receiver<T>` trait is introduced. The functions themselves ARE the receivers. The documentation makes the pattern explicit without adding abstraction overhead.

## Consequences

- **Positive:** The context primitive is visible in documentation without code overhead.
- **Positive:** New receivers (future distributions) are documented by convention.
- **Positive:** Cross-language tests verify that Rust and TypeScript receivers extract identical information.
- **Trade-off:** No compile-time enforcement that new functions are registered — relies on code review.
