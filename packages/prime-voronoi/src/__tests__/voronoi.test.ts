import { describe, it, expect } from 'vitest'
import { voronoiNearest2d, voronoiF1F2_2d, lloydRelaxStep2d, delaunay2d, inCircumcircle } from '../index.js'

const EPS = 1e-4

// ── voronoiNearest2d ──────────────────────────────────────────────────────────

describe('voronoiNearest2d', () => {
  it('empty seeds → null', () => {
    expect(voronoiNearest2d([0, 0], [])).toBeNull()
  })

  it('single seed', () => {
    const [idx, dist] = voronoiNearest2d([1, 0], [[0, 0]])!
    expect(idx).toBe(0)
    expect(dist).toBeCloseTo(1, 5)
  })

  it('selects closest', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0], [0, 1]]
    const [idx] = voronoiNearest2d([0.1, 0.1], seeds)!
    expect(idx).toBe(0)
  })

  it('query on seed — zero distance', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0]]
    const [idx, dist] = voronoiNearest2d([1, 0], seeds)!
    expect(idx).toBe(1)
    expect(dist).toBeCloseTo(0, 5)
  })

  it('deterministic', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0], [0.5, 0.5]]
    const a = voronoiNearest2d([0.3, 0.3], seeds)
    const b = voronoiNearest2d([0.3, 0.3], seeds)
    expect(a).toEqual(b)
  })
})

// ── voronoiF1F2_2d ────────────────────────────────────────────────────────────

describe('voronoiF1F2_2d', () => {
  it('empty seeds → null', () => {
    expect(voronoiF1F2_2d([0, 0], [])).toBeNull()
  })

  it('f1 < f2 for two seeds', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0]]
    const [f1, f2] = voronoiF1F2_2d([0.3, 0], seeds)!
    expect(f1).toBeLessThan(f2)
    expect(f1).toBeCloseTo(0.3, 4)
    expect(f2).toBeCloseTo(0.7, 4)
  })

  it('deterministic', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0], [0.5, 0.5]]
    const a = voronoiF1F2_2d([0.3, 0.3], seeds)
    const b = voronoiF1F2_2d([0.3, 0.3], seeds)
    expect(a).toEqual(b)
  })
})

// ── lloydRelaxStep2d ──────────────────────────────────────────────────────────

describe('lloydRelaxStep2d', () => {
  it('empty seeds → []', () => {
    expect(lloydRelaxStep2d([], [[0.5, 0.5]])).toEqual([])
  })

  it('no samples → seeds unchanged', () => {
    const seeds: [number, number][] = [[0, 0], [1, 0]]
    const relaxed = lloydRelaxStep2d(seeds, [])
    expect(relaxed[0]).toEqual(seeds[0])
    expect(relaxed[1]).toEqual(seeds[1])
  })

  it('preserves seed count', () => {
    const seeds: [number, number][] = [[0, 0], [0.5, 0], [1, 0]]
    const samples: [number, number][] = Array.from<null>({ length: 10 }).map((_, i) => [i / 9, 0])
    expect(lloydRelaxStep2d(seeds, samples)).toHaveLength(3)
  })

  it('two seeds move toward centroids', () => {
    const seeds: [number, number][] = [[0.1, 0], [0.9, 0]]
    const samples: [number, number][] = Array.from<null>({ length: 101 }).map((_, i) => [i / 100, 0])
    const relaxed = lloydRelaxStep2d(seeds, samples)
    expect(relaxed[0][0]).toBeCloseTo(0.25, 1)
    expect(relaxed[1][0]).toBeCloseTo(0.75, 1)
  })

  it('deterministic', () => {
    const seeds: [number, number][] = [[0.1, 0.2], [0.8, 0.7]]
    const samples: [number, number][] = Array.from<null>({ length: 25 }).map((_, k) => [
      (k % 5) * 0.25,
      Math.floor(k / 5) * 0.25,
    ])
    const a = lloydRelaxStep2d(seeds, samples)
    const b = lloydRelaxStep2d(seeds, samples)
    expect(a).toEqual(b)
  })
})

// ── delaunay2d ───────────────────────────────────────────────────────────────

describe('delaunay2d', () => {
  it('single triangle', () => {
    const pts: [number, number][] = [[0, 0], [1, 0], [0.5, 1]]
    const tris = delaunay2d(pts)
    expect(tris).toHaveLength(1)
  })

  it('four points → two triangles', () => {
    const pts: [number, number][] = [[0, 0], [1, 0], [1, 1], [0, 1]]
    const tris = delaunay2d(pts)
    expect(tris).toHaveLength(2)
  })

  it('empty → empty', () => {
    expect(delaunay2d([])).toEqual([])
  })

  it('two points → empty (cannot form triangle)', () => {
    const pts: [number, number][] = [[0, 0], [1, 0]]
    expect(delaunay2d(pts)).toEqual([])
  })

  it('circumcircle property — no point inside any triangle circumcircle', () => {
    const pts: [number, number][] = [[0, 0], [4, 0], [2, 3], [1, 1], [3, 1]]
    const tris = delaunay2d(pts)
    expect(tris.length).toBeGreaterThan(0)
    for (const [i, j, k] of tris) {
      const [ax, ay] = pts[i]
      const [bx, by] = pts[j]
      const [cx, cy] = pts[k]
      for (let m = 0; m < pts.length; m++) {
        if (m === i || m === j || m === k) continue
        const [px, py] = pts[m]
        expect(inCircumcircle(px, py, ax, ay, bx, by, cx, cy)).toBe(false)
      }
    }
  })

  it('deterministic', () => {
    const pts: [number, number][] = [[0, 0], [1, 0], [0.5, 1], [0.5, 0.5]]
    const a = delaunay2d(pts)
    const b = delaunay2d(pts)
    expect(a).toEqual(b)
  })
})

// ── Cross-language parity (values verified against Rust prime-voronoi) ────────

describe('cross-language parity', () => {
  it('voronoiNearest2d single seed at origin — distance matches Rust', () => {
    // Rust: voronoi_nearest_2d((1.0, 0.0), &[(0.0, 0.0)]) = (0, 1.0)
    const result = voronoiNearest2d([1, 0], [[0, 0]])
    expect(result).not.toBeNull()
    expect(result![0]).toBe(0)
    expect(result![1]).toBeCloseTo(1.0, 5)
  })
  it('voronoiF1F2_2d picks nearest and second nearest', () => {
    // Two seeds: [0,0] and [2,0]. Query at [0.5, 0]. Nearest = [0,0] at 0.5, next = [2,0] at 1.5
    const result = voronoiF1F2_2d([0.5, 0], [[0, 0], [2, 0]])
    expect(result).not.toBeNull()
    expect(result![0]).toBeCloseTo(0.5, 4)
    expect(result![1]).toBeCloseTo(1.5, 4)
  })
  it('lloydRelaxStep2d moves seed toward centroid of samples', () => {
    // One seed at [0,0], two samples at [1,0] and [1,1]. Centroid = [1, 0.5].
    const relaxed = lloydRelaxStep2d([[0, 0]], [[1, 0], [1, 1]])
    expect(relaxed[0][0]).toBeCloseTo(1.0, 4)
    expect(relaxed[0][1]).toBeCloseTo(0.5, 4)
  })

  it('delaunay2d single triangle produces 1 triangle', () => {
    // Rust: delaunay_2d(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]) => 1 triangle
    const tris = delaunay2d([[0, 0], [1, 0], [0.5, 1]])
    expect(tris).toHaveLength(1)
  })

  it('delaunay2d four points produces 2 triangles', () => {
    // Rust: delaunay_2d(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]) => 2 triangles
    const tris = delaunay2d([[0, 0], [1, 0], [1, 1], [0, 1]])
    expect(tris).toHaveLength(2)
  })
})
