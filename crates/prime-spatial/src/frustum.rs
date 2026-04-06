//! Frustum culling: sphere and AABB against six half-spaces.

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

// ── Frustum cull (AABB) ──────────────────────────────────────────────────────

/// Test whether an AABB is at least partially inside a view frustum.
///
/// # Math
///
/// For each frustum plane, find the AABB vertex most in the direction of the
/// plane normal (the "positive vertex"). If that vertex is on the outside of
/// any plane, the entire AABB is outside the frustum.
///
/// ```text
/// p_vertex_i = if n_i >= 0 { aabb_max_i } else { aabb_min_i }
/// inside = ∀ plane: dot(n, p_vertex) + d >= 0
/// ```
///
/// # Arguments
/// * `aabb_min` — Corner of the AABB with smallest coordinates on every axis.
/// * `aabb_max` — Corner of the AABB with largest coordinates on every axis.
/// * `planes`   — Six frustum planes as `(nx, ny, nz, d)`.
///   Normals must point **inward** (toward the frustum interior).
///   `d` is the signed offset such that `nx*x + ny*y + nz*z + d >= 0` is inside.
///
/// # Returns
/// `true` if the AABB is at least partially inside the frustum (do NOT cull).
/// `false` if the AABB is fully outside at least one plane (safe to cull).
///
/// # Edge cases
/// * Zero-volume (degenerate) AABB → correct point-in-frustum test.
/// * AABB touching a plane exactly → considered inside (`true`).
///
/// # Example
/// ```rust
/// use prime_spatial::frustum_cull_aabb;
/// // Unit-cube frustum: planes at ±1 on each axis with inward normals.
/// let planes: [(f32,f32,f32,f32); 6] = [
///     ( 1.0, 0.0, 0.0,  1.0), // left:   x >= -1
///     (-1.0, 0.0, 0.0,  1.0), // right:  x <=  1
///     ( 0.0, 1.0, 0.0,  1.0), // bottom
///     ( 0.0,-1.0, 0.0,  1.0), // top
///     ( 0.0, 0.0, 1.0,  1.0), // near
///     ( 0.0, 0.0,-1.0,  1.0), // far
/// ];
/// // AABB inside the frustum
/// assert!(frustum_cull_aabb((-0.5, -0.5, -0.5), (0.5, 0.5, 0.5), &planes));
/// // AABB fully outside
/// assert!(!frustum_cull_aabb((5.0, 5.0, 5.0), (6.0, 6.0, 6.0), &planes));
/// ```
pub fn frustum_cull_aabb(
    aabb_min: (f32, f32, f32),
    aabb_max: (f32, f32, f32),
    planes: &[(f32, f32, f32, f32); 6],
) -> bool {
    planes.iter().all(|&(nx, ny, nz, d)| {
        // Test the positive vertex (the one most in the direction of the normal)
        let px = if nx >= 0.0 { aabb_max.0 } else { aabb_min.0 };
        let py = if ny >= 0.0 { aabb_max.1 } else { aabb_min.1 };
        let pz = if nz >= 0.0 { aabb_max.2 } else { aabb_min.2 };
        nx * px + ny * py + nz * pz + d >= 0.0
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── frustum_cull_sphere ───────────────────────────────────────────────────

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

    #[test]
    fn frustum_cull_sphere_large_sphere_spans_frustum_not_culled() {
        // Sphere so large it spans the entire frustum — should not be culled.
        let planes = unit_cube_frustum();
        assert!(!frustum_cull_sphere(&planes, (0.0, 0.0, 0.0), 100.0));
    }

    // ── frustum_cull_aabb ────────────────────────────────────────────────────

    #[test]
    fn frustum_cull_aabb_inside() {
        let planes = unit_cube_frustum();
        // Small box fully inside the frustum.
        assert!(frustum_cull_aabb((-0.5, -0.5, -0.5), (0.5, 0.5, 0.5), &planes));
    }

    #[test]
    fn frustum_cull_aabb_fully_outside() {
        let planes = unit_cube_frustum();
        // Box entirely to the right of the frustum.
        assert!(!frustum_cull_aabb((5.0, 5.0, 5.0), (6.0, 6.0, 6.0), &planes));
    }

    #[test]
    fn frustum_cull_aabb_intersecting() {
        let planes = unit_cube_frustum();
        // Box straddles the right face: partially inside.
        assert!(frustum_cull_aabb((0.5, -0.5, -0.5), (2.0, 0.5, 0.5), &planes));
    }

    #[test]
    fn frustum_cull_aabb_touching_plane() {
        let planes = unit_cube_frustum();
        // Box touches the right plane exactly at x = 1.
        assert!(frustum_cull_aabb((0.0, 0.0, 0.0), (1.0, 1.0, 1.0), &planes));
    }

    #[test]
    fn frustum_cull_aabb_degenerate_point_inside() {
        let planes = unit_cube_frustum();
        assert!(frustum_cull_aabb((0.0, 0.0, 0.0), (0.0, 0.0, 0.0), &planes));
    }

    #[test]
    fn frustum_cull_aabb_degenerate_point_outside() {
        let planes = unit_cube_frustum();
        assert!(!frustum_cull_aabb((5.0, 0.0, 0.0), (5.0, 0.0, 0.0), &planes));
    }

    #[test]
    fn frustum_cull_aabb_large_box_spans_frustum() {
        let planes = unit_cube_frustum();
        // Box much larger than the frustum — should be considered inside.
        assert!(frustum_cull_aabb((-100.0, -100.0, -100.0), (100.0, 100.0, 100.0), &planes));
    }

    #[test]
    fn frustum_cull_aabb_outside_single_axis() {
        let planes = unit_cube_frustum();
        // Box is inside on y and z, but fully outside on x (left of left plane).
        assert!(!frustum_cull_aabb((-5.0, -0.5, -0.5), (-3.0, 0.5, 0.5), &planes));
    }
}
