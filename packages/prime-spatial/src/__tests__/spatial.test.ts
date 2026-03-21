/**
 * Tests for `prime-spatial`.
 *
 * Cross-language reference values are taken from the Rust implementation at
 * `crates/prime-spatial/src/lib.rs`. If the Rust implementation changes, update
 * the expected values here to match.
 *
 * All identifiers are `const` only — no `let` anywhere in this file.
 */

import { describe, expect, it } from "vitest";
import {
  aabbClosestPoint,
  aabbContains,
  aabbOverlaps,
  aabbUnion,
  frustumCullSphere,
  rayAabb,
  rayPlane,
  raySphere,
} from "../index.js";

const EPSILON = 1e-5;

const approxEq = (a: number, b: number): boolean => Math.abs(a - b) < EPSILON;

const approxEq3 = (
  a: [number, number, number],
  b: [number, number, number],
): boolean => approxEq(a[0], b[0]) && approxEq(a[1], b[1]) && approxEq(a[2], b[2]);

// ── rayAabb ───────────────────────────────────────────────────────────────────

describe("rayAabb", () => {
  it("hits the front face — ray travelling +Z at z=-5 toward unit AABB", () => {
    // Cross-language: Rust ray_aabb returns Some(4.0) for this input.
    const t = rayAabb([0, 0, -5], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 4.0)).toBe(true);
  });

  it("misses when ray passes to the side", () => {
    const t = rayAabb([5, 0, -5], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).toBeNull();
  });

  it("misses when AABB is entirely behind the origin", () => {
    const t = rayAabb([0, 0, 5], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).toBeNull();
  });

  it("returns t_exit when origin is inside the AABB", () => {
    // Cross-language: Rust ray_aabb returns Some(1.0) for origin inside.
    const t = rayAabb([0, 0, 0], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 1.0)).toBe(true);
  });

  it("hits when ray is parallel to one axis but inside that slab", () => {
    // Ray along Z inside Y slab — should still hit.
    const t = rayAabb([0, 0, -5], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).not.toBeNull();
  });

  it("misses when ray is parallel to one axis and outside that slab", () => {
    const t = rayAabb([0, 3, -5], [0, 0, 1], [-1, -1, -1], [1, 1, 1]);
    expect(t).toBeNull();
  });
});

// ── raySphere ─────────────────────────────────────────────────────────────────

describe("raySphere", () => {
  it("hits the front of a unit sphere at origin — ray from z=-5", () => {
    // Cross-language: Rust ray_sphere returns Some(4.0).
    const t = raySphere([0, 0, -5], [0, 0, 1], [0, 0, 0], 1);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 4.0)).toBe(true);
  });

  it("misses when ray passes to the side", () => {
    const t = raySphere([5, 0, -5], [0, 0, 1], [0, 0, 0], 1);
    expect(t).toBeNull();
  });

  it("misses when sphere is entirely behind the origin", () => {
    const t = raySphere([0, 0, 5], [0, 0, 1], [0, 0, 0], 1);
    expect(t).toBeNull();
  });

  it("returns the exit t when origin is inside the sphere", () => {
    // Cross-language: Rust ray_sphere returns Some(2.0) for radius=2, origin at centre.
    const t = raySphere([0, 0, 0], [0, 0, 1], [0, 0, 0], 2);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 2.0)).toBe(true);
  });

  it("treats a tangent ray as a hit", () => {
    // Cross-language: Rust ray_sphere returns Some(5.0) for this tangent case.
    const t = raySphere([0, 1, -5], [0, 0, 1], [0, 0, 0], 1);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 5.0)).toBe(true);
  });
});

// ── rayPlane ──────────────────────────────────────────────────────────────────

describe("rayPlane", () => {
  it("hits the XY plane at z=0 from z=-3", () => {
    // Cross-language: Rust ray_plane returns Some(3.0).
    const t = rayPlane([0, 0, -3], [0, 0, 1], [0, 0, 1], 0);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 3.0)).toBe(true);
  });

  it("hits an offset plane at z=5 from the origin", () => {
    // Cross-language: Rust ray_plane returns Some(5.0).
    const t = rayPlane([0, 0, 0], [0, 0, 1], [0, 0, 1], 5);
    expect(t).not.toBeNull();
    expect(approxEq(t!, 5.0)).toBe(true);
  });

  it("returns null for a ray parallel to the plane", () => {
    const t = rayPlane([0, 0, 1], [1, 0, 0], [0, 0, 1], 0);
    expect(t).toBeNull();
  });

  it("returns null when the plane is behind the ray origin", () => {
    const t = rayPlane([0, 0, 0], [0, 0, 1], [0, 0, 1], -5);
    expect(t).toBeNull();
  });
});

// ── aabbOverlaps ──────────────────────────────────────────────────────────────

describe("aabbOverlaps", () => {
  it("detects overlapping AABBs", () => {
    expect(aabbOverlaps([0, 0, 0], [2, 2, 2], [1, 1, 1], [3, 3, 3])).toBe(true);
  });

  it("returns false for separated AABBs", () => {
    expect(aabbOverlaps([0, 0, 0], [1, 1, 1], [2, 0, 0], [3, 1, 1])).toBe(false);
  });

  it("counts touching faces as overlap", () => {
    expect(aabbOverlaps([0, 0, 0], [1, 1, 1], [1, 0, 0], [2, 1, 1])).toBe(true);
  });

  it("detects one AABB fully inside the other", () => {
    expect(aabbOverlaps([0, 0, 0], [10, 10, 10], [2, 2, 2], [3, 3, 3])).toBe(true);
  });
});

// ── aabbContains ──────────────────────────────────────────────────────────────

describe("aabbContains", () => {
  it("returns true for a point inside the AABB", () => {
    expect(aabbContains([0, 0, 0], [1, 1, 1], [0.5, 0.5, 0.5])).toBe(true);
  });

  it("returns false for a point outside the AABB", () => {
    expect(aabbContains([0, 0, 0], [1, 1, 1], [2, 0.5, 0.5])).toBe(false);
  });

  it("returns true for a point on the max corner (surface)", () => {
    expect(aabbContains([0, 0, 0], [1, 1, 1], [1, 1, 1])).toBe(true);
  });

  it("returns true for a point on the min corner (surface)", () => {
    expect(aabbContains([0, 0, 0], [1, 1, 1], [0, 0, 0])).toBe(true);
  });

  it("returns false for a point just outside (1.0001 on x)", () => {
    expect(aabbContains([0, 0, 0], [1, 1, 1], [1.0001, 0.5, 0.5])).toBe(false);
  });
});

// ── aabbUnion ─────────────────────────────────────────────────────────────────

describe("aabbUnion", () => {
  it("produces the correct enclosing AABB for two overlapping boxes", () => {
    // Cross-language: Rust aabb_union returns ((-1,0,0),(2,2,2)).
    const [mn, mx] = aabbUnion([0, 0, 0], [1, 1, 1], [-1, 0.5, 0.5], [2, 2, 2]);
    expect(approxEq3(mn, [-1, 0, 0])).toBe(true);
    expect(approxEq3(mx, [2, 2, 2])).toBe(true);
  });

  it("is idempotent — union of identical boxes returns the same box", () => {
    const [mn, mx] = aabbUnion([0, 0, 0], [1, 1, 1], [0, 0, 0], [1, 1, 1]);
    expect(approxEq3(mn, [0, 0, 0])).toBe(true);
    expect(approxEq3(mx, [1, 1, 1])).toBe(true);
  });

  it("handles separated boxes correctly", () => {
    const [mn, mx] = aabbUnion([0, 0, 0], [1, 1, 1], [3, 3, 3], [5, 5, 5]);
    expect(approxEq3(mn, [0, 0, 0])).toBe(true);
    expect(approxEq3(mx, [5, 5, 5])).toBe(true);
  });
});

// ── aabbClosestPoint ──────────────────────────────────────────────────────────

describe("aabbClosestPoint", () => {
  it("clamps an outside point to the nearest face", () => {
    // Cross-language: Rust aabb_closest_point returns (1.0, 0.5, 0.5).
    const q = aabbClosestPoint([0, 0, 0], [1, 1, 1], [3, 0.5, 0.5]);
    expect(approxEq3(q, [1, 0.5, 0.5])).toBe(true);
  });

  it("returns the point unchanged when it is inside the AABB", () => {
    const q = aabbClosestPoint([0, 0, 0], [1, 1, 1], [0.5, 0.5, 0.5]);
    expect(approxEq3(q, [0.5, 0.5, 0.5])).toBe(true);
  });

  it("returns the point unchanged when it is on the surface", () => {
    const q = aabbClosestPoint([0, 0, 0], [1, 1, 1], [1, 0.5, 0.5]);
    expect(approxEq3(q, [1, 0.5, 0.5])).toBe(true);
  });

  it("clamps a point outside on all three axes to the nearest corner", () => {
    const q = aabbClosestPoint([0, 0, 0], [1, 1, 1], [5, 5, 5]);
    expect(approxEq3(q, [1, 1, 1])).toBe(true);
  });
});

// ── frustumCullSphere ─────────────────────────────────────────────────────────

// Unit-cube frustum: planes at ±1 on each axis with inward normals.
// dot(n, p) + d >= 0 is inside.
const UNIT_FRUSTUM: [number, number, number, number][] = [
  [1, 0, 0, 1],  // left:   x >= -1
  [-1, 0, 0, 1], // right:  x <=  1
  [0, 1, 0, 1],  // bottom: y >= -1
  [0, -1, 0, 1], // top:    y <=  1
  [0, 0, 1, 1],  // near:   z >= -1
  [0, 0, -1, 1], // far:    z <=  1
];

describe("frustumCullSphere", () => {
  it("culls a sphere clearly outside the right plane", () => {
    // Cross-language: Rust frustum_cull_sphere returns true.
    expect(frustumCullSphere(UNIT_FRUSTUM, [5, 0, 0], 0.1)).toBe(true);
  });

  it("does not cull a sphere fully inside the frustum", () => {
    // Cross-language: Rust frustum_cull_sphere returns false.
    expect(frustumCullSphere(UNIT_FRUSTUM, [0, 0, 0], 0.5)).toBe(false);
  });

  it("does not cull a sphere that straddles a plane (intersecting)", () => {
    // Centre at x=1.5 with radius=1 — partially inside, partially outside.
    expect(frustumCullSphere(UNIT_FRUSTUM, [1.5, 0, 0], 1)).toBe(false);
  });

  it("does not cull a sphere exactly touching a plane from inside", () => {
    // Centre at x=0.9, radius=0.1 — just touches the right plane from inside.
    expect(frustumCullSphere(UNIT_FRUSTUM, [0.9, 0, 0], 0.1)).toBe(false);
  });

  it("culls a zero-radius sphere (point) that lies outside the frustum", () => {
    expect(frustumCullSphere(UNIT_FRUSTUM, [2, 0, 0], 0)).toBe(true);
  });

  it("does not cull a zero-radius sphere (point) inside the frustum", () => {
    expect(frustumCullSphere(UNIT_FRUSTUM, [0, 0, 0], 0)).toBe(false);
  });
});
