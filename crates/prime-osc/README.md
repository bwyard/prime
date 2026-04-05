# prime-osc

Oscillators and envelopes — LFO waveforms, stateless phase stepping, and ADSR envelope simulation.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `lfo_sine` / `lfo_cosine` / `lfo_triangle` / `lfo_sawtooth` / `lfo_square` — LFO shapes from a phase value
- `osc_step` — advance phase by one sample, returns `(sample, next_phase)`
- `adsr_step` — ADSR envelope simulation, returns `(level, next_state)`
- `AdsrParams` / `AdsrState` / `AdsrStage` — envelope state types

## Usage

```rust
use prime_osc::{osc_step, lfo_sine, adsr_step, AdsrParams, AdsrState};

// LFO — advance phase and sample
let (sample, next_phase) = osc_step(phase, freq, sample_rate, lfo_sine);

// ADSR envelope — thread state forward per sample
let params = AdsrParams { attack: 0.01, decay: 0.1, sustain: 0.7, release: 0.3 };
let (level, next_state) = adsr_step(state, &params, gate, dt);
```

## Design

Phase and envelope state are explicit return values — no mutation, no hidden counters.

## License

MIT
