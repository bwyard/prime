# prime-osc -- Math Reference

Oscillators and envelopes. All functions are pure: phase/state in, value + new state out.
No mutation, no side effects.

---

## LFO waveforms

All LFO functions take a normalised phase $\phi \in [0, 1)$ mapping to one full cycle.
Phase wraps automatically via `frac()`. Output range is $[-1, 1]$.

### lfo_sine

$$y = \sin(\phi \cdot 2\pi)$$

Standard sine wave. Zero at $\phi = 0$, peak $+1$ at $\phi = 0.25$,
zero at $\phi = 0.5$, trough $-1$ at $\phi = 0.75$.

### lfo_cosine

$$y = \cos(\phi \cdot 2\pi)$$

Cosine wave. Peak $+1$ at $\phi = 0$, zero at $\phi = 0.25$,
trough $-1$ at $\phi = 0.5$, zero at $\phi = 0.75$.

This is $\text{lfo\_sine}$ phase-shifted by $0.25$.

### lfo_triangle

Phase-aligned with sine (peak at $\phi = 0.25$):

$$p = \text{frac}(\phi + 0.75)$$

$$y = 2|2p - 1| - 1$$

Piecewise linear. Same zero-crossings and peak/trough locations as sine,
but with constant slope segments instead of curved transitions.

### lfo_sawtooth

Rising sawtooth:

$$y = 2 \cdot \text{frac}(\phi) - 1$$

Rises linearly from $-1$ to $+1$ over one cycle, then resets.
Output range: $[-1, 1)$.

### lfo_square

Square wave with adjustable duty cycle:

$$y = \begin{cases} +1 & \text{frac}(\phi) < w \\ -1 & \text{frac}(\phi) \geq w \end{cases}$$

where $w$ is the duty cycle width, clamped to $(0.001, 0.999)$.

- $w = 0.5$ -- symmetric 50% square wave
- $w = 0.1$ -- narrow 10% pulse (high for 10% of cycle)
- $w = 0.9$ -- wide 90% pulse

---

## Oscillator step

`osc_step` advances an oscillator by one sample and evaluates any shape function:

$$\phi' = \text{frac}\!\left(\phi + \frac{f}{f_s}\right)$$

$$y = \text{shape}(\phi')$$

where $f$ is frequency in Hz and $f_s$ is sample rate in Hz.

Returns $(y, \phi')$ -- thread $\phi'$ into the next call. The shape parameter
accepts any of the LFO functions (`lfo_sine`, `lfo_triangle`, etc.).

---

## ADSR envelope

Attack-Decay-Sustain-Release envelope controlled by a gate signal.

### Parameters

| Parameter | Description |
|-----------|-------------|
| `attack` | Time from $0$ to peak ($1.0$) in seconds |
| `decay` | Time from peak to sustain level in seconds |
| `sustain` | Held level $\in [0, 1]$ while gate is on |
| `release` | Time from sustain to $0$ after gate off, in seconds |

All times are clamped to a minimum of $10^{-4}$ to avoid division by zero.

### State machine

The envelope progresses through 5 stages:

```
Gate ON                              Gate OFF
  |                                    |
  v                                    v
DONE --> ATTACK --> DECAY --> SUSTAIN --> RELEASE --> DONE
  0        /\        \___      ___        \           0
          /  \           \    /            \
         /    \           \/                \
```

### Level calculations per stage

**Attack** (gate on):

$$v = \min\!\left(\frac{t_{\text{elapsed}}}{t_{\text{attack}}},\; 1\right)$$

Linear ramp from $0$ to $1$. Transitions to Decay when $t_{\text{elapsed}} \geq t_{\text{attack}}$.

**Decay** (gate on):

$$v = 1 + (s - 1) \cdot \min\!\left(\frac{t_{\text{elapsed}}}{t_{\text{decay}}},\; 1\right)$$

Linear interpolation from $1$ down to sustain level $s$.
Equivalent to $\text{lerp}(1, s, t/t_{\text{decay}})$.

**Sustain** (gate on):

$$v = s$$

Holds at sustain level indefinitely while gate remains on.

**Release** (gate off):

$$v = v_0 \cdot \left(1 - \min\!\left(\frac{t_{\text{elapsed}}}{t_{\text{release}}},\; 1\right)\right)$$

where $v_0$ is the envelope value at the moment the gate went off.
Linear decay from $v_0$ to $0$. Transitions to Done when $t_{\text{elapsed}} \geq t_{\text{release}}$.

**Done**:

$$v = 0$$

### Retriggering

If the gate turns on during Release or Done, the envelope restarts from Attack.
If the gate turns off during Attack or Decay (before reaching Sustain),
the envelope immediately transitions to Release from the current level.

### Pure state threading

```
(value, new_state) = adsr_step(state, params, gate, dt)
```

Thread `new_state` forward. `AdsrState` contains: stage, current value, elapsed time
in current stage. `AdsrState::IDLE` is the initial rest state (Done, value 0).
