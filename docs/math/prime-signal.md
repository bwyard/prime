# prime-signal -- Math Reference

Signal processing for game feel and real-time systems. All functions are pure:
state in, new state out. No mutation.

---

## smoothdamp -- Critically damped spring

Smoothly approaches a target with no overshoot and zero arrival velocity.
The standard "camera follow" function.

### Derivation

A critically damped spring obeys the 2nd-order ODE:

$$\ddot{x} + 2\omega\dot{x} + \omega^2 x = 0$$

where $\omega = \frac{2}{\tau}$ and $\tau$ is the smooth time (approximate time to reach target).
The exact solution is $x(t) = (A + Bt)e^{-\omega t}$, but evaluating the exponential
per frame is expensive. Instead we use a polynomial approximation of the decay factor.

### Implementation

The denominator $k$ approximates $e^{\omega \cdot dt}$ as a 3rd-order polynomial:

$$k = 1 + x + 0.48x^2 + 0.235x^3 \quad \text{where } x = \omega \cdot dt$$

This matches $e^x$ to within 0.1% for typical $dt < 0.1$s.

Update equations:

$$\omega = \frac{2}{\tau}$$

$$k = 1 + \omega\,dt + 0.48\,(\omega\,dt)^2 + 0.235\,(\omega\,dt)^3$$

$$\Delta = x_{\text{current}} - x_{\text{target}}$$

$$\text{temp} = (v + \omega \cdot \Delta) \cdot dt$$

$$v' = \frac{v - \omega \cdot \text{temp}}{k}$$

$$x' = x_{\text{target}} + \frac{\Delta + \text{temp}}{k}$$

Returns $(x', v')$. Both values thread forward to the next frame.

### Edge cases

- $\tau \leq 0$ clamped to $10^{-4}$ to avoid division by zero
- $dt = 0$ returns $(x_{\text{current}}, v)$ unchanged

### Vec2 / Vec3 variants

`smoothdamp_vec2` and `smoothdamp_vec3` apply the scalar smoothdamp independently
to each component.

---

## spring -- Hooke's law with damping

Damped harmonic oscillator. Unlike smoothdamp, this **allows overshoot** --
use for bouncy, physical-feeling motion.

### Math

The equation of motion for a damped spring (mass = 1):

$$\ddot{x} = -k_s(x - x_t) - c\,\dot{x}$$

where $k_s$ is stiffness and $c$ is the damping coefficient.

Integrated via symplectic Euler (preserves energy better than explicit Euler):

$$F = -k_s \cdot (x - x_{\text{target}}) - c \cdot v$$

$$v' = v + F \cdot dt$$

$$x' = x + v' \cdot dt$$

Note: velocity is updated **first**, then position uses the **new** velocity.
This is the symplectic Euler property that prevents energy drift.

### Damping regimes

For unit mass, the critical damping coefficient is:

$$c_{\text{crit}} = 2\sqrt{k_s}$$

| Regime | Condition | Behavior |
|--------|-----------|----------|
| Underdamped | $c < 2\sqrt{k_s}$ | Oscillates around target, decaying |
| Critically damped | $c = 2\sqrt{k_s}$ | Fastest non-oscillating convergence |
| Overdamped | $c > 2\sqrt{k_s}$ | Slow exponential approach, no oscillation |

Typical values: $k_s \in [50, 500]$, $c = 2\sqrt{k_s}$ for critical damping.

### Vec2 / Vec3 variants

`spring_vec2` and `spring_vec3` apply the scalar spring independently
to each component.

---

## low_pass -- Exponential moving average

One-pole IIR low-pass filter. Smooths noisy or rapidly changing input.

### Math

$$\alpha = 1 - e^{-dt / \tau_c}$$

$$y = y_{\text{prev}} + \alpha \cdot (x - y_{\text{prev}})$$

This is equivalent to $\text{lerp}(y_{\text{prev}}, x, \alpha)$.

### Time constant $\tau_c$

$\tau_c$ is the RC time constant: time for the output to reach $\approx 63.2\%$ ($1 - 1/e$)
of a step input.

| $\alpha$ | Behavior |
|----------|----------|
| $\to 0$ | Heavy smoothing, slow response |
| $\to 1$ | No smoothing, instant response |

The exponential form $1 - e^{-dt/\tau_c}$ makes the filter frame-rate independent:
doubling $dt$ produces the same result as two half-steps.

### Edge cases

- $\tau_c \leq 0$ clamped to $10^{-6}$
- $dt = 0$ returns $y_{\text{prev}}$ unchanged

---

## high_pass -- Complement of low_pass

Isolates fast changes and removes slow drift (DC offset).

### Math

$$y_{\text{lp}} = \text{low\_pass}(y_{\text{prev\_lp}}, x, \tau_c, dt)$$

$$y_{\text{hp}} = x - y_{\text{lp}}$$

Returns $(y_{\text{hp}}, y_{\text{lp}})$ -- thread $y_{\text{lp}}$ forward as state.

A constant (DC) input produces $y_{\text{hp}} \to 0$ as $y_{\text{lp}}$ converges to the input.
A sudden step produces a spike that decays with time constant $\tau_c$.

Use cases: detecting sudden inputs, removing gravity from accelerometer data,
isolating transients.

---

## deadzone -- Input remapping with response curve

Applies a deadzone threshold and power-curve shaping to a raw axis value (e.g. gamepad stick).

### Math

Given raw value $r \in [-1, 1]$:

$$\text{deadzone}(r, d, \gamma) = \begin{cases} 0 & |r| < d \\ \text{sign}(r) \cdot t^\gamma & |r| \geq d \end{cases}$$

where:

$$t = \frac{|r| - d}{1 - d}$$

The remap $t$ maps the live zone $[d, 1]$ to $[0, 1]$, so there is no discontinuous
jump at the deadzone boundary. The power curve $\gamma$ shapes the response:

| $\gamma$ | Response |
|-----------|----------|
| $1.0$ | Linear after deadzone |
| $2.0$ | Quadratic -- slow near deadzone, fast at max deflection |
| $0.5$ | Square root -- fast near deadzone, leveling off at max |

### Edge cases

- $|r| > 1$ clamped to $[-1, 1]$
- $d \geq 1$ always returns $0$
- $\gamma \leq 0$ clamped to $0.01$
