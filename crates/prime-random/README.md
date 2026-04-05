# prime-random

Seeded, deterministic randomness. Mulberry32 PRNG, probability distributions, geometric sampling, and quasi-random sequences. No hidden state — the seed is the RNG.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `prng_next` / `prng_range_f32` / `prng_bool` — core Mulberry32 PRNG
- `prng_gaussian` / `prng_exponential` — probability distributions
- `prng_disk_uniform` / `prng_annulus_uniform` — geometric sampling
- `prng_shuffled` / `prng_choose` / `weighted_choice` — collection sampling
- `van_der_corput` / Halton sequences — quasi-random low-discrepancy
- `CausalStep<T>` — builder-style API for threaded state

## Usage

```rust
use prime_random::{prng_next, prng_gaussian, prng_shuffled};

// Thread state forward explicitly — same seed always produces the same sequence
let (value, next_seed) = prng_next(42);
let (value2, next_seed2) = prng_next(next_seed);

// Gaussian sample
let (sample, next_seed) = prng_gaussian(42);

// Shuffle a slice without mutating it
let items = vec![1, 2, 3, 4, 5];
let (shuffled, _) = prng_shuffled(42, &items);
```

## Design

State is threaded explicitly: every function takes a seed and returns `(value, next_seed)`. No struct, no mutable write head, no global state. Deterministic across platforms.

## License

MIT
