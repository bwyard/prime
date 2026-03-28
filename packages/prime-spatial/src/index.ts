/**
 * `prime-spatial` — Spatial queries: ray tests, AABB operations, frustum culling.
 *
 * All exported functions are **pure** (LOAD + COMPUTE only). No mutation. No hidden state.
 * Same inputs always produce the same output.
 *
 * All 3-D points and vectors are `[number, number, number]` tuples.
 *
 * Mirrors the Rust crate at `crates/prime-spatial/src/lib.rs`.
 */

/** Floating-point epsilon used in parallelism and near-zero tests. */
const EPS = 1e-5;

// ── Private helpers ───────────────────────────────────────────────────────────

const dot3 = (
  a: [number, number, number],
  b: [number, number, number],
): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

const sub3 = (
  a: [number, number, number],
  b: [number, number, number],
): [number, number, number] => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];

const clamp = (v: number, lo: number, hi: number): number =>
  Math.max(lo, Math.min(hi, v));

// ── Ray–AABB ──────────────────────────────────────────────────────────────────

/**
 * Ray–AABB intersection using the slab method (Kay–Kajiya).
 *
 * Math:
 *   For each axis i: t_min_i = (aabb_min_i - origin_i) / dir_i
 *                    t_max_i = (aabb_max_i - origin_i) / dir_i
 *   t_enter = max(t_min_x, t_min_y, t_min_z)
 *   t_exit  = min(t_max_x, t_max_y, t_max_z)
 *   Hit when t_enter <= t_exit and t_exit > 0.
 *
 * @param rayOrigin - World-space ray origin.
 * @param rayDir    - Ray direction (need not be normalised; zero components are handled).
 * @param aabbMin   - Corner of the AABB with smallest coordinates on every axis.
 * @param aabbMax   - Corner of the AABB with largest coordinates on every axis.
 * @returns `t > 0` (the ray parameter at the first hit surface), or `null` when
 *   the ray misses or the AABB is entirely behind the origin.
 *
 * Edge cases:
 * - Ray parallel to a slab face and outside it → null.
 * - Ray origin inside AABB → t_exit (t_enter < 0, t_exit > 0).
 * - Zero-length direction component handled via Infinity.
 *
 * @example
 * const t = rayAabb([0,0,-5], [0,0,1], [-1,-1,-1], [1,1,1]);
 * // t === 4  (hits front face at z = -1)
 */
export const rayAabb = (
  rayOrigin: [number, number, number],
  rayDir: [number, number, number],
  aabbMin: [number, number, number],
  aabbMax: [number, number, number],
): number | null => {
  const invX = 1.0 / rayDir[0];
  const invY = 1.0 / rayDir[1];
  const invZ = 1.0 / rayDir[2];

  const ax = (aabbMin[0] - rayOrigin[0]) * invX;
  const bx = (aabbMax[0] - rayOrigin[0]) * invX;
  const tx1 = Math.min(ax, bx);
  const tx2 = Math.max(ax, bx);

  const ay = (aabbMin[1] - rayOrigin[1]) * invY;
  const by = (aabbMax[1] - rayOrigin[1]) * invY;
  const ty1 = Math.min(ay, by);
  const ty2 = Math.max(ay, by);

  const az = (aabbMin[2] - rayOrigin[2]) * invZ;
  const bz = (aabbMax[2] - rayOrigin[2]) * invZ;
  const tz1 = Math.min(az, bz);
  const tz2 = Math.max(az, bz);

  const tEnter = Math.max(tx1, ty1, tz1);
  const tExit = Math.min(tx2, ty2, tz2);

  if (tExit < 0.0 || tEnter > tExit) {
    return null;
  }

  // If origin is inside, t_enter < 0 — return t_exit instead.
  const t = tEnter >= 0.0 ? tEnter : tExit;
  return t;
};

// ── Ray–Sphere ────────────────────────────────────────────────────────────────

/**
 * Ray–sphere intersection.
 *
 * Math:
 *   Let oc = origin - center.
 *   Solve |origin + t*dir - center|² = radius²
 *   ⟹ dot(dir,dir)·t² + 2·dot(dir,oc)·t + dot(oc,oc) - r² = 0
 *   Using half-b form: h = dot(dir,oc), discriminant = h² - a·c
 *     where a = dot(dir,dir), c = dot(oc,oc) - r².
 *
 * @param rayOrigin - World-space ray origin.
 * @param rayDir    - Ray direction (need not be normalised).
 * @param center    - Sphere centre in world space.
 * @param radius    - Sphere radius (must be >= 0).
 * @returns `t` at the nearest positive intersection, or `null` if no positive hit.
 *
 * Edge cases:
 * - Ray origin inside sphere → returns the exit intersection (positive t).
 * - Tangent ray (discriminant ≈ 0) → treated as a hit.
 *
 * @example
 * const t = raySphere([0,0,-5], [0,0,1], [0,0,0], 1);
 * // t === 4  (front of sphere at z = -1)
 */
export const raySphere = (
  rayOrigin: [number, number, number],
  rayDir: [number, number, number],
  center: [number, number, number],
  radius: number,
): number | null => {
  const oc = sub3(rayOrigin, center);
  const a = dot3(rayDir, rayDir);
  const h = dot3(rayDir, oc); // half-b
  const c = dot3(oc, oc) - radius * radius;
  const discriminant = h * h - a * c;

  if (discriminant < 0.0) {
    return null;
  }

  const sqrtD = Math.sqrt(discriminant);

  // Try the nearer root first.
  const t0 = (-h - sqrtD) / a;
  if (t0 > EPS) {
    return t0;
  }

  // Try the farther root (origin is inside the sphere).
  const t1 = (-h + sqrtD) / a;
  if (t1 > EPS) {
    return t1;
  }

  return null;
};

// ── Ray–Plane ─────────────────────────────────────────────────────────────────

/**
 * Ray–plane intersection.
 *
 * Math:
 *   Plane equation: dot(normal, p) = d.
 *   Substitute p = origin + t·dir:
 *     dot(normal, origin + t·dir) = d
 *     dot(normal, origin) + t·dot(normal, dir) = d
 *     t = (d - dot(normal, origin)) / dot(normal, dir)
 *
 * @param rayOrigin   - World-space ray origin.
 * @param rayDir      - Ray direction (need not be normalised).
 * @param planeNormal - Plane normal (should be unit length for meaningful `planeD`).
 * @param planeD      - Scalar from the plane equation dot(normal, p) = d.
 * @returns `t` if the ray intersects the plane at `t > 0`, or `null` if the ray is
 *   parallel to the plane or the intersection is behind the origin.
 *
 * Edge cases:
 * - `dot(normal, dir) ≈ 0` → ray is parallel → null.
 * - `t <= 0` → plane is behind the origin → null.
 *
 * @example
 * // XY plane (z = 0), normal = [0,0,1], d = 0.
 * const t = rayPlane([0,0,-3], [0,0,1], [0,0,1], 0);
 * // t === 3
 */
export const rayPlane = (
  rayOrigin: [number, number, number],
  rayDir: [number, number, number],
  planeNormal: [number, number, number],
  planeD: number,
): number | null => {
  const denom = dot3(planeNormal, rayDir);
  if (Math.abs(denom) < EPS) {
    return null; // Parallel
  }
  const t = (planeD - dot3(planeNormal, rayOrigin)) / denom;
  return t > EPS ? t : null;
};

// ── AABB overlap ──────────────────────────────────────────────────────────────

/**
 * Test whether two axis-aligned bounding boxes overlap.
 *
 * Touching faces (shared boundary) counts as overlap.
 *
 * Math:
 *   Two AABBs overlap iff on every axis:
 *     max_a_i >= min_b_i  AND  max_b_i >= min_a_i
 *
 * @param minA - First AABB corner with smallest coordinates.
 * @param maxA - First AABB corner with largest coordinates.
 * @param minB - Second AABB corner with smallest coordinates.
 * @param maxB - Second AABB corner with largest coordinates.
 * @returns `true` if the AABBs overlap or touch, `false` otherwise.
 *
 * Edge cases:
 * - Degenerate (zero-volume) AABBs still produce correct overlap results.
 *
 * @example
 * aabbOverlaps([0,0,0], [1,1,1], [0.5,0.5,0.5], [2,2,2]); // true
 * aabbOverlaps([0,0,0], [1,1,1], [2,0,0], [3,1,1]);        // false
 */
export const aabbOverlaps = (
  minA: [number, number, number],
  maxA: [number, number, number],
  minB: [number, number, number],
  maxB: [number, number, number],
): boolean =>
  maxA[0] >= minB[0] &&
  maxB[0] >= minA[0] &&
  maxA[1] >= minB[1] &&
  maxB[1] >= minA[1] &&
  maxA[2] >= minB[2] &&
  maxB[2] >= minA[2];

// ── AABB contains point ───────────────────────────────────────────────────────

/**
 * Test whether a point lies inside or on the surface of an AABB.
 *
 * Math:
 *   p is contained iff on every axis: min_i <= p_i <= max_i.
 *
 * @param min - AABB corner with smallest coordinates.
 * @param max - AABB corner with largest coordinates.
 * @param p   - Point to test.
 * @returns `true` if `p` is inside or on the boundary of the AABB.
 *
 * @example
 * aabbContains([0,0,0], [1,1,1], [0.5,0.5,0.5]); // true
 * aabbContains([0,0,0], [1,1,1], [0,0,0]);         // true (on surface)
 * aabbContains([0,0,0], [1,1,1], [2,0,0]);          // false
 */
export const aabbContains = (
  min: [number, number, number],
  max: [number, number, number],
  p: [number, number, number],
): boolean =>
  p[0] >= min[0] &&
  p[0] <= max[0] &&
  p[1] >= min[1] &&
  p[1] <= max[1] &&
  p[2] >= min[2] &&
  p[2] <= max[2];

// ── AABB union ────────────────────────────────────────────────────────────────

/**
 * Compute the smallest AABB that contains two input AABBs.
 *
 * Math:
 *   union_min_i = min(min_a_i, min_b_i)
 *   union_max_i = max(max_a_i, max_b_i)
 *
 * @param minA - First AABB corner with smallest coordinates.
 * @param maxA - First AABB corner with largest coordinates.
 * @param minB - Second AABB corner with smallest coordinates.
 * @param maxB - Second AABB corner with largest coordinates.
 * @returns `[unionMin, unionMax]` — the tightest AABB enclosing both inputs.
 *
 * @example
 * const [mn, mx] = aabbUnion([0,0,0], [1,1,1], [-1,0.5,0.5], [2,2,2]);
 * // mn === [-1, 0, 0],  mx === [2, 2, 2]
 */
export const aabbUnion = (
  minA: [number, number, number],
  maxA: [number, number, number],
  minB: [number, number, number],
  maxB: [number, number, number],
): [[number, number, number], [number, number, number]] => [
  [Math.min(minA[0], minB[0]), Math.min(minA[1], minB[1]), Math.min(minA[2], minB[2])],
  [Math.max(maxA[0], maxB[0]), Math.max(maxA[1], maxB[1]), Math.max(maxA[2], maxB[2])],
];

// ── AABB closest point ────────────────────────────────────────────────────────

/**
 * Find the point on or inside an AABB closest to a query point.
 *
 * Math:
 *   For each axis i: result_i = clamp(p_i, min_i, max_i).
 *   If p is inside the AABB the result equals p.
 *
 * @param min - AABB corner with smallest coordinates.
 * @param max - AABB corner with largest coordinates.
 * @param p   - Query point.
 * @returns The point in the AABB (surface or interior) nearest to `p`.
 *
 * Edge cases:
 * - p inside AABB → returns p unchanged.
 * - p on surface → returns p unchanged (clamp is identity).
 *
 * @example
 * aabbClosestPoint([0,0,0], [1,1,1], [3, 0.5, 0.5]); // [1, 0.5, 0.5]
 * aabbClosestPoint([0,0,0], [1,1,1], [0.5, 0.5, 0.5]); // [0.5, 0.5, 0.5]
 */
export const aabbClosestPoint = (
  min: [number, number, number],
  max: [number, number, number],
  p: [number, number, number],
): [number, number, number] => [
  clamp(p[0], min[0], max[0]),
  clamp(p[1], min[1], max[1]),
  clamp(p[2], min[2], max[2]),
];

// ── Frustum cull (sphere) ─────────────────────────────────────────────────────

/**
 * Test whether a sphere is outside a view frustum (should be culled).
 *
 * Math:
 *   A frustum is defined by planes, each with equation:
 *     dot(normal, p) + d >= 0  ⟹ "inside" half-space.
 *   A sphere is OUTSIDE the frustum if it is entirely in the outside half-space
 *   of any single plane:
 *     dot(normal, center) + d < -radius
 *
 * @param planes - Frustum planes as `[nx, ny, nz, d]`, typically ordered
 *   `[left, right, bottom, top, near, far]`. Normals must point **inward**
 *   (toward the frustum interior). `d` is the signed offset such that
 *   `dot(n, p) + d >= 0` is inside.
 * @param center - Sphere centre in the same space as the plane equations.
 * @param radius - Sphere radius (must be >= 0).
 * @returns `true`  — sphere is fully outside at least one plane → safe to cull.
 *          `false` — sphere may be visible (inside or intersecting all half-spaces).
 *
 * Edge cases:
 * - Zero-radius sphere (point) → correct point-in-frustum test.
 * - Sphere touching a plane exactly → not culled (`false`).
 *
 * @example
 * // Unit cube frustum: planes at ±1 on each axis with inward normals.
 * const planes: [number,number,number,number][] = [
 *   [ 1, 0, 0,  1], // left
 *   [-1, 0, 0,  1], // right
 *   [ 0, 1, 0,  1], // bottom
 *   [ 0,-1, 0,  1], // top
 *   [ 0, 0, 1,  1], // near
 *   [ 0, 0,-1,  1], // far
 * ];
 * frustumCullSphere(planes, [5, 0, 0], 0.1);  // true  (outside right plane)
 * frustumCullSphere(planes, [0, 0, 0], 0.5);  // false (inside)
 */
export const frustumCullSphere = (
  planes: readonly [number, number, number, number][],
  center: [number, number, number],
  radius: number,
): boolean =>
  planes.some(
    ([nx, ny, nz, d]) =>
      nx * center[0] + ny * center[1] + nz * center[2] + d < -radius,
  );

// ── Frustum cull (AABB) ──────────────────────────────────────────────────────

/**
 * Test whether an AABB is inside (or intersecting) a view frustum.
 *
 * Returns `true` if the AABB's positive vertex (p-vertex) is inside all six
 * frustum half-spaces, meaning the box is at least partially visible.
 * Returns `false` when the AABB is fully outside any single plane.
 *
 * Math:
 *   For each plane, select the AABB corner most in the direction of the plane
 *   normal (p-vertex). If that corner is outside, the entire AABB is outside.
 *   The AABB passes if its p-vertex is inside every plane.
 *
 * @param aabbMin - Corner with smallest coordinates on every axis.
 * @param aabbMax - Corner with largest coordinates on every axis.
 * @param planes  - Six frustum planes `[nx, ny, nz, d]` with inward normals.
 * @returns `true` if the AABB is at least partially inside the frustum,
 *          `false` if it is fully outside any plane.
 *
 * @example
 * frustumCullAabb([-0.5,-0.5,-0.5], [0.5,0.5,0.5], UNIT_FRUSTUM) // true
 * frustumCullAabb([5,5,5], [6,6,6], UNIT_FRUSTUM)                 // false
 */
export const frustumCullAabb = (
  aabbMin: [number, number, number],
  aabbMax: [number, number, number],
  planes: readonly [number, number, number, number][],
): boolean =>
  planes.every(([nx, ny, nz, d]) => {
    const px = nx >= 0 ? aabbMax[0] : aabbMin[0]
    const py = ny >= 0 ? aabbMax[1] : aabbMin[1]
    const pz = nz >= 0 ? aabbMax[2] : aabbMin[2]
    return nx * px + ny * py + nz * pz + d >= 0
  });
