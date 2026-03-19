use glam::Vec2;

/// Translate a 2D query point.
///
/// # Math
///   translate(p, offset) = p - offset
///
/// Subtract offset to move the query point into the shape's local space.
pub fn translate(p: Vec2, offset: Vec2) -> Vec2 { p - offset }

/// Rotate a 2D query point by angle (counter-clockwise).
///
/// # Math
///   rotate(p, θ) = [cos θ  sin θ; -sin θ  cos θ] * p
pub fn rotate_2d(p: Vec2, angle_rad: f32) -> Vec2 {
    let (s, c) = angle_rad.sin_cos();
    Vec2::new(c * p.x + s * p.y, -s * p.x + c * p.y)
}

/// Scale a 2D query point.
///
/// # Math
///   scale(p, f) = p / f
///
/// NOTE: the SDF result must also be divided by `f` after sampling.
///
/// # Arguments
/// * `p` - query point
/// * `factor` - scale factor (> 0)
pub fn scale(p: Vec2, factor: f32) -> Vec2 { p / factor }

/// Infinite tiling repeat of 2D space.
///
/// # Math
///   repeat(p, period) = mod(p + period/2, period) - period/2
pub fn repeat(p: Vec2, period: Vec2) -> Vec2 {
    let half = period * 0.5;
    (p + half).rem_euclid(period) - half
}

/// Mirror a 2D point across the Y axis (fold in X).
pub fn mirror_x(p: Vec2) -> Vec2 { Vec2::new(p.x.abs(), p.y) }

/// Mirror a 2D point across the X axis (fold in Y).
pub fn mirror_y(p: Vec2) -> Vec2 { Vec2::new(p.x, p.y.abs()) }

/// Elongate a 2D shape by stretching space along each axis.
///
/// # Math
///   elongate(p, h) = p - clamp(p, -h, h)
///
/// Stretches the SDF uniformly in the given direction.
pub fn elongate(p: Vec2, h: Vec2) -> Vec2 { p - p.clamp(-h, h) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn translate_moves_point() {
        let p = translate(Vec2::new(3.0, 0.0), Vec2::new(1.0, 0.0));
        assert!((p.x - 2.0).abs() < EPSILON);
    }

    #[test]
    fn rotate_90_degrees() {
        let p = rotate_2d(Vec2::new(1.0, 0.0), PI / 2.0);
        assert!((p.x - 0.0).abs() < EPSILON);
        assert!((p.y - (-1.0)).abs() < EPSILON);
    }

    #[test]
    fn mirror_x_folds() {
        let p = mirror_x(Vec2::new(-2.0, 3.0));
        assert_eq!(p, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn repeat_wraps() {
        let period = Vec2::new(4.0, 4.0);
        let p1 = repeat(Vec2::new(0.0, 0.0), period);
        let p2 = repeat(Vec2::new(4.0, 0.0), period);
        assert!((p1 - p2).length() < EPSILON);
    }

    #[test]
    fn elongate_zero_h_is_identity() {
        let p = elongate(Vec2::new(3.0, 2.0), Vec2::ZERO);
        assert!((p - Vec2::new(3.0, 2.0)).length() < EPSILON);
    }

    #[test]
    fn elongate_point_within_band_collapses_to_zero() {
        let p = elongate(Vec2::new(0.5, 0.0), Vec2::new(2.0, 1.0));
        assert!(p.length() < EPSILON);
    }

    #[test]
    fn elongate_point_beyond_band() {
        let p = elongate(Vec2::new(5.0, 0.0), Vec2::new(2.0, 1.0));
        assert!((p - Vec2::new(3.0, 0.0)).length() < EPSILON);
    }

    #[test]
    fn scale_halves_coordinates() {
        let p = scale(Vec2::new(4.0, 6.0), 2.0);
        assert!((p - Vec2::new(2.0, 3.0)).length() < EPSILON);
    }

    #[test]
    fn scale_identity() {
        let p = scale(Vec2::new(3.0, 5.0), 1.0);
        assert!((p - Vec2::new(3.0, 5.0)).length() < EPSILON);
    }
}
