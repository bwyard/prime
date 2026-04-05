# prime-signal

Signal processing for game feel and animation. Smoothdamp, springs, low/high-pass filters, and deadzone shaping — all pure functions that return new state.

Part of the [prime](https://github.com/bwyard/prime) math ecosystem.

## What's inside

- `smoothdamp` / `smoothdamp_vec2` / `smoothdamp_vec3` — Unity-style smooth follow
- `spring` / `spring_vec2` / `spring_vec3` — spring-damper simulation
- `low_pass` / `high_pass` — first-order filters
- `deadzone` — input deadzone with curve shaping

## Usage

```rust
use prime_signal::{smoothdamp, spring};

// Smooth-follow a target value — returns (new_pos, new_velocity)
let (pos, vel) = smoothdamp(current_pos, target, velocity, smooth_time, dt);

// Spring physics — returns (new_pos, new_velocity)
let (pos, vel) = spring(pos, vel, target, stiffness, damping, dt);
```

## Design

No `&mut` in public signatures. Every function takes current state and returns the next state as a tuple. Thread state forward explicitly between frames.

## License

MIT
