# prime-spatial — Math Reference

Pure spatial queries: ray intersection, AABB operations, and frustum culling.
All functions operate on `(f32, f32, f32)` tuples. No mutation, no hidden state.

---

## Ray-Sphere Intersection

Solve for the ray parameter $t$ where a ray hits a sphere.

**Setup.** Let $\mathbf{o}$ = ray origin, $\mathbf{d}$ = ray direction, $\mathbf{c}$ = sphere center, $r$ = radius. Define $\mathbf{oc} = \mathbf{o} - \mathbf{c}$.

Substitute $\mathbf{p}(t) = \mathbf{o} + t\mathbf{d}$ into $|\mathbf{p} - \mathbf{c}|^2 = r^2$:

$$(\mathbf{d} \cdot \mathbf{d})\,t^2 + 2(\mathbf{d} \cdot \mathbf{oc})\,t + (\mathbf{oc} \cdot \mathbf{oc} - r^2) = 0$$

Using the half-$b$ optimisation with $h = \mathbf{d} \cdot \mathbf{oc}$, $a = \mathbf{d} \cdot \mathbf{d}$, $c = \mathbf{oc} \cdot \mathbf{oc} - r^2$:

$$\Delta = h^2 - a \cdot c$$

- $\Delta < 0$: no intersection (ray misses).
- $\Delta = 0$: tangent hit (one root).
- $\Delta > 0$: two roots $t_{0,1} = \frac{-h \mp \sqrt{\Delta}}{a}$.

The implementation returns the smallest positive $t$. If the origin is inside the sphere ($t_0 < 0$), it returns $t_1$ (the exit point).

---

## Ray-AABB Intersection (Slab Method)

Uses the Kay-Kajiya slab method. An AABB is the intersection of three axis-aligned slabs.

For each axis $i \in \{x, y, z\}$, compute entry and exit parameters:

$$t_{\min,i} = \frac{\text{aabb\_min}_i - o_i}{d_i}, \quad t_{\max,i} = \frac{\text{aabb\_max}_i - o_i}{d_i}$$

If $d_i = 0$, the reciprocal is $\pm\infty$, which correctly classifies origins inside or outside that slab.

Swap so $t_{\min,i} \le t_{\max,i}$, then:

$$t_{\text{enter}} = \max(t_{\min,x},\; t_{\min,y},\; t_{\min,z})$$
$$t_{\text{exit}} = \min(t_{\max,x},\; t_{\max,y},\; t_{\max,z})$$

**Hit condition:** $t_{\text{enter}} \le t_{\text{exit}}$ and $t_{\text{exit}} > 0$.

Returns $t_{\text{enter}}$ when positive; otherwise $t_{\text{exit}}$ (origin inside the AABB).

---

## Ray-Plane Intersection

A plane is defined by unit normal $\mathbf{n}$ and scalar $d$ such that $\mathbf{n} \cdot \mathbf{p} = d$ for all points on the plane.

Substitute $\mathbf{p}(t) = \mathbf{o} + t\mathbf{d}$:

$$\mathbf{n} \cdot (\mathbf{o} + t\mathbf{d}) = d$$

$$t = \frac{d - \mathbf{n} \cdot \mathbf{o}}{\mathbf{n} \cdot \mathbf{d}}$$

- If $|\mathbf{n} \cdot \mathbf{d}| < \varepsilon$: ray is parallel to the plane, no hit.
- If $t \le 0$: plane is behind the ray origin, no hit.

---

## AABB Operations

### Overlap Test

Two AABBs overlap if and only if their projections overlap on every axis:

$$\text{overlap} = \bigwedge_{i \in \{x,y,z\}} \bigl(\max_A^i \ge \min_B^i\bigr) \wedge \bigl(\max_B^i \ge \min_A^i\bigr)$$

Touching faces ($\max_A^i = \min_B^i$) count as overlap.

### Point Containment

A point $\mathbf{p}$ is inside an AABB if:

$$\forall\, i:\quad \min_i \le p_i \le \max_i$$

### Union (Enclosing AABB)

The tightest AABB enclosing two AABBs:

$$\text{union\_min}_i = \min(\min_A^i, \min_B^i), \quad \text{union\_max}_i = \max(\max_A^i, \max_B^i)$$

### Closest Point

The point on (or inside) the AABB nearest to a query point $\mathbf{p}$:

$$q_i = \text{clamp}(p_i,\; \min_i,\; \max_i)$$

If $\mathbf{p}$ is already inside, $\mathbf{q} = \mathbf{p}$.

---

## Frustum Culling

A frustum is defined by six planes, each with inward-pointing normal $\mathbf{n}$ and offset $d$ such that $\mathbf{n} \cdot \mathbf{p} + d \ge 0$ means "inside."

### Sphere Culling

A sphere (center $\mathbf{c}$, radius $r$) is **outside** the frustum if it lies entirely in the outer half-space of any plane:

$$\exists\, \text{plane}: \quad \mathbf{n} \cdot \mathbf{c} + d < -r$$

If this condition is true for any plane, the sphere is culled. Otherwise it may be visible.

### AABB Culling (Positive Vertex Method)

For each frustum plane, find the AABB vertex most aligned with the plane normal (the "positive vertex" or "p-vertex"):

$$\text{pv}_i = \begin{cases} \text{aabb\_max}_i & \text{if } n_i \ge 0 \\ \text{aabb\_min}_i & \text{if } n_i < 0 \end{cases}$$

If the positive vertex is outside any plane, the entire AABB is outside:

$$\text{visible} = \bigwedge_{\text{plane}} \bigl(\mathbf{n} \cdot \mathbf{pv} + d \ge 0\bigr)$$

This is a conservative test: it never incorrectly culls a visible AABB, but may keep some AABBs that are actually outside (false positives at frustum corners).
