# Research Notes — Parallel Spatial Sampling (Scatter-Cull)
**Branch:** feat/parallel-spatial-sampling
**Started:** 2026-04-05
**Handoff version:** v2 (v1 discarded — see note below)

---

## Note on v1

The v1 handoff described Approaches C, D, E as running Bridson inside each partition.
That was wrong. The actual design is scatter-cull. v1 results are preserved below
under "Bridson Rewrite" since the benchmarks are still valid baseline data.
Everything from "Scatter-Cull Design" onward reflects v2.

---

## Hypothesis

*(Bree's words, from design session and handoff v2)*

Standard Poisson disk sampling (Bridson 2007) constructs a valid point set incrementally
— placing one point at a time, confirming each satisfies the minimum distance constraint
before accepting it. Sequential and confirmatory by nature.

Wei (2008) parallelizes this but preserves the same fundamental direction: regions
populate concurrently, but the logic remains confirmatory — throw a candidate, check
for conflict, accept or reject. Inter-region coordination is still required at boundaries
to resolve violations during construction.

**The proposed approach — scatter-cull:** rather than building up to a valid set,
mass-drop significantly more points than needed into bounded regions simultaneously,
then cull down to validity. No inter-region coordination is required during the scatter
phase. Each region is intentionally over-populated independently. The minimum distance
constraint is applied as a post-pass filter — removing points that violate proximity —
rather than as a gate on insertion. Because every region has overage, boundary conflicts
are resolved from surplus rather than negotiated between regions during construction.

**The grain of rice analogy (Bree):** you do not place each grain precisely. You pour
a handful into a bounded area and remove the ones that land too close together. The pour
is cheap and parallelizable. The cull is the real work, but it operates on a complete
over-populated set rather than an incrementally growing one.

**Distinction from Wei (from handoff v2):** Wei parallelizes the placement pass while
preserving confirmatory logic. Scatter-cull parallelizes the population pass and defers
all constraint work to a single cull pass, eliminating inter-region conflict resolution
during construction.

**Why this may outperform Bridson and Wei for real-world RNG (from handoff v2):**
The scatter phase can draw from true random sources — hardware entropy, physical noise
— rather than a seeded PRNG advancing one candidate at a time. The statistical character
of the distribution reflects the random source directly, rather than being shaped by
sequential acceptance logic.

**Goal (Bree, 2026-04-05):** See if scatter-cull can be faster than Bridson.
"This seems more realistic." Purely observational right now — no target numbers,
we want to see what the data says.

---

## What Is Being Tested

Two implementation strategies across three bounded region geometries.

**Strategy 1 — Partition-Bridson:** run standard Bridson inside each partition independently.
Kept as reference/comparison — not the hypothesis, but needed to isolate whether the geometry
alone contributes to any performance or quality difference.

**Strategy 2 — Scatter-Cull:** mass-drop then cull inside each partition. The hypothesis.

Three geometries, both strategies each:
- Approach C — rectangular partitions
- Approach D — Voronoi K₁₀ (recursive)
- Approach E — half-heart Bézier

Benchmarked against:
- Serial Bridson (baseline — rewrite already done)
- Wei's parallel method (to be implemented)
- Standard Poisson disk quality metrics (blue noise spectrum, coverage uniformity, min-dist guarantee)

**No conclusions are drawn until benchmarking is complete. All approaches are experimental.**

---

## Open Questions / Deferred Decisions

| Item | Status | Note |
|------|--------|------|
| `seed: u64` in public API | Deferred | Keep `u32` for now, add `u64` later — all of `prime-random` is `u32` |
| Approach E shift transform | Deferred | Bree: "point at (4,4) on a grid then moving diagonally to something like (-5,10), just a straight diagonal — play with the shift later, use a formula to calculate what's best" |
| Approach D survivor rate target | Observational | Bree: "purely observational right now" — measure and log, no target |
| `bridson_parallel/src/` | Doesn't exist | Referenced in handoff v2 but was never created. What we have been working on is effectively it. Work goes in `prime-spatial`. |

---

## Concrete Parameters (from handoff v2 design session)

**Approach C (rectangular):**
- Target: 10,000 final points
- Overage: 50% → drop 15,000 points across 40 partitions
- Per partition: 500 drops, cull to ~250 survivors
- Overage ratio: 1.5×
- Small-scale validation: target 10, drop 12 (1.2× overage, 16% excess)

**Approach D (Voronoi K₁₀):**
- K = 10 Voronoi sites per level
- `log₁₀(n)` levels of recursion
- Survivor rate: observational — validate empirically, claim is scale-invariance across levels

**Approach E (half-heart):**
- 5 seed points shifted along a diagonal → 10 derived points
- Each adjacent pair: closed cell = one straight edge + one cubic Bézier half-heart lobe
- Bulge biased toward apex; sharp cusp at origin; smooth rounded apex transition
- Each of the 10 points connects to all 4 domain boundaries, angle constraint [30°, 120°]
- Expected: 500–700 irregular partition faces via planar arrangement

---

## Measurement Plan

For each approach, record in `ACCURACY.md`:
1. **Performance** — wall time total; scatter phase and cull phase separately (Criterion)
2. **Survivor rate** — dropped / accepted per approach
3. **Min-dist guarantee** — should be 100%; any failure is a bug
4. **Coverage uniformity** — divide domain into cells, measure point count variance
5. **Determinism** — same seed → identical output (seeded variants only)

Blue noise spectral analysis (radial power spectrum) — deferred until basic results in.

---

## Implementation Order (from handoff v2)

1. ~~Bridson rewrite~~ ✅ done
2. Approach C — rectangular scatter-cull (simplest, validates the pattern end to end)
3. Approach D — Voronoi K₁₀ scatter-cull (irregular cells + recursion)
4. Approach E — half-heart scatter-cull (most complex, implement last)

Wire Criterion benchmarks after each approach before moving to the next.

---

## Results

### Bridson Rewrite (complete ✅)

Benchmark: `cargo bench -p prime-random -- poisson_disk` (min_dist=5.0, 30 attempts, seed=42)

| Domain  | Pure fold (`poisson_disk_2d`) | Internal mutation (`poisson_disk`) | Speedup  |
|---------|-------------------------------|------------------------------------|----------|
| 50×50   | 221.61 µs                     | 162.06 µs                          | 1.4×     |
| 100×100 | 1249.8 µs                     | 696.18 µs                          | 1.8×     |
| 200×200 | 7594.3 µs                     | 2874.3 µs                          | 2.6×     |
| 500×500 | 157.52 ms                     | 18.24 ms                           | **8.6×** |

*[Claude notation: speedup is non-linear, grows with domain size. Grid clone cost compounds
with point count. Both implementations coexist — `poisson_disk_2d` preserved for comparison.]*

- Determinism: confirmed — identical-output test passes
- Tests: removed `poisson_disk_returns_seed` (new API does not thread seed out),
  added `poisson_disk_invalid_inputs_return_empty`

### Approach C — Rectangular Scatter-Cull
*[pending]*

### Approach D — Voronoi K₁₀ Scatter-Cull
*[pending]*

### Approach E — Half-Heart Scatter-Cull
*[pending]*

### Wei 2008 Baseline
*[pending — implement before drawing conclusions from scatter-cull comparison]*

---

## References

- Bridson, R. (2007). Fast Poisson disk sampling in arbitrary dimensions. *SIGGRAPH sketches.*
- Wei, L.-Y. (2008). Parallel Poisson disk sampling. *ACM Trans. Graph.* 27(3).
- ADR-001: Pure functions only — internal mutation allowed when external contract is pure
- ADR-007: ADVANCE-EXCEPTION — data-dependent termination
