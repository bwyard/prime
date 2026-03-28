# prime-sdf — Math Reference

Signed distance functions (SDFs), CSG operations, smooth blending, and domain transforms.
All functions are pure. Uses `glam::Vec2` / `glam::Vec3` for points and vectors.

An SDF maps a point $\mathbf{p}$ to a signed scalar: negative inside, zero on the surface, positive outside.

---

## 2D Primitives

### Circle

$$d(\mathbf{p}) = |\mathbf{p} - \mathbf{c}| - r$$

### Axis-Aligned Box (2D)

Let $\mathbf{q} = |\mathbf{p} - \mathbf{c}| - \mathbf{h}$ where $\mathbf{h}$ is the half-extents vector.

$$d(\mathbf{p}) = |\max(\mathbf{q}, 0)| + \min(\max(q_x, q_y),\; 0)$$

The first term handles the exterior (distance to the nearest edge or corner). The second term handles the interior (largest penetration depth, which is negative inside).

### Rounded Box

$$d(\mathbf{p}) = \text{box\_2d}(\mathbf{p}, \mathbf{c}, \mathbf{h}) - r$$

Subtracting the corner radius from the box SDF rounds all corners.

### Capsule (2D)

Project $\mathbf{p}$ onto segment $\overline{AB}$ with clamped parameter $t \in [0, 1]$:

$$t = \text{clamp}\!\left(\frac{(\mathbf{p}-\mathbf{a}) \cdot (\mathbf{b}-\mathbf{a})}{|\mathbf{b}-\mathbf{a}|^2},\; 0,\; 1\right)$$

$$d(\mathbf{p}) = |(\mathbf{p} - \mathbf{a}) - t(\mathbf{b} - \mathbf{a})| - r$$

### Triangle

For each edge, compute the nearest point on the edge segment (clamped projection). The unsigned distance is the minimum over all three edges. The sign is determined by the cross-product winding test against each edge.

### Ring (Annulus)

Let $m = (r_{\text{outer}} + r_{\text{inner}}) / 2$ and $w = (r_{\text{outer}} - r_{\text{inner}}) / 2$.

$$d(\mathbf{p}) = \bigl|\,|\mathbf{p} - \mathbf{c}| - m\,\bigr| - w$$

---

## 3D Primitives

### Sphere

$$d(\mathbf{p}) = |\mathbf{p} - \mathbf{c}| - r$$

### Axis-Aligned Box (3D)

Let $\mathbf{q} = |\mathbf{p} - \mathbf{c}| - \mathbf{h}$.

$$d(\mathbf{p}) = |\max(\mathbf{q}, 0)| + \min(\max(q_x, q_y, q_z),\; 0)$$

### Capsule (3D)

Same projection formula as 2D capsule but in 3D:

$$t = \text{clamp}\!\left(\frac{(\mathbf{p}-\mathbf{a}) \cdot (\mathbf{b}-\mathbf{a})}{|\mathbf{b}-\mathbf{a}|^2},\; 0,\; 1\right), \quad d(\mathbf{p}) = |(\mathbf{p}-\mathbf{a}) - t(\mathbf{b}-\mathbf{a})| - r$$

### Cylinder

Decompose into lateral distance (XZ plane) and axial distance (Y axis):

$$d_{\text{lateral}} = \sqrt{(p_x - c_x)^2 + (p_z - c_z)^2} - r$$
$$d_{\text{axial}} = |p_y - c_y| - h/2$$

Combine as a 2D box SDF over $(d_{\text{lateral}}, d_{\text{axial}})$:

$$d(\mathbf{p}) = |\max(\mathbf{q}, 0)| + \min(\max(q_x, q_y),\; 0), \quad \mathbf{q} = (d_{\text{lateral}}, d_{\text{axial}})$$

### Torus

Let $\mathbf{q} = \bigl(\,\sqrt{(p_x - c_x)^2 + (p_z - c_z)^2} - R,\;\; p_y - c_y\,\bigr)$ where $R$ = major radius.

$$d(\mathbf{p}) = |\mathbf{q}| - r_{\text{minor}}$$

### Infinite Plane

$$d(\mathbf{p}) = \mathbf{n} \cdot \mathbf{p} - \text{offset}$$

where $\mathbf{n}$ is the unit normal.

---

## CSG (Boolean) Operations

All CSG operations combine two SDF values $d_1, d_2$ into a new SDF value.

| Operation | Formula | Meaning |
|---|---|---|
| Union | $\min(d_1, d_2)$ | Inside either shape |
| Intersection | $\max(d_1, d_2)$ | Inside both shapes |
| Subtraction | $\max(d_1, -d_2)$ | Inside $d_1$ but outside $d_2$ |
| XOR | $\max(\min(d_1, d_2), -\max(d_1, d_2))$ | Inside exactly one shape |

---

## Smooth Operations

Smooth blending uses a polynomial smooth min/max with blending parameter $k > 0$. As $k \to 0$, smooth operations converge to their sharp counterparts.

### Smooth Union (Polynomial Smooth Min)

From Inigo Quilez:

$$h = \frac{\max(k - |d_1 - d_2|,\; 0)}{k}$$

$$\text{smooth\_union}(d_1, d_2, k) = \min(d_1, d_2) - \frac{h^2 \cdot k}{4}$$

The $h^2 k / 4$ correction term creates a smooth blend region of width $k$ around the intersection seam.

### Smooth Intersection

$$\text{smooth\_intersection}(d_1, d_2, k) = \max(d_1, d_2) + \frac{h^2 \cdot k}{4}$$

with $h$ defined identically.

### Smooth Subtraction

$$\text{smooth\_subtract}(d_1, d_2, k) = \text{smooth\_intersection}(d_1, -d_2, k)$$

---

## Domain Transforms

Domain transforms modify the query point $\mathbf{p}$ before evaluating the SDF. They reshape space itself rather than the distance field.

### Translation

$$\mathbf{p}' = \mathbf{p} - \text{offset}$$

Moves the shape by translating the query point in the opposite direction.

### Rotation (2D)

$$\mathbf{p}' = \begin{bmatrix} \cos\theta & \sin\theta \\ -\sin\theta & \cos\theta \end{bmatrix} \mathbf{p}$$

### Scale

$$\mathbf{p}' = \mathbf{p} / f$$

**Important:** The SDF result must also be multiplied by $f$ to preserve correct distances.

### Infinite Repetition

$$\mathbf{p}' = \text{mod}(\mathbf{p} + \mathbf{T}/2,\; \mathbf{T}) - \mathbf{T}/2$$

where $\mathbf{T}$ is the repetition period per axis. This folds all of space into a single cell of size $\mathbf{T}$, centered at the origin.

### Mirror

Fold space across an axis by taking the absolute value of one coordinate:

$$\text{mirror\_x}: \quad p_x' = |p_x|$$
$$\text{mirror\_y}: \quad p_y' = |p_y|$$

### Elongation

$$\mathbf{p}' = \mathbf{p} - \text{clamp}(\mathbf{p}, -\mathbf{h}, \mathbf{h})$$

Stretches the shape along each axis by $\mathbf{h}$. Points within the band $[-h_i, h_i]$ collapse to 0 on that axis; points beyond are shifted inward by $h_i$.
