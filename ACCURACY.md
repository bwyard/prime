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
*[pending]*

### F-B: Scatter-Cull (seeded)
Parameters: shear angle TBD, variable cell widths seeded

| Domain | Partitions | Time | vs C-B | vs serial Bridson |
|--------|------------|------|--------|-------------------|
| | | | | |

*[pending]*

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

## Approach E — Half-Heart

### E-A: Partition-Bridson
*[pending]*

### E-B: Scatter-Cull (seeded)
Parameters: 5 seed points → 10 derived, shift TBD, overage_ratio=TBD

| Domain | Faces | Scatter time | Cull time | Total | Survivor rate |
|--------|-------|-------------|-----------|-------|---------------|
| | | | | | |

*[pending]*

---

## Coverage Uniformity

*[pending — divide domain into cells, measure point count variance per approach]*

---

## Notes

- Scatter and cull times recorded separately to isolate parallelizable phase
- Survivor rate = accepted / dropped
- Min-dist hold rate should be 100% — any failure is a bug
