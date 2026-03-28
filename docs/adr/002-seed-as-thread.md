# ADR-002: Seed as Thread Position

**Status:** Accepted
**Date:** 2026-03-17

## Context
Traditional RNG libraries use mutable state: `rng.next()` modifies internal state. This hides control flow, prevents replay, and breaks determinism in concurrent contexts.

## Decision
The seed IS the RNG state. `prng_next(seed) -> (value, next_seed)` threads state forward through return values. The caller controls advancement. No classes, no mutation, no write head.

## Consequences
- **Positive:** Determinism is free. Save a seed, replay from that point.
- **Positive:** Consent model. Only the holder of `next_seed` can advance the sequence.
- **Positive:** Fork/join. Pass the same seed to two functions for correlated sequences, or different seeds for independent ones.
- **Positive:** No DELETE needed. Stop threading the seed = sequence is inert.
- **Negative:** Callers must explicitly thread seeds (vs implicit `rng.next()`).
- **Mitigation:** The fold pattern handles threading naturally: `(0..n).fold((init, seed), |state, _| step(state))`
