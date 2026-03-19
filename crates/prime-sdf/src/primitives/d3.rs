use glam::Vec3;

/// Signed distance from point `p` to a sphere.
///
/// # Math
///   d(p) = |p - c| - r
///
/// # Example
/// ```rust
/// use glam::Vec3;
/// use prime_sdf::sphere;
/// let d = sphere(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 1.0);
/// assert!((d - 1.0).abs() < 1e-6);
/// ```
pub fn sphere(p: Vec3, center: Vec3, radius: f32) -> f32 {
    (p - center).length() - radius
}

/// Signed distance from point `p` to an axis-aligned 3D box.
///
/// # Math
///   q = |p - c| - h
///   d(p) = |max(q, 0)| + min(max(q.x, q.y, q.z), 0)
pub fn box_3d(p: Vec3, center: Vec3, half_extents: Vec3) -> f32 {
    let q = (p - center).abs() - half_extents;
    q.max(Vec3::ZERO).length() + q.x.max(q.y).max(q.z).min(0.0)
}

/// Signed distance from point `p` to a 3D capsule.
///
/// # Math
/// Project p onto segment AB, clamp t in [0,1], measure to nearest point minus radius.
pub fn capsule_3d(p: Vec3, a: Vec3, b: Vec3, radius: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let t = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * t).length() - radius
}

/// Signed distance from point `p` to a cylinder.
///
/// # Math
/// Distance in XZ plane minus radius gives the lateral SDF.
/// Distance in Y minus half-height gives the end-cap SDF.
/// Combine as a 2D box in (lateral, axial) space.
///
/// # Arguments
/// * `p` - query point
/// * `center` - cylinder center
/// * `height` - total height
/// * `radius` - cylinder radius
pub fn cylinder(p: Vec3, center: Vec3, height: f32, radius: f32) -> f32 {
    use glam::Vec2;
    let d = p - center;
    let lateral = Vec2::new(d.x, d.z).length() - radius;
    let axial = d.y.abs() - height * 0.5;
    let q = Vec2::new(lateral, axial);
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0)
}

/// Signed distance from point `p` to a torus.
///
/// # Math
///   q = (|p.xz| - major_r, p.y)
///   d(p) = |q| - minor_r
///
/// # Arguments
/// * `p` - query point
/// * `center` - torus center
/// * `major_r` - distance from torus center to tube center
/// * `minor_r` - tube radius
pub fn torus(p: Vec3, center: Vec3, major_r: f32, minor_r: f32) -> f32 {
    use glam::Vec2;
    let d = p - center;
    let q = Vec2::new(Vec2::new(d.x, d.z).length() - major_r, d.y);
    q.length() - minor_r
}

/// Signed distance from point `p` to an infinite plane.
///
/// # Math
///   d(p) = dot(p, n) - offset
///
/// Where n is the unit normal.
///
/// # Arguments
/// * `p` - query point
/// * `normal` - plane normal (should be unit length)
/// * `offset` - signed distance from origin to plane along normal
pub fn plane(p: Vec3, normal: Vec3, offset: f32) -> f32 {
    p.dot(normal) - offset
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;
    fn approx_eq(a: f32, b: f32) -> bool { (a - b).abs() < EPSILON }

    #[test]
    fn sphere_outside() { assert!(approx_eq(sphere(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 1.0), 1.0)); }
    #[test]
    fn sphere_inside() { assert!(approx_eq(sphere(Vec3::ZERO, Vec3::ZERO, 2.0), -2.0)); }
    #[test]
    fn sphere_on_surface() { assert!(approx_eq(sphere(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 1.0), 0.0)); }
    #[test]
    fn box_3d_outside() { assert!(approx_eq(box_3d(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE), 2.0)); }
    #[test]
    fn plane_above() { assert!(approx_eq(plane(Vec3::new(0.0, 5.0, 0.0), Vec3::Y, 0.0), 5.0)); }
    #[test]
    fn plane_below() { assert!(approx_eq(plane(Vec3::new(0.0, -3.0, 0.0), Vec3::Y, 0.0), -3.0)); }
    #[test]
    fn torus_outside() {
        assert!(approx_eq(torus(Vec3::new(6.0, 0.0, 0.0), Vec3::ZERO, 3.0, 1.0), 2.0));
    }
    #[test]
    fn torus_inside() {
        assert!(approx_eq(torus(Vec3::new(3.0, 0.5, 0.0), Vec3::ZERO, 3.0, 1.0), -0.5));
    }
    #[test]
    fn torus_on_surface() {
        assert!(approx_eq(torus(Vec3::new(4.0, 0.0, 0.0), Vec3::ZERO, 3.0, 1.0), 0.0));
    }
    #[test]
    fn cylinder_outside() {
        assert!(approx_eq(cylinder(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, 2.0, 1.0), 2.0));
    }
    #[test]
    fn cylinder_inside() {
        assert!(approx_eq(cylinder(Vec3::ZERO, Vec3::ZERO, 2.0, 1.0), -1.0));
    }
    #[test]
    fn cylinder_on_curved_surface() {
        assert!(approx_eq(cylinder(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 2.0, 1.0), 0.0));
    }
    #[test]
    fn capsule_3d_midpoint() {
        let d = capsule_3d(Vec3::new(1.0, 1.0, 0.0), Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
        assert!(approx_eq(d, 0.5));
    }
    #[test]
    fn capsule_3d_endpoint() {
        let d = capsule_3d(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
        assert!(approx_eq(d, 0.5));
    }
    #[test]
    fn capsule_3d_on_surface() {
        let d = capsule_3d(Vec3::new(0.5, 1.0, 0.0), Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
        assert!(approx_eq(d, 0.0));
    }
}
