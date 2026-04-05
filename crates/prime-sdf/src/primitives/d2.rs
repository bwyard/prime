use glam::Vec2;

/// Signed distance from point `p` to a circle.
///
/// # Math
///
///   d(p) = |p - c| - r
///
/// Negative inside, positive outside, zero on boundary.
///
/// # Arguments
/// * `p` - query point
/// * `center` - circle center
/// * `radius` - circle radius (must be > 0 for meaningful results)
///
/// # Returns
/// Signed distance: negative inside, positive outside, zero on surface.
///
/// # Example
/// ```rust
/// use glam::Vec2;
/// use prime_sdf::circle;
/// let d = circle(Vec2::new(1.0, 0.0), Vec2::ZERO, 2.0);
/// assert!((d - (-1.0)).abs() < 1e-6);
/// ```
pub fn circle(p: Vec2, center: Vec2, radius: f32) -> f32 {
    (p - center).length() - radius
}

/// Signed distance from point `p` to an axis-aligned box.
///
/// # Math
///
///   q = |p - c| - h
///   d(p) = |max(q, 0)| + min(max(q.x, q.y), 0)
///
/// # Arguments
/// * `p` - query point
/// * `center` - box center
/// * `half_extents` - half-widths in x and y
///
/// # Example
/// ```rust
/// use glam::Vec2;
/// use prime_sdf::box_2d;
/// let d = box_2d(Vec2::new(3.0, 0.0), Vec2::ZERO, Vec2::new(1.0, 1.0));
/// assert!((d - 2.0).abs() < 1e-6);
/// ```
pub fn box_2d(p: Vec2, center: Vec2, half_extents: Vec2) -> f32 {
    let q = (p - center).abs() - half_extents;
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0)
}

/// Signed distance from point `p` to a rounded box.
///
/// # Math
/// Same as box_2d but with corner radius `r` subtracted from result.
///
/// # Arguments
/// * `p` - query point
/// * `center` - box center
/// * `half_extents` - half-widths (before rounding)
/// * `radius` - corner rounding radius
pub fn rounded_box(p: Vec2, center: Vec2, half_extents: Vec2, radius: f32) -> f32 {
    box_2d(p, center, half_extents) - radius
}

/// Signed distance from point `p` to a 2D capsule (stadium shape).
///
/// # Math
/// Project p onto line segment AB, clamp t in \[0,1\], measure distance to
/// nearest point minus radius.
///
/// # Arguments
/// * `p` - query point
/// * `a` - capsule start
/// * `b` - capsule end
/// * `radius` - capsule radius
pub fn capsule_2d(p: Vec2, a: Vec2, b: Vec2, radius: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let t = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * t).length() - radius
}

/// Signed distance from point `p` to a line segment with thickness.
///
/// # Math
/// Same as capsule_2d — a thickened line segment is a capsule.
pub fn line_segment(p: Vec2, a: Vec2, b: Vec2, thickness: f32) -> f32 {
    capsule_2d(p, a, b, thickness)
}

/// Signed distance from point `p` to a triangle.
///
/// # Math
/// Takes minimum of signed distances to each edge half-plane,
/// using cross products to determine inside/outside.
///
/// # Arguments
/// * `p` - query point
/// * `a`, `b`, `c` - triangle vertices (counter-clockwise order)
pub fn triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let e0 = b - a;
    let e1 = c - b;
    let e2 = a - c;
    let v0 = p - a;
    let v1 = p - b;
    let v2 = p - c;
    let pq0 = v0 - e0 * (v0.dot(e0) / e0.dot(e0)).clamp(0.0, 1.0);
    let pq1 = v1 - e1 * (v1.dot(e1) / e1.dot(e1)).clamp(0.0, 1.0);
    let pq2 = v2 - e2 * (v2.dot(e2) / e2.dot(e2)).clamp(0.0, 1.0);
    let s = (e0.x * e2.y - e0.y * e2.x).signum();
    let d = (pq0.dot(pq0).min(pq1.dot(pq1)).min(pq2.dot(pq2))).sqrt();
    let inside = (s * (v0.x * e0.y - v0.y * e0.x))
        .min(s * (v1.x * e1.y - v1.y * e1.x))
        .min(s * (v2.x * e2.y - v2.y * e2.x));
    d * (-inside.signum())
}

/// Signed distance from point `p` to a ring (annulus).
///
/// # Math
///   d(p) = | |p - c| - (outer_r + inner_r) / 2 | - (outer_r - inner_r) / 2
///
/// # Arguments
/// * `p` - query point
/// * `center` - ring center
/// * `outer_r` - outer radius
/// * `inner_r` - inner radius (must be < outer_r)
pub fn ring(p: Vec2, center: Vec2, outer_r: f32, inner_r: f32) -> f32 {
    let mid = (outer_r + inner_r) * 0.5;
    let half_width = (outer_r - inner_r) * 0.5;
    ((p - center).length() - mid).abs() - half_width
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;
    fn approx_eq(a: f32, b: f32) -> bool { (a - b).abs() < EPSILON }

    #[test]
    fn circle_outside() { assert!(approx_eq(circle(Vec2::new(3.0, 0.0), Vec2::ZERO, 1.0), 2.0)); }
    #[test]
    fn circle_inside() { assert!(approx_eq(circle(Vec2::ZERO, Vec2::ZERO, 2.0), -2.0)); }
    #[test]
    fn circle_on_surface() { assert!(approx_eq(circle(Vec2::new(1.0, 0.0), Vec2::ZERO, 1.0), 0.0)); }
    #[test]
    fn box_2d_outside() { assert!(approx_eq(box_2d(Vec2::new(3.0, 0.0), Vec2::ZERO, Vec2::new(1.0, 1.0)), 2.0)); }
    #[test]
    fn box_2d_inside() { assert!(box_2d(Vec2::new(0.5, 0.0), Vec2::ZERO, Vec2::new(1.0, 1.0)) < 0.0); }
    #[test]
    fn capsule_2d_midpoint() {
        let d = capsule_2d(Vec2::new(0.0, 1.0), Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 0.5);
        assert!(approx_eq(d, 0.5));
    }
    #[test]
    fn ring_on_inner_surface() {
        let d = ring(Vec2::new(1.0, 0.0), Vec2::ZERO, 2.0, 1.0);
        assert!(approx_eq(d, 0.0));
    }
    #[test]
    fn triangle_inside() {
        let d = triangle(
            Vec2::new(0.5, 0.5),
            Vec2::ZERO,
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 2.0),
        );
        assert!(approx_eq(d, -0.5));
    }
    #[test]
    fn triangle_outside() {
        let d = triangle(
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 2.0),
        );
        assert!(approx_eq(d, 1.0));
    }
    #[test]
    fn triangle_on_surface() {
        let d = triangle(
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 2.0),
        );
        assert!(approx_eq(d, 0.0));
    }
}
