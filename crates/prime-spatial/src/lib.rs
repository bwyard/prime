//! `prime-spatial` — Spatial queries: ray tests, AABB operations, frustum culling.
//!
//! All public functions are **pure** (LOAD + COMPUTE only). No `&mut`. No hidden state.
//! Same inputs always produce the same output.
//!
//! All 3-D points and vectors are plain `(f32, f32, f32)` tuples for zero-cost interop.
//!
//! # Modules
//! - Ray intersection — AABB, sphere, plane
//! - AABB — overlap, containment, union, closest point
//! - Frustum — sphere culling against six half-spaces

/// Floating-point epsilon used in parallelism and near-zero tests.
const EPS: f32 = 1e-5;

// ── Private helpers ────────────────────────────────────────────────────────────

#[inline(always)]
fn dot3(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

#[inline(always)]
fn sub3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}

/// Clamp a scalar to [lo, hi].
#[inline(always)]
fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

// ── Ray–AABB ──────────────────────────────────────────────────────────────────

/// Ray–AABB intersection using the slab method (Kay–Kajiya).
///
/// # Math
///   For each axis i: t_min_i = (aabb_min_i - origin_i) / dir_i
///                    t_max_i = (aabb_max_i - origin_i) / dir_i
///   t_enter = max(t_min_x, t_min_y, t_min_z)
///   t_exit  = min(t_max_x, t_max_y, t_max_z)
///   Hit when t_enter <= t_exit and t_exit > 0.
///
/// # Arguments
/// * `ray_origin` - World-space ray origin.
/// * `ray_dir`    - Ray direction (need not be normalised; zero components are handled).
/// * `aabb_min`   - Corner of the AABB with smallest coordinates on every axis.
/// * `aabb_max`   - Corner of the AABB with largest coordinates on every axis.
///
/// # Returns
/// `Some(t)` where `t > 0` is the ray parameter at the first hit surface.
/// Returns `None` when the ray misses or the AABB is entirely behind the origin.
///
/// # Edge cases
/// * Ray parallel to a slab face and outside it → None.
/// * Ray origin inside AABB → Some(t_exit) (t_enter < 0, t_exit > 0).
/// * Zero-length direction component handled via `f32::INFINITY`.
///
/// # Example
/// ```rust
/// use prime_spatial::ray_aabb;
/// let t = ray_aabb((0.0, 0.0, -5.0), (0.0, 0.0, 1.0),
///                   (-1.0, -1.0, -1.0), (1.0, 1.0, 1.0));
/// assert!(t.is_some());
/// let t_val = t.unwrap();
/// assert!((t_val - 4.0).abs() < 1e-5);
/// ```
pub fn ray_aabb(
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    aabb_min: (f32, f32, f32),
    aabb_max: (f32, f32, f32),
) -> Option<f32> {
    // Per-axis slab test. When dir component is zero the reciprocal is ±infinity,
    // which correctly maps out-of-slab origins to [+inf, -inf] (no hit) and
    // in-slab origins to [-inf, +inf] (always inside that slab).
    let inv_x = 1.0 / ray_dir.0;
    let inv_y = 1.0 / ray_dir.1;
    let inv_z = 1.0 / ray_dir.2;

    let (tx1, tx2) = {
        let a = (aabb_min.0 - ray_origin.0) * inv_x;
        let b = (aabb_max.0 - ray_origin.0) * inv_x;
        (a.min(b), a.max(b))
    };
    let (ty1, ty2) = {
        let a = (aabb_min.1 - ray_origin.1) * inv_y;
        let b = (aabb_max.1 - ray_origin.1) * inv_y;
        (a.min(b), a.max(b))
    };
    let (tz1, tz2) = {
        let a = (aabb_min.2 - ray_origin.2) * inv_z;
        let b = (aabb_max.2 - ray_origin.2) * inv_z;
        (a.min(b), a.max(b))
    };

    let t_enter = tx1.max(ty1).max(tz1);
    let t_exit = tx2.min(ty2).min(tz2);

    if t_exit < 0.0 || t_enter > t_exit {
        return None;
    }

    // Return the first positive t. If origin is inside, t_enter < 0 so return t_exit.
    let t = if t_enter >= 0.0 { t_enter } else { t_exit };
    Some(t)
}

// ── Ray–Sphere ────────────────────────────────────────────────────────────────

/// Ray–sphere intersection.
///
/// # Math
///   Let oc = origin - center.
///   Solve |origin + t*dir - center|² = radius²
///   ⟹ dot(dir,dir)·t² + 2·dot(dir,oc)·t + dot(oc,oc) - r² = 0
///   discriminant = b² - 4ac  where b = dot(dir, oc), a = dot(dir,dir), c = dot(oc,oc) - r²
///   Using half-b form: h = dot(dir,oc), discriminant = h² - a·c.
///
/// # Arguments
/// * `ray_origin` - World-space ray origin.
/// * `ray_dir`    - Ray direction (need not be normalised).
/// * `center`     - Sphere centre in world space.
/// * `radius`     - Sphere radius (must be ≥ 0).
///
/// # Returns
/// `Some(t)` at the nearest positive intersection, or `None` if no positive hit.
///
/// # Edge cases
/// * Ray origin inside sphere → returns the exit intersection (positive t).
/// * Tangent ray (discriminant ≈ 0) → treated as a hit.
///
/// # Example
/// ```rust
/// use prime_spatial::ray_sphere;
/// let t = ray_sphere((0.0, 0.0, -5.0), (0.0, 0.0, 1.0), (0.0, 0.0, 0.0), 1.0);
/// assert!(t.is_some());
/// assert!((t.unwrap() - 4.0).abs() < 1e-5);
/// ```
pub fn ray_sphere(
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    center: (f32, f32, f32),
    radius: f32,
) -> Option<f32> {
    let oc = sub3(ray_origin, center);
    let a = dot3(ray_dir, ray_dir);
    let h = dot3(ray_dir, oc); // half-b
    let c = dot3(oc, oc) - radius * radius;
    let discriminant = h * h - a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_d = discriminant.sqrt();
    // Try the nearer root first.
    let t0 = (-h - sqrt_d) / a;
    if t0 > EPS {
        return Some(t0);
    }
    // Try the farther root (origin is inside the sphere).
    let t1 = (-h + sqrt_d) / a;
    if t1 > EPS {
        return Some(t1);
    }
    None
}

// ── Ray–Plane ─────────────────────────────────────────────────────────────────

/// Ray–plane intersection.
///
/// # Math
///   Plane equation: dot(normal, p) = d.
///   Substitute p = origin + t·dir:
///     dot(normal, origin + t·dir) = d
///     dot(normal, origin) + t·dot(normal, dir) = d
///     t = (d - dot(normal, origin)) / dot(normal, dir)
///
/// # Arguments
/// * `ray_origin`   - World-space ray origin.
/// * `ray_dir`      - Ray direction (need not be normalised).
/// * `plane_normal` - Plane normal (should be unit length for meaningful `d`).
/// * `plane_d`      - Scalar from the plane equation dot(normal, p) = d.
///
/// # Returns
/// `Some(t)` if the ray intersects the plane at `t > 0`, `None` if the ray is
/// parallel to the plane or the intersection is behind the origin.
///
/// # Edge cases
/// * `dot(normal, dir) ≈ 0` → ray is parallel → None.
/// * `t ≤ 0` → plane is behind the origin → None.
///
/// # Example
/// ```rust
/// use prime_spatial::ray_plane;
/// // XY plane (z = 0), normal = (0,0,1), d = 0.
/// let t = ray_plane((0.0, 0.0, -3.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0), 0.0);
/// assert!(t.is_some());
/// assert!((t.unwrap() - 3.0).abs() < 1e-5);
/// ```
pub fn ray_plane(
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    plane_normal: (f32, f32, f32),
    plane_d: f32,
) -> Option<f32> {
    let denom = dot3(plane_normal, ray_dir);
    if denom.abs() < EPS {
        return None; // Parallel
    }
    let t = (plane_d - dot3(plane_normal, ray_origin)) / denom;
    if t > EPS {
        Some(t)
    } else {
        None
    }
}

// ── AABB overlap ──────────────────────────────────────────────────────────────

/// Test whether two axis-aligned bounding boxes overlap.
///
/// Touching faces (shared boundary) counts as overlap.
///
/// # Math
///   Two AABBs overlap iff on every axis:
///     max_a_i >= min_b_i  AND  max_b_i >= min_a_i
///
/// # Arguments
/// * `min_a`, `max_a` - First AABB corners (min ≤ max component-wise).
/// * `min_b`, `max_b` - Second AABB corners (min ≤ max component-wise).
///
/// # Returns
/// `true` if the AABBs overlap or touch, `false` otherwise.
///
/// # Edge cases
/// * Degenerate (zero-volume) AABBs still produce correct overlap results.
///
/// # Example
/// ```rust
/// use prime_spatial::aabb_overlaps;
/// assert!(aabb_overlaps(
///     (0.0, 0.0, 0.0), (1.0, 1.0, 1.0),
///     (0.5, 0.5, 0.5), (2.0, 2.0, 2.0),
/// ));
/// assert!(!aabb_overlaps(
///     (0.0, 0.0, 0.0), (1.0, 1.0, 1.0),
///     (2.0, 0.0, 0.0), (3.0, 1.0, 1.0),
/// ));
/// ```
pub fn aabb_overlaps(
    min_a: (f32, f32, f32),
    max_a: (f32, f32, f32),
    min_b: (f32, f32, f32),
    max_b: (f32, f32, f32),
) -> bool {
    max_a.0 >= min_b.0
        && max_b.0 >= min_a.0
        && max_a.1 >= min_b.1
        && max_b.1 >= min_a.1
        && max_a.2 >= min_b.2
        && max_b.2 >= min_a.2
}

// ── AABB contains point ───────────────────────────────────────────────────────

/// Test whether a point lies inside or on the surface of an AABB.
///
/// # Math
///   p is contained iff on every axis: min_i <= p_i <= max_i.
///
/// # Arguments
/// * `min` - AABB corner with smallest coordinates.
/// * `max` - AABB corner with largest coordinates.
/// * `p`   - Point to test.
///
/// # Returns
/// `true` if `p` is inside or on the boundary of the AABB.
///
/// # Example
/// ```rust
/// use prime_spatial::aabb_contains;
/// assert!(aabb_contains((0.0,0.0,0.0), (1.0,1.0,1.0), (0.5,0.5,0.5)));
/// assert!(aabb_contains((0.0,0.0,0.0), (1.0,1.0,1.0), (0.0,0.0,0.0))); // on surface
/// assert!(!aabb_contains((0.0,0.0,0.0), (1.0,1.0,1.0), (2.0,0.0,0.0)));
/// ```
pub fn aabb_contains(
    min: (f32, f32, f32),
    max: (f32, f32, f32),
    p: (f32, f32, f32),
) -> bool {
    p.0 >= min.0 && p.0 <= max.0
        && p.1 >= min.1 && p.1 <= max.1
        && p.2 >= min.2 && p.2 <= max.2
}

// ── AABB union ────────────────────────────────────────────────────────────────

/// Compute the smallest AABB that contains two input AABBs.
///
/// # Math
///   union_min_i = min(min_a_i, min_b_i)
///   union_max_i = max(max_a_i, max_b_i)
///
/// # Arguments
/// * `min_a`, `max_a` - First AABB.
/// * `min_b`, `max_b` - Second AABB.
///
/// # Returns
/// `(union_min, union_max)` — the tightest AABB enclosing both inputs.
///
/// # Example
/// ```rust
/// use prime_spatial::aabb_union;
/// let (mn, mx) = aabb_union(
///     (0.0, 0.0, 0.0), (1.0, 1.0, 1.0),
///     (-1.0, 0.5, 0.5), (2.0, 2.0, 2.0),
/// );
/// assert_eq!(mn, (-1.0, 0.0, 0.0));
/// assert_eq!(mx, (2.0, 2.0, 2.0));
/// ```
pub fn aabb_union(
    min_a: (f32, f32, f32),
    max_a: (f32, f32, f32),
    min_b: (f32, f32, f32),
    max_b: (f32, f32, f32),
) -> ((f32, f32, f32), (f32, f32, f32)) {
    let union_min = (
        min_a.0.min(min_b.0),
        min_a.1.min(min_b.1),
        min_a.2.min(min_b.2),
    );
    let union_max = (
        max_a.0.max(max_b.0),
        max_a.1.max(max_b.1),
        max_a.2.max(max_b.2),
    );
    (union_min, union_max)
}

// ── AABB closest point ────────────────────────────────────────────────────────

/// Find the point on or inside an AABB closest to a query point.
///
/// # Math
///   For each axis i: result_i = clamp(p_i, min_i, max_i).
///   If p is inside the AABB the result equals p.
///
/// # Arguments
/// * `min` - AABB corner with smallest coordinates.
/// * `max` - AABB corner with largest coordinates.
/// * `p`   - Query point.
///
/// # Returns
/// The point in the AABB (surface or interior) nearest to `p`.
///
/// # Edge cases
/// * p inside AABB → returns p unchanged.
/// * p on surface → returns p unchanged (clamp is identity).
///
/// # Example
/// ```rust
/// use prime_spatial::aabb_closest_point;
/// // Point outside: nearest point is on the face.
/// let q = aabb_closest_point((0.0,0.0,0.0), (1.0,1.0,1.0), (3.0, 0.5, 0.5));
/// assert!((q.0 - 1.0).abs() < 1e-5);
/// assert!((q.1 - 0.5).abs() < 1e-5);
/// assert!((q.2 - 0.5).abs() < 1e-5);
/// // Point inside: returns the same point.
/// let inside = aabb_closest_point((0.0,0.0,0.0), (1.0,1.0,1.0), (0.5, 0.5, 0.5));
/// assert_eq!(inside, (0.5, 0.5, 0.5));
/// ```
pub fn aabb_closest_point(
    min: (f32, f32, f32),
    max: (f32, f32, f32),
    p: (f32, f32, f32),
) -> (f32, f32, f32) {
    (
        clamp(p.0, min.0, max.0),
        clamp(p.1, min.1, max.1),
        clamp(p.2, min.2, max.2),
    )
}

// ── Frustum cull (sphere) ─────────────────────────────────────────────────────

/// Test whether a sphere is outside a view frustum (should be culled).
///
/// # Math
///   A frustum is defined by six planes, each with equation:
///     dot(normal, p) + d >= 0  ⟹ "inside" half-space.
///   A sphere is OUTSIDE the frustum if it is entirely in the outside half-space
///   of any single plane:
///     dot(normal, center) + d < -radius
///
/// # Arguments
/// * `planes` - Six frustum planes as `(nx, ny, nz, d)`, ordered
///   `[left, right, bottom, top, near, far]`.
///   Normals must point **inward** (toward the frustum interior).
///   `d` is the signed offset such that `dot(n, p) + d >= 0` is inside.
/// * `center` - Sphere centre in the same space as the plane equations.
/// * `radius`  - Sphere radius (must be ≥ 0).
///
/// # Returns
/// `true`  — sphere is fully outside at least one plane → safe to cull.
/// `false` — sphere may be visible (inside or intersecting all half-spaces).
///
/// # Edge cases
/// * Zero-radius sphere (point) → correct point-in-frustum test.
/// * Sphere touching a plane exactly → not culled (`false`).
///
/// # Example
/// ```rust
/// use prime_spatial::frustum_cull_sphere;
/// // Unit cube frustum: planes at ±1 on each axis with inward normals.
/// let planes: [(f32,f32,f32,f32); 6] = [
///     ( 1.0, 0.0, 0.0,  1.0), // left:   x >= -1  ⟹  dot((1,0,0),p) + 1 >= 0
///     (-1.0, 0.0, 0.0,  1.0), // right:  x <=  1  ⟹  dot((-1,0,0),p) + 1 >= 0
///     ( 0.0, 1.0, 0.0,  1.0), // bottom
///     ( 0.0,-1.0, 0.0,  1.0), // top
///     ( 0.0, 0.0, 1.0,  1.0), // near
///     ( 0.0, 0.0,-1.0,  1.0), // far
/// ];
/// // Sphere clearly outside on the right side.
/// assert!(frustum_cull_sphere(&planes, (5.0, 0.0, 0.0), 0.1));
/// // Sphere inside.
/// assert!(!frustum_cull_sphere(&planes, (0.0, 0.0, 0.0), 0.5));
/// ```
pub fn frustum_cull_sphere(
    planes: &[(f32, f32, f32, f32); 6],
    center: (f32, f32, f32),
    radius: f32,
) -> bool {
    for &(nx, ny, nz, d) in planes.iter() {
        let dist = nx * center.0 + ny * center.1 + nz * center.2 + d;
        if dist < -radius {
            return true; // Entirely outside this half-space → cull
        }
    }
    false
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn approx_eq3(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
        approx_eq(a.0, b.0) && approx_eq(a.1, b.1) && approx_eq(a.2, b.2)
    }

    // ── ray_aabb ─────────────────────────────────────────────────────────────

    #[test]
    fn ray_aabb_hit_front_face() {
        // Ray travelling +Z, AABB from -1 to 1 on all axes, starting at z=-5.
        let t = ray_aabb(
            (0.0, 0.0, -5.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 4.0)); // hits front face at z = -1, t = 4
    }

    #[test]
    fn ray_aabb_miss_beside() {
        // Ray passes to the side of the AABB.
        let t = ray_aabb(
            (5.0, 0.0, -5.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_none());
    }

    #[test]
    fn ray_aabb_miss_behind() {
        // AABB is behind the ray origin.
        let t = ray_aabb(
            (0.0, 0.0, 5.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_none());
    }

    #[test]
    fn ray_aabb_origin_inside() {
        // Ray starts inside the box — should return exit t.
        let t = ray_aabb(
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 1.0)); // exits at z = 1
    }

    #[test]
    fn ray_aabb_parallel_inside_slab() {
        // Ray parallel to XZ plane, travels inside the Y slab.
        let t = ray_aabb(
            (0.0, 0.0, -5.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_some());
    }

    #[test]
    fn ray_aabb_parallel_outside_slab() {
        // Ray parallel to XZ plane, outside the Y slab.
        let t = ray_aabb(
            (0.0, 3.0, -5.0),
            (0.0, 0.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, 1.0),
        );
        assert!(t.is_none());
    }

    // ── ray_sphere ───────────────────────────────────────────────────────────

    #[test]
    fn ray_sphere_hit() {
        let t = ray_sphere(
            (0.0, 0.0, -5.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            1.0,
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 4.0)); // front of sphere at z = -1
    }

    #[test]
    fn ray_sphere_miss() {
        let t = ray_sphere(
            (5.0, 0.0, -5.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            1.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn ray_sphere_behind() {
        // Sphere is entirely behind the ray origin.
        let t = ray_sphere(
            (0.0, 0.0, 5.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            1.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn ray_sphere_origin_inside() {
        // Origin is inside the sphere — should return exit t.
        let t = ray_sphere(
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            2.0,
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 2.0)); // exits at z = 2
    }

    #[test]
    fn ray_sphere_tangent() {
        // Ray just grazes the sphere at the equator.
        let t = ray_sphere(
            (0.0, 1.0, -5.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 0.0),
            1.0,
        );
        // Tangent hit at z = 0.
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 5.0));
    }

    // ── ray_plane ────────────────────────────────────────────────────────────

    #[test]
    fn ray_plane_hit_xy_plane() {
        // XY plane at z = 0: normal = (0,0,1), d = 0.
        let t = ray_plane(
            (0.0, 0.0, -3.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
            0.0,
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 3.0));
    }

    #[test]
    fn ray_plane_hit_offset() {
        // Plane z = 5: normal = (0,0,1), d = 5.
        let t = ray_plane(
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
            5.0,
        );
        assert!(t.is_some());
        assert!(approx_eq(t.unwrap(), 5.0));
    }

    #[test]
    fn ray_plane_miss_parallel() {
        // Ray travelling parallel to the XY plane never hits it.
        let t = ray_plane(
            (0.0, 0.0, 1.0),
            (1.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            0.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn ray_plane_behind() {
        // Plane is at z = -5 behind a +Z pointing ray at z = 0.
        let t = ray_plane(
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
            -5.0,
        );
        assert!(t.is_none());
    }

    // ── aabb_overlaps ─────────────────────────────────────────────────────────

    #[test]
    fn aabb_overlaps_overlap() {
        assert!(aabb_overlaps(
            (0.0, 0.0, 0.0),
            (2.0, 2.0, 2.0),
            (1.0, 1.0, 1.0),
            (3.0, 3.0, 3.0),
        ));
    }

    #[test]
    fn aabb_overlaps_no_overlap() {
        assert!(!aabb_overlaps(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (2.0, 0.0, 0.0),
            (3.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn aabb_overlaps_touching() {
        // Faces exactly touching — counts as overlap.
        assert!(aabb_overlaps(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (1.0, 0.0, 0.0),
            (2.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn aabb_overlaps_one_inside_other() {
        assert!(aabb_overlaps(
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 10.0),
            (2.0, 2.0, 2.0),
            (3.0, 3.0, 3.0),
        ));
    }

    // ── aabb_contains ─────────────────────────────────────────────────────────

    #[test]
    fn aabb_contains_inside() {
        assert!(aabb_contains(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.5, 0.5, 0.5),
        ));
    }

    #[test]
    fn aabb_contains_outside() {
        assert!(!aabb_contains(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (2.0, 0.5, 0.5),
        ));
    }

    #[test]
    fn aabb_contains_on_surface() {
        // All corners of the AABB are on its own surface.
        assert!(aabb_contains(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (1.0, 1.0, 1.0),
        ));
        assert!(aabb_contains(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.0, 0.0, 0.0),
        ));
    }

    #[test]
    fn aabb_contains_just_outside() {
        assert!(!aabb_contains(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (1.0001, 0.5, 0.5),
        ));
    }

    // ── aabb_union ────────────────────────────────────────────────────────────

    #[test]
    fn aabb_union_disjoint() {
        let (mn, mx) = aabb_union(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (3.0, 3.0, 3.0),
            (4.0, 4.0, 4.0),
        );
        assert!(approx_eq3(mn, (0.0, 0.0, 0.0)));
        assert!(approx_eq3(mx, (4.0, 4.0, 4.0)));
    }

    #[test]
    fn aabb_union_overlapping() {
        let (mn, mx) = aabb_union(
            (-1.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.0, -1.0, 0.0),
            (2.0, 2.0, 2.0),
        );
        assert!(approx_eq3(mn, (-1.0, -1.0, 0.0)));
        assert!(approx_eq3(mx, (2.0, 2.0, 2.0)));
    }

    #[test]
    fn aabb_union_identical() {
        let (mn, mx) = aabb_union(
            (1.0, 2.0, 3.0),
            (4.0, 5.0, 6.0),
            (1.0, 2.0, 3.0),
            (4.0, 5.0, 6.0),
        );
        assert!(approx_eq3(mn, (1.0, 2.0, 3.0)));
        assert!(approx_eq3(mx, (4.0, 5.0, 6.0)));
    }

    // ── aabb_closest_point ────────────────────────────────────────────────────

    #[test]
    fn aabb_closest_point_outside_x() {
        let q = aabb_closest_point(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (3.0, 0.5, 0.5),
        );
        assert!(approx_eq3(q, (1.0, 0.5, 0.5)));
    }

    #[test]
    fn aabb_closest_point_inside() {
        let q = aabb_closest_point(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.5, 0.5, 0.5),
        );
        assert!(approx_eq3(q, (0.5, 0.5, 0.5)));
    }

    #[test]
    fn aabb_closest_point_corner() {
        // Point is past the (+x, +y, +z) corner.
        let q = aabb_closest_point(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (5.0, 5.0, 5.0),
        );
        assert!(approx_eq3(q, (1.0, 1.0, 1.0)));
    }

    #[test]
    fn aabb_closest_point_on_face() {
        // Point already on the face.
        let q = aabb_closest_point(
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.0, 0.5, 0.5),
        );
        assert!(approx_eq3(q, (0.0, 0.5, 0.5)));
    }

    // ── frustum_cull_sphere ───────────────────────────────────────────────────

    /// Unit-cube frustum: six planes at ±1 on each axis with inward normals.
    ///   left plane:   dot((+1,0,0), p) + 1 >= 0  ⟹  x >= -1
    ///   right plane:  dot((-1,0,0), p) + 1 >= 0  ⟹  x <=  1
    ///   etc.
    fn unit_cube_frustum() -> [(f32, f32, f32, f32); 6] {
        [
            (1.0, 0.0, 0.0, 1.0),  // left
            (-1.0, 0.0, 0.0, 1.0), // right
            (0.0, 1.0, 0.0, 1.0),  // bottom
            (0.0, -1.0, 0.0, 1.0), // top
            (0.0, 0.0, 1.0, 1.0),  // near
            (0.0, 0.0, -1.0, 1.0), // far
        ]
    }

    #[test]
    fn frustum_cull_sphere_inside_not_culled() {
        let planes = unit_cube_frustum();
        assert!(!frustum_cull_sphere(&planes, (0.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn frustum_cull_sphere_fully_outside_culled() {
        let planes = unit_cube_frustum();
        // Sphere at x = 5, well outside the right plane (x <= 1).
        assert!(frustum_cull_sphere(&planes, (5.0, 0.0, 0.0), 0.1));
    }

    #[test]
    fn frustum_cull_sphere_intersecting_not_culled() {
        let planes = unit_cube_frustum();
        // Sphere centred just outside the right face but large enough to intersect.
        assert!(!frustum_cull_sphere(&planes, (1.5, 0.0, 0.0), 1.0));
    }

    #[test]
    fn frustum_cull_sphere_touching_plane_not_culled() {
        let planes = unit_cube_frustum();
        // Sphere touching the right plane exactly (center at x=2, radius=1 ⟹ touch at x=1).
        assert!(!frustum_cull_sphere(&planes, (2.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn frustum_cull_sphere_zero_radius_inside() {
        let planes = unit_cube_frustum();
        assert!(!frustum_cull_sphere(&planes, (0.0, 0.0, 0.0), 0.0));
    }

    #[test]
    fn frustum_cull_sphere_zero_radius_outside() {
        let planes = unit_cube_frustum();
        assert!(frustum_cull_sphere(&planes, (3.0, 0.0, 0.0), 0.0));
    }
}
