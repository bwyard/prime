/**
 * prime-voronoi — Voronoi diagrams, Lloyd relaxation, and Delaunay triangulation.
 *
 * All exported functions are pure (LOAD + COMPUTE only). No mutation, no side effects.
 */

// ── Voronoi nearest ───────────────────────────────────────────────────────────

/**
 * Find the nearest seed index and Euclidean distance from `query`.
 *
 * Math: `(index, dist) = argmin_i dist(query, seeds[i])`
 *
 * @param query - `[x, y]` query point
 * @param seeds - array of `[x, y]` seed points
 * @returns `[index, distance]` of nearest seed, or `null` if seeds is empty
 *
 * @example
 * voronoiNearest2d([0.1, 0.1], [[0, 0], [1, 0], [0, 1]]) // [0, ~0.141]
 */
export const voronoiNearest2d = (
  query: [number, number],
  seeds: readonly [number, number][],
): [number, number] | null => {
  if (seeds.length === 0) return null

  const [idx, d2] = seeds.reduce(
    ([bestIdx, bestD2], [sx, sy], i) => {
      const dx = query[0] - sx
      const dy = query[1] - sy
      const d2 = dx * dx + dy * dy
      return d2 < bestD2 ? [i, d2] : [bestIdx, bestD2]
    },
    [0, Infinity] as [number, number],
  )

  return [idx, Math.sqrt(d2)]
}

// ── Voronoi F1 + F2 ───────────────────────────────────────────────────────────

/**
 * Compute F1 (nearest) and F2 (second-nearest) Euclidean distances.
 *
 * Used for edge detection and cellular noise patterns on Voronoi diagrams.
 *
 * @param query - `[x, y]` query point
 * @param seeds - array of `[x, y]` seed points
 * @returns `[f1, f2]` or `null` if seeds is empty
 *
 * @example
 * voronoiF1F2_2d([0.3, 0], [[0, 0], [1, 0]]) // [0.3, 0.7]
 */
export const voronoiF1F2_2d = (
  query: [number, number],
  seeds: readonly [number, number][],
): [number, number] | null => {
  if (seeds.length === 0) return null

  const [f1d2, f2d2] = seeds.reduce(
    ([f1, f2], [sx, sy]) => {
      const dx = query[0] - sx
      const dy = query[1] - sy
      const d2 = dx * dx + dy * dy
      if (d2 < f1) return [d2, f1]
      if (d2 < f2) return [f1, d2]
      return [f1, f2]
    },
    [Infinity, Infinity] as [number, number],
  )

  return [Math.sqrt(f1d2), Math.sqrt(f2d2)]
}

// ── Lloyd relaxation ──────────────────────────────────────────────────────────

/**
 * One step of sample-based Lloyd relaxation in 2-D.
 *
 * Assigns each sample to its nearest seed, then moves each seed to the centroid
 * of its assigned samples. Seeds with no samples assigned remain in place.
 *
 * Math:
 * ```
 * For each sample s: assign to nearest seed j
 * new_seed[i] = mean of all samples assigned to seed i
 *             = original seed[i] if no samples assigned
 * ```
 *
 * @param seeds   - current seed positions `[x, y][]`
 * @param samples - evaluation points for Voronoi cell estimation
 * @returns new seed positions (same length as seeds)
 *
 * @example
 * const relaxed = lloydRelaxStep2d([[0.1, 0], [0.9, 0]], grid100x1)
 * // Seeds move toward [0.25, 0] and [0.75, 0]
 */
export const lloydRelaxStep2d = (
  seeds: readonly [number, number][],
  samples: readonly [number, number][],
): [number, number][] => {
  if (seeds.length === 0) return []

  // Accumulate [sum_x, sum_y, count] per seed
  const init: [number, number, number][] = seeds.map(() => [0, 0, 0])

  const accum = samples.reduce((acc, [sx, sy]) => {
    // Find nearest seed index
    const nearest = seeds.reduce(
      ([bi, bd2], [px, py], i) => {
        const dx = sx - px
        const dy = sy - py
        const d2 = dx * dx + dy * dy
        return d2 < bd2 ? [i, d2] : [bi, bd2]
      },
      [0, Infinity] as [number, number],
    )[0]

    return acc.map(([sumX, sumY, count], i): [number, number, number] =>
      i === nearest ? [sumX + sx, sumY + sy, count + 1] : [sumX, sumY, count],
    )
  }, init)

  return accum.map(([sumX, sumY, count], i): [number, number] =>
    count === 0 ? seeds[i] : [sumX / count, sumY / count],
  )
}

// ── Delaunay triangulation ───────────────────────────────────────────────────

/**
 * Circumcircle test: is point (px, py) strictly inside the circumcircle of
 * triangle (ax,ay), (bx,by), (cx,cy)?
 *
 * Orientation-independent — works for both CW and CCW triangles.
 */
export const inCircumcircle = (
  px: number, py: number,
  ax: number, ay: number,
  bx: number, by: number,
  cx: number, cy: number,
): boolean => {
  const dx = ax - px
  const dy = ay - py
  const ex = bx - px
  const ey = by - py
  const fx = cx - px
  const fy = cy - py

  const dx2dy2 = dx * dx + dy * dy
  const ex2ey2 = ex * ex + ey * ey
  const fx2fy2 = fx * fx + fy * fy

  const det = dx * (ey * fx2fy2 - fy * ex2ey2)
            - dy * (ex * fx2fy2 - fx * ex2ey2)
            + dx2dy2 * (ex * fy - ey * fx)

  // Check triangle orientation (sign of cross product)
  const orient = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)

  return orient > 0 ? det > 0 : det < 0
}

/**
 * Delaunay triangulation via Bowyer-Watson. Returns list of triangle index triples.
 *
 * Each triple `[i, j, k]` references indices into the input `points` array.
 * Points on the super-triangle boundary are excluded from the output.
 *
 * Math: Bowyer-Watson incrementally inserts points, removing triangles whose
 * circumcircle contains the new point, then re-triangulates the polygonal hole.
 *
 * @param points - array of `[x, y]` points
 * @returns array of `[i, j, k]` triangle index triples
 *
 * @example
 * delaunay2d([[0, 0], [1, 0], [0.5, 1]]) // [[2, 1, 0]]
 */
export const delaunay2d = (
  points: readonly [number, number][],
): [number, number, number][] => {
  const n = points.length
  if (n < 3) return []

  // ADVANCE-EXCEPTION: Bowyer-Watson requires triangle set mutation.
  // Internal only — public API is pure: points -> triangles

  // Compute bounding box
  const bounds = points.reduce(
    ([minX, minY, maxX, maxY], [x, y]) => [
      Math.min(minX, x), Math.min(minY, y),
      Math.max(maxX, x), Math.max(maxY, y),
    ],
    [Infinity, Infinity, -Infinity, -Infinity],
  )

  const dx = bounds[2] - bounds[0]
  const dy = bounds[3] - bounds[1]
  const dMax = Math.max(dx, dy, 1e-6)
  const midX = (bounds[0] + bounds[2]) * 0.5
  const midY = (bounds[1] + bounds[3]) * 0.5

  // Super-triangle vertices (indices: n, n+1, n+2)
  const allPoints: [number, number][] = [
    ...points,
    [midX - 20 * dMax, midY - dMax],
    [midX, midY + 20 * dMax],
    [midX + 20 * dMax, midY - dMax],
  ]

  // Start with super-triangle
  let triangles: [number, number, number][] = [[n, n + 1, n + 2]] // ADVANCE-EXCEPTION

  for (let i = 0; i < n; i++) { // ADVANCE-EXCEPTION
    const [px, py] = allPoints[i]

    // Find bad triangles (circumcircle contains point i)
    const bad: number[] = []
    for (let t = 0; t < triangles.length; t++) { // ADVANCE-EXCEPTION
      const [a, b, c] = triangles[t]
      const [ax, ay] = allPoints[a]
      const [bx, by] = allPoints[b]
      const [cx, cy] = allPoints[c]
      if (inCircumcircle(px, py, ax, ay, bx, by, cx, cy)) {
        bad.push(t)
      }
    }

    // Find boundary polygon (edges in exactly one bad triangle)
    const edges: [number, number][] = []
    for (const t of bad) { // ADVANCE-EXCEPTION
      const [a, b, c] = triangles[t]
      const triEdges: [number, number][] = [[a, b], [b, c], [c, a]]
      for (const [e0, e1] of triEdges) {
        const shared = bad.some(other =>
          other !== t && (() => {
            const [oa, ob, oc] = triangles[other]
            const otherEdges: [number, number][] = [[oa, ob], [ob, oc], [oc, oa]]
            return otherEdges.some(([o0, o1]) =>
              (e0 === o0 && e1 === o1) || (e0 === o1 && e1 === o0),
            )
          })(),
        )
        if (!shared) {
          edges.push([e0, e1])
        }
      }
    }

    // Remove bad triangles (reverse order to preserve indices)
    const sortedBad = [...bad].sort((a, b) => b - a)
    for (const t of sortedBad) { // ADVANCE-EXCEPTION
      // swap_remove equivalent
      const last = triangles.length - 1
      if (t < last) {
        triangles[t] = triangles[last]
      }
      triangles = triangles.slice(0, -1)
    }

    // Create new triangles from boundary edges to inserted point
    for (const [e0, e1] of edges) { // ADVANCE-EXCEPTION
      triangles.push([i, e0, e1])
    }
  }

  // Remove triangles that reference super-triangle vertices
  return triangles.filter(([a, b, c]) => a < n && b < n && c < n)
}
