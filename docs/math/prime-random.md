# prime-random — Mathematical Reference

Formulas, derivations, and convergence proofs for every algorithm in `prime-random`.

---

## 1. Mulberry32 PRNG

Hash-based PRNG with full $2^{32}$ period.

$$z_0 = (\text{seed} + \texttt{0x6D2B79F5}) \bmod 2^{32}$$
$$z_1 = (z_0 \oplus (z_0 \gg 15)) \cdot (z_0 \mid 1)$$
$$z_2 = z_1 \oplus \left(z_1 + (z_1 \oplus (z_1 \gg 7)) \cdot (z_1 \mid 61)\right)$$
$$\text{output} = (z_2 \oplus (z_2 \gg 14)) \;/\; 2^{32} \in [0, 1)$$

**Next seed:** $z_0$ (the incremented value before mixing).

**Properties:**
- Period: $2^{32}$ (every u32 seed maps to a unique next seed)
- Statistically uniform: passes SmallCrush, most of Crush
- Avalanche: single-bit input change flips ~50% of output bits
- Deterministic: same seed always produces same output

**Why Mulberry32:** Simpler than PCG (no 128-bit math), faster than xoshiro (fewer operations), sufficient statistical quality for procedural generation and Monte Carlo. The thesis requires a pure function `seed -> (value, nextSeed)` — Mulberry32's counter-based design maps directly to this.

---

## 2. Box-Muller Transform (Gaussian)

Converts two uniform samples to standard normal $N(0,1)$.

### Derivation

Given $u_1, u_2 \sim \text{Uniform}(0, 1)$, define:

$$r = \sqrt{-2 \ln u_1}, \quad \theta = 2\pi u_2$$

Then $(z_0, z_1) = (r\cos\theta, \; r\sin\theta)$ are independent $N(0,1)$.

**Proof sketch:** The joint density of $(z_0, z_1)$ in polar coordinates is:

$$f(r, \theta) = \frac{1}{2\pi} e^{-r^2/2} \cdot r$$

Integrating: $P(R \le r) = 1 - e^{-r^2/2}$, so $R = \sqrt{-2\ln U_1}$ by inverse CDF. $\Theta = 2\pi U_2$ is uniform on $[0, 2\pi)$. Independence follows from the factored density.

**Edge case:** $u_1$ is clamped to $\varepsilon$ to avoid $\ln(0) = -\infty$.

---

## 3. Exponential Distribution

Inverse CDF method for $X \sim \text{Exp}(\lambda)$.

$$F(x) = 1 - e^{-\lambda x} \implies F^{-1}(u) = -\frac{\ln(1-u)}{\lambda}$$

**Properties:**
- Mean: $E[X] = 1/\lambda$
- Variance: $\text{Var}(X) = 1/\lambda^2$
- Memoryless: $P(X > s+t \mid X > s) = P(X > t)$

---

## 4. Area-Uniform Geometric Sampling

### Disk

Sample uniformly by area inside a disk of radius $R$.

The area element in polar coordinates is $dA = \rho \, d\rho \, d\theta$. The CDF of $\rho$:

$$F(\rho) = \frac{\pi \rho^2}{\pi R^2} = \frac{\rho^2}{R^2}$$

Inverse CDF: $\rho = R\sqrt{u}$, with $\theta = 2\pi v$.

**Why sqrt:** Without sqrt, points cluster near the center. The sqrt compensates for the increasing circumference at larger radii.

### Annulus

Sample uniformly by area in annulus $[r_1, r_2]$:

$$F(\rho) = \frac{\rho^2 - r_1^2}{r_2^2 - r_1^2}$$

$$\rho = \sqrt{r_1^2 + u \cdot (r_2^2 - r_1^2)}$$

For Bridson's annulus $[r, 2r]$:

$$\rho = \sqrt{r^2 + 3r^2 u} = r\sqrt{1 + 3u}$$

**Previous bug:** The original code used $\rho = r + ur$ (linear), which oversamples the inner ring by a factor of ~2. The corrected formula gives equal probability per unit area.

---

## 5. Bridson's Algorithm (2007)

Poisson disk sampling: place points with minimum distance $r$ apart.

### Algorithm

1. Initialize grid with cell size $r/\sqrt{2}$ (guarantees at most one point per cell)
2. Place first point randomly, add to active list
3. While active list is non-empty:
   - Pick random active point $p$
   - Generate $k$ candidates in annulus $[r, 2r]$ around $p$ (area-uniform)
   - If any candidate is $\ge r$ from all existing neighbors: accept, add to active list
   - If all $k$ candidates fail: remove $p$ from active list

### Grid acceleration

Neighbor check uses a 5x5 cell neighborhood (radius 2 cells). Since cell size $= r/\sqrt{2}$, any point within distance $r$ must be in this neighborhood.

### Termination

Each step either adds a point to the active list or removes one. Points are added at most once (each occupies a unique grid cell). The grid has $\lceil W/c \rceil \times \lceil H/c \rceil$ cells. Therefore the algorithm terminates in at most $2 \cdot |\text{cells}|$ steps.

### Packing density

Theoretical maximum for disk packing: $\pi/(2\sqrt{3}) \approx 0.9069$.
Bridson typically achieves 60-80% of theoretical maximum density.
Area-uniform annulus sampling improves acceptance rate vs linear sampling.

### Implementation

Uses `std::iter::successors` (Rust) and a `successors` utility (TypeScript) — pure state-machine iteration with no mutation. Each step takes `&State` and returns a new owned `State`.

---

## 6. Van der Corput / Halton Sequences

### Van der Corput

The radical inverse $\phi_b(n)$ reflects the base-$b$ digits of $n$ around the decimal point:

$$n = \sum_{i=0}^{k} d_i \cdot b^i \implies \phi_b(n) = \sum_{i=0}^{k} d_i \cdot b^{-(i+1)}$$

**Examples (base 2):**
- $\phi_2(1) = \phi_2(1_2) = 0.1_2 = 0.5$
- $\phi_2(5) = \phi_2(101_2) = 0.101_2 = 0.625$

### Halton sequence

$d$-dimensional Halton uses the first $d$ primes as bases:

$$H_d(n) = (\phi_2(n), \phi_3(n), \phi_5(n), \ldots)$$

### Low-discrepancy property

The star discrepancy $D^*_N$ of the Halton sequence satisfies:

$$D^*_N = O\left(\frac{(\log N)^d}{N}\right)$$

vs pseudo-random sequences: $D^*_N = O(1/\sqrt{N})$.

By the **Koksma-Hlawka inequality**, integration error is bounded by:

$$\left| \frac{1}{N}\sum_{i=1}^{N} f(x_i) - \int f \right| \le V(f) \cdot D^*_N$$

where $V(f)$ is the variation of $f$ in the sense of Hardy and Krause. This means quasi-random sequences converge faster for functions with bounded variation.

---

## 7. Monte Carlo Integration

### Plain estimator

$$\hat{I} = \frac{b-a}{n} \sum_{i=1}^{n} f(x_i), \quad x_i \sim \text{Uniform}(a, b)$$

**Unbiasedness:** $E[\hat{I}] = (b-a) \cdot E[f(U)] = \int_a^b f(x) \, dx$

**Variance:** $\text{Var}(\hat{I}) = \frac{(b-a)^2}{n} \text{Var}(f(U))$

**Convergence:** $\text{RMSE} = O(1/\sqrt{n})$ by CLT.

### Stratified estimator

Divide $[a, b]$ into $n$ equal strata of width $h = (b-a)/n$. Sample one point per stratum:

$$\hat{I}_{\text{strat}} = \frac{b-a}{n} \sum_{i=0}^{n-1} f\left(a + (i + u_i) \cdot h\right), \quad u_i \sim \text{Uniform}(0, 1)$$

**Variance reduction:** For smooth $f$:

$$\text{Var}(\hat{I}_{\text{strat}}) \le \frac{(b-a)^2}{12n^2} \int_a^b |f'(x)|^2 \, dx$$

This gives $O(1/n)$ convergence for $C^1$ functions — quadratically faster than plain MC.

### Welford's online algorithm

Numerically stable running variance (avoids catastrophic cancellation):

$$\delta = x_k - \bar{x}_{k-1}$$
$$\bar{x}_k = \bar{x}_{k-1} + \delta / k$$
$$M_{2,k} = M_{2,k-1} + \delta \cdot (x_k - \bar{x}_k)$$
$$\text{Var} = M_{2,n} / (n-1)$$

---

## 8. Weighted Choice

### Algorithm

Given weights $w_1, \ldots, w_k$ with $W = \sum w_i > 0$:

1. Sample $u \sim \text{Uniform}(0, W)$
2. Walk weights: find smallest $j$ such that $\sum_{i=1}^{j} w_i \ge u$

**Correctness:** $P(\text{choose } j) = w_j / W$

**Complexity:** $O(k)$ per sample.

---

## 9. Fisher-Yates Shuffle

### Algorithm

For $i = n-1$ down to $1$: swap element $i$ with a random element from $[0, i]$.

### Correctness

Each of the $n!$ permutations is equally probable.

**Proof by induction:** After step $i = n-1$, any element is equally likely to be in position $n-1$ (probability $1/n$). After step $i = n-2$, any remaining element is equally likely in position $n-2$ (probability $1/(n-1)$ conditional). The joint probability of any permutation is:

$$\frac{1}{n} \cdot \frac{1}{n-1} \cdot \frac{1}{n-2} \cdots \frac{1}{1} = \frac{1}{n!}$$

**Complexity:** $O(n)$ time, $O(n)$ space (returns new Vec, original unchanged).

---

## 10. Entropy Escape Hatch

$$\text{prng\_next\_with\_entropy}(\text{seed}, e) = (v, \; s' \oplus e)$$

where $(v, s') = \text{prng\_next}(\text{seed})$.

XOR preserves the seed-threading contract. With $e = 0$, behavior is identical to `prng_next`. External randomness can be mixed in without breaking the pure function signature.

---

## Benchmark Results (2026-03-28)

| Function | Time | Notes |
|----------|------|-------|
| prng_next (1K chain) | 1.90 µs | ~2 ns/op, near theoretical minimum |
| prng_gaussian (1K chain) | 1.91 µs | 5% overhead vs prng_next |
| poisson_disk_2d 100x100 | 1.11 ms | Grid cloning dominates |
| monte_carlo_1d n=10K | 95.8 µs | Bottleneck is f(x), not PRNG |
| monte_carlo_1d_stratified n=10K | 64.3 µs | 33% faster, O(1/n) convergence |
| van_der_corput 1K | 20.4 µs | Division-heavy digit extraction |
| halton_2d 1K | 19.5 µs | ~2x vdc as expected |
| weighted_choice n=100 | 127 ns | O(n) linear scan |
