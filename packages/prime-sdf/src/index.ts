/**
 * `prime-sdf` — Signed Distance Functions: primitives, boolean ops, smooth ops, domain transforms.
 *
 * All exported functions are **pure** (LOAD + COMPUTE only). No mutation. No hidden state.
 * Same inputs always produce the same output.
 *
 * 2-D points are `[number, number]` tuples.
 * 3-D points are `[number, number, number]` tuples.
 *
 * Mirrors the Rust crate at `crates/prime-sdf/src/`.
 *
 * SDF convention: negative inside shape, positive outside, zero on surface.
 */

// ── Private helpers ───────────────────────────────────────────────────────────

type Vec2 = [number, number];
type Vec3 = [number, number, number];

const len2 = ([x, y]: Vec2): number => Math.sqrt(x * x + y * y);
const len3 = ([x, y, z]: Vec3): number => Math.sqrt(x * x + y * y + z * z);
const dot2 = ([ax, ay]: Vec2, [bx, by]: Vec2): number => ax * bx + ay * by;
const dot3 = ([ax, ay, az]: Vec3, [bx, by, bz]: Vec3): number => ax * bx + ay * by + az * bz;
const clamp = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, v));
const clamp01 = (v: number): number => clamp(v, 0, 1);

// ── 2D Primitives ─────────────────────────────────────────────────────────────

/**
 * Signed distance from point `p` to a circle.
 *
 * Math: d(p) = |p - center| - radius
 *
 * @param p - Query point
 * @param center - Circle center
 * @param radius - Circle radius (> 0)
 * @returns Negative inside, positive outside, zero on surface.
 * @example circle([1, 0], [0, 0], 2) // -1
 */
export const circle = (p: Vec2, center: Vec2, radius: number): number =>
  len2([p[0] - center[0], p[1] - center[1]]) - radius;

/**
 * Signed distance from point `p` to an axis-aligned 2D box.
 *
 * Math: q = |p - center| - halfExtents
 *       d(p) = |max(q, 0)| + min(max(q.x, q.y), 0)
 *
 * @param p - Query point
 * @param center - Box center
 * @param halfExtents - Half-widths in x and y
 * @example box2d([3, 0], [0, 0], [1, 1]) // 2
 */
export const box2d = (p: Vec2, center: Vec2, halfExtents: Vec2): number => {
  const qx = Math.abs(p[0] - center[0]) - halfExtents[0];
  const qy = Math.abs(p[1] - center[1]) - halfExtents[1];
  return len2([Math.max(qx, 0), Math.max(qy, 0)]) + Math.min(Math.max(qx, qy), 0);
};

/**
 * Signed distance from point `p` to a rounded 2D box.
 *
 * Math: rounded_box(p, c, h, r) = box2d(p, c, h) - r
 *
 * @param p - Query point
 * @param center - Box center
 * @param halfExtents - Half-widths (before rounding)
 * @param radius - Corner rounding radius
 */
export const roundedBox = (p: Vec2, center: Vec2, halfExtents: Vec2, radius: number): number =>
  box2d(p, center, halfExtents) - radius;

/**
 * Signed distance from point `p` to a 2D capsule (stadium).
 *
 * Math: project p onto segment AB, clamp t in [0,1], measure distance to
 *       nearest point minus radius.
 *
 * @param p - Query point
 * @param a - Capsule start
 * @param b - Capsule end
 * @param radius - Capsule radius
 */
export const capsule2d = (p: Vec2, a: Vec2, b: Vec2, radius: number): number => {
  const pa: Vec2 = [p[0] - a[0], p[1] - a[1]];
  const ba: Vec2 = [b[0] - a[0], b[1] - a[1]];
  const t = clamp01(dot2(pa, ba) / dot2(ba, ba));
  return len2([pa[0] - ba[0] * t, pa[1] - ba[1] * t]) - radius;
};

/**
 * Signed distance from point `p` to a line segment with thickness.
 *
 * Math: same as capsule2d — a thickened line segment is a capsule.
 */
export const lineSegment = (p: Vec2, a: Vec2, b: Vec2, thickness: number): number =>
  capsule2d(p, a, b, thickness);

/**
 * Signed distance from point `p` to a triangle.
 *
 * Math: minimum signed distance to each edge half-plane via cross products.
 *
 * @param p - Query point
 * @param a - Vertex A (counter-clockwise)
 * @param b - Vertex B
 * @param c - Vertex C
 */
export const triangle = (p: Vec2, a: Vec2, b: Vec2, c: Vec2): number => {
  const e0: Vec2 = [b[0] - a[0], b[1] - a[1]];
  const e1: Vec2 = [c[0] - b[0], c[1] - b[1]];
  const e2: Vec2 = [a[0] - c[0], a[1] - c[1]];
  const v0: Vec2 = [p[0] - a[0], p[1] - a[1]];
  const v1: Vec2 = [p[0] - b[0], p[1] - b[1]];
  const v2: Vec2 = [p[0] - c[0], p[1] - c[1]];
  const t0 = clamp01(dot2(v0, e0) / dot2(e0, e0));
  const t1 = clamp01(dot2(v1, e1) / dot2(e1, e1));
  const t2 = clamp01(dot2(v2, e2) / dot2(e2, e2));
  const pq0: Vec2 = [v0[0] - e0[0] * t0, v0[1] - e0[1] * t0];
  const pq1: Vec2 = [v1[0] - e1[0] * t1, v1[1] - e1[1] * t1];
  const pq2: Vec2 = [v2[0] - e2[0] * t2, v2[1] - e2[1] * t2];
  const s = Math.sign(e0[0] * e2[1] - e0[1] * e2[0]);
  const d = Math.sqrt(Math.min(dot2(pq0, pq0), dot2(pq1, pq1), dot2(pq2, pq2)));
  const inside = Math.min(
    s * (v0[0] * e0[1] - v0[1] * e0[0]),
    s * (v1[0] * e1[1] - v1[1] * e1[0]),
    s * (v2[0] * e2[1] - v2[1] * e2[0]),
  );
  return d * -Math.sign(inside);
};

/**
 * Signed distance from point `p` to a ring (annulus).
 *
 * Math: d(p) = | |p - center| - (outerR + innerR) / 2 | - (outerR - innerR) / 2
 *
 * @param p - Query point
 * @param center - Ring center
 * @param outerR - Outer radius
 * @param innerR - Inner radius (must be < outerR)
 */
export const ring = (p: Vec2, center: Vec2, outerR: number, innerR: number): number => {
  const mid = (outerR + innerR) * 0.5;
  const halfWidth = (outerR - innerR) * 0.5;
  return Math.abs(len2([p[0] - center[0], p[1] - center[1]]) - mid) - halfWidth;
};

// ── 3D Primitives ─────────────────────────────────────────────────────────────

/**
 * Signed distance from point `p` to a sphere.
 *
 * Math: d(p) = |p - center| - radius
 *
 * @param p - Query point
 * @param center - Sphere center
 * @param radius - Sphere radius
 * @example sphere([2, 0, 0], [0, 0, 0], 1) // 1
 */
export const sphere = (p: Vec3, center: Vec3, radius: number): number =>
  len3([p[0] - center[0], p[1] - center[1], p[2] - center[2]]) - radius;

/**
 * Signed distance from point `p` to an axis-aligned 3D box.
 *
 * Math: q = |p - center| - halfExtents
 *       d(p) = |max(q, 0)| + min(max(q.x, q.y, q.z), 0)
 */
export const box3d = (p: Vec3, center: Vec3, halfExtents: Vec3): number => {
  const qx = Math.abs(p[0] - center[0]) - halfExtents[0];
  const qy = Math.abs(p[1] - center[1]) - halfExtents[1];
  const qz = Math.abs(p[2] - center[2]) - halfExtents[2];
  return len3([Math.max(qx, 0), Math.max(qy, 0), Math.max(qz, 0)]) + Math.min(Math.max(qx, qy, qz), 0);
};

/**
 * Signed distance from point `p` to a 3D capsule.
 *
 * Math: project p onto segment AB, clamp t in [0,1], measure to nearest point minus radius.
 */
export const capsule3d = (p: Vec3, a: Vec3, b: Vec3, radius: number): number => {
  const pa: Vec3 = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
  const ba: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const t = clamp01(dot3(pa, ba) / dot3(ba, ba));
  return len3([pa[0] - ba[0] * t, pa[1] - ba[1] * t, pa[2] - ba[2] * t]) - radius;
};

/**
 * Signed distance from point `p` to a cylinder.
 *
 * Math: combine lateral (xz) and axial (y) distances as a 2D box.
 *
 * @param p - Query point
 * @param center - Cylinder center
 * @param height - Total height
 * @param radius - Cylinder radius
 */
export const cylinder = (p: Vec3, center: Vec3, height: number, radius: number): number => {
  const dx = p[0] - center[0];
  const dy = p[1] - center[1];
  const dz = p[2] - center[2];
  const lateral = Math.sqrt(dx * dx + dz * dz) - radius;
  const axial = Math.abs(dy) - height * 0.5;
  return len2([Math.max(lateral, 0), Math.max(axial, 0)]) + Math.min(Math.max(lateral, axial), 0);
};

/**
 * Signed distance from point `p` to a torus.
 *
 * Math: q = (|p.xz| - majorR, p.y)
 *       d(p) = |q| - minorR
 *
 * @param p - Query point
 * @param center - Torus center
 * @param majorR - Distance from torus center to tube center
 * @param minorR - Tube radius
 */
export const torus = (p: Vec3, center: Vec3, majorR: number, minorR: number): number => {
  const dx = p[0] - center[0];
  const dy = p[1] - center[1];
  const dz = p[2] - center[2];
  const qx = Math.sqrt(dx * dx + dz * dz) - majorR;
  return len2([qx, dy]) - minorR;
};

/**
 * Signed distance from point `p` to an infinite plane.
 *
 * Math: d(p) = dot(p, normal) - offset
 *
 * @param p - Query point
 * @param normal - Plane normal (unit length)
 * @param offset - Signed distance from origin to plane along normal
 */
export const plane = (p: Vec3, normal: Vec3, offset: number): number =>
  dot3(p, normal) - offset;

// ── Boolean Ops ───────────────────────────────────────────────────────────────

/**
 * Boolean union of two SDF values.
 *
 * Math: union(d1, d2) = min(d1, d2)
 */
export const union = (d1: number, d2: number): number => Math.min(d1, d2);

/**
 * Boolean intersection of two SDF values.
 *
 * Math: intersection(d1, d2) = max(d1, d2)
 */
export const intersection = (d1: number, d2: number): number => Math.max(d1, d2);

/**
 * Subtract shape d2 from shape d1.
 *
 * Math: subtract(d1, d2) = max(d1, -d2)
 */
export const subtract = (d1: number, d2: number): number => Math.max(d1, -d2);

/**
 * Exclusive OR of two SDF regions.
 *
 * Math: xor(d1, d2) = max(min(d1, d2), -max(d1, d2))
 */
export const xor = (d1: number, d2: number): number =>
  Math.max(Math.min(d1, d2), -Math.max(d1, d2));

// ── Smooth Ops ────────────────────────────────────────────────────────────────

/**
 * Smooth union of two SDF values (polynomial smooth min, Inigo Quilez).
 *
 * Math: h = max(k - |d1 - d2|, 0) / k
 *       smooth_union = min(d1, d2) - h*h*k*0.25
 *
 * @param d1 - First SDF value
 * @param d2 - Second SDF value
 * @param k - Blend radius (> 0); larger = wider blend
 */
export const smoothUnion = (d1: number, d2: number, k: number): number => {
  const h = Math.max(k - Math.abs(d1 - d2), 0) / k;
  return Math.min(d1, d2) - h * h * k * 0.25;
};

/**
 * Smooth intersection of two SDF values.
 *
 * Math: h = max(k - |d1 - d2|, 0) / k
 *       smooth_intersection = max(d1, d2) + h*h*k*0.25
 */
export const smoothIntersection = (d1: number, d2: number, k: number): number => {
  const h = Math.max(k - Math.abs(d1 - d2), 0) / k;
  return Math.max(d1, d2) + h * h * k * 0.25;
};

/**
 * Smooth subtraction of d2 from d1.
 *
 * Math: smooth_subtract(d1, d2, k) = smooth_intersection(d1, -d2, k)
 */
export const smoothSubtract = (d1: number, d2: number, k: number): number =>
  smoothIntersection(d1, -d2, k);

// ── Domain Transforms ─────────────────────────────────────────────────────────

/**
 * Translate a 2D query point.
 *
 * Math: translate(p, offset) = p - offset
 */
export const translate = (p: Vec2, offset: Vec2): Vec2 =>
  [p[0] - offset[0], p[1] - offset[1]];

/**
 * Rotate a 2D query point by angle (counter-clockwise).
 *
 * Math: rotate(p, θ) = [cos θ  sin θ; -sin θ  cos θ] × p
 */
export const rotate2d = (p: Vec2, angleRad: number): Vec2 => {
  const s = Math.sin(angleRad);
  const c = Math.cos(angleRad);
  return [c * p[0] + s * p[1], -s * p[0] + c * p[1]];
};

/**
 * Scale a 2D query point.
 *
 * Math: scale(p, f) = p / f
 *
 * NOTE: the SDF result must also be divided by `f` after sampling.
 */
export const scale2d = (p: Vec2, factor: number): Vec2 =>
  [p[0] / factor, p[1] / factor];

/**
 * Infinite tiling repeat of 2D space.
 *
 * Math: repeat(p, period) = mod(p + period/2, period) - period/2
 */
export const repeat2d = (p: Vec2, period: Vec2): Vec2 => {
  const halfX = period[0] * 0.5;
  const halfY = period[1] * 0.5;
  const modX = ((p[0] + halfX) % period[0] + period[0]) % period[0] - halfX;
  const modY = ((p[1] + halfY) % period[1] + period[1]) % period[1] - halfY;
  return [modX, modY];
};

/**
 * Mirror a 2D point across the Y axis (fold in X).
 */
export const mirrorX = (p: Vec2): Vec2 => [Math.abs(p[0]), p[1]];

/**
 * Mirror a 2D point across the X axis (fold in Y).
 */
export const mirrorY = (p: Vec2): Vec2 => [p[0], Math.abs(p[1])];

/**
 * Elongate a 2D shape by stretching space along each axis.
 *
 * Math: elongate(p, h) = p - clamp(p, -h, h)
 */
export const elongate = (p: Vec2, h: Vec2): Vec2 => [
  p[0] - clamp(p[0], -h[0], h[0]),
  p[1] - clamp(p[1], -h[1], h[1]),
];
