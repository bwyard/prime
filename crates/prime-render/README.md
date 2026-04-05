# prime-render

Pure sample-level audio scan loop — fold a stateless step function over N audio samples. The proof that a song is a pure function of time.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `render` — fold a mono step function over a sample buffer
- `render_stereo` — fold a stereo step function over an interleaved buffer
- `render_fold` — fold with an accumulator (collect events, gather analysis data)

## Usage

```rust
use prime_render::render;

// Step function: (state, sample_index) -> (sample, next_state)
let step = |state: MyState, i: usize| -> (f32, MyState) {
    let phase = state.phase + state.freq / sample_rate;
    let sample = (phase * std::f32::consts::TAU).sin();
    (sample, MyState { phase, ..state })
};

let buffer = render(initial_state, num_samples, step);
```

## Design

No audio thread, no callbacks, no mutation. The render loop is a pure fold — same initial state always produces the same buffer. Useful for offline rendering, testing, and proving that audio synthesis is deterministic.

## License

MIT
