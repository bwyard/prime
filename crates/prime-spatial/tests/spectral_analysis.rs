/// Spectral analysis for scatter-cull blue noise research.
///
/// Verifies that scatter-cull approaches produce blue noise — low power at low
/// spatial frequencies, rising to a plateau at high frequencies.
///
/// Two complementary analyses:
///   1. Pair Correlation Function (PCF / radial distribution function g(r)):
///      - g(r) = 0 for r < min_dist (exclusion zone — hard constraint)
///      - g(r) ≈ 1 for r >> min_dist (uncorrelated background)
///      - Blue noise: sharp rise at r = min_dist, no oscillations
///   2. Power spectrum via 2D DFT (128×128 rasterized grid):
///      - Blue noise: power[low_freq] < power[mid_freq]
///      - White noise: flat spectrum
///      - Regular grid: spikes at harmonics
///
/// Run with: cargo test -p prime-spatial --test spectral_analysis -- --nocapture

use prime_spatial::{
    scatter_global_rect,
    scatter_global_voronoi,
    scatter_global_half_heart,
    scatter_global_sdf_ellipse,
};
use prime_random::poisson_disk;

// ── Pair correlation function g(r) ───────────────────────────────────────────

/// Computes the pair correlation function (radial distribution function) g(r).
///
/// For each distance bin at r ± dr/2:
///   - Count all point pairs whose separation falls in that bin
///   - Normalize by the expected count under a uniform Poisson process
///
/// # Expected behavior for blue noise
/// - g(0..min_dist) = 0          (exclusion zone)
/// - g(r) rises sharply at r ≈ min_dist
/// - g(r) ≈ 1 for r >> min_dist (uncorrelated)
fn pair_correlation(
    points: &[(f32, f32)],
    width: f32,
    height: f32,
    min_dist: f32,
    n_bins: usize,
) -> Vec<f32> {
    let r_max   = min_dist * 6.0;
    let dr      = r_max / n_bins as f32;
    let area    = width * height;
    let n       = points.len();
    let density = n as f32 / area;
    let mut bins = vec![0u32; n_bins];

    for i in 0..n {
        for j in (i + 1)..n {
            let dx  = points[i].0 - points[j].0;
            let dy  = points[i].1 - points[j].1;
            let r   = (dx * dx + dy * dy).sqrt();
            let bin = (r / dr) as usize;
            if bin < n_bins {
                bins[bin] += 2; // count both i→j and j→i directions
            }
        }
    }

    bins.iter()
        .enumerate()
        .map(|(k, &count)| {
            let r        = (k as f32 + 0.5) * dr;
            let expected = density * 2.0 * std::f32::consts::PI * r * dr * n as f32;
            if expected < 1e-6 { 0.0 } else { count as f32 / expected }
        })
        .collect()
}

// ── Power spectrum via 2D DFT ─────────────────────────────────────────────────

/// Computes the radially-averaged power spectrum of a point set.
///
/// Steps:
///   1. Rasterize points onto an n×n grid (count per cell, subtract mean)
///   2. Apply 2D DFT via row-FFTs then column-FFTs
///   3. Compute |X[k]|² per bin, then radially average
///   4. Normalize by count per radial bin
///
/// # Expected behavior for blue noise
/// - power[low_freq_bin] < power[mid_freq_bin]  (rising spectrum)
fn power_spectrum_radial(
    points: &[(f32, f32)],
    width: f32,
    height: f32,
    resolution: usize,
) -> Vec<f32> {
    use rustfft::{FftPlanner, num_complex::Complex};

    let n = resolution;

    // Rasterize
    let mut grid = vec![0.0f32; n * n];
    for &(x, y) in points {
        let gx = ((x / width)  * n as f32) as usize;
        let gy = ((y / height) * n as f32) as usize;
        if gx < n && gy < n {
            grid[gy * n + gx] += 1.0;
        }
    }

    // Subtract mean (remove DC contribution)
    let mean = grid.iter().sum::<f32>() / (n * n) as f32;
    let mut buffer: Vec<Complex<f32>> = grid
        .iter()
        .map(|&v| Complex::new(v - mean, 0.0))
        .collect();

    // Row FFTs
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    for row in 0..n {
        let slice = &mut buffer[row * n..(row + 1) * n];
        fft.process(slice);
    }

    // Column FFTs: transpose → FFT each row → transpose back
    let mut transposed = vec![Complex::new(0.0f32, 0.0f32); n * n];
    for row in 0..n {
        for col in 0..n {
            transposed[col * n + row] = buffer[row * n + col];
        }
    }
    for row in 0..n {
        let slice = &mut transposed[row * n..(row + 1) * n];
        fft.process(slice);
    }
    for row in 0..n {
        for col in 0..n {
            buffer[row * n + col] = transposed[col * n + row];
        }
    }

    // Radial average of power |X[k]|²
    let max_freq = n / 2;
    let mut power  = vec![0.0f32; max_freq];
    let mut counts = vec![0u32;   max_freq];
    for ky in 0..n {
        for kx in 0..n {
            let fkx  = if kx < n / 2 { kx as f32 } else { kx as f32 - n as f32 };
            let fky  = if ky < n / 2 { ky as f32 } else { ky as f32 - n as f32 };
            let freq = (fkx * fkx + fky * fky).sqrt() as usize;
            if freq < max_freq {
                power[freq]  += buffer[ky * n + kx].norm_sqr();
                counts[freq] += 1;
            }
        }
    }

    power
        .iter()
        .zip(counts.iter())
        .map(|(&p, &c)| if c > 0 { p / c as f32 } else { 0.0 })
        .collect()
}

// ── Report helpers ────────────────────────────────────────────────────────────

fn print_pcf(label: &str, pcf: &[f32], min_dist: f32, n_bins: usize) {
    let r_max = min_dist * 6.0;
    let dr    = r_max / n_bins as f32;
    println!("  PCF g(r) — {label}");
    for (k, &g) in pcf.iter().enumerate().take(12) {
        let r = (k as f32 + 0.5) * dr;
        let bar_len = (g * 20.0).min(40.0) as usize;
        let bar     = "#".repeat(bar_len);
        println!("    r={:5.2}  g={:6.3}  {bar}", r, g);
    }
    println!();
}

fn print_spectrum(label: &str, spectrum: &[f32]) {
    println!("  Power spectrum — {label}");
    for (freq, &p) in spectrum.iter().enumerate().take(16) {
        let bar_len = (p * 0.01).min(40.0) as usize;
        let bar     = "#".repeat(bar_len);
        println!("    freq={:3}  power={:10.2}  {bar}", freq, p);
    }
    println!();
}

// ── Main test ─────────────────────────────────────────────────────────────────

#[test]
fn spectral_analysis_blue_noise() {
    let width    = 100.0_f32;
    let height   = 100.0_f32;
    let min_dist = 5.0_f32;
    let seed     = 42u32;
    let pi4      = std::f32::consts::FRAC_PI_4;
    let pcf_bins = 24usize;
    let fft_res  = 128usize;

    println!("\n=== Blue Noise Spectral Analysis — 100×100, min_dist=5, seed=42 ===");
    println!("PCF: g(r)=0 in exclusion zone, g(r)→1 uncorrelated background");
    println!("Spectrum: blue noise has power[low] < power[mid]\n");

    // ── Bridson reference ─────────────────────────────────────────────────────
    let bridson  = poisson_disk(width, height, min_dist, 30, seed);
    let pcf_br   = pair_correlation(&bridson, width, height, min_dist, pcf_bins);
    let spec_br  = power_spectrum_radial(&bridson, width, height, fft_res);
    println!("── Bridson (reference)  pts={} ──", bridson.len());
    print_pcf("Bridson", &pcf_br, min_dist, pcf_bins);
    print_spectrum("Bridson", &spec_br);

    // ── scatter_global_rect ───────────────────────────────────────────────────
    // overage=13 calibrated to match Bridson point count on 100×100, min_dist=5
    let rect     = scatter_global_rect(width, height, min_dist, 4, 4, 30, 13.0, seed);
    let pcf_rect = pair_correlation(&rect, width, height, min_dist, pcf_bins);
    let spec_rect = power_spectrum_radial(&rect, width, height, fft_res);
    println!("── scatter_global_rect  pts={} ──", rect.len());
    print_pcf("global_rect", &pcf_rect, min_dist, pcf_bins);
    print_spectrum("global_rect", &spec_rect);

    // ── scatter_global_voronoi ────────────────────────────────────────────────
    let voronoi     = scatter_global_voronoi(width, height, min_dist, 10, 3, 30, 20.0, seed);
    let pcf_vor     = pair_correlation(&voronoi, width, height, min_dist, pcf_bins);
    let spec_vor    = power_spectrum_radial(&voronoi, width, height, fft_res);
    println!("── scatter_global_voronoi  pts={} ──", voronoi.len());
    print_pcf("global_voronoi", &pcf_vor, min_dist, pcf_bins);
    print_spectrum("global_voronoi", &spec_vor);

    // ── scatter_global_half_heart ─────────────────────────────────────────────
    let half_heart  = scatter_global_half_heart(
        width, height, min_dist, 5, pi4, -15.0, 10.0, 30, 20.0, seed,
    );
    let pcf_hh      = pair_correlation(&half_heart, width, height, min_dist, pcf_bins);
    let spec_hh     = power_spectrum_radial(&half_heart, width, height, fft_res);
    println!("── scatter_global_half_heart  pts={} ──", half_heart.len());
    print_pcf("global_half_heart", &pcf_hh, min_dist, pcf_bins);
    print_spectrum("global_half_heart", &spec_hh);

    // ── scatter_global_sdf_ellipse ────────────────────────────────────────────
    let sdf_ellipse  = scatter_global_sdf_ellipse(
        width, height, min_dist, 4, 4, 1.0, 1.2, 300, 20.0, seed,
    );
    let pcf_sdf      = pair_correlation(&sdf_ellipse, width, height, min_dist, pcf_bins);
    let spec_sdf     = power_spectrum_radial(&sdf_ellipse, width, height, fft_res);
    println!("── scatter_global_sdf_ellipse  pts={} ──", sdf_ellipse.len());
    print_pcf("global_sdf_ellipse", &pcf_sdf, min_dist, pcf_bins);
    print_spectrum("global_sdf_ellipse", &spec_sdf);

    // ── Comparison table ──────────────────────────────────────────────────────

    // Blue noise metric: average power over low-freq bins 1..=3 vs high-freq bins 10..=14.
    // Using averages over ranges is more robust than single-bin comparison at small FFT sizes
    // (128×128 grid, ~250 points has inherent spectral variance at individual bins).
    // Blue noise definition: mean power at low frequencies < mean power at high frequencies.
    let low_range  = 1usize..=3;
    let high_range = 10usize..=14;

    let avg_power = |spec: &[f32], range: std::ops::RangeInclusive<usize>| -> f32 {
        let vals: Vec<f32> = range.filter_map(|i| spec.get(i).copied()).collect();
        if vals.is_empty() { 0.0 } else { vals.iter().sum::<f32>() / vals.len() as f32 }
    };

    println!("=== Comparison table ===");
    println!("  Blue noise metric: avg_power(bins 1-3) < avg_power(bins 10-14)");
    println!(
        "  {:<25}  {:>6}  {:>10}  {:>10}  {:>13}  {:>13}  {:>8}",
        "approach", "pts", "PCF[0]", "PCF[peak]", "avg_low(1-3)", "avg_high(10-14)", "verdict"
    );

    let approaches: &[(&str, &[(f32, f32)], &[f32], &[f32])] = &[
        ("Bridson",              &bridson,     &pcf_br,   &spec_br),
        ("global_rect",          &rect,        &pcf_rect, &spec_rect),
        ("global_voronoi",       &voronoi,     &pcf_vor,  &spec_vor),
        ("global_half_heart",    &half_heart,  &pcf_hh,   &spec_hh),
        ("global_sdf_ellipse",   &sdf_ellipse, &pcf_sdf,  &spec_sdf),
    ];

    for &(label, pts, pcf, spec) in approaches {
        let pcf0     = pcf.first().copied().unwrap_or(0.0);
        let pcf_max  = pcf.iter().cloned().fold(0.0_f32, f32::max);
        let avg_low  = avg_power(spec, low_range.clone());
        let avg_high = avg_power(spec, high_range.clone());
        let verdict  = if avg_low < avg_high { "BLUE" } else { "NOT-BLUE" };
        println!(
            "  {:<25}  {:>6}  {:>10.4}  {:>10.4}  {:>13.2}  {:>13.2}  {:>8}",
            label, pts.len(), pcf0, pcf_max, avg_low, avg_high, verdict
        );
    }
    println!();

    // ── Blue noise property assertions ────────────────────────────────────────

    // PCF[0] MUST be 0 for every approach — this is the min-dist exclusion zone guarantee.
    // If this fails, the sampling algorithm has a hard bug.
    let r_max      = min_dist * 6.0;
    let dr         = r_max / pcf_bins as f32;
    let bin0_r_max = dr; // upper edge of bin 0

    println!("=== Assertions ===");
    println!("  PCF bin-0 covers r ∈ [0, {:.3}] (must be < min_dist={:.1} → g(r)=0)", bin0_r_max, min_dist);

    for &(label, pts, pcf, _spec) in approaches {
        let pcf0 = pcf.first().copied().unwrap_or(1.0);
        println!("  {:<25}  PCF[0]={:.4}  pts={}", label, pcf0, pts.len());
        assert!(
            pcf0 == 0.0,
            "{label}: PCF[0]={pcf0:.4} should be 0.0 — exclusion zone violated (min-dist guarantee broken)"
        );
    }

    // Spectrum rising: avg power over low-freq bins 1-3 must be less than high-freq bins 10-14.
    // Averaging over ranges removes single-bin noise at 128×128 resolution.
    // This is the defining blue noise property.
    println!("\n  Blue noise spectrum check: avg_power(bins 1-3) < avg_power(bins 10-14)");
    for &(label, _pts, _pcf, spec) in approaches {
        let avg_low  = avg_power(spec, low_range.clone());
        let avg_high = avg_power(spec, high_range.clone());
        let result   = if avg_low < avg_high { "BLUE" } else { "NOT-BLUE" };
        println!(
            "  {:<25}  low={:10.2}  high={:10.2}  → {}",
            label, avg_low, avg_high, result
        );
    }

    // Assert the blue noise property for every approach.
    for &(label, _pts, _pcf, spec) in approaches {
        let avg_low  = avg_power(spec, low_range.clone());
        let avg_high = avg_power(spec, high_range.clone());
        assert!(
            avg_low < avg_high,
            "{label}: spectrum is not blue noise — avg_power(bins 1-3)={avg_low:.2} >= avg_power(bins 10-14)={avg_high:.2}"
        );
    }

    println!("\n  All assertions passed.");
}
