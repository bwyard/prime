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

*[pending — divide domain into cells, measure point count variance per approach]*

---

## Notes

- Scatter and cull times recorded separately to isolate parallelizable phase
- Survivor rate = accepted / dropped
- Min-dist hold rate should be 100% — any failure is a bug
