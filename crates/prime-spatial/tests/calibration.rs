/// Calibrated overage tests — binary search for minimum overage to match Bridson density.
///
/// Each test binary-searches (25 iterations, ceiling=50) for the smallest overage value
/// that produces >= Bridson point count, then runs the approach at that overage and reports
/// the same stats as coverage_stats.rs.
///
/// Non-calibrated fixed-overage tests are in coverage_stats.rs.
/// Run with: cargo test -p prime-spatial --test calibration -- --nocapture

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
        "  {:<40} pts={:>4}  seam_kept={:.3}  CV(5x5)={:.3}  CV(10x10)={:.3}  empty(5x5)={}/25",
        label, total, seam_rate, cv_5, cv_10, empty_5
    );
    assert!(
        global_min_dist_holds(points, min_dist),
        "{label}: global min-dist violated after global cull"
    );
}

/// Binary search: find smallest overage in [1.0, ceiling] that achieves >= target points.
/// 25 bisection iterations → precision ≈ ceiling/2^25.
fn calibrate_overage<F: FnMut(f32) -> usize>(target: usize, ceiling: f32, mut f: F) -> f32 {
    let (lo, hi) = (0..25).fold((1.0f32, ceiling), |(lo, hi), _| {
        let mid = (lo + hi) / 2.0;
        if f(mid) >= target { (lo, mid) } else { (mid, hi) }
    });
    (lo + hi) / 2.0
}

// ── Calibrated two-pass — 100×100 ────────────────────────────────────────────

#[test]
fn calibrated_coverage_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated overage — 100×100, min_dist=5.0, target={} pts (ceiling=50) ===", target);
    println!("  Binary search (25 iter): minimum overage to reach Bridson density\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let cb_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_rect(w, h, d, 4, 4, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let cb_cells  = scatter_cull_rect(w, h, d, 4, 4, 30, cb_ov, s);
    let cb_before = cb_cells.iter().map(|c| c.len()).sum::<usize>();
    let cb        = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("C-B rect 4×4  (ov={:.3})", cb_ov), cb_before, &cb, w, h, d);

    let d_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_voronoi(w, h, d, 10, 3, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let d_cells  = scatter_cull_voronoi(w, h, d, 10, 3, 30, d_ov, s);
    let d_before = d_cells.iter().map(|c| c.len()).sum::<usize>();
    let dv       = global_cull_to_min_dist(d_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("D   Voronoi K=10 (ov={:.3})", d_ov), d_before, &dv, w, h, d);

    let dr_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let dr_cells  = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 300, dr_ov, s);
    let dr_before = dr_cells.iter().map(|c| c.len()).sum::<usize>();
    let dr        = global_cull_to_min_dist(dr_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("D-R recursive L=2 (ov={:.3})", dr_ov), dr_before, &dr, w, h, d);

    let fv_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sheared(w, h, d, 4, 4, 0.0, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let fv_cells  = scatter_cull_sheared(w, h, d, 4, 4, 0.0, 30, fv_ov, s);
    let fv_before = fv_cells.iter().map(|c| c.len()).sum::<usize>();
    let fv        = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("F   shear=0   (ov={:.3})", fv_ov), fv_before, &fv, w, h, d);

    let fs_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sheared(w, h, d, 4, 4, 0.5, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let fs_cells  = scatter_cull_sheared(w, h, d, 4, 4, 0.5, 30, fs_ov, s);
    let fs_before = fs_cells.iter().map(|c| c.len()).sum::<usize>();
    let fs        = global_cull_to_min_dist(fs_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("F   shear=0.5 (ov={:.3})", fs_ov), fs_before, &fs, w, h, d);

    let e1_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -9.0, 6.0, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let e1_cells  = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -9.0, 6.0, 30, e1_ov, s);
    let e1_before = e1_cells.iter().map(|c| c.len()).sum::<usize>();
    let e1        = global_cull_to_min_dist(e1_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("E   shift(-9,6)   (ov={:.3})", e1_ov), e1_before, &e1, w, h, d);

    let e2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let e2_cells  = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 30, e2_ov, s);
    let e2_before = e2_cells.iter().map(|c| c.len()).sum::<usize>();
    let e2        = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("E   shift(-15,10) (ov={:.3})", e2_ov), e2_before, &e2, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:   {}", target);
    println!("    C-B  ov={:.3} → {} pts ({:.1}%)", cb_ov, cb.len(), cb.len() as f32 / target as f32 * 100.0);
    println!("    D    ov={:.3} → {} pts ({:.1}%)", d_ov,  dv.len(), dv.len() as f32 / target as f32 * 100.0);
    println!("    D-R  ov={:.3} → {} pts ({:.1}%)", dr_ov, dr.len(), dr.len() as f32 / target as f32 * 100.0);
    println!("    F v  ov={:.3} → {} pts ({:.1}%)", fv_ov, fv.len(), fv.len() as f32 / target as f32 * 100.0);
    println!("    F s  ov={:.3} → {} pts ({:.1}%)", fs_ov, fs.len(), fs.len() as f32 / target as f32 * 100.0);
    println!("    E1   ov={:.3} → {} pts ({:.1}%)", e1_ov, e1.len(), e1.len() as f32 / target as f32 * 100.0);
    println!("    E2   ov={:.3} → {} pts ({:.1}%)", e2_ov, e2.len(), e2.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated single-pass — 100×100 ─────────────────────────────────────────

#[test]
fn calibrated_single_pass_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;
    let pi4 = std::f32::consts::FRAC_PI_4;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated single-pass — 100×100, min_dist=5.0, target={} pts (ceiling=50) ===", target);
    println!("  seam_kept=1.0 always (no per-cell cull; single global pass only)\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let gr_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_rect(w, h, d, 4, 4, 30, ov, s).len()
    });
    let gr = scatter_global_rect(w, h, d, 4, 4, 30, gr_ov, s);
    print_stats(&format!("global_rect (ov={:.3})", gr_ov), gr.len(), &gr, w, h, d);

    let gv_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_voronoi(w, h, d, 10, 3, 30, ov, s).len()
    });
    let gv = scatter_global_voronoi(w, h, d, 10, 3, 30, gv_ov, s);
    print_stats(&format!("global_voronoi K=10 (ov={:.3})", gv_ov), gv.len(), &gv, w, h, d);

    let ghh_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_half_heart(w, h, d, 5, pi4, -15.0, 10.0, 30, ov, s).len()
    });
    let ghh = scatter_global_half_heart(w, h, d, 5, pi4, -15.0, 10.0, 30, ghh_ov, s);
    print_stats(&format!("global_half_heart (-15,10) (ov={:.3})", ghh_ov), ghh.len(), &ghh, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:       {}", target);
    println!("    global_rect      ov={:.3} → {} pts ({:.1}%)", gr_ov,  gr.len(),  gr.len()  as f32 / target as f32 * 100.0);
    println!("    global_voronoi   ov={:.3} → {} pts ({:.1}%)", gv_ov,  gv.len(),  gv.len()  as f32 / target as f32 * 100.0);
    println!("    global_half_heart ov={:.3} → {} pts ({:.1}%)", ghh_ov, ghh.len(), ghh.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated approach G — 100×100 ──────────────────────────────────────────

#[test]
fn calibrated_approach_g_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated approach G — SDF Ellipse, 100×100, target={} pts (ceiling=50) ===", target);
    println!("  coverage=1.2; area-proportional drop_n per cell; total_target=300\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let g2c_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let g2c_cells  = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, g2c_ov, s);
    let g2c_before = g2c_cells.iter().map(|c| c.len()).sum::<usize>();
    let g2c        = global_cull_to_min_dist(g2c_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("G two-pass  circle  (ov={:.3})", g2c_ov), g2c_before, &g2c, w, h, d);

    let g2e_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let g2e_cells  = scatter_cull_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, g2e_ov, s);
    let g2e_before = g2e_cells.iter().map(|c| c.len()).sum::<usize>();
    let g2e        = global_cull_to_min_dist(g2e_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("G two-pass  ellipse (ov={:.3})", g2e_ov), g2e_before, &g2e, w, h, d);

    let g1c_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, ov, s).len()
    });
    let g1c = scatter_global_sdf_ellipse(w, h, d, 4, 4, 1.0, 1.2, 300, g1c_ov, s);
    print_stats(&format!("G single-pass circle  (ov={:.3})", g1c_ov), g1c.len(), &g1c, w, h, d);

    let g1e_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, ov, s).len()
    });
    let g1e = scatter_global_sdf_ellipse(w, h, d, 4, 4, 2.0, 1.2, 300, g1e_ov, s);
    print_stats(&format!("G single-pass ellipse (ov={:.3})", g1e_ov), g1e.len(), &g1e, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:     {}", target);
    println!("    G2-circle  ov={:.3} → {} pts ({:.1}%)", g2c_ov, g2c.len(), g2c.len() as f32 / target as f32 * 100.0);
    println!("    G2-ellipse ov={:.3} → {} pts ({:.1}%)", g2e_ov, g2e.len(), g2e.len() as f32 / target as f32 * 100.0);
    println!("    G1-circle  ov={:.3} → {} pts ({:.1}%)", g1c_ov, g1c.len(), g1c.len() as f32 / target as f32 * 100.0);
    println!("    G1-ellipse ov={:.3} → {} pts ({:.1}%)", g1e_ov, g1e.len(), g1e.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated G-CC — 100×100 ────────────────────────────────────────────────

#[test]
fn calibrated_clipped_circle_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated G-CC — Clipped Circle, 100×100, target={} pts (ceiling=50) ===", target);
    println!("  cov=1.2: circle extends 20% past tile midpoint");
    println!("  cov=1.45: circle covers corners (sqrt(2)/2 ≈ 1.414); total_target=300\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let gcc2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.2, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let gcc2_cells  = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.2, 300, gcc2_ov, s);
    let gcc2_before = gcc2_cells.iter().map(|c| c.len()).sum::<usize>();
    let gcc2        = global_cull_to_min_dist(gcc2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("G-CC two-pass  cov=1.2  (ov={:.3})", gcc2_ov), gcc2_before, &gcc2, w, h, d);

    let gcc2h_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.45, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let gcc2h_cells  = scatter_cull_clipped_circle(w, h, d, 4, 4, 1.45, 300, gcc2h_ov, s);
    let gcc2h_before = gcc2h_cells.iter().map(|c| c.len()).sum::<usize>();
    let gcc2h        = global_cull_to_min_dist(gcc2h_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("G-CC two-pass  cov=1.45 (ov={:.3})", gcc2h_ov), gcc2h_before, &gcc2h, w, h, d);

    let gcc1_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_clipped_circle(w, h, d, 4, 4, 1.2, 300, ov, s).len()
    });
    let gcc1 = scatter_global_clipped_circle(w, h, d, 4, 4, 1.2, 300, gcc1_ov, s);
    print_stats(&format!("G-CC single-pass cov=1.2  (ov={:.3})", gcc1_ov), gcc1.len(), &gcc1, w, h, d);

    let gcc1h_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_clipped_circle(w, h, d, 4, 4, 1.45, 300, ov, s).len()
    });
    let gcc1h = scatter_global_clipped_circle(w, h, d, 4, 4, 1.45, 300, gcc1h_ov, s);
    print_stats(&format!("G-CC single-pass cov=1.45 (ov={:.3})", gcc1h_ov), gcc1h.len(), &gcc1h, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:          {}", target);
    println!("    G-CC2 cov=1.2  ov={:.3} → {} pts ({:.1}%)", gcc2_ov,  gcc2.len(),  gcc2.len()  as f32 / target as f32 * 100.0);
    println!("    G-CC2 cov=1.45 ov={:.3} → {} pts ({:.1}%)", gcc2h_ov, gcc2h.len(), gcc2h.len() as f32 / target as f32 * 100.0);
    println!("    G-CC1 cov=1.2  ov={:.3} → {} pts ({:.1}%)", gcc1_ov,  gcc1.len(),  gcc1.len()  as f32 / target as f32 * 100.0);
    println!("    G-CC1 cov=1.45 ov={:.3} → {} pts ({:.1}%)", gcc1h_ov, gcc1h.len(), gcc1h.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated approach H — 100×100 ──────────────────────────────────────────

#[test]
fn calibrated_approach_h_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated approach H — Triangle Cells, 100×100, target={} pts (ceiling=50) ===", target);
    println!("  4×4 grid → 32 triangles; area-proportional drop; total_target=300\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let h2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_triangles(w, h, d, 4, 4, 0.2, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let h2_cells  = scatter_cull_triangles(w, h, d, 4, 4, 0.2, 300, h2_ov, s);
    let h2_before = h2_cells.iter().map(|c: &Vec<(f32, f32)>| c.len()).sum::<usize>();
    let h2        = global_cull_to_min_dist(h2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("H two-pass jitter=0.2 (ov={:.3})", h2_ov), h2_before, &h2, w, h, d);

    let h1_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_triangles(w, h, d, 4, 4, 0.2, 300, ov, s).len()
    });
    let h1 = scatter_global_triangles(w, h, d, 4, 4, 0.2, 300, h1_ov, s);
    print_stats(&format!("H single-pass jitter=0.2 (ov={:.3})", h1_ov), h1.len(), &h1, w, h, d);

    let h1r_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_triangles(w, h, d, 4, 4, 0.0, 300, ov, s).len()
    });
    let h1r = scatter_global_triangles(w, h, d, 4, 4, 0.0, 300, h1r_ov, s);
    print_stats(&format!("H single-pass jitter=0.0 (ov={:.3})", h1r_ov), h1r.len(), &h1r, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:     {}", target);
    println!("    H2 j=0.2  ov={:.3} → {} pts ({:.1}%)", h2_ov,  h2.len(),  h2.len()  as f32 / target as f32 * 100.0);
    println!("    H1 j=0.2  ov={:.3} → {} pts ({:.1}%)", h1_ov,  h1.len(),  h1.len()  as f32 / target as f32 * 100.0);
    println!("    H1 j=0.0  ov={:.3} → {} pts ({:.1}%)", h1r_ov, h1r.len(), h1r.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated G-Inset + G-Corner — 100×100 ──────────────────────────────────

#[test]
fn calibrated_g_inset_100x100() {
    let w = 100.0_f32;
    let h = 100.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated G-Inset + G-Corner, 100×100, target={} pts (ceiling=50) ===", target);
    println!("  G-Inset: ellipse inset by d/2; geometric guarantee seam_kept=1.0");
    println!("  G-Corner: G-Inset + corner fill circles of radius d/2; total_target=300\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let gi2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sdf_ellipse_inset(w, h, d, 4, 4, 300, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let gi2_cells  = scatter_cull_sdf_ellipse_inset(w, h, d, 4, 4, 300, gi2_ov, s);
    let gi2_before = gi2_cells.iter().map(|c| c.len()).sum::<usize>();
    let gi2        = global_cull_to_min_dist(gi2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("G-Inset two-pass   (ov={:.3})", gi2_ov), gi2_before, &gi2, w, h, d);

    let gi1_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse_inset(w, h, d, 4, 4, 300, ov, s).len()
    });
    let gi1 = scatter_global_sdf_ellipse_inset(w, h, d, 4, 4, 300, gi1_ov, s);
    print_stats(&format!("G-Inset single-pass (ov={:.3})", gi1_ov), gi1.len(), &gi1, w, h, d);

    let gc_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse_corner_fill(w, h, d, 4, 4, 300, ov, s).len()
    });
    let gc = scatter_global_sdf_ellipse_corner_fill(w, h, d, 4, 4, 300, gc_ov, s);
    print_stats(&format!("G-Corner fill       (ov={:.3})", gc_ov), gc.len(), &gc, w, h, d);

    println!("\n  Calibrated summary:");
    println!("    Bridson target:    {}", target);
    println!("    G-Inset2 ov={:.3} → {} pts ({:.1}%)", gi2_ov, gi2.len(), gi2.len() as f32 / target as f32 * 100.0);
    println!("    G-Inset1 ov={:.3} → {} pts ({:.1}%)", gi1_ov, gi1.len(), gi1.len() as f32 / target as f32 * 100.0);
    println!("    G-Corner ov={:.3} → {} pts ({:.1}%)", gc_ov,  gc.len(),  gc.len()  as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated two-pass — 200×200 ────────────────────────────────────────────

#[test]
fn calibrated_coverage_200x200() {
    let w = 200.0_f32;
    let h = 200.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated overage — 200×200, min_dist=5.0, target={} pts (ceiling=50) ===", target);
    println!("  Binary search (25 iter): minimum overage to reach Bridson density\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let cb_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_rect(w, h, d, 6, 6, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let cb_cells  = scatter_cull_rect(w, h, d, 6, 6, 30, cb_ov, s);
    let cb_before = cb_cells.iter().map(|c| c.len()).sum::<usize>();
    let cb        = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("C-B rect 6×6  (ov={:.3})", cb_ov), cb_before, &cb, w, h, d);

    let fv_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sheared(w, h, d, 6, 6, 0.0, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let fv_cells  = scatter_cull_sheared(w, h, d, 6, 6, 0.0, 30, fv_ov, s);
    let fv_before = fv_cells.iter().map(|c| c.len()).sum::<usize>();
    let fv        = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("F   shear=0   (ov={:.3})", fv_ov), fv_before, &fv, w, h, d);

    let fs_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sheared(w, h, d, 6, 6, 0.5, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let fs_cells  = scatter_cull_sheared(w, h, d, 6, 6, 0.5, 30, fs_ov, s);
    let fs_before = fs_cells.iter().map(|c| c.len()).sum::<usize>();
    let fs        = global_cull_to_min_dist(fs_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("F   shear=0.5 (ov={:.3})", fs_ov), fs_before, &fs, w, h, d);

    let e2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 80, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let e2_cells  = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 80, e2_ov, s);
    let e2_before = e2_cells.iter().map(|c| c.len()).sum::<usize>();
    let e2        = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("E   shift(-15,10) (ov={:.3})", e2_ov), e2_before, &e2, w, h, d);

    let gr_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_rect(w, h, d, 6, 6, 30, ov, s).len()
    });
    let gr = scatter_global_rect(w, h, d, 6, 6, 30, gr_ov, s);
    print_stats(&format!("global_rect 6×6 (ov={:.3})", gr_ov), gr.len(), &gr, w, h, d);

    let ghh_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 80, ov, s).len()
    });
    let ghh = scatter_global_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 80, ghh_ov, s);
    print_stats(&format!("global_half_heart (ov={:.3})", ghh_ov), ghh.len(), &ghh, w, h, d);

    println!("\n  Calibrated summary (200×200):");
    println!("    Bridson target: {}", target);
    println!("    C-B  ov={:.3} → {} pts ({:.1}%)", cb_ov,  cb.len(),  cb.len()  as f32 / target as f32 * 100.0);
    println!("    F v  ov={:.3} → {} pts ({:.1}%)", fv_ov,  fv.len(),  fv.len()  as f32 / target as f32 * 100.0);
    println!("    F s  ov={:.3} → {} pts ({:.1}%)", fs_ov,  fs.len(),  fs.len()  as f32 / target as f32 * 100.0);
    println!("    E2   ov={:.3} → {} pts ({:.1}%)", e2_ov,  e2.len(),  e2.len()  as f32 / target as f32 * 100.0);
    println!("    gr   ov={:.3} → {} pts ({:.1}%)", gr_ov,  gr.len(),  gr.len()  as f32 / target as f32 * 100.0);
    println!("    ghh  ov={:.3} → {} pts ({:.1}%)", ghh_ov, ghh.len(), ghh.len() as f32 / target as f32 * 100.0);
    println!();
}

// ── Calibrated two-pass — 500×500 ────────────────────────────────────────────

#[test]
fn calibrated_coverage_500x500() {
    let w = 500.0_f32;
    let h = 500.0_f32;
    let d = 5.0_f32;
    let s = 42u32;

    let bridson = poisson_disk(w, h, d, 30, s);
    let target  = bridson.len();

    println!("\n=== Calibrated overage — 500×500, min_dist=5.0, target={} pts (ceiling=50) ===", target);
    println!("  Binary search (25 iter): minimum overage to reach Bridson density\n");
    print_stats("Bridson (reference)", bridson.len(), &bridson, w, h, d);

    let cb_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_rect(w, h, d, 10, 10, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let cb_cells  = scatter_cull_rect(w, h, d, 10, 10, 30, cb_ov, s);
    let cb_before = cb_cells.iter().map(|c| c.len()).sum::<usize>();
    let cb        = global_cull_to_min_dist(cb_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("C-B rect 10×10 (ov={:.3})", cb_ov), cb_before, &cb, w, h, d);

    let fv_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_sheared(w, h, d, 10, 10, 0.0, 30, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let fv_cells  = scatter_cull_sheared(w, h, d, 10, 10, 0.0, 30, fv_ov, s);
    let fv_before = fv_cells.iter().map(|c| c.len()).sum::<usize>();
    let fv        = global_cull_to_min_dist(fv_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("F   shear=0   (ov={:.3})", fv_ov), fv_before, &fv, w, h, d);

    let e2_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 500, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let e2_cells  = scatter_cull_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 500, e2_ov, s);
    let e2_before = e2_cells.iter().map(|c| c.len()).sum::<usize>();
    let e2        = global_cull_to_min_dist(e2_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("E   shift(-15,10) (ov={:.3})", e2_ov), e2_before, &e2, w, h, d);

    let gr_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_rect(w, h, d, 10, 10, 30, ov, s).len()
    });
    let gr = scatter_global_rect(w, h, d, 10, 10, 30, gr_ov, s);
    print_stats(&format!("global_rect 10×10 (ov={:.3})", gr_ov), gr.len(), &gr, w, h, d);

    let ghh_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 500, ov, s).len()
    });
    let ghh = scatter_global_half_heart(w, h, d, 5, std::f32::consts::FRAC_PI_4, -15.0, 10.0, 500, ghh_ov, s);
    print_stats(&format!("global_half_heart (ov={:.3})", ghh_ov), ghh.len(), &ghh, w, h, d);

    let gi1_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse_inset(w, h, d, 10, 10, 6000, ov, s).len()
    });
    let gi1 = scatter_global_sdf_ellipse_inset(w, h, d, 10, 10, 6000, gi1_ov, s);
    print_stats(&format!("G-Inset single-pass (ov={:.3})", gi1_ov), gi1.len(), &gi1, w, h, d);

    let gc_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_sdf_ellipse_corner_fill(w, h, d, 10, 10, 6000, ov, s).len()
    });
    let gc = scatter_global_sdf_ellipse_corner_fill(w, h, d, 10, 10, 6000, gc_ov, s);
    print_stats(&format!("G-Corner fill       (ov={:.3})", gc_ov), gc.len(), &gc, w, h, d);

    let dr_ov = calibrate_overage(target, 50.0, |ov| {
        let cells = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 1000, ov, s);
        global_cull_to_min_dist(cells.into_iter().flatten().collect(), w, h, d).len()
    });
    let dr_cells  = scatter_cull_voronoi_recursive(w, h, d, 10, 2, 3, 1000, dr_ov, s);
    let dr_before = dr_cells.iter().map(|c| c.len()).sum::<usize>();
    let dr        = global_cull_to_min_dist(dr_cells.into_iter().flatten().collect(), w, h, d);
    print_stats(&format!("D-R recursive L=2 (ov={:.3})", dr_ov), dr_before, &dr, w, h, d);

    let h1r_ov = calibrate_overage(target, 50.0, |ov| {
        scatter_global_triangles(w, h, d, 10, 10, 0.0, 6000, ov, s).len()
    });
    let h1r = scatter_global_triangles(w, h, d, 10, 10, 0.0, 6000, h1r_ov, s);
    print_stats(&format!("H single-pass jitter=0.0 (ov={:.3})", h1r_ov), h1r.len(), &h1r, w, h, d);

    println!("\n  Calibrated summary (500×500):");
    println!("    Bridson target: {}", target);
    println!("    C-B  ov={:.3} → {} pts ({:.1}%)", cb_ov,   cb.len(),  cb.len()  as f32 / target as f32 * 100.0);
    println!("    F v  ov={:.3} → {} pts ({:.1}%)", fv_ov,   fv.len(),  fv.len()  as f32 / target as f32 * 100.0);
    println!("    E2   ov={:.3} → {} pts ({:.1}%)", e2_ov,   e2.len(),  e2.len()  as f32 / target as f32 * 100.0);
    println!("    gr   ov={:.3} → {} pts ({:.1}%)", gr_ov,   gr.len(),  gr.len()  as f32 / target as f32 * 100.0);
    println!("    ghh  ov={:.3} → {} pts ({:.1}%)", ghh_ov,  ghh.len(), ghh.len() as f32 / target as f32 * 100.0);
    println!("    gi1  ov={:.3} → {} pts ({:.1}%)", gi1_ov,  gi1.len(), gi1.len() as f32 / target as f32 * 100.0);
    println!("    gc   ov={:.3} → {} pts ({:.1}%)", gc_ov,   gc.len(),  gc.len()  as f32 / target as f32 * 100.0);
    println!("    D-R  ov={:.3} → {} pts ({:.1}%)", dr_ov,   dr.len(),  dr.len()  as f32 / target as f32 * 100.0);
    println!("    H1r  ov={:.3} → {} pts ({:.1}%)", h1r_ov,  h1r.len(), h1r.len() as f32 / target as f32 * 100.0);
    println!();
}
