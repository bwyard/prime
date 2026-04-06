/// Coverage uniformity statistics for all scatter-cull approaches.
///
/// Each test runs an approach, then applies global_cull_to_min_dist to produce
/// output equivalent to Bridson (global min-dist guaranteed). Reports:
///
///   - Total accepted points (post global cull)
///   - Coverage CV at two grid resolutions (lower = more uniform)
///   - Empty cells in a 5×5 grid
///   - Seam survivor rate = points kept after global cull / points before
///     (measures how much seam work the global cull has to do)
///   - Min-dist hold rate (must always be 1.0 after global cull)
///
/// These are observational — no hard thresholds except min-dist hold = 1.0.
/// Run with: cargo test -p prime-spatial --test coverage_stats -- --nocapture

use prime_spatial::{
    scatter_cull_rect,
    scatter_cull_voronoi,
    scatter_cull_voronoi_recursive,
    scatter_cull_sheared,
    scatter_cull_half_heart,
    global_cull_to_min_dist,
};
use prime_random::poisson_disk;

// ── Measurement helpers ───────────────────────────────────────────────────────

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
    if mean < 1e-6 { return f32::MAX; }
    let var  = counts.iter().map(|&c| (c - mean).powi(2)).sum::<f32>() / counts.len() as f32;
    var.sqrt() / mean
}

fn empty_cells(points: &[(f32, f32)], width: f32, height: f32, grid_n: usize) -> usize {
    let cell_w = width  / grid_n as f32;
    let cell_h = height / grid_n as f32;
    (0..grid_n).flat_map(|row| {
        (0..grid_n).map(move |col| {
            points.iter().filter(|&&(x, y)| {
                (x / cell_w) as usize == col && (y / cell_h) as usize == row
            }).count()
        })
    })
    .filter(|&c| c == 0)
    .count()
}

/// Verify global min-dist on a flat point set.
fn global_min_dist_holds(points: &[(f32, f32)], min_dist: f32) -> bool {
    let md2 = min_dist * min_dist;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let dx = points[i].0 - points[j].0;
            let dy = points[i].1 - points[j].1;
            if dx * dx + dy * dy < md2 - 1e-6 { return false; }
        }
    }
    true
}

fn print_stats(
    label: &str,
    before_global: usize,
    points: &[(f32, f32)],
    width: f32,
    height: f32,
    min_dist: f32,
) {
    let total     = points.len();
    let cv_5      = coverage_cv(points, width, height, 5);
    let cv_10     = coverage_cv(points, width, height, 10);
    let empty_5   = empty_cells(points, width, height, 5);
    let seam_rate = if before_global == 0 { 1.0 }
                    else { total as f32 / before_global as f32 };

    println!(
        "  {:<35} pts={:>4}  seam_kept={:.3}  CV(5x5)={:.3}  CV(10x10)={:.3}  empty(5x5)={}/25",
        label, total, seam_rate, cv_5, cv_10, empty_5
    );

    assert!(
        global_min_dist_holds(points, min_dist),
        "{label}: global min-dist violated after global cull"
    );
}

// ── 100×100 domain ────────────────────────────────────────────────────────────

#[test]
fn coverage_comparison_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Coverage uniformity — 100×100, min_dist=5.0 (post global cull) ===");
    println!("  seam_kept = fraction of intra-cell points surviving global min-dist pass");
    println!("  CV = coefficient of variation; lower = more uniform\n");

    // Reference: serial Bridson (global min-dist by construction)
    let bridson = poisson_disk(w, h, d, 30, s);
    let before  = bridson.len();
    print_stats("Bridson (reference)", before, &bridson, w, h, d);

    // C-B: scatter-cull rect
    let cb_cells = scatter_cull_rect(w, h, d, 4, 4, 30, 1.5, s);
    let cb_before: usize = cb_cells.iter().map(|c| c.len()).sum();
    let cb = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("C-B scatter-cull rect 4x4", cb_before, &cb, w, h, d);

    // D: Voronoi K=10
    let d_cells = scatter_cull_voronoi(w, h, d, 10, 3, 30, 1.5, s);
    let d_before: usize = d_cells.iter().map(|c| c.len()).sum();
    let dv = global_cull_to_min_dist(d_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D   scatter-cull Voronoi K=10", d_before, &dv, w, h, d);

    // D-R: recursive Voronoi L=2
    let dr_cells = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 300, 1.5, s);
    let dr_before: usize = dr_cells.iter().map(|c| c.len()).sum();
    let dr = global_cull_to_min_dist(dr_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D-R recursive Voronoi L=2", dr_before, &dr, w, h, d);

    // F: variable-rect
    let fv_cells = scatter_cull_sheared(w, h, d, 4, 4, 0.0, 30, 2.0, s);
    let fv_before: usize = fv_cells.iter().map(|c| c.len()).sum();
    let fv = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   variable-rect (shear=0)", fv_before, &fv, w, h, d);

    // F: shear=0.5
    let fs_cells = scatter_cull_sheared(w, h, d, 4, 4, 0.5, 30, 2.0, s);
    let fs_before: usize = fs_cells.iter().map(|c| c.len()).sum();
    let fs = global_cull_to_min_dist(fs_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   shear=0.5 brick", fs_before, &fs, w, h, d);

    // E: shift(-9, 6)
    let e1_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -9.0, 6.0, 30, 1.5, s);
    let e1_before: usize = e1_cells.iter().map(|c| c.len()).sum();
    let e1 = global_cull_to_min_dist(e1_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-9,6)", e1_before, &e1, w, h, d);

    // E: shift(-15, 10)
    let e2_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 30, 1.5, s);
    let e2_before: usize = e2_cells.iter().map(|c| c.len()).sum();
    let e2 = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-15,10)", e2_before, &e2, w, h, d);

    println!();
}

// ── 200×200 domain ────────────────────────────────────────────────────────────

#[test]
fn coverage_comparison_200x200() {
    let w = 200.0_f32;
    let h = 200.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Coverage uniformity — 200×200, min_dist=5.0 (post global cull) ===");
    println!("  seam_kept = fraction of intra-cell points surviving global min-dist pass\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    let before  = bridson.len();
    print_stats("Bridson (reference)", before, &bridson, w, h, d);

    let cb_cells = scatter_cull_rect(w, h, d, 6, 6, 30, 1.5, s);
    let cb_before: usize = cb_cells.iter().map(|c| c.len()).sum();
    let cb = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("C-B scatter-cull rect 6x6", cb_before, &cb, w, h, d);

    let d_cells = scatter_cull_voronoi(w, h, d, 10, 3, 50, 1.5, s);
    let d_before: usize = d_cells.iter().map(|c| c.len()).sum();
    let dv = global_cull_to_min_dist(d_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D   scatter-cull Voronoi K=10", d_before, &dv, w, h, d);

    let dr_cells = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 800, 1.5, s);
    let dr_before: usize = dr_cells.iter().map(|c| c.len()).sum();
    let dr = global_cull_to_min_dist(dr_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D-R recursive Voronoi L=2", dr_before, &dr, w, h, d);

    let fv_cells = scatter_cull_sheared(w, h, d, 6, 6, 0.0, 30, 2.0, s);
    let fv_before: usize = fv_cells.iter().map(|c| c.len()).sum();
    let fv = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   variable-rect (shear=0)", fv_before, &fv, w, h, d);

    let fs_cells = scatter_cull_sheared(w, h, d, 6, 6, 0.5, 30, 2.0, s);
    let fs_before: usize = fs_cells.iter().map(|c| c.len()).sum();
    let fs = global_cull_to_min_dist(fs_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   shear=0.5 brick", fs_before, &fs, w, h, d);

    let e1_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -9.0, 6.0, 80, 1.5, s);
    let e1_before: usize = e1_cells.iter().map(|c| c.len()).sum();
    let e1 = global_cull_to_min_dist(e1_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-9,6)", e1_before, &e1, w, h, d);

    let e2_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 80, 1.5, s);
    let e2_before: usize = e2_cells.iter().map(|c| c.len()).sum();
    let e2 = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-15,10)", e2_before, &e2, w, h, d);

    println!();
}
