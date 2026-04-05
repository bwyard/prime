# prime-diffusion

Stochastic diffusion processes — Ornstein-Uhlenbeck and Geometric Brownian Motion, with seeded and unseeded variants.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `ou_step` / `ou_step_seeded` — Ornstein-Uhlenbeck mean-reverting process
- `gbm_step` / `gbm_step_seeded` — Geometric Brownian Motion

## Usage

```rust
use prime_diffusion::{ou_step_seeded, gbm_step_seeded};

// Ornstein-Uhlenbeck — mean-reverting random walk (useful for AI behavior, price simulation)
// Returns (next_value, next_seed)
let (x_next, seed_next) = ou_step_seeded(x, mu, theta, sigma, dt, seed);

// Geometric Brownian Motion — multiplicative random walk (asset prices, population growth)
let (x_next, seed_next) = gbm_step_seeded(x, mu, sigma, dt, seed);
```

## Design

Seeded variants use `prime-random` to thread noise deterministically. Unseeded variants accept a pre-sampled Wiener increment `w` for integration with external noise sources.

## License

MIT
