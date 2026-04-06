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
- Approach C — rectangular partitions (equal-size, axis-aligned)
- Approach D — Voronoi K₁₀ (recursive)
- Approach E — half-heart Bézier
- Approach F — sheared variable-size partitions (straight edges, non-orthogonal)

Benchmarked against:
- Serial Bridson (baseline — rewrite already done)
- Wei's parallel method (to be implemented)
- Standard Poisson disk quality metrics (blue noise spectrum, coverage uniformity, min-dist guarantee)

**No conclusions are drawn until benchmarking is complete. All approaches are experimental.**

---

## Structural Density Ceiling — Finding and Resolution

*(Observational — from calibration data 2026-04-05)*

### Two-pass ceiling (per-cell cull → global cull)

All two-pass scatter-cull approaches fail to reach Bridson's point density even at extreme overage (20×).
Maximum achievable at 100×100, min_dist=5.0: 94–96% of Bridson's count (E best at 96.1%).

**Identified cause:** cell seams create dead zones. The per-cell cull removes candidates near cell
boundaries before the global cull can resolve them. Adjacent cells independently discard their
seam-region candidates, leaving dead zones that no amount of overage can fill.

### Single-pass resolution (scatter + single global cull)

The solution tested: skip per-cell cull entirely. Scatter using cell structure for seed organisation
only, flatten all candidates, one global `cull_to_min_dist` pass. Candidates from both sides of every
seam compete in the same pass.

**`scatter_global_rect` result (2026-04-05):** reaches 258/259 pts (99.6% Bridson) at overage=12.73.
No ceiling. Two-pass C-B hits the ceiling at overage=20 with 244 pts (94.2%).

**CV(5×5) at overage=2.0: 0.121** — slightly better than Bridson's 0.128 on the coarse grid.

| Approach             | Calibrated overage | Achieved pts | % of Bridson |
|----------------------|--------------------|--------------|--------------|
| global_rect          | **12.73**          | **258**      | **99.6%**    |
| global_voronoi K=10  | 20.0 (ceiling)     | 251          | 96.9%        |
| global_half_heart    | 20.0 (ceiling)     | 218          | 84.2%        |

`global_voronoi` (251 pts, 96.9%) and `global_half_heart` (249 pts, 96.1%) still hit the 20× ceiling.
Both scatter uniformly across the full domain via K streams — functionally a single large uniform scatter
(~6000 candidates at ceiling). Diminishing returns at this density prevent filling all packing-optimal positions.
CV(5×5) at ceiling: global_half_heart=0.104, global_voronoi=0.134 — both better than Bridson's 0.128.

**Why global_rect breaks the ceiling while voronoi/half_heart don't:**
global_rect generates 16×target×overage candidates with spatially-tiled scatter — each of 16 cells
contributes dedicated candidates to its sub-region. At calibrated overage=12.73: ~6110 candidates
with complete spatial coverage. global_voronoi/half_heart scatter K×target×overage uniformly across
the full domain — no sub-region guarantee. Their ceiling is the fundamental limit of pure random
scatter over the full domain at ~6000 candidates.

**Conclusion:** the structural density ceiling was entirely an artifact of the two-pass cull architecture.
Removing per-cell cull resolves it when scatter is spatially tiled (global_rect: 99.6% Bridson).
For architecturally uniform scatter (global_voronoi, global_half_heart), a smaller ceiling (96–97%)
remains — a property of the scatter strategy, not the cull architecture.

---

## Open Questions / Deferred Decisions

| Item | Status | Note |
|------|--------|------|
| `seed: u64` in public API | Deferred | Keep `u32` for now, add `u64` later — all of `prime-random` is `u32` |
| Approach E shift transform | Deferred | Bree: "point at (4,4) on a grid then moving diagonally to something like (-5,10), just a straight diagonal — play with the shift later, use a formula to calculate what's best" |
| Approach D survivor rate target | Observational | Bree: "purely observational right now" — measure and log, no target |
| `bridson_parallel/src/` | Doesn't exist | Referenced in handoff v2 but was never created. What we have been working on is effectively it. Work goes in `prime-spatial`. |
| Approach F shear angle | Deferred | Use half-cell-width offset per row (brick pattern) as first candidate, then sweep angles observationally |
| Approach F variable-size strategy | Deferred | Options: (a) seeded random weights normalised to domain, (b) explicit width/height arrays, (c) size drawn from a distribution |

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

## Implementation Order (from handoff v2, updated 2026-04-05)

1. ~~Bridson rewrite~~ ✅ done
2. ~~Approach C — rectangular scatter-cull~~ ✅ done (equal-size, axis-aligned)
3. ~~Wei 2008 baseline~~ ✅ done (single-threaded, in `research.rs`)
4. ~~Approach D — Voronoi K₁₀ scatter-cull~~ ✅ done (single-level, K=10, 3 Lloyd iters)
5. ~~Approach F — sheared variable-size scatter-cull~~ ✅ done (shear=0.5 and variable_rect variants)
6. ~~Approach E — half-heart scatter-cull~~ ✅ first pass done (diagonal + shift Voronoi)
7. ~~Single-pass global-cull variants~~ ✅ done (scatter_global_rect, scatter_global_voronoi, scatter_global_half_heart)

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

See `ACCURACY.md` for full data.

**C-A (Partition-Bridson, reference):** single-threaded overhead makes it slightly slower
than serial Bridson. Will improve with Rayon `par_iter`.

**C-B (Scatter-Cull):** 24–143× faster than partition-Bridson single-threaded.
172× faster than serial Bridson at 500×500. Min-dist guarantee 100%. Deterministic.

### Approach D — Voronoi K₁₀ Scatter-Cull

See `ACCURACY.md` for full data.

**D-B (Scatter-Cull, K=10, 3 Lloyd iters):** slower than C-B at 100×100 (Lloyd overhead),
faster at 200×200 and 500×500. 215× faster than serial Bridson at 500×500.

Unexpected finding: D-B outperforms C-B at scale despite the Lloyd relaxation cost.
Hypothesis: K=10 coarse Voronoi cells → less total cull work per cell at larger domains;
Lloyd convergence produces more uniform per-cell point density than rectangular partitioning.

### Approach E — Half-Heart Scatter-Cull

See `ACCURACY.md` for full data.

First observational pass. Sites: N seeds along a 45° diagonal + N shifted copies
= 2N Voronoi sites. The paired layout creates elongated irregular cells oriented
along the diagonal.

**Key observation:** shift(-9,6) at 100×100 with 10 cells is 16.6µs — marginally
faster than C-B at 17.2µs. No Lloyd overhead, no rectangular grid overhead.

**Open question — boundary cells:** cells that straddle the domain edge are clipped
by the boundary. Interior cells keep the elongated lobe shape; edge cells become
irregular fragments. This is acceptable — the goal is abnormal partition shapes,
not exact half-heart geometry. Proportion of interior vs clipped cells not yet measured.

**Next step:** fixed n_seeds across domain sizes for fair comparison; vary
diagonal_angle; vary shift magnitude. Still purely observational.

### Approach F — Sheared Variable-Size Scatter-Cull

See `ACCURACY.md` for full data.

**F shear=0.5:** slower than C-B across all domains. 11.6× slower at 500×500.
**F variable_rect (no shear):** slower than C-B but less extreme.

Performance penalty is partly implementation overhead — F uses the full domain grid for culling
rather than a per-cell grid (required because sheared candidates can land anywhere in the domain
after domain-bounds filtering). This is an optimisation deferred post-research.

**Open question (observational):** Does F produce better coverage uniformity than C despite
the performance cost? Coverage CV comparison pending — the whole point of F is geometric
quality, not speed.

### Wei 2008 Baseline

See `ACCURACY.md` for full data.

Single-threaded Wei is 2.4–7.5× **slower** than serial Bridson. Phase overhead and convergence
loop dominate without actual parallel execution. Wei's performance case requires Rayon `par_iter`
on the inner tile loops — expected to scale near-linearly with core count up to tile count.

---

---

## Approach F — Sheared Variable-Size Partitions

*(Design session 2026-04-05. Bree: "still straight lines but not 90 degree angles" and "non-equal sized partitions".)*

**Motivation:** Approach C uses equal-size axis-aligned cells. The regular orthogonal grid produces
repeating seam artifacts at the same x and y coordinates across all rows/columns. Two proposed changes:

**F-1: Non-equal cell sizes.** Cell widths and heights are drawn from a seeded distribution
rather than computed as `domain / partition_count`. Each row gets a different column width profile.
This breaks the vertical seam alignment. Sizes must sum to the domain dimension exactly.

**F-2: Shear (non-orthogonal edges).** Cells are parallelograms rather than rectangles. Row $r$
has its origin shifted by $r \cdot s$ in the x direction, where $s$ is a shear parameter.

$$x_{\text{corner}} = \text{col} \cdot w_{\text{col}} + \text{row} \cdot s$$

A candidate point $(p_x, p_y)$ belongs to the sheared cell if:

$$\text{col} = \left\lfloor \frac{p_x - \frac{p_y}{h_{\text{row}}} \cdot s}{w_{\text{col}}} \right\rfloor$$

The scatter phase generates points in the parallelogram unit basis:
given $u, v \in [0,1]$:
$$x = x_{\text{corner}} + u \cdot w + v \cdot s, \quad y = y_{\text{corner}} + v \cdot h$$

This is a linear transformation of the unit square — the Jacobian is 1 (area preserving), so
point density per unit area is unchanged from the axis-aligned case.

**Combined effect:** non-equal sizes + shear eliminate both the periodic column seams (unequal widths)
and the horizontal seam alignment (shear offsets rows). Predicted result: more uniform blue-noise
character at partition boundaries.

**F current status:** planned, not yet implemented. Implement after D is complete.

---

## References

- Bridson, R. (2007). Fast Poisson disk sampling in arbitrary dimensions. *SIGGRAPH sketches.*
- Wei, L.-Y. (2008). Parallel Poisson disk sampling. *ACM Trans. Graph.* 27(3).
- ADR-001: Pure functions only — internal mutation allowed when external contract is pure
- ADR-007: ADVANCE-EXCEPTION — data-dependent termination
