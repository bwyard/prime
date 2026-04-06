//! Axis-aligned bounding box operations: overlap, containment, union, closest point.

use crate::ray::clamp;

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

    #[test]
    fn aabb_overlap_degenerate_touching() {
        assert!(aabb_overlaps((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    }

    #[test]
    fn aabb_overlap_degenerate_apart() {
        assert!(!aabb_overlaps((0.0, 0.0, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
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

    #[test]
    fn aabb_contains_degenerate_on_point() {
        assert!(aabb_contains((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
        assert!(!aabb_contains((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.1)));
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

    #[test]
    fn aabb_closest_point_degenerate() {
        let cp = aabb_closest_point((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (5.0, 5.0, 5.0));
        assert!(approx_eq3(cp, (1.0, 1.0, 1.0)));
    }
}
