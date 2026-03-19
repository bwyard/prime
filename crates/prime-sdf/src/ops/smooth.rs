/// Smooth union of two SDF values (polynomial smooth min).
///
/// # Math
/// From Inigo Quilez (iquilezles.org/articles/distfunctions/):
///
///   h = max(k - |d1 - d2|, 0) / k
///   smooth_union = min(d1, d2) - h*h*k*0.25
///
/// As k → 0, approaches regular union. Larger k = wider blend.
///
/// # Arguments
/// * `d1`, `d2` - SDF values to blend
/// * `k` - blend radius (> 0)
pub fn smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = ((k - (d1 - d2).abs()) / k).max(0.0);
    d1.min(d2) - h * h * k * 0.25
}

/// Smooth intersection of two SDF values.
///
/// # Math
///   h = max(k - |d1 - d2|, 0) / k
///   smooth_intersection = max(d1, d2) + h*h*k*0.25
pub fn smooth_intersection(d1: f32, d2: f32, k: f32) -> f32 {
    let h = ((k - (d1 - d2).abs()) / k).max(0.0);
    d1.max(d2) + h * h * k * 0.25
}

/// Smooth subtraction of d2 from d1.
///
/// # Math
///   smooth_subtract(d1, d2, k) = smooth_intersection(d1, -d2, k)
pub fn smooth_subtract(d1: f32, d2: f32, k: f32) -> f32 {
    smooth_intersection(d1, -d2, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_union_approaches_union_as_k_approaches_zero() {
        let d1 = 1.0_f32;
        let d2 = 2.0_f32;
        let su = smooth_union(d1, d2, 0.001);
        let u = d1.min(d2);
        assert!((su - u).abs() < 0.01);
    }

    #[test]
    fn smooth_union_blends_near_boundary() {
        let d = 1.0_f32;
        let k = 0.5_f32;
        assert!(smooth_union(d, d, k) < d);
    }

    #[test]
    fn smooth_intersection_approaches_intersection() {
        let d1 = 1.0_f32;
        let d2 = 2.0_f32;
        let si = smooth_intersection(d1, d2, 0.001);
        let i = d1.max(d2);
        assert!((si - i).abs() < 0.01);
    }
}
