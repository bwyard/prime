# ADR-004: Cross-Language Determinism

**Status:** Accepted
**Date:** 2026-03-17

## Context
Game math must produce identical results across platforms. Floating-point behavior differs between languages and compilers.

## Decision
Every function implemented in both Rust and TypeScript must produce bit-identical results for the same inputs. Rust is the reference implementation. TypeScript tests verify against Rust-computed values.

The PRNG (Mulberry32) uses identical bit manipulation in both languages:
- Rust: `wrapping_add`, `wrapping_mul`
- TypeScript: `>>> 0`, `Math.imul()`

## Consequences
- **Positive:** Save state in Rust, load in TypeScript (or vice versa). Same seed = same world.
- **Positive:** Proves the math is language-agnostic, not compiler-dependent.
- **Negative:** Must avoid language-specific float optimizations (fused multiply-add, etc).
- **Negative:** f32 precision limits cross-language matching to ~5 decimal places.
