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
*[pending]*

---

## Approach C — Rectangular Partitions

### C-A: Partition-Bridson
*[pending]*

### C-B: Scatter-Cull (seeded)
Parameters: 40 partitions, 500 drops/partition, overage_ratio=1.5

| Domain | Scatter time | Cull time | Total | Survivor rate | Min-dist hold |
|--------|-------------|-----------|-------|---------------|---------------|
| | | | | | |

*[pending]*

---

## Approach D — Voronoi K₁₀

### D-A: Partition-Bridson
*[pending]*

### D-B: Scatter-Cull (seeded)
Parameters: K=10, lloyd_iterations=3, overage_ratio=TBD

| Domain | Recursion depth | Scatter time | Cull time | Total | Survivor rate |
|--------|----------------|-------------|-----------|-------|---------------|
| | | | | | |

*[pending]*

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
