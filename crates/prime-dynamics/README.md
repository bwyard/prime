# prime-dynamics

Dynamical systems — ODE solvers (RK4, Euler), chaotic attractors (Lorenz, Rössler, Duffing), population models, L-systems, and numerical differentiation.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

**ODE solvers:** `rk4_step` / `rk4_step3` / `euler_step`

**Chaotic systems:** `lorenz_step` / `rossler_step` / `duffing_step`

**Population models:** `lotka_volterra_step` / `sir_step` (predator-prey, epidemic)

**Reaction-diffusion:** `gray_scott_step`

**L-systems:** `lsystem_step` / `lsystem_generate` with `LRule`

**Numerical math:** `derivative` / `derivative2` / `gradient_2d`

**Logistic map:** `logistic`

## Usage

```rust
use prime_dynamics::{rk4_step, lorenz_step, lsystem_generate, LRule};

// Integrate dy/dt = -y using RK4
let y_next = rk4_step(y, t, dt, |y, _t| -y);

// Lorenz attractor — returns (dx, dy, dz)
let (dx, dy, dz) = lorenz_step(x, y, z, sigma, rho, beta, dt);

// L-system plant growth
let rules = vec![LRule { from: 'F', to: "FF+[+F-F-F]-[-F+F+F]".to_string() }];
let result = lsystem_generate("F", &rules, 3);
```

## License

MIT
