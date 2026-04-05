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
//! - Frustum — sphere and AABB culling against six half-spaces

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

// ── Research module — comparison baselines not for production use ─────────────
pub mod research;

// ── Approach C: Rectangular Partitions ───────────────────────────────────────
//
// Two strategies over the same geometry, kept side by side for benchmarking:
//
//   poisson_rect_partitioned  — Strategy A: run Bridson inside each partition
//   scatter_cull_rect         — Strategy B: scatter-cull inside each partition
//
// Both return Vec<Vec<(f32, f32)>> — one inner Vec per partition.
// Points within each partition satisfy min_dist from each other.
// Seam handling differs per strategy (see each fn).

use prime_random::{poisson_disk, prng_next};
use prime_voronoi::{voronoi_sites_seeded, voronoi_partition, lloyd_relax_n};

// Shared: build a spatial acceptance grid and cull a point set down to min-dist validity.
// Used by scatter_cull_rect's cull phase. Returns the surviving points.
// ADVANCE-EXCEPTION: cull loop terminates when all candidates are processed — bounded.
fn cull_to_min_dist(
    candidates: Vec<(f32, f32)>,
    x_start: f32,
    y_start: f32,
    width: f32,
    height: f32,
    min_dist: f32,
) -> Vec<(f32, f32)> {
    let cell_size   = min_dist / 2.0_f32.sqrt();
    let cols        = (width  / cell_size).ceil() as usize + 1;
    let rows        = (height / cell_size).ceil() as usize + 1;
    let min_dist_sq = min_dist * min_dist;

    let mut grid:     Vec<Option<usize>> = vec![None; cols * rows];
    let mut accepted: Vec<(f32, f32)>    = Vec::new();

    for (wx, wy) in candidates {
        // Translate to local partition coordinates for grid indexing
        let lx = wx - x_start;
        let ly = wy - y_start;

        if lx < 0.0 || lx >= width || ly < 0.0 || ly >= height {
            continue;
        }

        let gcx = (lx / cell_size) as usize;
        let gcy = (ly / cell_size) as usize;

        let too_close = (gcy.saturating_sub(2)..(gcy + 3).min(rows))
            .flat_map(|gy| (gcx.saturating_sub(2)..(gcx + 3).min(cols))
                .map(move |gx| (gx, gy)))
            .filter_map(|(gx, gy)| grid[gy * cols + gx])
            .any(|pi| {
                let (qx, qy) = accepted[pi];
                let dx = wx - qx;
                let dy = wy - qy;
                dx * dx + dy * dy < min_dist_sq
            });

        if !too_close {
            let idx      = accepted.len();
            grid[gcy * cols + gcx] = Some(idx);
            accepted.push((wx, wy));
        }
    }

    accepted
}

/// Approach C — Strategy A: Rectangular partitions with Bridson per cell.
///
/// # Math
///
/// Divides the domain into `partition_cols × partition_rows` equal rectangles.
/// Runs `poisson_disk` independently inside each cell with a seam inset of
/// $\frac{d_{min}}{2}$ on shared edges, so no cross-partition distance checks
/// are needed. Per-partition seed:
///
/// $$s_{r,c} = s_0 \cdot 6364136223846793005 + (r \cdot P_c + c)$$
///
/// # Arguments
/// * `width`, `height`       — domain dimensions (must be > 0)
/// * `min_dist`              — minimum distance between any two points
/// * `max_attempts`          — Bridson candidate attempts per active point (30 standard)
/// * `partition_cols`        — number of columns in the rectangular grid
/// * `partition_rows`        — number of rows in the rectangular grid
/// * `seed`                  — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per partition (row-major order).
/// Points are in world coordinates. Empty Vec on invalid inputs.
///
/// # Edge cases
/// Returns empty if any dimension or `min_dist` is ≤ 0, or if partition count is 0.
///
/// # Example
/// ```rust
/// # use prime_spatial::poisson_rect_partitioned;
/// let partitions = poisson_rect_partitioned(100.0, 100.0, 5.0, 30, 4, 4, 42);
/// assert_eq!(partitions.len(), 16);
/// assert!(partitions.iter().all(|p| !p.is_empty()));
/// ```
pub fn poisson_rect_partitioned(
    width: f32,
    height: f32,
    min_dist: f32,
    max_attempts: usize,
    partition_cols: usize,
    partition_rows: usize,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || partition_cols == 0 || partition_rows == 0
    {
        return Vec::new();
    }

    let cell_w = width  / partition_cols as f32;
    let cell_h = height / partition_rows as f32;
    let inset  = min_dist / 2.0;

    (0..partition_rows).flat_map(|row| {
        (0..partition_cols).map(move |col| {
            // Per-partition seed — wrapping_mul mix avoids correlation
            let p_seed = (seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((row * partition_cols + col) as u64)
                as u32;

            let x_start = col as f32 * cell_w + inset;
            let y_start = row as f32 * cell_h + inset;
            let p_width  = (cell_w - inset * 2.0).max(0.0);
            let p_height = (cell_h - inset * 2.0).max(0.0);

            if p_width <= min_dist || p_height <= min_dist {
                return Vec::new();
            }

            // Run Bridson in local coordinates, translate to world
            poisson_disk(p_width, p_height, min_dist, max_attempts, p_seed)
                .into_iter()
                .map(|(x, y)| (x + x_start, y + y_start))
                .collect()
        })
    })
    .collect()
}

/// Approach C — Strategy B: Rectangular partitions with scatter-cull per cell.
///
/// # Math
///
/// Divides the domain into `partition_cols × partition_rows` equal rectangles.
/// Each cell receives a pure random drop of $N = \lfloor target \cdot r \rfloor$ points
/// where $r$ is the overage ratio. A cull pass then removes points violating $d_{min}$.
/// No seam inset — boundary conflicts are resolved from surplus during the cull pass.
///
/// Per-partition seed uses the same mixing strategy as `poisson_rect_partitioned`.
///
/// # Arguments
/// * `width`, `height`          — domain dimensions (must be > 0)
/// * `min_dist`                 — minimum distance between any two points
/// * `partition_cols`           — number of columns
/// * `partition_rows`           — number of rows
/// * `target_per_partition`     — desired survivors per partition
/// * `overage_ratio`            — drop multiplier, e.g. 1.5 for 50% overage
/// * `seed`                     — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per partition (row-major order).
/// Points are in world coordinates. Empty Vec on invalid inputs.
///
/// # Edge cases
/// Returns empty if any dimension, `min_dist`, or `overage_ratio` is ≤ 0,
/// or if partition count or target is 0.
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_rect;
/// let partitions = scatter_cull_rect(100.0, 100.0, 5.0, 4, 4, 250, 1.5, 42);
/// assert_eq!(partitions.len(), 16);
/// ```
pub fn scatter_cull_rect(
    width: f32,
    height: f32,
    min_dist: f32,
    partition_cols: usize,
    partition_rows: usize,
    target_per_partition: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0 || overage_ratio <= 0.0
        || partition_cols == 0 || partition_rows == 0 || target_per_partition == 0
    {
        return Vec::new();
    }

    let cell_w   = width  / partition_cols as f32;
    let cell_h   = height / partition_rows as f32;
    let drop_n   = (target_per_partition as f32 * overage_ratio) as usize;

    (0..partition_rows).flat_map(|row| {
        (0..partition_cols).map(move |col| {
            let p_seed = (seed as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((row * partition_cols + col) as u64)
                as u32;

            let x_start = col as f32 * cell_w;
            let y_start = row as f32 * cell_h;

            // Phase 1 — Scatter: pure random drop, no distance checking.
            // scan threads PRNG state forward — the canonical pure pattern for
            // generating a sequence of seeded values.
            let candidates: Vec<(f32, f32)> = (0..drop_n)
                .scan(p_seed, |s, _| {
                    let (xf, s1) = prng_next(*s);
                    let (yf, s2) = prng_next(s1);
                    *s = s2;
                    Some((x_start + xf * cell_w, y_start + yf * cell_h))
                })
                .collect();

            // Phase 2 — Cull: apply min-dist constraint as post-pass
            cull_to_min_dist(candidates, x_start, y_start, cell_w, cell_h, min_dist)
        })
    })
    .collect()
}

// ── Approach D: Voronoi K₁₀ Scatter-Cull ─────────────────────────────────────
//
// Strategy B (scatter-cull) over irregular Voronoi cells.
//
// Steps:
//   1. Generate K random Voronoi sites in the domain.
//   2. Lloyd-relax the sites for `lloyd_iters` steps → centroidal placement.
//   3. Scatter `k * target_per_cell * overage_ratio` random candidates in the domain.
//   4. Partition candidates by nearest site → one candidate set per Voronoi cell.
//   5. Cull each cell's candidates to min-dist validity.
//
// No Bridson reference for Approach D in this file — partition-Bridson over
// Voronoi cells is structurally identical to approach C-A (run Bridson per
// cell) and adds no new geometric information. The interesting comparison is
// scatter-cull Voronoi (D) vs scatter-cull rectangular (C-B).

/// Scatter-cull Poisson disk sampling over K Voronoi cells (Approach D).
///
/// Generates `k` Voronoi sites, Lloyd-relaxes them for `lloyd_iters` steps,
/// then applies scatter-cull independently inside each Voronoi cell.
///
/// Unlike rectangular partitions, Voronoi cells are irregular polygons — their
/// boundaries are implicitly defined by nearest-site distance. No explicit
/// cell polygon computation is required: a candidate point belongs to the cell
/// whose site is nearest.
///
/// # Math
///
/// **Site placement:**
/// Random sites $\{s_i\}_{i=0}^{k-1}$ drawn from the domain, then moved toward
/// centroidal positions via sample-based Lloyd relaxation:
///
/// $$s_i^{(t+1)} = \frac{1}{|\mathcal{C}_i^{(t)}|} \sum_{p \in \mathcal{C}_i^{(t)}} p$$
///
/// where $\mathcal{C}_i^{(t)}$ is the set of grid samples nearest to $s_i$ at step $t$.
///
/// **Scatter:**
/// Total candidates $N = k \cdot \text{target} \cdot \text{overage}$ scattered uniformly
/// in the full domain:
///
/// $$N = k \cdot \text{target\_per\_cell} \cdot \text{overage\_ratio}$$
///
/// **Partition:**
/// $$\text{cell}_i = \bigl\{\, p \in \text{candidates} : \arg\min_j \lVert p - s_j \rVert = i \,\bigr\}$$
///
/// **Cull:** each cell's candidates filtered to min-dist validity using the
/// same O(1)-grid cull as `scatter_cull_rect`.
///
/// # Arguments
/// * `width`, `height`      — domain dimensions (must be > 0)
/// * `min_dist`             — minimum allowed distance between any two accepted points
/// * `k`                    — number of Voronoi sites (partitions)
/// * `lloyd_iters`          — Lloyd relaxation steps (0 = use raw random sites)
/// * `target_per_cell`      — approximate desired survivors per Voronoi cell
/// * `overage_ratio`        — scatter multiplier (>1.0 ensures surplus; 1.5 recommended)
/// * `seed`                 — deterministic seed
///
/// # Returns
/// `Vec<Vec<(f32, f32)>>` — one inner Vec per Voronoi cell (length = `k`).
/// Points within each cell satisfy min-dist. Cross-cell min-dist is NOT
/// guaranteed — boundary conflicts exist at cell seams (same as Approach C).
///
/// # Example
/// ```rust
/// # use prime_spatial::scatter_cull_voronoi;
/// let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
/// assert_eq!(cells.len(), 10);
/// assert!(cells.iter().map(|c| c.len()).sum::<usize>() > 0);
/// ```
pub fn scatter_cull_voronoi(
    width: f32,
    height: f32,
    min_dist: f32,
    k: usize,
    lloyd_iters: usize,
    target_per_cell: usize,
    overage_ratio: f32,
    seed: u32,
) -> Vec<Vec<(f32, f32)>> {
    if width <= 0.0 || height <= 0.0 || min_dist <= 0.0
        || overage_ratio <= 0.0 || k == 0 || target_per_cell == 0
    {
        return Vec::new();
    }

    // Step 1 — Generate K random sites in the domain.
    let raw_sites = voronoi_sites_seeded(k, 0.0, 0.0, width, height, seed);

    // Step 2 — Lloyd-relax sites toward centroidal positions.
    // Sample the domain with a regular grid for centroid estimation.
    // Grid density: sqrt(k) * 4 samples per axis — enough for accurate centroids
    // without excessive allocation.
    let lloyd_sites = if lloyd_iters == 0 {
        raw_sites
    } else {
        let grid_n    = ((k as f32).sqrt() as usize * 4).max(10);
        let step_x    = width  / grid_n as f32;
        let step_y    = height / grid_n as f32;
        let samples: Vec<(f32, f32)> = (0..grid_n)
            .flat_map(|row| {
                (0..grid_n).map(move |col| {
                    (col as f32 * step_x + step_x * 0.5,
                     row as f32 * step_y + step_y * 0.5)
                })
            })
            .collect();
        lloyd_relax_n(&raw_sites, &samples, lloyd_iters)
    };

    // Step 3 — Scatter candidates uniformly across the full domain.
    // Seed for scatter phase derived from the input seed, different from site seed.
    let scatter_seed = seed.wrapping_mul(1_664_525u32).wrapping_add(1_013_904_223u32);
    let total_candidates = k * target_per_cell * overage_ratio as usize;

    let candidates: Vec<(f32, f32)> = (0..total_candidates)
        .scan(scatter_seed, |s, _| {
            let (xf, s1) = prng_next(*s);
            let (yf, s2) = prng_next(s1);
            *s = s2;
            Some((xf * width, yf * height))
        })
        .collect();

    // Step 4 — Partition candidates by nearest Voronoi site.
    let partitioned = voronoi_partition(&lloyd_sites, &candidates);

    // Step 5 — Cull each Voronoi cell's candidates to min-dist validity.
    // Voronoi cells are irregular — use a conservative bounding box for the
    // cull grid (the full domain) since we don't know exact cell extents.
    // The cull grid still achieves O(1) per candidate — cell bounding boxes
    // are an optimisation left for when performance data motivates it.
    partitioned
        .into_iter()
        .map(|cell_candidates| {
            cull_to_min_dist(cell_candidates, 0.0, 0.0, width, height, min_dist)
        })
        .collect()
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

    #[test]
    fn frustum_cull_sphere_large_sphere_spans_frustum_not_culled() {
        // Sphere so large it spans the entire frustum — should not be culled.
        let planes = unit_cube_frustum();
        assert!(!frustum_cull_sphere(&planes, (0.0, 0.0, 0.0), 100.0));
    }

    // ── degenerate AABB (min == max, zero-volume point) ───────────────────────

    #[test]
    fn aabb_overlap_degenerate_touching() {
        assert!(aabb_overlaps((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    }

    #[test]
    fn aabb_overlap_degenerate_apart() {
        assert!(!aabb_overlaps((0.0, 0.0, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    }

    #[test]
    fn aabb_contains_degenerate_on_point() {
        assert!(aabb_contains((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
        assert!(!aabb_contains((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.1)));
    }

    #[test]
    fn aabb_closest_point_degenerate() {
        let cp = aabb_closest_point((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (5.0, 5.0, 5.0));
        assert!(approx_eq3(cp, (1.0, 1.0, 1.0)));
    }

    // ── zero-direction ray (one component = 0) ────────────────────────────────

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

    // ── scatter_cull_voronoi ─────────────────────────────────────────────────

    #[test]
    fn scatter_cull_voronoi_cell_count() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        assert_eq!(cells.len(), 10, "should return exactly k cells");
    }

    #[test]
    fn scatter_cull_voronoi_invalid_inputs_return_empty() {
        assert!(scatter_cull_voronoi(0.0, 100.0, 5.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 0.0, 5.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 0.0, 10, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 5.0, 0, 3, 20, 1.5, 42).is_empty());
        assert!(scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 0, 1.5, 42).is_empty());
    }

    #[test]
    fn scatter_cull_voronoi_min_dist_holds() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let min_dist = 5.0_f32;
        for cell in &cells {
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let dx = cell[i].0 - cell[j].0;
                    let dy = cell[i].1 - cell[j].1;
                    let dist = (dx * dx + dy * dy).sqrt();
                    assert!(
                        dist >= min_dist - 1e-4,
                        "min-dist violated within cell: {dist:.4} < {min_dist}"
                    );
                }
            }
        }
    }

    #[test]
    fn scatter_cull_voronoi_deterministic() {
        let a = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let b = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        assert_eq!(a, b, "same seed must produce identical output");
    }

    #[test]
    fn scatter_cull_voronoi_different_seeds_differ() {
        let a = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 42);
        let b = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 20, 1.5, 99);
        // Different seeds should almost certainly produce different outputs
        let total_a: usize = a.iter().map(|c| c.len()).sum();
        let total_b: usize = b.iter().map(|c| c.len()).sum();
        // At least point counts differ or positions differ
        assert!(total_a != total_b || a != b);
    }

    #[test]
    fn scatter_cull_voronoi_zero_lloyd_iters() {
        // Should work fine with no relaxation — sites are raw random
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 0, 20, 1.5, 42);
        assert_eq!(cells.len(), 10);
    }

    // ── Statistical: coverage uniformity ────────────────────────────────────
    // Divide the domain into a coarse grid, count points per cell, compute
    // coefficient of variation (CV = stddev/mean). Lower CV = more uniform.
    // This is an observational test — no hard threshold, just a sanity check
    // that we get some points in most cells.

    fn coverage_cv(points: &[(f32, f32)], width: f32, height: f32, grid_n: usize) -> f32 {
        let cell_w = width  / grid_n as f32;
        let cell_h = height / grid_n as f32;
        let counts: Vec<f32> = (0..grid_n).flat_map(|row| {
            (0..grid_n).map(move |col| {
                points.iter().filter(|&&(x, y)| {
                    let c = (x / cell_w) as usize;
                    let r = (y / cell_h) as usize;
                    c == col && r == row
                }).count() as f32
            })
        }).collect();
        let mean = counts.iter().sum::<f32>() / counts.len() as f32;
        if mean < 1e-6 { return 0.0; }
        let var  = counts.iter().map(|&c| (c - mean).powi(2)).sum::<f32>() / counts.len() as f32;
        var.sqrt() / mean  // coefficient of variation
    }

    #[test]
    fn scatter_cull_voronoi_coverage_nonzero() {
        let cells = scatter_cull_voronoi(100.0, 100.0, 5.0, 10, 3, 30, 1.5, 42);
        let flat: Vec<(f32, f32)> = cells.into_iter().flatten().collect();
        assert!(!flat.is_empty(), "should produce at least some points");
        let cv = coverage_cv(&flat, 100.0, 100.0, 5);
        // Just log the CV — this is observational, not a pass/fail threshold
        let _ = cv; // CV will be reported in benchmarks
    }
}
