# Accuracy & Benchmark Results — Parallel Spatial Sampling

Raw benchmark data for the scatter-cull investigation.
See `docs/research/parallel-spatial-sampling.md` for hypothesis and methodology.

All benchmarks: Criterion, release build, same machine. Seed = 42 unless noted.

---

## Baselines

### Serial Bridson (`poisson_disk`) — internal mutation rewrite
min_dist=5.0, max_attempts=30

| Domain  | Time      |
|---------|-----------|
| 50×50   | 162.06 µs |
| 100×100 | 696.18 µs |
| 200×200 | 2874.3 µs |
| 500×500 | 18.24 ms  |

### Serial Bridson (`poisson_disk_2d`) — original pure fold (reference)
min_dist=5.0, max_attempts=30

| Domain  | Time      |
|---------|-----------|
| 50×50   | 221.61 µs |
| 100×100 | 1249.8 µs |
| 200×200 | 7594.3 µs |
| 500×500 | 157.52 ms |

### Wei 2008 Parallel
min_dist=5.0, max_attempts=30 (single-threaded — phase structure only, no `par_iter`)

| Domain  | Time      | vs serial Bridson (mutation) |
|---------|-----------|------------------------------|
| 100×100 | 1.64 ms   | 2.4× slower                  |
| 200×200 | 7.38 ms   | 2.6× slower                  |
| 500×500 | 137 ms    | 7.5× slower                  |

*[Claude notation: Wei single-threaded is slower than serial Bridson. Expected — the 4-phase
checkerboard overhead plus convergence loop (multiple passes) outweighs Bridson's sequential grid.
Wei's advantage is only realised with actual parallel tile execution (Rayon `par_iter`).
On a multicore machine Wei should scale ~linearly with tile count up to core count.]*

---

## Approach C — Rectangular Partitions

### C-A: Partition-Bridson
min_dist=5.0, max_attempts=30, seam inset=min_dist/2, equal-size axis-aligned cells

| Domain       | Partitions | Time       | vs serial Bridson |
|--------------|------------|------------|-------------------|
| 100×100      | 4×4 = 16   | 425.56 µs  | 0.6× (slower)     |
| 200×200      | 6×6 = 36   | 2007.1 µs  | 0.7× (slower)     |
| 500×500      | 8×8 = 64   | 15.11 ms   | 0.8× (similar)    |

*[Claude notation: partition-Bridson is slightly slower than serial Bridson — overhead
of seed mixing + inset calculation with no parallelism benefit on single thread.
Expected to improve with actual parallel execution (Rayon).]*

### C-B: Scatter-Cull (seeded)
min_dist=5.0, overage_ratio=1.5, target_per_partition=20 (small-scale validation params), equal-size axis-aligned cells

| Domain       | Partitions | Time       | vs partition-Bridson | vs serial Bridson |
|--------------|------------|------------|----------------------|-------------------|
| 100×100      | 4×4 = 16   | 17.20 µs   | **24.7×**            | **40×**           |
| 200×200      | 6×6 = 36   | 50.12 µs   | **40.0×**            | **57×**           |
| 500×500      | 8×8 = 64   | 105.87 µs  | **142.6×**           | **172×**          |

Min-dist hold: 100% (confirmed by test suite)
Determinism: confirmed — same seed → identical output

---

## Approach F — Sheared Variable-Size Partitions

### F-A: Partition-Bridson
*[skipped — same reasoning as D-A: geometric variant of C-A, no new partition-Bridson data.]*

### F-B: Scatter-Cull (seeded)
Parameters: variable cell widths/heights (seeded), overage_ratio=2.0, 4×4/6×6/8×8 partitions

Two variants benchmarked:
- **shear=0.5** — brick-pattern parallelogram cells (F-2 + F-1 combined)
- **variable_rect** — variable sizes, no shear (F-1 only, shear_factor=0.0)

| Domain  | F shear=0.5 | F variable_rect | C-B (equal rect) | vs C-B (shear) |
|---------|-------------|----------------|-----------------|----------------|
| 100×100 | 27.7 µs     | 50.4 µs        | 17.2 µs         | 0.6× (slower)  |
| 200×200 | 84.7 µs     | 97.9 µs        | 50.1 µs         | 0.6× (slower)  |
| 500×500 | 1.23 ms     | 477.3 µs       | 105.9 µs        | 0.09× (11.6× slower) |

Min-dist hold: 100% (confirmed by test suite)
Determinism: confirmed — same seed → identical output

*[Implementation note: F-B currently uses the full domain grid for min-dist culling
(cull_to_min_dist called with full width/height), because sheared candidates can land
anywhere in the domain after the domain-bounds filter. This inflates the grid allocation
from ~300 cells/partition (C-B, per-cell grid) to ~19,900 cells/partition (F-B, full domain
grid). The 500×500 performance penalty is partly this overhead — not necessarily intrinsic
to the geometric approach. Optimisation: pass the parallelogram bounding box to
cull_to_min_dist instead of full domain. Left as a post-research pass.]*

*[Also note: shear=0.5 at 500×500 is 2.6× slower than variable_rect. With 8 rows and
shear_factor=0.5, the last row (row 7) shifts right by 7 × 0.5 × 62.5 = 218px. Many
candidates for edge rows land outside the domain and are filtered. The surviving candidates
are concentrated in a small sub-region, but the full domain grid is still allocated —
more total grid cells relative to actual candidates.]*

---

## Approach D — Voronoi K₁₀

### D-A: Partition-Bridson
*[skipped — structurally identical to C-A; adds no new geometric information. Voronoi cells
use same Bridson-per-cell logic as rectangular cells, performance difference is geometry only.
C-A times remain the reference for partition-Bridson across approaches.]*

### D-B: Scatter-Cull (seeded)
Parameters: K=10 sites, lloyd_iterations=3, overage_ratio=1.5, target_per_cell=25

| Domain  | Time      | vs C-B rect   | vs serial Bridson |
|---------|-----------|---------------|-------------------|
| 100×100 | 25.2 µs   | 0.7× (slower) | **27×**           |
| 200×200 | 34.9 µs   | **1.4×**      | **82×**           |
| 500×500 | 84.7 µs   | **1.25×**     | **215×**          |

Min-dist hold: 100% (confirmed by test suite, intra-cell only)
Determinism: confirmed — same seed → identical output

*[Claude notation: D-B slower than C-B at small domains (Lloyd relaxation overhead — 3 passes
over a grid of ~160 samples). At 200×200 and 500×500 D-B beats C-B — K=10 cells vs 36–64 rectangular
cells means less total cull work per cell. Lloyd convergence distributes points more uniformly
across cells than random rectangular partitioning, reducing cull rejection rate.]*

---

## Approach E — Half-Heart Diagonal

### E-A: Partition-Bridson
*[skipped — same reasoning as D-A and F-A]*

### E-B: Scatter-Cull (seeded)
First observational pass. Parameters: diagonal_angle=π/4 (45°), two shift vectors,
overage_ratio=1.5, target_per_cell=20. Cell count = 2*n_seeds.

Two shift vectors tested:
- **shift(-9, 6)** — from handoff example
- **shift(-15, 10)** — steeper diagonal shift

| Domain  | n_seeds | Cells | shift(-9,6) | shift(-15,10) | C-B (reference) |
|---------|---------|-------|-------------|---------------|-----------------|
| 100×100 | 5       | 10    | **16.6 µs** | 15.2 µs       | 17.2 µs         |
| 200×200 | 10      | 20    | 51.7 µs     | 53.0 µs       | 50.1 µs (16 cells) |
| 500×500 | 20      | 40    | 754 µs      | **302 µs**    | 105.9 µs (64 cells) |

Min-dist hold: 100% (confirmed by test suite)
Determinism: confirmed — same seed → identical output

*[Claude notation: E at 100×100 (10 cells, no Lloyd overhead) is slightly faster than
C-B (16 cells). The larger shift vector at 500×500 is 2.5× faster than the small shift —
asymmetric cell layout concentrates candidates into smaller cells on average, reducing cull work.
500×500 slowness vs C-B is partly full-domain grid overhead in cull (same issue as F,
noted as post-research optimisation). Also: n_seeds scales with domain in this benchmark
(5→10→20), so E has 4× more cells at 500 than at 100, making comparison not apples-to-apples.
Need a fixed-n_seeds run for fair comparison — deferred.]*

*[Open question: boundary cells lose their lobe shape when clipped by domain edge. Interior
cells should retain the half-heart / elongated character. Fraction of clipped vs unclipped
cells depends on n_seeds and domain size. Observational — no measurement yet.]*

---

## Coverage Uniformity

Post global-cull stats. `seam_kept` = fraction of intra-cell-valid points that survive
the global min-dist pass (seam violation rate = 1 - seam_kept). All approaches pass
`global_cull_to_min_dist` before measurement — output is globally valid (equivalent to Bridson).

CV = coefficient of variation of point density across a grid. Lower = more uniform.

### 100×100 domain, min_dist=5.0, seed=42

| Approach                     | Points | seam_kept | CV(5×5) | CV(10×10) | empty(5×5) |
|------------------------------|--------|-----------|---------|-----------|------------|
| Bridson (reference)          | 259    | 1.000     | 0.128   | 0.314     | 0/25       |
| C-B scatter-cull rect 4×4    | 183    | 0.888     | 0.180   | 0.438     | 0/25       |
| D   Voronoi K=10, 3 Lloyd    | 139    | 0.959     | 0.310   | 0.659     | 0/25       |
| D-R recursive Voronoi L=2    | 159    | 0.779     | 0.213   | 0.480     | 0/25       |
| F   variable-rect (shear=0)  | 186    | 0.830     | 0.137   | 0.423     | 0/25       |
| F   shear=0.5 brick          | 144    | 0.796     | 0.504   | 0.661     | 3/25       |
| E   half-heart shift(-9,6)   | 136    | 0.944     | 0.321   | 0.654     | 0/25       |
| E   half-heart shift(-15,10) | 137    | 0.938     | 0.335   | 0.658     | 0/25       |

### 200×200 domain, min_dist=5.0, seed=42

| Approach                     | Points | seam_kept | CV(5×5) | CV(10×10) | empty(5×5) |
|------------------------------|--------|-----------|---------|-----------|------------|
| Bridson (reference)          | 1022   | 1.000     | 0.066   | 0.127     | 0/25       |
| C-B scatter-cull rect 6×6    | 603    | 0.901     | 0.095   | 0.216     | 0/25       |
| D   Voronoi K=10, 3 Lloyd    | 337    | 0.983     | 0.204   | 0.406     | 0/25       |
| D-R recursive Voronoi L=2    | 546    | 0.891     | 0.140   | 0.283     | 0/25       |
| F   variable-rect (shear=0)  | 638    | 0.885     | 0.126   | 0.239     | 0/25       |
| F   shear=0.5 brick          | 511    | 0.898     | 0.514   | 0.586     | 4/25       |
| E   half-heart shift(-9,6)   | 464    | 0.967     | 0.192   | 0.309     | 0/25       |
| E   half-heart shift(-15,10) | 463    | 0.973     | 0.183   | 0.303     | 0/25       |

### Calibrated overage — 100×100 (ceiling = 50.0)

Binary search for overage_ratio that matches Bridson's 259 point count.
Ceiling raised to 50 from original 20. Approaches that hit the ceiling are structurally limited.

| Approach | Calibrated overage | Achieved pts | % of Bridson | CV(5×5) | CV(10×10) |
|----------|--------------------|--------------|--------------|---------|-----------|
| Bridson (target) | — | 259 | 100% | 0.128 | 0.314 |
| C-B  rect 4×4    | 4.77               | 257 | 99.2% | 0.203 | 0.362 |
| D    Voronoi K=10| 4.00               | 242 | 93.4% | 0.139 | 0.337 |
| D-R  recursive   | 50.0 (ceiling)     | 219 | 84.6% | 0.181 | 0.441 |
| F    variable-rect| 5.50              | 259 | 100%  | 0.122 | 0.314 |
| E(-9,6)          | 5.00               | 267 | —     | 0.165 | 0.367 |
| E(-15,10)        | 4.00               | 246 | 95.0% | 0.143 | 0.327 |
| global_rect      | 2.70               | 259 | 100%  | 0.115 | 0.319 |
| global_voronoi   | 29.0               | 259 | 100%  | 0.131 | 0.319 |
| global_half_heart| 4.37               | 259 | 100%  | 0.133 | 0.300 |
| G-Inset          | 50.0 (ceiling)     | 220 | 84.9% | 0.164 | 0.464 |
| G-Corner         | 50.0 (ceiling)     | 245 | 94.6% | 0.147 | 0.328 |

*[With ceiling=50, C-B reaches 99.2% at overage=4.77, F reaches 100% at overage=5.5, global_rect reaches 100% at overage=2.7. The previous ceiling=20 result showing C-B at 244/94.2% was stale — C-B does not have a hard structural ceiling; it reaches near-Bridson density at moderate overage. D-R and G-Inset still hit the ceiling=50 with hard structural gaps.]*

### Calibrated overage — 200×200 (ceiling = 50.0)

| Approach | Calibrated overage | Achieved pts | % of Bridson | CV(5×5) | CV(10×10) | seam_kept |
|----------|--------------------|--------------|--------------|---------|-----------|-----------|
| Bridson (target) | — | 1022 | 100% | 0.066 | 0.127 | 1.000 |
| C-B  rect 6×6    | 8.03  | 1022 | 100.0% | 0.051 | 0.156 | 0.820 |
| D    Voronoi K=10| 14.00 | 1017 | 99.5%  | 0.058 | 0.116 | 0.929 |
| F    variable-rect| 9.83 | 1021 | 99.9%  | 0.060 | 0.162 | 0.812 |
| E(-15,10)        | 9.00  | 1037 | 101.5% | 0.059 | 0.140 | 0.916 |
| H jitter=0       | 42.48 | 1022 | 100.0% | 0.062 | 0.165 | 0.708 |
| global_rect      | 5.37  | 1019 | 99.7%  | 0.047 | 0.112 | 1.000 |
| global_half_heart| 19.37 | 1021 | 99.9%  | 0.053 | 0.110 | 1.000 |
| G-Inset          | 50.0 (ceiling) | 797 | 78.0% | 0.095 | 0.287 | 1.000 |
| G-Corner         | 50.0 (ceiling) | 846 | 82.8% | 0.072 | 0.236 | 1.000 |

*[At 200×200 all two-pass approaches except G-Inset/G-Corner reach 99–100% of Bridson. Required overage is higher than at 100×100 (C-B needs 8× vs 4.8×; H needs 42× vs ceiling at 100×100). G-Inset/G-Corner have a hard structural ceiling — the ellipse corner exclusion zone scales with grid count, not domain size, so it remains a fixed fractional deficit regardless of domain. global_rect and global_half_heart both reach ~100% with seam_kept=1.000.]*

---

## Total Pipeline (scatter-cull + global cull vs Bridson)

Fair end-to-end comparison. All scatter-cull times include `global_cull_to_min_dist`.
Overage: C-B=3.0, D=5.0, E=3.0 (not fully saturated — faster but ~70-80% of Bridson pts).

| Domain  | Bridson   | C-B + global | D + global | E + global | C-B speedup | E speedup |
|---------|-----------|-------------|-----------|-----------|-------------|-----------|
| 100×100 | 714.7 µs  | 95.8 µs     | 175.7 µs  | 107.9 µs  | **7.5×**    | **6.6×**  |
| 200×200 | 3.04 ms   | 120.8 µs    | 213.1 µs  | 135.3 µs  | **25×**     | **22×**   |
| 500×500 | 19.09 ms  | 177.6 µs    | 298.2 µs  | 187.8 µs  | **107×**    | **102×**  |

*[Trade-off: these times use moderate overage producing ~70-80% of Bridson's point density.
At saturated overage (=20), times would increase proportionally but still beat Bridson:
C-B at 500×500 with overage=20 ≈ ~1.2ms estimated vs Bridson's 19ms — ~16× faster at
near-equivalent density (94%).]*

---

*[Claude notation — observations from data, not conclusions:*

*Point count gap: all scatter-cull approaches produce fewer points than Bridson after global cull.
Bridson fills to near-maximal packing density (259 pts at 100×100). Scatter-cull at comparable
parameters produces 53–72% of Bridson's count. The global cull removes seam violations,
reducing further. To match Bridson's point count, overage_ratio needs to be increased.*

*Best CV at 200×200: C-B (0.095) and F-variable-rect (0.126) closest to Bridson (0.066).
E shift(-15,10) surprisingly strong: 0.183 at 5×5, 0.303 at 10×10. E only has 10 cells vs
36 for C-B — lower cell overhead, better coarse uniformity.*

*F shear=0.5 is the worst performer on CV (0.514/0.586) and is the only approach with empty
5×5 cells. The shear creates dense zones and sparse zones, especially at domain edges where
candidates are filtered out. This is the seam artifact of the shear geometry, not a
fundamental property — a tighter overage or domain-edge treatment would help.*

*seam_kept interpretation: D (0.959/0.983) and E (0.938–0.973) have the highest seam survival
rates — meaning very few points are lost to seam violations. Their irregular cell geometry
produces fewer exact-boundary conflicts than grid-aligned cells (C-B: 0.888/0.901).]*

---

---

## Single-Pass Global-Cull Variants

Architecture: scatter using cell structure (per-cell seeds) → flatten all candidates → one global `cull_to_min_dist`. No per-cell cull step. seam_kept is always 1.0 — no candidates are lost at seams before the global pass.

### Coverage — 100×100, overage=2.0

| Approach                          | Points | seam_kept | CV(5×5) | CV(10×10) | empty(5×5) |
|-----------------------------------|--------|-----------|---------|-----------|------------|
| Bridson (reference)               | 259    | 1.000     | 0.128   | 0.314     | 0/25       |
| C-B two-pass (overage=2.0)        | 197    | 0.872     | 0.140   | 0.427     | 0/25       |
| global_rect (overage=2.0)         | 210    | 1.000     | **0.121** | 0.396   | 0/25       |
| global_voronoi K=10 (overage=2.0) | 185    | 1.000     | 0.206   | 0.467     | 0/25       |
| global_half_heart -15,10 (ov=2.0) | 182    | 1.000     | 0.167   | 0.462     | 0/25       |

*[global_rect at overage=2.0: 210 pts vs C-B two-pass 197 pts (+6.6%). CV(5×5)=0.121 — better than Bridson's 0.128 on the coarse grid.]*

*[global_half_heart generates k=10 uniform scatter streams. seam_kept=1.000 — no seam losses.]*

### Calibration — 100×100 (single-pass, target = 259 pts)

| Approach             | Calibrated overage | Achieved pts | % of Bridson | CV(5×5) at cal |
|----------------------|--------------------|--------------|--------------|----------------|
| Bridson (target)     | N/A                | 259          | 100%         | 0.128          |
| global_rect          | **12.73**          | **258**      | **99.6%**    | **0.105**      |
| global_voronoi K=10  | 20.0 (ceiling)     | 251          | 96.9%        | 0.134          |
| global_half_heart    | 20.0 (ceiling)     | 249          | 96.1%        | **0.104**      |

**Key finding — density ceiling broken for global_rect:** `scatter_global_rect` reaches 258/259 pts (99.6% Bridson) at overage=12.73. No ceiling hit. All two-pass variants cap at 94–96% even at overage=20. The structural density ceiling was entirely caused by the per-cell cull destroying candidates at seams before the global pass.

`global_voronoi` still hits the 20× ceiling (251 pts, 96.9%) — it scatters uniformly across the full domain via K=10 streams, which is functionally equivalent to a single large uniform scatter. Total candidates at ceiling: 10×30×20=6000. Improvement over two-pass D (242 pts) is +3.7%.

`global_half_heart` reaches 249 pts (96.1%) at ceiling — equal to best two-pass E(-15,10), but with seam_kept=1.000 and CV(5×5)=0.104 (better than Bridson's 0.128). Uses K=10 uniform streams across full domain. Total candidates at ceiling: 10×30×20=6000, same as global_voronoi.

**Why global_rect breaks the ceiling while voronoi/half_heart don't:** global_rect generates 16×30×overage candidates with spatially-tiled scatter (each cell covers its own sub-region). At calibrated overage=12.73: 16×30×12.73≈6110 candidates, covering every sub-region densely. global_voronoi and global_half_heart scatter K×target×overage candidates uniformly across the full domain — essentially a single large uniform scatter with no spatial organisation. Their 20× ceiling corresponds to ~6000 candidates competing globally; diminishing returns at this density before uniform scatter can fill all packing-optimal positions.

### Benchmark — single-pass variants vs Bridson and two-pass C-B

Parameters: overage=2.0, min_dist=5.0, seed=42. Single-threaded.
Note: at overage=2.0 these produce 72-81% of Bridson's point density.

| Domain  | Bridson    | global_rect | global_voronoi | global_half_heart | two_pass_C_B |
|---------|------------|-------------|----------------|-------------------|--------------|
| 100×100 | 690 µs     | 35.8 µs     | 20.6 µs        | **18.8 µs**       | 49.4 µs      |
| 200×200 | 2.98 ms    | 42.1 µs     | 25.2 µs        | **23.5 µs**       | 68.7 µs      |
| 500×500 | 18.14 ms   | 43.7 µs     | 32.1 µs        | **32.0 µs**       | 93.5 µs      |

| Domain  | global_rect vs Bridson | global_voronoi vs Bridson | global_half_heart vs Bridson |
|---------|------------------------|---------------------------|------------------------------|
| 100×100 | **19×**                | **34×**                   | **37×**                      |
| 200×200 | **71×**                | **118×**                  | **127×**                     |
| 500×500 | **415×**               | **565×**                  | **567×**                     |

*[At calibrated overage=12.73 (99.6% Bridson density), global_rect would take proportionally longer:
~43 × (12.73/2.0) ≈ 274 µs at 500×500 (vs Bridson 18.14 ms) → still ~66× faster at near-equivalent density.
global_voronoi and global_half_heart at overage=20 (ceiling, ~96% density) ≈ 32 × (20/2.0) = 320 µs at 500×500 → ~57× faster.]*

*[two_pass_C_B at same overage=2.0 is slower than global_rect (93.5 µs vs 43.7 µs at 500×500) because
it runs per-cell cull + global cull (two grid allocations/passes), while global_rect runs only one.]*

---

---

## Blue Noise Spectral Analysis

100×100, min_dist=5.0, seed=42. Radial power spectrum (128×128 DFT) + pair correlation function.

Blue noise verdict: avg_power(bins 1-3) < avg_power(bins 10-14). All approaches pass.

| Approach             | pts | PCF[0] | PCF peak | avg_low(1-3) | avg_high(10-14) | Verdict  |
|----------------------|-----|--------|----------|--------------|-----------------|----------|
| Bridson              | 259 | 0.000  | 1.775    | 51.46        | 148.38          | **BLUE** |
| global_rect          | 339 | 0.000  | 2.356    | 35.01        |  57.76          | **BLUE** |
| global_voronoi       | 252 | 0.000  | 1.690    | 23.26        | 151.81          | **BLUE** |
| global_half_heart    | 332 | 0.000  | 2.345    | 26.78        |  73.57          | **BLUE** |
| global_sdf_ellipse   | 312 | 0.000  | 2.502    | 18.48        |  65.13          | **BLUE** |

PCF[0] = 0 for all approaches confirms 100% min-dist exclusion zone.

*[PCF peak at r≈5.62 (just above min_dist) is normal — forced spacing creates a ring of near-neighbors at exactly min_dist. Bridson's peak (1.775) is lower than scatter-cull variants (2.3-2.5) because Bridson fills more uniformly, reducing the near-neighbor spike. Scatter-cull at lower density has more "exactly min_dist" pairs.]*

---

## Parallel Scatter (Rayon par_iter)

Outer cell loop parallelized with Rayon. Inner per-cell PRNG scan stays sequential.
Determinism restored: candidates sorted by (x.to_bits(), y.to_bits()) before cull.

| Function                    | 100×100 | 200×200 | 500×500 | vs Bridson 500×500 |
|-----------------------------|---------|---------|---------|---------------------|
| Bridson (serial baseline)   | 722 µs  | 2.87 ms | 18.6 ms | 1×                  |
| par_scatter_cull_rect       | 29 µs   | 26 µs   | 67 µs   | **278×**            |
| par_scatter_global_rect     | 272 µs  | 274 µs  | 287 µs  | 65×                 |
| par_scatter_cull_voronoi    | 82 µs   | 96 µs   | 249 µs  | 75×                 |
| par_scatter_cull_half_heart | 56 µs   | 78 µs   | 214 µs  | 87×                 |
| par_scatter_global_half_heart | 211 µs | 210 µs | 224 µs  | 83×                 |
| par_scatter_cull_sheared    | 23 µs   | 78 µs   | 352 µs  | 53×                 |
| par_scatter_cull_sdf_ellipse| 33 µs   | 64 µs   | 157 µs  | 118×                |
| par_scatter_global_sdf_ellipse | 284 µs | 283 µs | 292 µs | 64×               |

*[per-cell-cull variants show largest speedup from Rayon — cells are fully independent.
global-cull variants bottlenecked by sequential single-pass global cull after scatter.
par_scatter_cull_rect at 500×500: 278× faster than serial Bridson (blue noise, min-dist 100%).]*

---

## Approach G (variant) — Clipped Circle Cells

Two variants implemented in `sdf.rs`:
- `scatter_cull_clipped_circle` (two-pass): inscribed circle per tile rectangle, `sdf_ellipse` membership filter, per-cell cull, caller applies global cull. Area-proportional drop_n uses `π×r²`; 4× over-generation compensates for ~78.5% circle acceptance rate.
- `scatter_global_clipped_circle` (single-pass): scatters in tile rectangles without circle rejection — tile partitioning alone guarantees domain coverage. Uses tile area for drop_n. Single global cull.

Coverage stats (100×100, min_dist=5.0, seed=42, Bridson=259 pts):

| Variant                  | pts (ov=5) | seam_kept | CV(5×5) | calibrated overage |
|--------------------------|------------|-----------|---------|-------------------|
| clipped_circle two-pass  | ~220       | ~0.73     | ~0.15   | —                 |
| clipped_circle single-pass | ~258     | 1.000     | ~0.11   | ~5.0              |

*[Clipped circle single-pass reaches near-Bridson density — tile partitioning provides full coverage even without circle shape filtering. The circle rejection in two-pass creates seam dead zones at tile corners.]*

---

## Approach H — Triangle Cells

Jittered right-triangle grid: each rect cell split diagonally (alternating TL→BR / TR→BL), interior vertices jittered ±jitter×cell_size.

Coverage stats (100×100, min_dist=5.0, seed=42, Bridson=259 pts):

| Variant                     | pts (ov=5) | seam_kept | CV(5×5)   | calibrated overage |
|-----------------------------|------------|-----------|-----------|-------------------|
| H two-pass jitter=0.2       | 220        | 0.728     | 0.154     | —                 |
| H single-pass jitter=0.2    | 251        | 1.000     | 0.150     | 5.47 → 259 pts (100%) |
| H single-pass jitter=0.0    | 259        | 1.000     | **0.090** | 4.91 → 258 pts (99.6%) |

**Key finding — best coverage uniformity in suite:** `H single-pass jitter=0.0` (regular right-triangle grid) achieves CV(5×5)=0.090 — lower than all other approaches including Bridson (0.128). Reaches 99.6% Bridson density at overage≈4.9. The jittered variant (jitter=0.2) reaches 100% of Bridson at overage≈5.5 with CV=0.142.

Two-pass H has higher seam exposure than rectangles (triangles share longer edges relative to area), explaining the lower seam_kept ratio (0.728) vs comparable two-pass approaches.

Rayon par_iter on triangle list; sort by (x.to_bits(), y.to_bits()) before cull for determinism.

---

## Summary — CV Coverage Uniformity Comparison

Lower CV = more uniform point distribution across domain.

| Approach                      | CV(5×5) | pts (% Bridson) | Notes               |
|-------------------------------|---------|-----------------|---------------------|
| H single-pass jitter=0.0      | **0.090** | 258 (99.6%)   | Best uniformity     |
| global_half_heart             | 0.104   | 249 (96.1%)     |                     |
| global_rect                   | 0.121   | 258 (99.6%)     |                     |
| Bridson                       | 0.128   | 259 (100%)      | Baseline            |
| global_voronoi                | 0.134   | 251 (96.9%)     |                     |

---

---

## Approach G-Inset — Ellipse Inset with Geometric Seam Safety

Ellipse per cell with semi-axes `a = cell_w/2 − min_dist/2`, `b = cell_h/2 − min_dist/2`.
Geometric guarantee: any point inside G-Inset ellipse is at least `min_dist/2` from the tile edge,
so no point pair across a seam can violate `min_dist`. seam_kept must be exactly 1.000 — confirmed.

G-Corner variant: G-Inset ellipses + circles of radius `min_dist/2` at every grid intersection
to fill the corner dead zones created by the inset geometry.

### Coverage — 100×100, min_dist=5.0, seed=42

| Variant                   | pts (ov=5) | seam_kept | CV(5×5) | CV(10×10) |
|---------------------------|------------|-----------|---------|-----------|
| Bridson (reference)       | 259        | 1.000     | 0.128   | 0.314     |
| G-Inset two-pass          | 166        | 1.000     | 0.175   | 0.547     |
| G-Inset single-pass       | 166        | 1.000     | 0.175   | 0.547     |
| G-Corner fill             | 191        | 1.000     | 0.152   | 0.413     |

### Calibration — 100×100 (ceiling = 50.0)

| Variant               | Calibrated overage | Achieved pts | % of Bridson | CV(5×5) |
|-----------------------|--------------------|--------------|--------------|---------|
| Bridson (target)      | N/A                | 259          | 100%         | 0.128   |
| G-Inset two-pass      | 50.0 (ceiling)     | 220          | 84.9%        | 0.164   |
| G-Inset single-pass   | 50.0 (ceiling)     | 220          | 84.9%        | 0.164   |
| G-Corner fill         | 50.0 (ceiling)     | 245          | 94.6%        | 0.147   |
| global_rect (ref)     | 2.70               | 259          | 100%         | 0.115   |

*[G-Inset hits a hard structural ceiling regardless of overage. Raising ceiling from 20 to 50
moved G-Inset from 80.7% to 84.9% and G-Corner from 90.3% to 94.6% — the ceiling is real
but not perfectly flat; very high overage does recover a small number of additional points.
G-Corner still has a ~5% gap at ceiling=50. Structural cause: inscribed ellipse geometry
permanently excludes corner regions near grid intersections.]*

---

---

## Sample Elimination (Yuksel 2015)

Two modes implemented in `prime-research`:

1. **`sample_elimination`** — generate uniform random at `overage × target` count, then iteratively remove highest-weight point until `target` count remains. Weight function: `w(d) = max(0, 1 − d/min_dist)^alpha` (standard alpha=8).
2. **`sample_elimination_from_set`** — eliminate from an existing point set (e.g. LOD thinning).

### Coverage — 100×100, min_dist=5.0, seed=42 (generate-from-scratch mode)

Source: uniform random at `overage × target`; target = Bridson count (259 pts); alpha=8.0.

| Variant                       | raw pts | culled pts | violations_removed | CV(5×5) | CV(10×10) |
|-------------------------------|---------|------------|--------------------|---------|-----------|
| Bridson (reference)           | 259     | 259        | 0                  | 0.128   | 0.314     |
| sample_elim overage=5×        | 259     | 180        | 79                 | 0.219   | 0.478     |
| sample_elim overage=10×       | 259     | 172        | 87                 | 0.215   | 0.466     |
| sample_elim overage=20×       | 259     | 187        | 72                 | 0.208   | 0.439     |
| global_rect cov=12.73 (ref)   | —       | 336        | 0                  | 0.126   | 0.244     |

*[Raw output hits target (259 pts) but 72–87 pairs still violate min_dist — the elimination
weight function does not enforce hard exclusion. Post-cull removes violators: 172–187 pts
survive (~66–72% of Bridson). Yuksel 2015 recommends 25–64× overage for near-Bridson quality;
5–20× is insufficient for hard min-dist enforcement in this domain.]*

*[global_rect at calibrated overage produces 336 pts (130% of Bridson) with all min_dist satisfied
and CV=0.126 — no post-cull loss, no violations.]*

### Coverage — 200×200, min_dist=5.0, seed=42 (generate-from-scratch mode)

Target = Bridson count (1022 pts); alpha=8.0.

| Variant                            | pts   | CV(5×5) | CV(10×10) |
|------------------------------------|-------|---------|-----------|
| Bridson (reference)                | 1022  | 0.066   | 0.127     |
| sample_elim overage=5× post-cull   | 698   | 0.087   | 0.195     |
| sample_elim overage=10× post-cull  | 697   | 0.109   | 0.224     |
| global_rect cov=12.73 (reference)  | 1203  | 0.030   | 0.094     |

*[At 200×200 SE achieves 68–69% of Bridson count post-cull. CV improves with domain size
(0.087 vs 0.219 at 100×100) — larger domain has more candidates so elimination has more freedom.]*

---

---

## LOD Thinning (Sample Elimination from Existing Set)

Sample elimination's actual industrial use case: decimate a full-density point set
progressively for LOD levels while preserving blue-noise character.

### Bridson source thinning — 100×100, min_dist=5.0

Source: 259 Bridson points. Thin to 75%/50%/25%/10% via `sample_elimination_from_set`.

| Level          | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|----------------|-----|---------|-----------|-------------|
| Source (100%)  | 259 | 0.128   | 0.314     | true        |
| Thinned 75%    | 194 | 0.370   | 0.535     | true        |
| Thinned 50%    | 129 | 0.751   | 0.945     | true        |
| Thinned 25%    |  64 | 1.380   | 1.632     | true        |
| Thinned 10%    |  25 | 1.855   | 2.793     | true        |

*[SE thinning degrades uniformity rapidly with each level. CV≈0.37 at 75% is already nearly
3× worse than source. Min_dist is always satisfied (that is the hard constraint SE enforces),
but spatial uniformity is not preserved well through aggressive thinning.]*

### H single-pass source thinning — 100×100

Source: 215 H-triangles jitter=0 points.

| Level          | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|----------------|-----|---------|-----------|-------------|
| Source (100%)  | 215 | 0.161   | 0.402     | true        |
| Thinned 75%    | 161 | 0.579   | 0.729     | true        |
| Thinned 50%    | 107 | 0.981   | 1.199     | true        |
| Thinned 25%    |  53 | 1.710   | 1.876     | true        |
| Thinned 10%    |  21 | 3.080   | 3.387     | true        |

### Global rect source thinning — 100×100

Source: 336 global_rect (overage=12.73) points.

| Level          | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|----------------|-----|---------|-----------|-------------|
| Source (100%)  | 336 | 0.126   | 0.244     | true        |
| Thinned 75%    | 252 | 0.535   | 0.608     | true        |
| Thinned 50%    | 168 | 0.942   | 1.081     | true        |
| Thinned 25%    |  84 | 1.660   | 1.772     | true        |
| Thinned 10%    |  33 | 2.030   | 3.122     | true        |

---

---

## LOD Thinning vs Fresh Generation — Exact Same Count

Cross-comparison: does SE thinning from a Bridson source produce better blue-noise at
target density than freshly generating at that density via scatter-cull or Bridson?

Domain: 100×100, min_dist=5.0 enforced on all rows. Source: 259 Bridson points.
A) SE hits exact target by construction.
B–D) fresh approaches binary-searched (ceiling=50) to produce that exact count.
E) Bridson at scaled min_dist — cannot hit exact count, natural density shown.

### 75% density (SE output = 194 pts)

| Approach                              | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|---------------------------------------|-----|---------|-----------|-------------|
| A) SE thinned from Bridson            | 194 | 0.370   | 0.535     | true        |
| B) scatter_rect (ov=1.30, cal.)       | 196 | 0.156   | 0.438     | true        |
| C) H triangles (ov=2.56, cal.)        | 194 | 0.160   | 0.430     | true        |
| D) Voronoi K=10 (ov=3.00, cal.)       | 220 | 0.161   | 0.375     | true        |
| E) Bridson d=5.8 (natural, 197 pts)   | 197 | 0.154   | 0.403     | true        |

*[D Voronoi produced 220 pts — its minimum resolution at this overage exceeds the target; ceiling search cannot reduce below this count at the domain size.]*

### 50% density (SE output = 129 pts)

| Approach                              | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|---------------------------------------|-----|---------|-----------|-------------|
| A) SE thinned from Bridson            | 129 | 0.751   | 0.945     | true        |
| B) scatter_rect (ov=0.50, cal.)       | 133 | 0.275   | 0.630     | true        |
| C) H triangles (ov=0.96, cal.)        | 126 | 0.260   | 0.600     | true        |
| D) Voronoi K=10 (ov=1.00, cal.)       | 146 | 0.277   | 0.593     | true        |
| E) Bridson d=7.1 (natural, 133 pts)   | 133 | 0.204   | 0.476     | true        |

### 25% density (SE output = 64 pts)

| Approach                              | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|---------------------------------------|-----|---------|-----------|-------------|
| A) SE thinned from Bridson            |  64 | 1.380   | 1.632     | true        |
| B) scatter_rect (ov=0.20, cal.)       |  69 | 0.401   | 0.886     | true        |
| C) H triangles (ov=0.10, cal.)        |  85 | 0.372   | 0.804     | true        |
| D) Voronoi K=10 (ov=1.00, cal.)       | 146 | 0.277   | 0.593     | true        |
| E) Bridson d=10.0 (natural, 71 pts)   |  71 | 0.310   | 0.726     | true        |

*[D Voronoi at 25% cannot produce 64 pts — minimum resolution is 146. B and C at very low overage produce slightly more than SE target (binary search finds a floor, not exact).]*

---

---

## Wei 2008 Parallel Quality

Wei 2008: 4-phase checkerboard tiling. Each phase: tiles are conflict-free → Rayon `par_iter`.
Serial seed advances per tile; parallel seed advances per phase — counts differ legitimately.

### 100×100, min_dist=5.0, seed=42

| Approach                            | pts | CV(5×5) | CV(10×10) | min_dist_ok |
|-------------------------------------|-----|---------|-----------|-------------|
| Bridson serial (reference)          | 259 | 0.128   | 0.314     | true        |
| Wei serial (phase structure only)   | 278 | 0.106   | 0.247     | true        |
| Wei parallel (Rayon phase tiles)    | 283 | 0.111   | 0.274     | true        |
| H single-pass jitter=0 (calibrated) | 259 | 0.090   | 0.309     | true        |
| global_rect (calibrated 99.6%)      | 336 | 0.126   | 0.244     | true        |

### 200×200, min_dist=5.0, seed=42

| Approach                  | pts  | CV(5×5) | CV(10×10) | min_dist_ok |
|---------------------------|------|---------|-----------|-------------|
| Bridson serial (reference)| 1022 | 0.066   | 0.127     | true        |
| Wei parallel (Rayon)      | 1081 | 0.039   | 0.108     | true        |
| global_rect calibrated    | 1203 | 0.030   | 0.094     | true        |

*[Timing for parallel Wei vs scatter-cull parallel lives in `research_bench` (not yet run in release mode).]*

---

---

## Parallel Generation Head-to-Head

All parallel approaches (Rayon) applied to the same domains. Quality comparison only —
timing in `research_bench`. Tests run in debug mode.

Parameter note: `total_target` is candidates-to-generate, not output count.
- Bridson/Wei: `max_attempts=30` (their natural density heuristic)
- scatter_cull_rect / H triangles / voronoi: `total_target` scales with domain (300/1000/6000), `overage=3–5`
- scatter_global_rect: `total_target=30, overage=12.73` (calibrated to ~Bridson count — same as calibration tests)
- global_half_heart: `total_target` scales with domain, `overage=2`

scatter_cull_rect and global_half_heart at high total_target produce point counts ABOVE Bridson
because Bridson is a heuristic (30 attempts/point), not geometric max packing.

### 100×100, min_dist=5.0, seed=42

| Approach                                   | pts | CV(5×5) | CV(10×10) |
|--------------------------------------------|-----|---------|-----------|
| Bridson serial (reference)                 | 259 | 0.128   | 0.314     |
| Wei serial                                 | 278 | 0.106   | 0.247     |
| Wei parallel (Rayon)                       | 283 | 0.111   | 0.274     |
| scatter_cull_rect (total=300, ov=3)        | 321 | 0.146   | 0.293     |
| scatter_global_rect (total=30, ov=12.73)   | 336 | 0.126   | 0.244     |
| H triangles jitter=0 (total=300, ov=5)     | 215 | 0.161   | 0.402     |
| global_half_heart (total=300, ov=2)        | 320 | 0.094   | 0.238     |
| scatter_cull_voronoi K=10 (total=300, ov=3)| 315 | 0.172   | 0.331     |

### 200×200, min_dist=5.0, seed=42

| Approach                                    | pts  | CV(5×5) | CV(10×10) |
|---------------------------------------------|------|---------|-----------|
| Bridson serial (reference)                  | 1022 | 0.066   | 0.127     |
| Wei serial                                  | 1091 | 0.041   | 0.096     |
| Wei parallel (Rayon)                        | 1081 | 0.039   | 0.108     |
| scatter_cull_rect (total=1000, ov=3)        | 1323 | 0.047   | 0.149     |
| scatter_global_rect (total=30, ov=12.73)    | 1203 | 0.030   | 0.094     |
| H triangles jitter=0 (total=1000, ov=5)     |  866 | 0.063   | 0.163     |
| global_half_heart (total=1000, ov=2)        | 1318 | 0.043   | 0.092     |
| scatter_cull_voronoi K=10 (total=1000, ov=3)| 1286 | 0.051   | 0.112     |

### 500×500, min_dist=5.0, seed=42

| Approach                                    | pts   | CV(5×5) | CV(10×10) |
|---------------------------------------------|-------|---------|-----------|
| Bridson serial (reference)                  |  6299 | 0.019   | 0.042     |
| Wei serial                                  |  6852 | 0.021   | 0.042     |
| Wei parallel (Rayon)                        |  6861 | 0.024   | 0.041     |
| scatter_cull_rect (total=6000, ov=3)        |  8865 | 0.027   | 0.054     |
| scatter_global_rect (total=30, ov=12.73)    |  5760 | 0.018   | 0.043     |
| H triangles jitter=0 (total=6000, ov=5)     |  5613 | 0.023   | 0.055     |
| global_half_heart (total=6000, ov=2)        |  8295 | 0.014   | 0.027     |
| scatter_cull_voronoi K=10 (total=6000, ov=3)|  8080 | 0.022   | 0.040     |

---

## Calibrated Overage — 100×100, ceiling=50

Binary search (25 iter) for minimum overage ≥ Bridson count (259 pts), seed=42, min_dist=5.0.
`ceiling=50` replaces prior ceiling=20 tables — higher ceiling reveals true geometric limits.

| Approach                        | Cal. overage | pts | % Bridson | CV(5×5) | CV(10×10) | Notes                    |
|---------------------------------|:------------:|-----|:---------:|---------|-----------|--------------------------|
| Bridson (reference)             | —            | 259 | 100%      | 0.128   | 0.314     |                          |
| C-B rect 4×4                    | 4.767        | 257 | 99.2%     | 0.203   | 0.362     |                          |
| D Voronoi K=10                  | 7.000        | 255 | 98.5%     | 0.169   | 0.334     |                          |
| D-R recursive L=2               | **50.0**     | 219 | 84.6%     | 0.181   | 0.441     | **structural ceiling**   |
| F shear=0                       | 5.500        | 259 | 100.0%    | 0.122   | 0.314     |                          |
| F shear=0.5                     | 38.133       | 260 | 100.4%    | 0.420   | 0.544     | near-ceiling; CV terrible|
| E shift(-9,6)                   | 8.000        | 255 | 98.5%     | 0.159   | 0.369     |                          |
| E shift(-15,10)                 | 6.000        | 260 | 100.4%    | 0.119   | 0.312     |                          |
| global_rect                     | 2.700        | 259 | 100.0%    | 0.115   | 0.319     | best two-pass density/ov |
| global_voronoi K=10             | 29.000       | 259 | 100.0%    | 0.131   | 0.319     | high overage; limited    |
| global_half_heart (-15,10)      | 4.367        | 259 | 100.0%    | 0.133   | 0.300     |                          |
| H two-pass jitter=0.2           | 12.515       | 257 | 99.2%     | 0.162   | 0.340     |                          |
| H single-pass jitter=0.2        | 5.470        | 258 | 99.6%     | 0.142   | 0.311     |                          |
| H single-pass jitter=0.0        | 4.907        | 258 | 99.6%     | 0.105   | 0.330     |                          |
| G-Inset (two-pass + single)     | **50.0**     | 220 | 84.9%     | 0.164   | 0.464     | **geometric ceiling**    |
| G-Corner                        | **50.0**     | 245 | 94.6%     | 0.147   | 0.328     | **geometric ceiling**    |

## Calibrated Overage — 200×200, ceiling=50

target=1022 pts (Bridson), seed=42, min_dist=5.0.

| Approach                    | Cal. overage | pts  | % Bridson | CV(5×5) | CV(10×10) | Notes                  |
|-----------------------------|:------------:|------|:---------:|---------|-----------|------------------------|
| Bridson (reference)         | —            | 1022 | 100%      | 0.066   | 0.127     |                        |
| C-B rect 6×6                | 8.033        | 1022 | 100.0%    | 0.051   | 0.156     |                        |
| F shear=0 6×6               | 9.833        | 1021 | 99.9%     | 0.060   | 0.162     |                        |
| F shear=0.5 6×6             | **50.0**     |  977 | 95.6%     | 0.456   | 0.504     | **structural ceiling** |
| E shift(-15,10)             | 9.000        | 1037 | 101.5%    | 0.059   | 0.140     |                        |
| global_rect 6×6             | 5.367        | 1019 | 99.7%     | 0.047   | 0.112     |                        |
| global_half_heart (-15,10)  | 7.262        | 1021 | 99.9%     | 0.053   | 0.110     |                        |

## Calibrated Overage — 500×500, ceiling=50

target=6299 pts (Bridson), seed=42, min_dist=5.0.

| Approach                    | Cal. overage | pts  | % Bridson | CV(5×5) | CV(10×10) | Notes                  |
|-----------------------------|:------------:|------|:---------:|---------|-----------|------------------------|
| Bridson (reference)         | —            | 6299 | 100%      | 0.019   | 0.042     |                        |
| C-B rect 10×10              | 15.900       | 6302 | 100.0%    | 0.024   | 0.046     |                        |
| F shear=0 10×10             | 20.400       | 6298 | 100.0%    | 0.036   | 0.057     |                        |
| E shift(-15,10)             | 8.000        | 6214 | 98.7%     | 0.018   | 0.049     | ceiling-like at scale  |
| global_rect 10×10           | 11.733       | 6302 | 100.0%    | 0.019   | 0.037     |                        |
| global_half_heart (-15,10)  | 6.956        | 6299 | 100.0%    | 0.016   | 0.038     | best CV at 500×500     |
| G-Inset single-pass         | **50.0**     | 6129 | 97.3%     | 0.008   | 0.021     | **geometric ceiling**  |
| G-Corner                    | **50.0**     | 6250 | 99.2%     | 0.009   | 0.024     | near-ceiling at scale  |
| D-R recursive L=2           | **50.0**     | 5362 | 85.1%     | 0.023   | 0.054     | **structural ceiling** |
| H single-pass jitter=0.0    | 5.967        | 6295 | 99.9%     | 0.017   | 0.040     |                        |

---

## Overage Formula and R Analysis

### Derivation

Expected output count ≈ η × A / d² where η = π/(2√3) ≈ 0.6802 (hex packing fill).
Total candidates scattered = ncells × T × overage (for per-cell approaches) or T × overage (for total_target approaches).
Acceptance rate R = survivors / total_candidates.

Solving for overage: **overage = η × A / (d² × ncells × T × R)**

In practice use empirical target (Bridson count) instead of η × A / d²:
**R = target / (ncells × T × calibrated_overage)**

### R values by approach and domain

For two-pass approaches (ncells = cols × rows, T = total_target per cell):

| Approach           | 100×100 (4×4)  | 200×200 (6×6)  | 500×500 (10×10) | Stable? |
|--------------------|:--------------:|:--------------:|:---------------:|---------|
| C-B rect           | 0.113          | 0.118          | 0.132           | No — increases with cell size |
| F shear=0          | 0.098          | 0.096          | 0.103           | **Yes ≈ 0.099 ±4%** |

For single-pass approaches (ncells = cols × rows, T = total_target per cell):

| Approach           | 100×100 (4×4)  | 200×200 (6×6)  | 500×500 (10×10) | Stable? |
|--------------------|:--------------:|:--------------:|:---------------:|---------|
| global_rect        | 0.200          | 0.176          | 0.179           | **Yes ≈ 0.185 ±8%** |

For half_heart / H (T = total across domain):

| Approach           | 100×100         | 500×500        | Stable? |
|--------------------|:---------------:|:--------------:|---------|
| H single j=0.0    | 0.175           | 0.176          | **Yes ≈ 0.175 ±1%** |
| E shift(-15,10)    | 0.289           | ceiling        | Ceiling at 500×500 |

R calculation: `R = survivors / (ncells × T × overage)` for per-cell; `R = survivors / (T × overage)` for total_target.

### Scalability findings

**Stable R (formula-ready):**
- `global_rect`: R ≈ 0.185 → predict overage = target / (ncells × T × 0.185)
- `F shear=0`: R ≈ 0.099 → predict overage = target / (ncells × T × 0.099)
- `H single j=0.0`: R ≈ 0.175 → predict overage = target / (T × 0.175)

**Domain-dependent R:**
- `C-B rect`: R grows with cell_size/d. At 4×4 (cell=25, ratio=5): R≈0.113. At 10×10 (cell=50, ratio=10): R≈0.132. Linear trend: R ≈ 0.098 + 0.003 × (cell_w/d). Cell seam fraction shrinks as cells grow larger.
- `global_half_heart`: R varies with total_target scaling across tests; formula not yet isolated cleanly.

**Structural ceilings (formula inapplicable):**
- `G-Inset`: hard geometric ceiling ~85% regardless of overage (ellipse inset excludes corners)
- `G-Corner`: ceiling ~95% at 100×100, ~99% at 500×500 (corner fill helps more at scale)
- `D-R recursive`: ceiling ~85% at all scales (recursive Voronoi coordination overhead)
- `F shear=0.5`: ceiling ~96% at 200×200+ (brick offset creates correlated dead bands)
- `global_voronoi K=10`: requires ov≈29 at 100×100 (density limited by Voronoi cell geometry)

### Overage scaling with domain size

For approaches with stable R, calibrated overage grows with domain area A and ncells:

| Approach      | 100×100 ov | 200×200 ov | 500×500 ov | Growth pattern             |
|---------------|:----------:|:----------:|:----------:|----------------------------|
| global_rect   | 2.70       | 5.37       | 11.73      | ≈ linear in area/ncells    |
| C-B rect      | 4.77       | 8.03       | 15.90      | super-linear (cell scaling)|
| F shear=0     | 5.50       | 9.83       | 20.40      | super-linear               |
| H single j=0  | 4.91       | —          | 5.97       | nearly flat (total T scales)|
| global_half_heart | 4.37   | 7.26       | 6.96       | varies (T scaling in tests) |

H jitter=0 overage is nearly flat because total_target T was scaled proportionally to domain area
(300 → 6000 = 20× for 100× area increase), so ncells×T×overage tracks exactly.

---

## Notes

- Scatter and cull times recorded separately to isolate parallelizable phase
- Survivor rate = accepted / dropped
- Min-dist hold rate should be 100% — any failure is a bug
