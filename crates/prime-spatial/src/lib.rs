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
//! - Sampling — Poisson-disk scatter-cull strategies over various cell geometries

/// Floating-point epsilon used in parallelism and near-zero tests.
pub(crate) const EPS: f32 = 1e-5;

pub mod ray;
pub mod aabb;
pub mod frustum;
pub mod sampling;

pub use ray::{ray_aabb, ray_sphere, ray_plane};
pub use aabb::{aabb_overlaps, aabb_contains, aabb_union, aabb_closest_point};
pub use frustum::{frustum_cull_sphere, frustum_cull_aabb};
pub use sampling::{
    global_cull_to_min_dist,
    rect::{poisson_rect_partitioned, scatter_cull_rect, scatter_global_rect},
    voronoi::{scatter_cull_voronoi, scatter_cull_voronoi_recursive, scatter_global_voronoi},
    half_heart::{scatter_cull_half_heart, scatter_global_half_heart},
    sheared::scatter_cull_sheared,
    sdf::{scatter_cull_sdf_ellipse, scatter_global_sdf_ellipse,
          scatter_cull_clipped_circle, scatter_global_clipped_circle,
          scatter_cull_sdf_ellipse_inset, scatter_global_sdf_ellipse_inset,
          scatter_global_sdf_ellipse_corner_fill},
    triangle::{scatter_cull_triangles, scatter_global_triangles},
};

