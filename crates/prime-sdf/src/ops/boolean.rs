/// Boolean union of two SDF values.
///
/// # Math
///   union(d1, d2) = min(d1, d2)
///
/// Returns the point inside either shape.
pub fn union(d1: f32, d2: f32) -> f32 { d1.min(d2) }

/// Boolean intersection of two SDF values.
///
/// # Math
///   intersection(d1, d2) = max(d1, d2)
///
/// Returns the point inside both shapes.
pub fn intersection(d1: f32, d2: f32) -> f32 { d1.max(d2) }

/// Subtract shape 2 from shape 1.
///
/// # Math
///   subtract(d1, d2) = max(d1, -d2)
///
/// Returns the region inside d1 but outside d2.
pub fn subtract(d1: f32, d2: f32) -> f32 { d1.max(-d2) }

/// Exclusive OR of two SDF regions.
///
/// # Math
///   xor(d1, d2) = max(min(d1, d2), -max(d1, d2))
pub fn xor(d1: f32, d2: f32) -> f32 { d1.min(d2).max(-d1.max(d2)) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_takes_min() { assert_eq!(union(1.0, 2.0), 1.0); assert_eq!(union(-1.0, 2.0), -1.0); }
    #[test]
    fn intersection_takes_max() { assert_eq!(intersection(1.0, 2.0), 2.0); }
    #[test]
    fn subtract_removes_second() { assert!(subtract(1.0, -2.0) > 0.0); }
}
