/**
 * prime-splines — Curve interpolation: Bezier, Hermite, Catmull-Rom, B-spline, slerp.
 *
 * All exported functions are pure (LOAD + COMPUTE only). No mutation, no side effects,
 * no hidden state. Same inputs always produce the same output.
 */

// ── Quadratic Bezier ──────────────────────────────────────────────────────────

/**
 * Quadratic Bezier interpolation (3 control points).
 *
 * Math: `B(t) = (1-t)²·p0 + 2(1-t)t·p1 + t²·p2`
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - start point
 * @param p1 - control point
 * @param p2 - end point
 * @returns interpolated value
 *
 * @example
 * bezierQuadratic(0, 0, 1, 0) // 0
 * bezierQuadratic(1, 0, 1, 0) // 0
 * bezierQuadratic(0.5, 0, 1, 0) // 0.5
 */
export const bezierQuadratic = (t: number, p0: number, p1: number, p2: number): number => {
  const u = 1 - t
  return u * u * p0 + 2 * u * t * p1 + t * t * p2
}

/**
 * Quadratic Bezier interpolation on `[x, y, z]` tuples.
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - start `[x, y, z]`
 * @param p1 - control `[x, y, z]`
 * @param p2 - end `[x, y, z]`
 * @returns interpolated `[x, y, z]`
 */
export const bezierQuadratic3d = (
  t: number,
  p0: [number, number, number],
  p1: [number, number, number],
  p2: [number, number, number],
): [number, number, number] => [
  bezierQuadratic(t, p0[0], p1[0], p2[0]),
  bezierQuadratic(t, p0[1], p1[1], p2[1]),
  bezierQuadratic(t, p0[2], p1[2], p2[2]),
]

// ── Cubic Bezier ──────────────────────────────────────────────────────────────

/**
 * Cubic Bezier interpolation (4 control points).
 *
 * Math: `B(t) = (1-t)³·p0 + 3(1-t)²t·p1 + 3(1-t)t²·p2 + t³·p3`
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - start point
 * @param p1 - first control point
 * @param p2 - second control point
 * @param p3 - end point
 * @returns interpolated value
 *
 * @example
 * bezierCubic(0, 0, 1, 2, 3) // 0
 * bezierCubic(1, 0, 1, 2, 3) // 3
 */
export const bezierCubic = (
  t: number,
  p0: number,
  p1: number,
  p2: number,
  p3: number,
): number => {
  const u = 1 - t
  return u * u * u * p0 + 3 * u * u * t * p1 + 3 * u * t * t * p2 + t * t * t * p3
}

/**
 * Cubic Bezier interpolation on `[x, y, z]` tuples.
 */
export const bezierCubic3d = (
  t: number,
  p0: [number, number, number],
  p1: [number, number, number],
  p2: [number, number, number],
  p3: [number, number, number],
): [number, number, number] => [
  bezierCubic(t, p0[0], p1[0], p2[0], p3[0]),
  bezierCubic(t, p0[1], p1[1], p2[1], p3[1]),
  bezierCubic(t, p0[2], p1[2], p2[2], p3[2]),
]

// ── Hermite cubic ─────────────────────────────────────────────────────────────

/**
 * Cubic Hermite interpolation: endpoints and tangents.
 *
 * Math:
 * ```
 * h(t) = (2t³-3t²+1)·p0 + (t³-2t²+t)·m0 + (-2t³+3t²)·p1 + (t³-t²)·m1
 * ```
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - value at t=0
 * @param m0 - tangent at t=0
 * @param p1 - value at t=1
 * @param m1 - tangent at t=1
 * @returns interpolated value
 *
 * @example
 * hermite(0, 0, 1, 1, 1) // 0
 * hermite(1, 0, 1, 1, 1) // 1
 * hermite(0.5, 0, 1, 1, 1) // 0.5 (straight line)
 */
export const hermite = (
  t: number,
  p0: number,
  m0: number,
  p1: number,
  m1: number,
): number => {
  const t2 = t * t
  const t3 = t2 * t
  const h00 = 2 * t3 - 3 * t2 + 1
  const h10 = t3 - 2 * t2 + t
  const h01 = -2 * t3 + 3 * t2
  const h11 = t3 - t2
  return h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
}

/**
 * Cubic Hermite interpolation on `[x, y, z]` tuples.
 */
export const hermite3d = (
  t: number,
  p0: [number, number, number],
  m0: [number, number, number],
  p1: [number, number, number],
  m1: [number, number, number],
): [number, number, number] => [
  hermite(t, p0[0], m0[0], p1[0], m1[0]),
  hermite(t, p0[1], m0[1], p1[1], m1[1]),
  hermite(t, p0[2], m0[2], p1[2], m1[2]),
]

// ── Catmull-Rom ───────────────────────────────────────────────────────────────

/**
 * Uniform Catmull-Rom spline segment.
 *
 * Interpolates between `p1` (t=0) and `p2` (t=1) using `p0` and `p3` as
 * phantom neighbours to compute tangents. Passes through `p1` and `p2`.
 *
 * Math:
 * ```
 * result = 0.5 * (2p1 + (-p0+p2)t + (2p0-5p1+4p2-p3)t² + (-p0+3p1-3p2+p3)t³)
 * ```
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - previous control point
 * @param p1 - start point (curve passes through here at t=0)
 * @param p2 - end point (curve passes through here at t=1)
 * @param p3 - next control point
 * @returns interpolated value
 *
 * @example
 * catmullRom(0, 0, 1, 2, 3) // 1
 * catmullRom(1, 0, 1, 2, 3) // 2
 * catmullRom(0.5, 0, 1, 2, 3) // 1.5 (linear uniform spacing)
 */
export const catmullRom = (
  t: number,
  p0: number,
  p1: number,
  p2: number,
  p3: number,
): number => {
  const t2 = t * t
  const t3 = t2 * t
  return 0.5 * (
    2 * p1
    + (-p0 + p2) * t
    + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2
    + (-p0 + 3 * p1 - 3 * p2 + p3) * t3
  )
}

/**
 * Catmull-Rom spline segment on `[x, y, z]` tuples.
 */
export const catmullRom3d = (
  t: number,
  p0: [number, number, number],
  p1: [number, number, number],
  p2: [number, number, number],
  p3: [number, number, number],
): [number, number, number] => [
  catmullRom(t, p0[0], p1[0], p2[0], p3[0]),
  catmullRom(t, p0[1], p1[1], p2[1], p3[1]),
  catmullRom(t, p0[2], p1[2], p2[2], p3[2]),
]

// ── Uniform cubic B-spline ────────────────────────────────────────────────────

/**
 * Uniform cubic B-spline segment.
 *
 * Does NOT pass through control points (unlike Catmull-Rom). Contained within
 * the convex hull of the four control points.
 *
 * Math (matrix form, 1/6 factor):
 * ```
 * B(t) = (1/6) * ((-t³+3t²-3t+1)p0 + (3t³-6t²+4)p1 + (-3t³+3t²+3t+1)p2 + t³p3)
 * ```
 *
 * @param t  - parameter in [0, 1]
 * @param p0 - first control point
 * @param p1 - second control point
 * @param p2 - third control point
 * @param p3 - fourth control point
 * @returns interpolated B-spline value
 *
 * @example
 * bSplineCubic(0.5, 0, 1, 2, 3) // 1.5 (collinear → linear)
 */
export const bSplineCubic = (
  t: number,
  p0: number,
  p1: number,
  p2: number,
  p3: number,
): number => {
  const t2 = t * t
  const t3 = t2 * t
  return (
    (-t3 + 3 * t2 - 3 * t + 1) * p0
    + (3 * t3 - 6 * t2 + 4) * p1
    + (-3 * t3 + 3 * t2 + 3 * t + 1) * p2
    + t3 * p3
  ) / 6
}

/**
 * Uniform cubic B-spline segment on `[x, y, z]` tuples.
 */
export const bSplineCubic3d = (
  t: number,
  p0: [number, number, number],
  p1: [number, number, number],
  p2: [number, number, number],
  p3: [number, number, number],
): [number, number, number] => [
  bSplineCubic(t, p0[0], p1[0], p2[0], p3[0]),
  bSplineCubic(t, p0[1], p1[1], p2[1], p3[1]),
  bSplineCubic(t, p0[2], p1[2], p2[2], p3[2]),
]

// ── Slerp ─────────────────────────────────────────────────────────────────────

/**
 * Spherical linear interpolation between two unit quaternions.
 *
 * Takes the shorter arc (negates q1 if dot < 0). Falls back to normalised
 * linear interpolation when the quaternions are nearly identical.
 *
 * Math:
 * ```
 * θ = acos(q0·q1)
 * result = sin((1-t)θ)/sin(θ) · q0 + sin(tθ)/sin(θ) · q1
 * ```
 *
 * @param t  - parameter in [0, 1]
 * @param q0 - start unit quaternion `[x, y, z, w]`
 * @param q1 - end unit quaternion `[x, y, z, w]`
 * @returns unit quaternion interpolated along the shorter arc
 *
 * @example
 * slerp(0, [0,0,0,1], [0,0,1,0]) // identity quaternion
 * slerp(1, [0,0,0,1], [0,0,1,0]) // [0,0,1,0]
 */
export const slerp = (
  t: number,
  q0: [number, number, number, number],
  q1: [number, number, number, number],
): [number, number, number, number] => {
  const dotRaw = q0[0] * q1[0] + q0[1] * q1[1] + q0[2] * q1[2] + q0[3] * q1[3]

  // Shorter arc: negate q1 if dot is negative
  const [q1x, q1y, q1z, q1w, dot] = dotRaw < 0
    ? [-q1[0], -q1[1], -q1[2], -q1[3], -dotRaw]
    : [q1[0],  q1[1],  q1[2],  q1[3],  dotRaw]

  // Near-identical quaternions → normalised linear interpolation
  if (dot > 0.9995) {
    const rx = q0[0] + t * (q1x - q0[0])
    const ry = q0[1] + t * (q1y - q0[1])
    const rz = q0[2] + t * (q1z - q0[2])
    const rw = q0[3] + t * (q1w - q0[3])
    const len = Math.sqrt(rx * rx + ry * ry + rz * rz + rw * rw)
    return [rx / len, ry / len, rz / len, rw / len]
  }

  const theta = Math.acos(dot)
  const sinTheta = Math.sin(theta)
  const w0 = Math.sin((1 - t) * theta) / sinTheta
  const w1 = Math.sin(t * theta) / sinTheta

  return [
    w0 * q0[0] + w1 * q1x,
    w0 * q0[1] + w1 * q1y,
    w0 * q0[2] + w1 * q1z,
    w0 * q0[3] + w1 * q1w,
  ]
}
