//! Ray intersection tests: AABB (slab method), sphere, plane.

use crate::EPS;

// ── Private helpers ────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn dot3(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

#[inline(always)]
pub(crate) fn sub3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}

/// Clamp a scalar to [lo, hi].
#[inline(always)]
pub(crate) fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
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

    #[test]
    fn ray_aabb_zero_x_dir_outside_slab_misses() {
        // x-component of dir is 0; origin is outside x-slab [0,1] → miss.
        let t = ray_aabb((5.0, 0.5, -5.0), (0.0, 0.0, 1.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0));
        assert!(t.is_none());
    }

    #[test]
    fn ray_aabb_zero_x_dir_inside_slab_hits() {
        // x-component of dir is 0; origin is inside x-slab → hits via z-travel.
        let t = ray_aabb((0.5, 0.5, -5.0), (0.0, 0.0, 1.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0));
        assert!(t.is_some());
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
}
