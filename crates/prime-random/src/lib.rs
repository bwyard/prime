//! prime-random — Seeded randomness — PCG64 RNG, Poisson disk, weighted choice.
//!
//! # Planned modules
//! - `rng` — PrimeRng: seeded PCG64 deterministic RNG (same seed → same sequence)
//! - `distribution` — Weighted choice, Poisson disk sampling
//! - `shuffle` — Fisher-Yates shuffle
//!
//! # Critical rule
//! NEVER use rand::random(), Math.random(), or any non-seeded random anywhere
//! in the PRIME/FORM/STAGE codebase. All randomness flows through PrimeRng.

// TODO: implement
