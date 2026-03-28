# prime-diffusion — Math Reference

Stochastic processes for mean-reverting and multiplicative dynamics.
All functions are pure. Noise is either caller-supplied or generated deterministically
from a threaded `u32` seed via `prime_random::prng_gaussian`.

---

## Ornstein-Uhlenbeck Process

The O-U process is the canonical mean-reverting stochastic differential equation (SDE):

$$dX_t = \theta(\mu - X_t)\,dt + \sigma\,dW_t$$

where:
- $\mu$ = long-run equilibrium (mean)
- $\theta > 0$ = mean-reversion speed
- $\sigma > 0$ = volatility
- $W_t$ = standard Wiener process

### Euler-Maruyama Discretization

The continuous SDE is discretized via the Euler-Maruyama method:

$$X_{n+1} = X_n + \theta(\mu - X_n)\,\Delta t + \sigma\,\sqrt{\Delta t}\;\cdot w_n$$

where $w_n \sim \mathcal{N}(0, 1)$ is a standard normal sample.

This is the formula implemented by `ou_step`.

### Properties

- **Stationary distribution:** $X_\infty \sim \mathcal{N}\!\left(\mu,\; \frac{\sigma^2}{2\theta}\right)$
- **Autocorrelation:** $\text{Cov}(X_t, X_{t+s}) = \frac{\sigma^2}{2\theta}\,e^{-\theta|s|}$
- **Deterministic limit** ($\sigma = 0$): exponential decay $X(t) = \mu + (X_0 - \mu)\,e^{-\theta t}$

### Special Cases

| Parameters | Behavior |
|---|---|
| $\Delta t = 0$ | No change: $X' = X$ |
| $\theta = 0$ | No mean reversion; pure random walk: $X' = X + \sigma\sqrt{\Delta t}\,w$ |
| $\sigma = 0$ | Deterministic decay toward $\mu$: $X' = X + \theta(\mu - X)\Delta t$ |

---

## Geometric Brownian Motion

GBM models multiplicative processes where values remain strictly positive. The SDE is:

$$dX_t = \mu\,X_t\,dt + \sigma\,X_t\,dW_t$$

### Exact Solution

Unlike the O-U process, GBM has a closed-form solution for one time step. Applying Ito's lemma to $\ln X_t$:

$$X_{t+\Delta t} = X_t \cdot \exp\!\left[\left(\mu - \tfrac{\sigma^2}{2}\right)\Delta t + \sigma\,\sqrt{\Delta t}\;\cdot w\right]$$

The $-\sigma^2/2$ term is the Ito correction, which arises because $\mathbb{E}[e^{\sigma W}] \ne e^{\sigma \mathbb{E}[W]}$ for random variables.

This exact formula is implemented by `gbm_step` (no discretization error).

### Properties

- **Always positive:** if $X_0 > 0$, then $X_t > 0$ for all $t$ (the exponential never crosses zero).
- **Log-normal distribution:** $\ln X_t \sim \mathcal{N}\!\left(\ln X_0 + (\mu - \sigma^2/2)t,\;\; \sigma^2 t\right)$
- **Expected value:** $\mathbb{E}[X_t] = X_0\,e^{\mu t}$

### Special Cases

| Parameters | Behavior |
|---|---|
| $\Delta t = 0$ | No change: $X' = X$ |
| $\sigma = 0$ | Deterministic exponential growth: $X' = X \cdot e^{\mu\,\Delta t}$ |
| $\mu = 0, \sigma = 0$ | Identity: $X' = X$ |
| $X = 0$ | Absorbing state: $X' = 0$ |

---

## Seeded Variants and prime-random Integration

Both `ou_step_seeded` and `gbm_step_seeded` generate noise deterministically via:

```
(w, next_seed) = prime_random::prng_gaussian(seed)
```

This produces a standard normal sample $w \sim \mathcal{N}(0,1)$ from the Mulberry32 PRNG
using the Box-Muller transform, and threads the seed forward. The pattern follows
prime-random's pure state-threading convention: the seed IS the RNG state, and
`(value, next_seed)` is returned as a tuple.

Chaining steps:

$$(\,X_1,\; s_1\,) = \text{step\_seeded}(X_0,\; \ldots,\; s_0)$$
$$(\,X_2,\; s_2\,) = \text{step\_seeded}(X_1,\; \ldots,\; s_1)$$

No mutable state. Each call is a pure function of its inputs.
