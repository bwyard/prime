/// Coverage uniformity statistics for all scatter-cull approaches.
///
/// Each test runs an approach, then applies global_cull_to_min_dist to produce
/// output equivalent to Bridson (global min-dist guaranteed). Reports:
///
///   - Total accepted points (post global cull)
///   - Coverage CV at two grid resolutions (lower = more uniform)
///   - Empty cells in a 5×5 grid
///   - Seam survivor rate = points kept after global cull / points before
///   - Min-dist hold rate (must always be 1.0 after global cull)
///
/// Calibration tests (binary search for overage) are in calibration.rs.
/// Run with: cargo test -p prime-spatial --test coverage_stats -- --nocapture

use prime_spatial::{
    scatter_cull_rect,
    scatter_cull_voronoi,
    scatter_cull_voronoi_recursive,
    scatter_cull_sheared,
    scatter_cull_half_heart,
    scatter_global_rect,
    scatter_global_voronoi,
    scatter_global_half_heart,
    scatter_cull_sdf_ellipse,
    scatter_global_sdf_ellipse,
    scatter_cull_clipped_circle,
    scatter_global_clipped_circle,
    scatter_cull_sdf_ellipse_inset,
    scatter_global_sdf_ellipse_inset,
    scatter_global_sdf_ellipse_corner_fill,
    scatter_cull_triangles,
    scatter_global_triangles,
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

// ── 100×100 — all approaches at fixed overage ─────────────────────────────────

#[test]
fn coverage_comparison_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Coverage uniformity — 100×100, min_dist=5.0 (post global cull) ===");
    println!("  seam_kept = fraction of intra-cell points surviving global min-dist pass");
    println!("  CV = coefficient of variation; lower = more uniform\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let cb_cells = scatter_cull_rect(w, h, d, 4, 4, 30, 1.5, s);
    let cb_before: usize = cb_cells.iter().map(|c| c.len()).sum();
    let cb = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("C-B scatter-cull rect 4x4", cb_before, &cb, w, h, d);

    let d_cells = scatter_cull_voronoi(w, h, d, 10, 3, 30, 1.5, s);
    let d_before: usize = d_cells.iter().map(|c| c.len()).sum();
    let dv = global_cull_to_min_dist(d_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D   scatter-cull Voronoi K=10", d_before, &dv, w, h, d);

    let dr_cells = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 300, 1.5, s);
    let dr_before: usize = dr_cells.iter().map(|c| c.len()).sum();
    let dr = global_cull_to_min_dist(dr_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("D-R recursive Voronoi L=2", dr_before, &dr, w, h, d);

    let fv_cells = scatter_cull_sheared(w, h, d, 4, 4, 0.0, 30, 2.0, s);
    let fv_before: usize = fv_cells.iter().map(|c| c.len()).sum();
    let fv = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   variable-rect (shear=0)", fv_before, &fv, w, h, d);

    let fs_cells = scatter_cull_sheared(w, h, d, 4, 4, 0.5, 30, 2.0, s);
    let fs_before: usize = fs_cells.iter().map(|c| c.len()).sum();
    let fs = global_cull_to_min_dist(fs_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("F   shear=0.5 brick", fs_before, &fs, w, h, d);

    let e1_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -9.0, 6.0, 30, 1.5, s);
    let e1_before: usize = e1_cells.iter().map(|c| c.len()).sum();
    let e1 = global_cull_to_min_dist(e1_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-9,6)", e1_before, &e1, w, h, d);

    let e2_cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 30, 1.5, s);
    let e2_before: usize = e2_cells.iter().map(|c| c.len()).sum();
    let e2 = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("E   half-heart shift(-15,10)", e2_before, &e2, w, h, d);

    println!();
}

// ── 200×200 — all approaches at fixed overage ─────────────────────────────────

#[test]
fn coverage_comparison_200x200() {
    let w = 200.0_f32;
    let h = 200.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Coverage uniformity — 200×200, min_dist=5.0 (post global cull) ===");
    println!("  seam_kept = fraction of intra-cell points surviving global min-dist pass\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

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

// ── Single-pass global-cull — 100×100 ────────────────────────────────────────

#[test]
fn coverage_single_pass_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;
    let pi4 = std::f32::consts::FRAC_PI_4;

    println!("\n=== Single-pass global cull — 100×100, min_dist=5.0 ===");
    println!("  seam_kept is always 1.0 here (no per-cell cull)");
    println!("  Compare point count vs Bridson and vs two-pass pipeline at same overage\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference, overage=N/A)", bridson.len(), &bridson, w, h, d);

    let cb2_cells = scatter_cull_rect(w, h, d, 4, 4, 30, 2.0, s);
    let cb2_before: usize = cb2_cells.iter().map(|c| c.len()).sum();
    let cb2 = global_cull_to_min_dist(cb2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("C-B two-pass (overage=2.0)", cb2_before, &cb2, w, h, d);

    let gr = scatter_global_rect(w, h, d, 4, 4, 30, 2.0, s);
    print_stats("global_rect (overage=2.0)", gr.len(), &gr, w, h, d);

    let gv = scatter_global_voronoi(w, h, d, 10, 3, 30, 2.0, s);
    print_stats("global_voronoi K=10 (overage=2.0)", gv.len(), &gv, w, h, d);

    let ghh = scatter_global_half_heart(w, h, d, 5, pi4, -15.0, 10.0, 30, 2.0, s);
    print_stats("global_half_heart shift(-15,10) (overage=2.0)", ghh.len(), &ghh, w, h, d);

    println!("\n  Points summary:");
    println!("    Bridson:          {}", bridson.len());
    println!("    C-B two-pass:     {}", cb2.len());
    println!("    global_rect:      {}", gr.len());
    println!("    global_voronoi:   {}", gv.len());
    println!("    global_half_heart:{}", ghh.len());
    println!();
}

// ── Approach G: SDF Ellipse — 100×100 ────────────────────────────────────────

#[test]
fn coverage_approach_g_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Approach G — SDF Ellipse, 100×100, min_dist=5.0 ===");
    println!("  coverage=1.2 means ellipses extend 20% past grid midpoints");
    println!("  area-proportional scatter: drop_n = (pi*a*b / domain_area) * total * overage\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let g2c_cells = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, 5.0, s);
    let g2c_before: usize = g2c_cells.iter().map(|c| c.len()).sum();
    let g2c = global_cull_to_min_dist(g2c_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("G two-pass circle  aspect=1.0 (ov=5)", g2c_before, &g2c, w, h, d);

    let g2e_cells = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, 5.0, s);
    let g2e_before: usize = g2e_cells.iter().map(|c| c.len()).sum();
    let g2e = global_cull_to_min_dist(g2e_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("G two-pass ellipse aspect=2.0 (ov=5)", g2e_before, &g2e, w, h, d);

    let g1c = scatter_global_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, 5.0, s);
    print_stats("G single-pass circle  aspect=1.0 (ov=5)", g1c.len(), &g1c, w, h, d);

    let g1e = scatter_global_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, 5.0, s);
    print_stats("G single-pass ellipse aspect=2.0 (ov=5)", g1e.len(), &g1e, w, h, d);

    println!("\n  Points summary:");
    println!("    Bridson:                {}", bridson.len());
    println!("    G two-pass circle:      {}", g2c.len());
    println!("    G two-pass ellipse:     {}", g2e.len());
    println!("    G single-pass circle:   {}", g1c.len());
    println!("    G single-pass ellipse:  {}", g1e.len());
    println!();
}

// ── Approach G-CC: Clipped-Circle Cells — 100×100 ────────────────────────────

#[test]
fn coverage_clipped_circle_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Approach G-CC — Clipped-Circle Cells, 100×100, min_dist=5.0 ===");
    println!("  coverage=1.2: circle extends 20% past tile midpoint");
    println!("  coverage=1.45: circle covers tile corners (sqrt(2)/2 ≈ 1.414)\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let gcc2_cells = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.2, 300, 5.0, s);
    let gcc2_before: usize = gcc2_cells.iter().map(|c| c.len()).sum();
    let gcc2 = global_cull_to_min_dist(gcc2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("G-CC two-pass   cov=1.2 (ov=5)", gcc2_before, &gcc2, w, h, d);

    let gcc2h_cells = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.45, 300, 5.0, s);
    let gcc2h_before: usize = gcc2h_cells.iter().map(|c| c.len()).sum();
    let gcc2h = global_cull_to_min_dist(gcc2h_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("G-CC two-pass   cov=1.45 (ov=5)", gcc2h_before, &gcc2h, w, h, d);

    let gcc1 = scatter_global_clipped_circle(w, h, d, 4, 4, 1.2, 300, 5.0, s);
    print_stats("G-CC single-pass cov=1.2 (ov=5)", gcc1.len(), &gcc1, w, h, d);

    let gcc1h = scatter_global_clipped_circle(w, h, d, 4, 4, 1.45, 300, 5.0, s);
    print_stats("G-CC single-pass cov=1.45 (ov=5)", gcc1h.len(), &gcc1h, w, h, d);

    println!("\n  Points summary:");
    println!("    Bridson:                  {}", bridson.len());
    println!("    G-CC two-pass  cov=1.2:   {}", gcc2.len());
    println!("    G-CC two-pass  cov=1.45:  {}", gcc2h.len());
    println!("    G-CC single-pass cov=1.2: {}", gcc1.len());
    println!("    G-CC single-pass cov=1.45:{}", gcc1h.len());
    println!();
}

// ── Approach H: Triangle Cells — 100×100 ─────────────────────────────────────

#[test]
fn coverage_approach_h_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Approach H — Triangle Cells, 100×100, min_dist=5.0 ===");
    println!("  4×4 grid → 32 triangles; jitter=0.2 (±20% on interior vertices)");
    println!("  area-proportional scatter: drop_n = (tri_area / domain_area) * total * overage\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let h2_cells = scatter_cull_triangles(w, h, d, 4, 4, 0.2, 300, 5.0, s);
    let h2_before: usize = h2_cells.iter().map(|c: &Vec<(f32, f32)>| c.len()).sum();
    let h2 = global_cull_to_min_dist(h2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("H two-pass  tri 4x4 jitter=0.2 (ov=5)", h2_before, &h2, w, h, d);

    let h1 = scatter_global_triangles(w, h, d, 4, 4, 0.2, 300, 5.0, s);
    print_stats("H single-pass tri 4x4 jitter=0.2 (ov=5)", h1.len(), &h1, w, h, d);

    let h1r = scatter_global_triangles(w, h, d, 4, 4, 0.0, 300, 5.0, s);
    print_stats("H single-pass tri 4x4 jitter=0.0 (ov=5)", h1r.len(), &h1r, w, h, d);

    println!("\n  Points summary:");
    println!("    Bridson:                    {}", bridson.len());
    println!("    H two-pass  jitter=0.2:     {}", h2.len());
    println!("    H single-pass jitter=0.2:   {}", h1.len());
    println!("    H single-pass jitter=0.0:   {}", h1r.len());
    println!();
}

// ── Approach G-Inset + Corner Fill — 100×100 ─────────────────────────────────

#[test]
fn coverage_g_inset_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    println!("\n=== Approach G-Inset + Corner Fill, 100×100, min_dist=5.0 ===");
    println!("  G-Inset: a = cell_w/2 − d/2, b = cell_h/2 − d/2");
    println!("  Geometric guarantee: no seam violations in two-pass (seam_kept expected = 1.000)");
    println!("  G-Corner: G-Inset + circles of radius d/2 at every grid intersection\n");

    let bridson = poisson_disk(w, h, d, 30, s);
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let gi2_cells  = scatter_cull_sdf_ellipse_inset(w, h, d, 4, 4, 300, 5.0, s);
    let gi2_before: usize = gi2_cells.iter().map(|c| c.len()).sum();
    let gi2 = global_cull_to_min_dist(gi2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats("G-Inset two-pass   (ov=5)", gi2_before, &gi2, w, h, d);

    let gi1 = scatter_global_sdf_ellipse_inset(w, h, d, 4, 4, 300, 5.0, s);
    print_stats("G-Inset single-pass (ov=5)", gi1.len(), &gi1, w, h, d);

    let gc = scatter_global_sdf_ellipse_corner_fill(w, h, d, 4, 4, 300, 5.0, s);
    print_stats("G-Corner fill       (ov=5)", gc.len(), &gc, w, h, d);

    println!("\n  Points summary:");
    println!("    Bridson:               {}", bridson.len());
    println!("    G-Inset two-pass:      {}", gi2.len());
    println!("    G-Inset single-pass:   {}", gi1.len());
    println!("    G-Corner fill:         {}", gc.len());
    println!();
}
