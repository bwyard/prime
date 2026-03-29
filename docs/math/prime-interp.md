# prime-interp -- Math Reference

Interpolation, easing, and smoothstep functions. All functions are pure:
$t$ in, value out. No mutation, no state.

---

## Basic interpolation

### lerp

Linear interpolation between two values.

$$\text{lerp}(a, b, t) = a + t \cdot (b - a) = a(1 - t) + bt$$

$t$ is **not** clamped -- values outside $[0, 1]$ extrapolate.

### lerp_clamped

Same as lerp with $t$ clamped to $[0, 1]$:

$$\text{lerp\_clamped}(a, b, t) = \text{lerp}(a, b, \text{clamp}(t, 0, 1))$$

### inv_lerp

Inverse of lerp -- finds the $t$ that produces value $v$ in range $[a, b]$:

$$\text{inv\_lerp}(a, b, v) = \frac{v - a}{b - a}$$

Returns $0$ when $a = b$ (degenerate case). Result is not clamped.

### remap

Maps a value from one range to another by composing inv_lerp and lerp:

$$\text{remap}(v, i_0, i_1, o_0, o_1) = \text{lerp}\!\left(o_0,\; o_1,\; \text{inv\_lerp}(i_0, i_1, v)\right)$$

---

## Wrapping

### repeat

Wraps $t$ into the range $[0, \text{length})$:

$$\text{repeat}(t, L) = t - \left\lfloor \frac{t}{L} \right\rfloor \cdot L$$

Works for negative $t$: `repeat(-0.3, 1.0) = 0.7`.

### pingpong

$t$ bounces back and forth between $0$ and $\text{length}$:

$$\text{pingpong}(t, L) = L - \left| \text{repeat}(t, 2L) - L \right|$$

---

## Smooth transitions

### smoothstep (Hermite S-curve)

Clamps $x$ to $[e_0, e_1]$, normalizes, then applies Hermite interpolation.
$C^1$ continuous (zero first derivative at both endpoints).

$$t = \text{clamp}\!\left(\frac{x - e_0}{e_1 - e_0},\; 0,\; 1\right)$$

$$S(t) = t^2(3 - 2t) = 3t^2 - 2t^3$$

Derivatives at endpoints: $S'(0) = 0$, $S'(1) = 0$.

### smootherstep (Perlin's C2 curve)

Ken Perlin's improved smoothstep with $C^2$ continuity -- zero first **and** second
derivative at both endpoints. Preferred for noise and terrain blending.

$$t = \text{clamp}\!\left(\frac{x - e_0}{e_1 - e_0},\; 0,\; 1\right)$$

$$S(t) = t^3(t(6t - 15) + 10) = 6t^5 - 15t^4 + 10t^3$$

Derivatives: $S'(0) = S'(1) = 0$, $S''(0) = S''(1) = 0$.

---

## Easing functions

All easing functions map $t \in [0, 1] \to [0, 1]$ with $f(0) = 0$, $f(1) = 1$.
Elastic and back easings may temporarily exceed $[0, 1]$.

Convention:
- **ease_in** -- slow start, fast end
- **ease_out** -- fast start, slow end
- **ease_in_out** -- slow at both ends, fast in the middle

### Polynomial easings

General pattern for degree $n$:

| Family | ease_in | ease_out | ease_in_out |
|--------|---------|----------|-------------|
| Quad ($n=2$) | $t^2$ | $1 - (1-t)^2$ | $t < 0.5$: $2t^2$, else $1 - \frac{(-2t+2)^2}{2}$ |
| Cubic ($n=3$) | $t^3$ | $1 - (1-t)^3$ | $t < 0.5$: $4t^3$, else $1 - \frac{(-2t+2)^3}{2}$ |
| Quart ($n=4$) | $t^4$ | $1 - (1-t)^4$ | $t < 0.5$: $8t^4$, else $1 - \frac{(-2t+2)^4}{2}$ |
| Quint ($n=5$) | $t^5$ | $1 - (1-t)^5$ | $t < 0.5$: $16t^5$, else $1 - \frac{(-2t+2)^5}{2}$ |

The ease_in_out form splits at $t = 0.5$ and applies $2^{n-1} \cdot t^n$ in the first half,
then the reflected ease_out in the second half.

### Sine

Based on quarter-period sine/cosine arcs:

$$\text{ease\_in\_sine}(t) = 1 - \cos\!\left(\frac{\pi t}{2}\right)$$

$$\text{ease\_out\_sine}(t) = \sin\!\left(\frac{\pi t}{2}\right)$$

$$\text{ease\_in\_out\_sine}(t) = -\frac{\cos(\pi t) - 1}{2}$$

### Exponential

Exponential growth/decay with base $2$:

$$\text{ease\_in\_expo}(t) = \begin{cases} 0 & t = 0 \\ 2^{10t - 10} & t > 0 \end{cases}$$

$$\text{ease\_out\_expo}(t) = \begin{cases} 1 & t = 1 \\ 1 - 2^{-10t} & t < 1 \end{cases}$$

$$\text{ease\_in\_out\_expo}(t) = \begin{cases} 0 & t = 0 \\ 1 & t = 1 \\ \frac{2^{20t-10}}{2} & t < 0.5 \\ \frac{2 - 2^{-20t+10}}{2} & t \geq 0.5 \end{cases}$$

### Circular

Based on quarter-circle arcs ($x^2 + y^2 = 1$):

$$\text{ease\_in\_circ}(t) = 1 - \sqrt{1 - t^2}$$

$$\text{ease\_out\_circ}(t) = \sqrt{1 - (t - 1)^2}$$

$$\text{ease\_in\_out\_circ}(t) = \begin{cases} \frac{1 - \sqrt{1 - (2t)^2}}{2} & t < 0.5 \\ \frac{\sqrt{1 - (-2t+2)^2} + 1}{2} & t \geq 0.5 \end{cases}$$

### Elastic (damped oscillation)

Combines exponential decay with sinusoidal oscillation. The period constant
$c_4 = \frac{2\pi}{3}$ produces approximately 3 oscillations.

$$\text{ease\_in\_elastic}(t) = \begin{cases} 0 & t = 0 \\ 1 & t = 1 \\ -2^{10t-10} \cdot \sin\!\left((10t - 10.75) \cdot \frac{2\pi}{3}\right) & \text{else} \end{cases}$$

$$\text{ease\_out\_elastic}(t) = \begin{cases} 0 & t = 0 \\ 1 & t = 1 \\ 2^{-10t} \cdot \sin\!\left((10t - 0.75) \cdot \frac{2\pi}{3}\right) + 1 & \text{else} \end{cases}$$

For ease_in_out, the period constant changes to $c_5 = \frac{2\pi}{4.5}$:

$$\text{ease\_in\_out\_elastic}(t) = \begin{cases} 0 & t = 0 \\ 1 & t = 1 \\ -\frac{2^{20t-10} \cdot \sin\!\left((20t - 11.125) \cdot c_5\right)}{2} & t < 0.5 \\ \frac{2^{-20t+10} \cdot \sin\!\left((20t - 11.125) \cdot c_5\right)}{2} + 1 & t \geq 0.5 \end{cases}$$

Values may exceed $[0, 1]$ -- the overshoot is intentional.

### Bounce (piecewise polynomial)

Simulates a bouncing ball with 4 progressively smaller bounces.
Constants: $n_1 = 7.5625$, $d_1 = 2.75$.

$$\text{ease\_out\_bounce}(t) = \begin{cases} n_1 \cdot t^2 & t < \frac{1}{d_1} \\ n_1(t - \frac{1.5}{d_1})^2 + 0.75 & t < \frac{2}{d_1} \\ n_1(t - \frac{2.25}{d_1})^2 + 0.9375 & t < \frac{2.5}{d_1} \\ n_1(t - \frac{2.625}{d_1})^2 + 0.984375 & \text{else} \end{cases}$$

Each segment is a parabola shifted to start at the previous bounce's peak.
The floor heights ($0.75$, $0.9375$, $0.984375$) are $1 - \frac{1}{4^k}$ for bounces $k = 1, 2, 3$.

$$\text{ease\_in\_bounce}(t) = 1 - \text{ease\_out\_bounce}(1 - t)$$

$$\text{ease\_in\_out\_bounce}(t) = \begin{cases} \frac{1 - \text{ease\_out\_bounce}(1 - 2t)}{2} & t < 0.5 \\ \frac{1 + \text{ease\_out\_bounce}(2t - 1)}{2} & t \geq 0.5 \end{cases}$$

### Back (overshoot)

Cubic with an overshoot parameter $s = 1.70158$ (approximately 10% overshoot past the target):

$$\text{ease\_in\_back}(t) = t^2 \left((s + 1)t - s\right)$$

$$\text{ease\_out\_back}(t) = (t-1)^2 \left((s+1)(t-1) + s\right) + 1$$

The constant $s = 1.70158$ is chosen so the overshoot peaks at exactly 10% past the
start/end value. At $t \approx 0.36$, ease_in_back reaches its minimum of $\approx -0.10$.
