/**
 * prime-random — Seeded deterministic randomness.
 *
 * All public functions are LOAD + COMPUTE. No STORE. No JUMP. No exceptions.
 *
 * The seed IS the thread position. Who holds the seed controls who can
 * advance it. Stop threading the seed forward to end the sequence —
 * no DELETE needed, the sequence is causally inert without its key.
 */
/**
 * Mulberry32 pure step — LOAD + COMPUTE only.
 *
 * @remarks
 * The seed IS the thread position. Pass nextSeed forward to continue.
 * Stop passing it to end the thread — no DELETE required.
 *
 * State: z0 = (seed + 0x6D2B79F5) & 0xFFFFFFFF
 * Output: bit-mixed z0 → float in [0, 1)
 * Period: 2^32
 *
 * @param seed - Current thread position (32-bit unsigned integer)
 * @returns [value in [0,1), nextSeed] — thread the nextSeed forward
 *
 * @example
 * const [v1, s1] = prngNext(42)
 * const [v2, s2] = prngNext(s1)
 */
export declare const prngNext: (seed: number) => [number, number];
/**
 * Pure float in [min, max).
 * @returns [value, nextSeed]
 * @example
 * const [v, s1] = prngRange(0, 2.0, 5.0) // 2 ≤ v < 5
 */
export declare const prngRange: (seed: number, min: number, max: number) => [number, number];
/**
 * Pure integer in [0, n).
 * @returns [value, nextSeed]
 * @example
 * const [i, s1] = prngRangeInt(0, 10) // 0 ≤ i < 10
 */
export declare const prngRangeInt: (seed: number, n: number) => [number, number];
/**
 * Pure boolean with probability p of true.
 * @returns [value, nextSeed]
 * @example
 * const [flip, s1] = prngBool(0, 0.5)
 */
export declare const prngBool: (seed: number, p: number) => [boolean, number];
/**
 * Pure Fisher-Yates shuffle — returns new array, original unchanged.
 * @returns [shuffledArray, nextSeed]
 * @example
 * const [shuffled, s1] = prngShuffled(42, [1, 2, 3, 4, 5])
 */
export declare const prngShuffled: <T>(seed: number, arr: readonly T[]) => [T[], number];
/**
 * Pure random element from array.
 * @returns [element | undefined, nextSeed]
 * @example
 * const [pick, s1] = prngChoose(0, ['a', 'b', 'c'])
 */
export declare const prngChoose: <T>(seed: number, arr: readonly T[]) => [T | undefined, number];
/**
 * Advance PRNG with external entropy mixed into the next seed.
 *
 * @remarks
 * Useful for injecting player input or network jitter into a deterministic
 * stream without breaking the pure-function contract.
 *
 * @param seed - Current thread position
 * @param entropy - External entropy value (XOR'd into next seed)
 * @returns [value in [0,1), nextSeed XOR'd with entropy]
 *
 * @example
 * const [v, s1] = prngNextWithEntropy(42, 0xDEADBEEF)
 */
export declare const prngNextWithEntropy: (seed: number, entropy: number) => [number, number];
/** A value with its causal parent recorded. */
export type CausalStep<T> = {
    readonly value: T;
    readonly parentSeed: number;
    readonly nextSeed: number;
};
/** prngNext with causal ancestry recorded. */
export declare const prngNextCausal: (seed: number) => CausalStep<number>;
/** prngGaussian with causal ancestry recorded. */
export declare const prngGaussianCausal: (seed: number) => CausalStep<number>;
/**
 * Weighted random choice — O(n) linear scan. Pure LOAD + COMPUTE.
 *
 * @remarks
 * Sample u ~ Uniform(0, sum(weights)).
 * Walk weights accumulating until accumulator ≤ 0 → return that index.
 *
 * @param seed - Thread position
 * @param weights - Non-negative weights. Must sum > 0.
 * @returns [chosenIndex, nextSeed]
 *
 * @example
 * const [i, s1] = weightedChoice(0, [1, 2, 1]) // index 1 is 2× likely
 */
export declare const weightedChoice: (seed: number, weights: readonly number[]) => [number, number];
/**
 * Box-Muller transform — single Gaussian sample from N(0, 1).
 *
 * # Math
 *   z = sqrt(-2 * ln(u1)) * cos(2 * pi * u2)
 *
 * @param seed - Thread position
 * @returns [gaussian value, nextSeed]
 *
 * @example
 * const [g, s1] = prngGaussian(42)
 */
export declare const prngGaussian: (seed: number) => [number, number];
/**
 * Box-Muller transform — both Gaussian values from the pair.
 *
 * # Math
 *   z0 = r * cos(theta), z1 = r * sin(theta)
 *   where r = sqrt(-2 * ln(u1)), theta = 2 * pi * u2
 *
 * @param seed - Thread position
 * @returns [z0, z1, nextSeed]
 *
 * @example
 * const [g0, g1, s1] = prngGaussianPair(42)
 */
export declare const prngGaussianPair: (seed: number) => [number, number, number];
/**
 * Exponential distribution sample via inverse CDF.
 *
 * # Math
 *   x = -ln(1 - u) / lambda
 *
 * @param seed - Thread position
 * @param lambda - Rate parameter (must be > 0)
 * @returns [exponential value, nextSeed]
 *
 * @example
 * const [e, s1] = prngExponential(42, 1.0)
 */
export declare const prngExponential: (seed: number, lambda: number) => [number, number];
/**
 * Uniform random point inside a disk of given radius.
 *
 * # Math
 *   angle = 2 * pi * u1
 *   dist = radius * sqrt(u2)   (sqrt for uniform area distribution)
 *
 * @param seed - Thread position
 * @param radius - Disk radius
 * @returns [x, y, nextSeed]
 *
 * @example
 * const [x, y, s1] = prngDiskUniform(42, 5.0)
 */
export declare const prngDiskUniform: (seed: number, radius: number) => [number, number, number];
/**
 * Uniform random point inside an annulus (ring) between rInner and rOuter.
 *
 * # Math
 *   angle = 2 * pi * u1
 *   dist = sqrt(rInner^2 + u2 * (rOuter^2 - rInner^2))
 *
 * @param seed - Thread position
 * @param rInner - Inner radius
 * @param rOuter - Outer radius
 * @returns [x, y, nextSeed]
 *
 * @example
 * const [x, y, s1] = prngAnnulusUniform(42, 3.0, 6.0)
 */
export declare const prngAnnulusUniform: (seed: number, rInner: number, rOuter: number) => [number, number, number];
/**
 * Van der Corput sequence — low-discrepancy 1D sequence.
 *
 * # Math
 *   Reflects the base-b digits of n about the decimal point.
 *
 * @param n - Sequence index (non-negative integer)
 * @param base - Number base (typically prime: 2, 3, 5, ...)
 * @returns Value in [0, 1)
 *
 * @example
 * const v = vanDerCorput(1, 2) // 0.5
 * const w = vanDerCorput(2, 2) // 0.25
 */
export declare const vanDerCorput: (n: number, base: number) => number;
/**
 * Halton sequence in 2D (bases 2 and 3).
 *
 * @param n - Sequence index
 * @returns [x, y] in [0, 1)^2
 *
 * @example
 * const [x, y] = halton2d(1) // [0.5, 0.333...]
 */
export declare const halton2d: (n: number) => [number, number];
/**
 * Halton sequence in 3D (bases 2, 3, and 5).
 *
 * @param n - Sequence index
 * @returns [x, y, z] in [0, 1)^3
 *
 * @example
 * const [x, y, z] = halton3d(1) // [0.5, 0.333..., 0.2]
 */
export declare const halton3d: (n: number) => [number, number, number];
/**
 * Monte Carlo integration of f over [a, b].
 *
 * # Math
 *   integral ≈ (b - a) * (1/n) * sum(f(x_i))
 *   where x_i ~ Uniform(a, b)
 *
 * @param seed - Thread position
 * @param f - Function to integrate
 * @param a - Lower bound
 * @param b - Upper bound
 * @param n - Number of samples
 * @returns [estimate, nextSeed]
 *
 * @example
 * const [integral, s1] = monteCarlo1d(42, Math.sin, 0, Math.PI, 10000)
 */
export declare const monteCarlo1d: (seed: number, f: (x: number) => number, a: number, b: number, n: number) => [number, number];
/**
 * Monte Carlo integration of f over [x0, x1] x [y0, y1].
 *
 * # Math
 *   integral ≈ area * (1/n) * sum(f(x_i, y_i))
 *   where (x_i, y_i) ~ Uniform([x0,x1] x [y0,y1])
 *
 * @param seed - Thread position
 * @param f - Function to integrate
 * @param x0 - Lower x bound
 * @param x1 - Upper x bound
 * @param y0 - Lower y bound
 * @param y1 - Upper y bound
 * @param n - Number of samples
 * @returns [estimate, nextSeed]
 *
 * @example
 * const [integral, s1] = monteCarlo2d(42, (x, y) => x * y, 0, 1, 0, 1, 10000)
 */
export declare const monteCarlo2d: (seed: number, f: (x: number, y: number) => number, x0: number, x1: number, y0: number, y1: number, n: number) => [number, number];
/**
 * Monte Carlo integration with variance estimate (Welford's online algorithm).
 *
 * # Math
 *   integral ≈ (b - a) * mean(f(x_i))
 *   variance via Welford's online algorithm for numerical stability
 *
 * @param seed - Thread position
 * @param f - Function to integrate
 * @param a - Lower bound
 * @param b - Upper bound
 * @param n - Number of samples
 * @returns [estimate, variance, nextSeed]
 *
 * @example
 * const [integral, variance, s1] = monteCarlo1dWithVariance(42, Math.sin, 0, Math.PI, 10000)
 */
export declare const monteCarlo1dWithVariance: (seed: number, f: (x: number) => number, a: number, b: number, n: number) => [number, number, number];
/**
 * SplitMix64 pure step — 64-bit PRNG with period 2^64.
 *
 * For applications needing longer sequences than Mulberry32's 2^32 period.
 * Same thesis contract: `(seed) -> [value, nextSeed]`.
 *
 * Uses JavaScript's native f64 arithmetic. Seeds are regular numbers
 * (safe integers up to 2^53-1). For full 64-bit fidelity, use BigInt variant.
 *
 * @param seed - Current thread position (non-negative integer)
 * @returns [value in [0,1), nextSeed]
 *
 * @example
 * const [v, s1] = prngNext64(42)
 */
export declare const prngNext64: (seed: number) => [number, number];
/**
 * 64-bit float in [min, max).
 * @returns [value, nextSeed]
 *
 * @example
 * const [v, s1] = prngRange64(42, 10.0, 20.0)
 */
export declare const prngRange64: (seed: number, min: number, max: number) => [number, number];
/**
 * 64-bit Gaussian via Box-Muller. Higher precision than f32 variant.
 * @returns [gaussian value, nextSeed]
 *
 * @example
 * const [z, s1] = prngGaussian64(42)
 */
export declare const prngGaussian64: (seed: number) => [number, number];
/**
 * Evaluate f(x) with memoization over a precomputed lookup table.
 *
 * Builds a table of n evenly-spaced samples in [a, b], then interpolates.
 * Thesis-compatible: the table is computed once (LOAD+COMPUTE), then
 * lookups are O(1) with linear interpolation (pure COMPUTE).
 *
 * @param f - Pure function to memoize
 * @param a - Lower bound of domain
 * @param b - Upper bound of domain
 * @param n - Number of table entries
 * @returns A closure that maps x -> f(x) approximately, with O(1) lookup cost
 *
 * @example
 * const fastSin = memoize1d(Math.sin, 0, Math.PI, 1000)
 * fastSin(1.0) // ≈ Math.sin(1.0)
 */
export declare const memoize1d: (f: (x: number) => number, a: number, b: number, n: number) => ((x: number) => number);
/**
 * Poisson disk sampling — minimum distance spacing in 2D. Pure LOAD + COMPUTE.
 *
 * @remarks
 * Bridson's algorithm (2007) expressed as a pure state fold (ADVANCE).
 * Each step is an immutable state transition: (state) → newState.
 * The seed threads through every random draw — no hidden state.
 *
 * Performance note: each step copies the spatial grid (O(cols×rows)).
 * For typical game use (domain < 2000×2000, minDist > 5) this is negligible.
 *
 * @param seed - Thread position — same seed → same point distribution
 * @param width - Sampling domain width
 * @param height - Sampling domain height
 * @param minDist - Minimum distance between any two points
 * @param maxAttempts - Candidates per active point (30 is standard)
 * @returns [points, nextSeed] — array of [x, y] tuples and the final seed
 *
 * @example
 * const [pts, s1] = poissonDisk2d(42, 100, 100, 10)
 */
export declare const poissonDisk2d: (seed: number, width: number, height: number, minDist: number, maxAttempts?: number) => [[number, number][], number];
//# sourceMappingURL=index.d.ts.map