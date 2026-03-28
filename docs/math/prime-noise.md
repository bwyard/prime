# prime-noise — Mathematical Reference

Formulas and derivations for every noise algorithm in `prime-noise`.

---

## 1. Internal Hash

Mulberry32-variant integer hash used to seed all lattice lookups.

$$z = (x + \texttt{0x6D2B79F5}) \bmod 2^{32}$$
$$z \leftarrow (z \oplus (z \gg 15)) \cdot (z \mid 1)$$
$$z \leftarrow z \oplus \bigl(z + (z \oplus (z \gg 7)) \cdot (z \mid 61)\bigr)$$
$$\text{output} = (z \oplus (z \gg 14)) \;/\; 2^{32} \in [0,\,1]$$

**2-D lattice hash:** $h(x_i, y_i) = \text{hash}(\text{hash}(x_i) + y_i) / 2^{32}$.

**3-D lattice hash:** $h(x_i, y_i, z_i) = \text{hash}(\text{hash}(\text{hash}(x_i) + y_i) + z_i) / 2^{32}$.

---

## 2. Smoothstep Fade Curve

Used by both value noise and Perlin noise to eliminate lattice discontinuities.

$$s(t) = 3t^2 - 2t^3$$

**Properties:**
- $s(0) = 0$, $s(1) = 1$
- $s'(0) = s'(1) = 0$ — zero derivative at endpoints guarantees $C^1$ continuity across cell boundaries

---

## 3. Value Noise (2-D and 3-D)

Hash-based noise. Each integer lattice corner gets a pseudo-random scalar; the field is reconstructed by smoothstep-interpolated blending.

### 2-D

Given continuous point $(x, y)$:

$$x_i = \lfloor x \rfloor, \quad y_i = \lfloor y \rfloor, \quad f_x = x - x_i, \quad f_y = y - y_i$$

$$t_x = s(f_x), \quad t_y = s(f_y)$$

$$v_{00} = h(x_i,\; y_i), \quad v_{10} = h(x_i+1,\; y_i), \quad v_{01} = h(x_i,\; y_i+1), \quad v_{11} = h(x_i+1,\; y_i+1)$$

$$\text{result} = \text{lerp}\bigl(\text{lerp}(v_{00}, v_{10}, t_x),\;\text{lerp}(v_{01}, v_{11}, t_x),\;t_y\bigr) \in [0,\,1]$$

### 3-D

Trilinear interpolation over the 8 corners of a unit cube:

$$\text{result} = \text{lerp}\bigl(\text{bilerp}(v_{000\ldots}),\;\text{bilerp}(v_{001\ldots}),\;t_z\bigr) \in [0,\,1]$$

---

## 4. Perlin Gradient Noise (2-D and 3-D)

Each lattice corner receives a pseudo-random **gradient vector**. The noise value is the smoothstep-blended combination of dot products between gradients and offset vectors.

### Gradient table

**2-D:** Eight unit vectors evenly spaced around the circle at $45^\circ$ intervals:

$$(1,0),\;\tfrac{1}{\sqrt{2}}(1,1),\;(0,1),\;\tfrac{1}{\sqrt{2}}(-1,1),\;(-1,0),\;\tfrac{1}{\sqrt{2}}(-1,-1),\;(0,-1),\;\tfrac{1}{\sqrt{2}}(1,-1)$$

**3-D:** Twelve vectors — midpoints of the edges of a unit cube, e.g. $(1,1,0),\;(-1,1,0),\;(1,0,1),\;\ldots$

### 2-D Algorithm

$$g_{ij} = \text{gradient}\bigl(h(x_i + i,\; y_i + j)\bigr), \quad i,j \in \{0,1\}$$

$$n_{ij} = g_{ij} \cdot (f_x - i,\; f_y - j)$$

$$\text{result} = \text{lerp}\bigl(\text{lerp}(n_{00}, n_{10}, t_x),\;\text{lerp}(n_{01}, n_{11}, t_x),\;t_y\bigr)$$

**Range:** Approximately $[-1, 1]$ (exact bounds depend on gradient table geometry). At integer lattice points all offsets are zero, so the result is exactly $0$.

### 3-D Algorithm

Same structure with 8 corners and trilinear blending. Gradients are chosen from the 12-vector table.

---

## 5. Simplex Noise (2-D and 3-D)

Evaluates on a simplex lattice (triangles in 2-D, tetrahedra in 3-D) instead of a square/cubic grid. Fewer contributions per sample and no axis-aligned artifacts.

### Skew and Unskew Transforms

The input point is skewed into a coordinate system where the simplicial grid maps to an integer lattice.

**2-D constants:**

$$F_2 = \frac{\sqrt{3} - 1}{2} \approx 0.3660, \qquad G_2 = \frac{3 - \sqrt{3}}{6} \approx 0.2113$$

**Skew:** $(x,y) \to (i,j)$ via $s = (x+y) \cdot F_2$, then $i = \lfloor x + s \rfloor$, $j = \lfloor y + s \rfloor$.

**Unskew:** recover the simplex-local offset $t = (i+j) \cdot G_2$, then $x_0 = x - (i - t)$, $y_0 = y - (j - t)$.

**3-D constants:**

$$F_3 = \tfrac{1}{3}, \qquad G_3 = \tfrac{1}{6}$$

### Contribution Kernel

Each simplex corner contributes:

$$c = \max(0,\; r^2 - d_x^2 - d_y^2)^4 \cdot (g \cdot \mathbf{d})$$

where $r^2 = 0.5$ (2-D) or $r^2 = 0.6$ (3-D), $g$ is the gradient vector, and $\mathbf{d}$ is the offset from the corner.

The $(\cdot)^4$ radial falloff gives compact support — each corner only affects a limited radius, eliminating the need to blend across the full cell.

### 2-D

Three corners (triangle). Final scaling: $\text{result} = 70 \cdot \sum_{k=0}^{2} c_k$.

### 3-D

Four corners (tetrahedron). Which tetrahedron is determined by sorting the offsets $(x_0, y_0, z_0)$. Final scaling: $\text{result} = 32 \cdot \sum_{k=0}^{3} c_k$.

---

## 6. Fractional Brownian Motion (FBM)

Layered octaves of Perlin noise, each at increasing frequency and decreasing amplitude.

$$\text{fbm}(x, y) = \sum_{i=0}^{N-1} a_i \cdot \text{perlin}(x \cdot f_i,\; y \cdot f_i)$$

where:

$$f_0 = 1, \quad a_0 = 1, \quad f_{i+1} = f_i \cdot \lambda, \quad a_{i+1} = a_i \cdot g$$

- $\lambda$ — **lacunarity** (frequency multiplier, typically $2.0$)
- $g$ — **gain** (amplitude multiplier, typically $0.5$)
- $N$ — **octaves** (number of layers, typically $1$--$8$)

**Theoretical maximum amplitude** (geometric series with $g = 0.5$):

$$\sum_{i=0}^{N-1} g^i = \frac{1 - g^N}{1 - g} \to 2.0 \text{ as } N \to \infty$$

The 3-D variant (`fbm_3d`) uses `perlin_3d` with the same summation structure.

---

## 7. Worley / Cellular Noise

Distance to the nearest randomly-placed feature point.

### Algorithm

1. Locate the integer cell $(x_i, y_i) = (\lfloor x \rfloor, \lfloor y \rfloor)$.
2. For each of the $3 \times 3 = 9$ neighbouring cells $(x_i + d_x,\; y_i + d_y)$, $d_x, d_y \in \{-1, 0, 1\}$:
   - Hash two independent offsets in $[0, 1]$ using `hash_2d_seeded` (seeds $s$ and $s+1$).
   - Feature point: $(x_i + d_x + f_x,\; y_i + d_y + f_y)$.
   - Compute Euclidean distance to the query point.
3. Return the minimum distance, clamped to $[0, 1]$.

$$d = \min_{(d_x, d_y)} \sqrt{(x - p_x)^2 + (y - p_y)^2}$$

**Why 9 cells:** A feature point at distance $> \sqrt{2}$ from any interior point of the centre cell cannot be the nearest, and the 9-cell search radius covers all candidates.

---

## 8. Domain Warping

Displaces the input coordinates by an independent noise field before sampling a final noise layer.

### 2-D

$$w_x = \text{fbm}(x, y), \quad w_y = \text{fbm}(x + 5.2,\; y + 1.3)$$

$$\text{result} = \text{fbm}(x + \alpha \cdot w_x,\; y + \alpha \cdot w_y)$$

where $\alpha$ is `warp_scale`. The constant offsets $(5.2, 1.3)$ decorrelate the two warp fields.

### 3-D

Three warp fields with offsets $(0,0,0)$, $(5.2, 1.3, 2.7)$, $(3.1, 7.4, 0.9)$:

$$\text{result} = \text{fbm}\bigl(x + \alpha w_x,\; y + \alpha w_y,\; z + \alpha w_z\bigr)$$

Domain warping produces swirling, turbulent structures with self-similar detail not achievable by plain FBM.

---

## 9. Curl Noise

A divergence-free vector field derived from the curl of scalar noise fields. Useful for incompressible fluid-like motion (particles, smoke, etc.).

### 2-D

Given a scalar field $N(x,y) = \text{perlin\_2d}(x,y)$, the curl is:

$$\text{curl}(x,y) = \left(\frac{\partial N}{\partial y},\; -\frac{\partial N}{\partial x}\right)$$

Partial derivatives are estimated via central differences:

$$\frac{\partial N}{\partial x} \approx \frac{N(x+\varepsilon, y) - N(x-\varepsilon, y)}{2\varepsilon}$$

**Divergence-free:** For any smooth scalar field $N$, $\nabla \cdot (\partial_y N, -\partial_x N) = \partial_{xy} N - \partial_{xy} N = 0$.

### 3-D

Uses three decorrelated scalar fields $N_1, N_2, N_3$ (offset by constant vectors to break correlation) and computes the vector curl:

$$\text{curl} = \begin{pmatrix} \partial_y N_3 - \partial_z N_2 \\ \partial_z N_1 - \partial_x N_3 \\ \partial_x N_2 - \partial_y N_1 \end{pmatrix}$$

**Divergence-free:** $\nabla \cdot (\nabla \times \mathbf{F}) = 0$ for any smooth vector field $\mathbf{F}$.

Decorrelation offsets: $N_1$ at origin, $N_2$ at $(5.2, 1.3, 2.7)$, $N_3$ at $(3.1, 7.4, 0.9)$.
