# Research Notes — Parallel Spatial Sampling
**Status:** In progress
**Branch:** feat/parallel-spatial-sampling
**Started:** 2026-04-05

---

## Hypothesis

**Scatter-Cull Spatial Sampling**

Standard Poisson disk sampling (Bridson) constructs a valid point set incrementally — placing one point at a time, confirming each satisfies the minimum distance constraint before accepting it. This is sequential and confirmatory by nature.

The proposed approach inverts this: rather than building up to a valid set, mass-drop a quantity of points significantly exceeding the expected final count into bounded regions simultaneously, then cull down to validity.

Each bounded region receives more points than it can keep. The minimum distance constraint is applied as a post-pass filter — removing points that violate proximity — rather than as a gate on insertion. The result is a valid Poisson disk sample derived from overage rather than constructed toward sufficiency.

The grain of rice analogy: you do not place each grain precisely. You pour a handful into a bounded area and remove the ones that land too close together. The pour is cheap and parallelizable. The cull is the real work, but it operates on a complete over-populated set rather than an incrementally growing one.

Why this may outperform standard Bridson for real-world RNG: the scatter phase can draw from true random sources (hardware entropy, physical noise) rather than a seeded PRNG advancing one candidate at a time. The statistical character of the resulting distribution reflects the quality of the random source directly, rather than being shaped by the sequential acceptance logic of the inner loop. For applications where distributional accuracy matters more than strict determinism, this is a meaningful difference.

**What is being tested:** Three bounded region geometries — rectangular partitions, recursive Voronoi cells, and parametric curve boundaries — are being implemented and compared against standard Bridson output for distribution quality, coverage uniformity, and performance at scale.

---

## What is being tested

Two parallel implementation tracks across three bounded region geometries, compared against two baselines.

### Determinism note

This is a deliberate research fork:

| Track | Determinism | Source of randomness | PRIME thesis contract |
|-------|-------------|---------------------|-----------------------|
| Partition-Bridson | ✅ Deterministic | Seeded PRNG (Mulberry32) | Preserved |
| Scatter-Cull (seeded) | ✅ Deterministic | Seeded PRNG (Mulberry32) | Preserved |
| Scatter-Cull (entropy) | ❌ Non-deterministic | Hardware entropy / physical noise | Intentional violation |

The entropy-fed scatter-cull is a deliberate departure from PRIME's pure contract. It is
included because the hypothesis claims it may produce statistically superior distributions —
the trade-off between reproducibility and distributional accuracy is part of what is being measured.

---

## Comparison Baselines

### Baseline 1 — Standard Bridson (Bridson 2007)
`poisson_disk` — sequential, confirmatory, seeded. The rewrite established its performance:
- 50×50: ~162 µs | 100×100: ~696 µs | 200×200: ~2.87 ms | 500×500: ~18.2 ms

### Baseline 2 — Wei's Parallel Algorithm (Wei 2008)
Li-Yi Wei, "Parallel Poisson Disk Sampling" — dart-throwing with parallel conflict resolution.
Wei uses a phase-based approach: divide into independent tiles, throw darts per phase, resolve
conflicts between adjacent tiles. This is the established parallel benchmark.
- **Not yet implemented.** Implement before drawing conclusions from the scatter-cull comparison.
- Reference: Wei, L.-Y. (2008). Parallel Poisson disk sampling. *ACM Trans. Graph.* 27(3).

---

## Implementation Tracks

### Track A — Partition-Bridson
Run Bridson independently inside each bounded region. Sequential within each partition, but
partitions are independent and could run in parallel. Seam buffers prevent cross-partition
violations without communication between partitions.

Three geometries:
- **Approach C** — Rectangular partitions (seam buffer inset of `min_dist/2`)
- **Approach D** — Voronoi K₁₀ cells via Lloyd relaxation
- **Approach E** — SDF curve boundaries (heart curve as canonical example)

Deterministic. PRIME thesis contract preserved.

### Track B — Scatter-Cull
Mass-drop a quantity of points (N × expected_fill_count) into each bounded region simultaneously,
then apply minimum distance culling as a post-pass. The pour is cheap and embarrassingly parallel.
The cull is the real work but operates on a complete over-populated set.

Three geometries (same as Track A — identical boundaries, different fill strategy):
- **Approach C-SC** — Rectangular partitions, scatter-cull fill
- **Approach D-SC** — Voronoi K₁₀ cells, scatter-cull fill
- **Approach E-SC** — SDF curve boundaries, scatter-cull fill

Two variants per geometry:
- **Seeded** — PRNG scatter, deterministic, thesis-compliant
- **Entropy** — hardware RNG scatter, non-deterministic, thesis-violating by design

---

## Measurement Plan

For each approach, measure:
1. **Performance** — wall time at 50×50, 100×100, 200×200, 500×500 (Criterion)
2. **Coverage** — point count vs theoretical max (Bridson typically achieves 60–80%)
3. **Uniformity** — variance in nearest-neighbor distances across the domain
4. **Seam quality** — point density within `min_dist` of partition boundaries
5. **Determinism** — same seed → identical output (seeded variants only)

---

## Approaches

### Bridson Rewrite (complete ✅)
- Replaced pure-fold with internal-mutation implementation
- External contract unchanged: `(params, seed) → Vec<(f32, f32)>`
- Both implementations coexist for benchmarking (`poisson_disk` + `poisson_disk_2d`)
- **Finding:** Internal mutation is 8.6× faster at 500×500 with pure external contract intact

### Approach C — Rectangular Partitions (Track A: Bridson | Track B: Scatter-Cull)
- Divide domain into N×M rectangular cells with seam buffer inset of `min_dist/2`
- Seeds: mixed per-partition via `wrapping_mul` to avoid seed correlation
- **Research question:** Does rectangular decomposition preserve distribution quality at seams?

### Approach D — Voronoi K₁₀ (Track A: Bridson | Track B: Scatter-Cull)
- K Voronoi cells via Lloyd relaxation (equal-area tendency)
- Fill each cell with rejection sampling against the polygon boundary + seam exclusion
- **Research question:** Does organic decomposition produce better seam quality than rectangular?

### Approach E — SDF Curve Partition (Track A: Bridson | Track B: Scatter-Cull)
- Domain split by a signed distance function of a closed curve
- Heart curve as canonical example; general `F: Fn(f32, f32) -> f32` as real API
- **Research question:** Is a pure function sufficient to define an arbitrary spatial boundary?

---

## Observations

_[Fill in as implementation proceeds]_

### Bridson Rewrite
Benchmark: `cargo bench -p prime-random -- poisson_disk` (min_dist=5.0, 30 attempts, seed=42)

| Domain    | Pure fold (poisson_disk_2d) | Internal mutation (poisson_disk) | Speedup |
|-----------|----------------------------|----------------------------------|---------|
| 50×50     | 221.61 µs                  | 162.06 µs                        | 1.4×    |
| 100×100   | 1249.8 µs                  | 696.18 µs                        | 1.8×    |
| 200×200   | 7594.3 µs                  | 2874.3 µs                        | 2.6×    |
| 500×500   | 157.52 ms                  | 18.24 ms                         | **8.6×** |

Speedup is non-linear — grows with domain size as expected. Grid clone cost compounds with
the number of points placed. At 500×500 the pure fold is nearly an order of magnitude slower.

- Determinism: confirmed — `poisson_disk` passes identical-output test
- Both implementations coexist for benchmarking; `poisson_disk_2d` marked as research-preserved
- Test delta: removed `poisson_disk_returns_seed` (new API does not thread seed out),
  added `poisson_disk_invalid_inputs_return_empty`

### Approach C
- Implementation notes: _[anything surprising]_
- Seam behavior: _[what the empty seam looks like at scale]_
- Determinism: _[confirmed?]_

### Approach D
- Lloyd relaxation convergence: _[how many iterations needed?]_
- Cell area uniformity: _[rough observation]_
- Determinism: _[confirmed?]_

### Approach E
- Heart SDF approximation accuracy: _[N=200 sample sufficient?]_
- Interior/exterior coverage: _[gap at boundary?]_
- Determinism: _[confirmed?]_

---

## Results

_[Bree writes this after implementation and testing]_

---

## References

- Bridson, R. (2007). Fast Poisson disk sampling in arbitrary dimensions. *SIGGRAPH sketches.*
- ADR-001: Pure functions only — internal mutation allowed when external contract is pure
- ADR-007: ADVANCE-EXCEPTION pattern — data-dependent termination
