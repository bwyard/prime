/**
 * prime-signal — Signal processing for game feel and real-time systems.
 */

/**
 * Smoothly damps a value toward a target. No overshoot.
 *
 * @remarks
 * Critically damped spring approximation:
 *   omega = 2 / smoothTime
 *   k = 1 + omega·dt + 0.48·omega²·dt² + 0.235·omega³·dt³
 *   change = current − target
 *   temp = (velocity + omega·change)·dt
 *   newVelocity = (velocity − omega·temp) / k
 *   newValue = target + (change + temp) / k
 *
 * @param current - Current value
 * @param target - Target to approach
 * @param velocity - Velocity from the previous frame
 * @param smoothTime - Approx time to reach target (seconds)
 * @param dt - Delta time (seconds)
 * @returns `[newValue, newVelocity]` — store both for the next frame
 *
 * @example
 * // Accumulate over frames with reduce:
 * const [pos] = Array.from({ length: 60 }).reduce(
 *   ([p, v]: [number, number]) => smoothdamp(p, 10, v, 0.3, 0.016),
 *   [0, 0] as [number, number]
 * )
 */
export const smoothdamp = (
  current: number,
  target: number,
  velocity: number,
  smoothTime: number,
  dt: number,
): [number, number] => {
  if (dt <= 0) return [current, velocity]
  const st = Math.max(smoothTime, 1e-4)
  const omega = 2 / st
  const x = omega * dt
  const k = 1 + x + 0.48 * x * x + 0.235 * x * x * x
  const change = current - target
  const temp = (velocity + omega * change) * dt
  const newVel = (velocity - omega * temp) / k
  const newVal = target + (change + temp) / k
  return [newVal, newVel]
}

/**
 * Spring simulation — Hooke's law with damping. Allows overshoot.
 *
 * @remarks
 * Symplectic Euler:
 *   force = −stiffness × (position − target) − damping × velocity
 *   velocity += force × dt
 *   position += velocity × dt
 *
 * Critical damping: damping = 2 × √stiffness (mass = 1).
 * Lower damping → oscillation. Higher → overdamped (slow).
 *
 * @param position - Current position
 * @param velocity - Current velocity
 * @param target - Target position
 * @param stiffness - Spring constant. Typical: 50–500.
 * @param damping - Damping. Critical: 2×√stiffness.
 * @param dt - Delta time (seconds)
 * @returns `[newPosition, newVelocity]` — store both for the next frame
 *
 * @example
 * const [pos] = Array.from({ length: 60 }).reduce(
 *   ([p, v]: [number, number]) => spring(p, v, 10, 100, 20, 0.016),
 *   [0, 0] as [number, number]
 * )
 */
export const spring = (
  position: number,
  velocity: number,
  target: number,
  stiffness: number,
  damping: number,
  dt: number,
): [number, number] => {
  if (dt <= 0) return [position, velocity]
  const force = -stiffness * (position - target) - damping * velocity
  const newVel = velocity + force * dt
  const newPos = position + newVel * dt
  return [newPos, newVel]
}

/**
 * Exponential low-pass filter — smooths noisy input.
 *
 * @remarks
 * One-pole IIR:
 *   alpha = 1 − e^(−dt / timeConstant)
 *   output = previous + alpha × (input − previous)
 *
 * Higher timeConstant → more smoothing (slower response).
 *
 * @param previous - Last filtered value (store between calls)
 * @param input - New raw input
 * @param timeConstant - RC time constant (seconds)
 * @param dt - Delta time (seconds)
 * @returns New filtered value
 *
 * @example
 * const filtered = Array.from({ length: 60 }).reduce(
 *   (prev: number) => lowPass(prev, 1.0, 0.1, 0.016),
 *   0
 * )
 */
export const lowPass = (
  previous: number,
  input: number,
  timeConstant: number,
  dt: number,
): number => {
  if (dt <= 0) return previous
  const tc = Math.max(timeConstant, 1e-6)
  const alpha = 1 - Math.exp(-dt / tc)
  return previous + alpha * (input - previous)
}

/**
 * Exponential high-pass filter — isolates fast changes, removes DC offset.
 *
 * @remarks
 * HP = input − LP(input)
 * Passes rapid changes, blocks slow drift.
 *
 * @param previousLp - Last low-pass state (from previous frame)
 * @param input - New raw input
 * @param timeConstant - Cutoff time constant (seconds)
 * @param dt - Delta time (seconds)
 * @returns `[output, newLpState]` — store `newLpState` for the next frame
 *
 * @example
 * const [fast] = Array.from({ length: 60 }).reduce(
 *   ([, lp]: [number, number]) => highPass(lp, rawInput, 0.05, 0.016),
 *   [0, 0] as [number, number],
 * )
 */
export const highPass = (
  previousLp: number,
  input: number,
  timeConstant: number,
  dt: number,
): [number, number] => {
  const newLp = lowPass(previousLp, input, timeConstant, dt)
  return [input - newLp, newLp]
}

/**
 * Apply deadzone and response curve to a raw axis value.
 *
 * @remarks
 * Given raw value r in [−1, 1]:
 *   if |r| < deadzone → 0
 *   else → sign × ((|r| − deadzone) / (1 − deadzone))^curve
 *
 * curve=1 → linear, curve=2 → quadratic, curve=0.5 → sqrt
 *
 * @param value - Raw axis in [−1, 1]
 * @param dz - Deadzone threshold in [0, 1)
 * @param curve - Response curve exponent (default 1.0)
 * @returns Processed value in [−1, 1]
 *
 * @example
 * deadzone(0.05, 0.1)  // 0 — inside deadzone
 * deadzone(1.0, 0.1)   // 1 — full deflection
 */
export const deadzone = (value: number, dz: number, curve = 1.0): number => {
  const v = Math.max(-1, Math.min(1, value))
  const d = Math.max(0, Math.min(0.9999, dz))
  const c = Math.max(0.01, curve)
  const abs = Math.abs(v)
  if (abs < d) return 0
  const t = (abs - d) / (1 - d)
  return Math.sign(v) * Math.pow(t, c)
}
