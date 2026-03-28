# prime-splines — Mathematical Reference

Formulas and derivations for every curve interpolation algorithm in `prime-splines`.

---

## 1. Quadratic Bezier Curve

Three control points $P_0, P_1, P_2$. The curve interpolates $P_0$ at $t=0$ and $P_2$ at $t=1$.

$$B(t) = (1-t)^2 P_0 + 2(1-t)t\,P_1 + t^2 P_2$$

Equivalently, using Bernstein polynomials $B_{i,n}(t) = \binom{n}{i} t^i (1-t)^{n-i}$:

$$B(t) = \sum_{i=0}^{2} B_{i,2}(t)\,P_i$$

The De Casteljau construction interprets this as repeated linear interpolation:

$$P_{01} = \text{lerp}(P_0, P_1, t), \quad P_{12} = \text{lerp}(P_1, P_2, t), \quad B(t) = \text{lerp}(P_{01}, P_{12}, t)$$

---

## 2. Cubic Bezier Curve

Four control points $P_0, P_1, P_2, P_3$. Bernstein form:

$$B(t) = (1-t)^3 P_0 + 3(1-t)^2 t\,P_1 + 3(1-t)t^2\,P_2 + t^3 P_3$$

**Properties:**
- $B(0) = P_0$, $B(1) = P_3$ (endpoint interpolation)
- $B'(0) = 3(P_1 - P_0)$, $B'(1) = 3(P_3 - P_2)$ (tangent at endpoints)
- Convex hull property: the curve lies within the convex hull of the control points
- Affine invariance: transforming control points transforms the curve

3-D variant applies the formula independently per component.

---

## 3. Cubic Hermite Interpolation

Specified by endpoint positions $P_0, P_1$ and endpoint tangents $M_0, M_1$.

### Basis functions

$$h_{00}(t) = 2t^3 - 3t^2 + 1$$
$$h_{10}(t) = t^3 - 2t^2 + t$$
$$h_{01}(t) = -2t^3 + 3t^2$$
$$h_{11}(t) = t^3 - t^2$$

### Interpolant

$$H(t) = h_{00}\,P_0 + h_{10}\,M_0 + h_{01}\,P_1 + h_{11}\,M_1$$

**Properties:**
- $H(0) = P_0$, $H(1) = P_1$ (position interpolation)
- $H'(0) = M_0$, $H'(1) = M_1$ (tangent interpolation)
- $C^1$ continuity when adjacent segments share tangent values

**Relation to cubic Bezier:** A Hermite segment $(P_0, M_0, P_1, M_1)$ is equivalent to a cubic Bezier with control points $(P_0,\; P_0 + M_0/3,\; P_1 - M_1/3,\; P_1)$.

---

## 4. Catmull-Rom Spline

Uniform variant ($\tau = 0.5$). Interpolates between $P_1$ and $P_2$ using neighbours $P_0$ and $P_3$ to derive tangents automatically.

### Tangent derivation

$$M_0 = \frac{P_2 - P_0}{2}, \qquad M_1 = \frac{P_3 - P_1}{2}$$

Then evaluate as Hermite: $\text{catmull\_rom}(t) = H(t, P_1, M_0, P_2, M_1)$.

### Direct matrix form

$$C(t) = \frac{1}{2} \begin{pmatrix} 1 & t & t^2 & t^3 \end{pmatrix} \begin{pmatrix} 0 & 2 & 0 & 0 \\ -1 & 0 & 1 & 0 \\ 2 & -5 & 4 & -1 \\ -1 & 3 & -3 & 1 \end{pmatrix} \begin{pmatrix} P_0 \\ P_1 \\ P_2 \\ P_3 \end{pmatrix}$$

Expanding:

$$C(t) = \tfrac{1}{2}\bigl[2P_1 + (-P_0 + P_2)\,t + (2P_0 - 5P_1 + 4P_2 - P_3)\,t^2 + (-P_0 + 3P_1 - 3P_2 + P_3)\,t^3\bigr]$$

**Properties:**
- $C(0) = P_1$, $C(1) = P_2$ (passes through control points)
- $C^1$ continuous across segment boundaries when knots are shared
- Tension parameter $\tau = 0.5$ is hardcoded (uniform Catmull-Rom)

---

## 5. Uniform Cubic B-Spline

Approximating spline: the curve does NOT pass through the control points. It is $C^2$ continuous and lies within the convex hull of each four-point window.

### Basis matrix

$$B(t) = \frac{1}{6} \begin{pmatrix} 1 & t & t^2 & t^3 \end{pmatrix} \begin{pmatrix} 1 & 4 & 1 & 0 \\ -3 & 0 & 3 & 0 \\ 3 & -6 & 3 & 0 \\ -1 & 3 & -3 & 1 \end{pmatrix} \begin{pmatrix} P_0 \\ P_1 \\ P_2 \\ P_3 \end{pmatrix}$$

Expanding:

$$B(t) = \frac{1}{6}\bigl[(-t^3 + 3t^2 - 3t + 1)\,P_0 + (3t^3 - 6t^2 + 4)\,P_1 + (-3t^3 + 3t^2 + 3t + 1)\,P_2 + t^3\,P_3\bigr]$$

**Boundary values:**
- $B(0) = (P_0 + 4P_1 + P_2)/6$ (weighted average at segment start)
- $B(1) = (P_1 + 4P_2 + P_3)/6$ (weighted average at segment end)

**Properties:**
- $C^2$ continuous across segment boundaries (vs $C^1$ for Catmull-Rom)
- Local support: moving one control point only affects 4 segments
- Variation-diminishing: no more crossings with any hyperplane than the control polygon

---

## 6. Spherical Linear Interpolation (Slerp)

Interpolates between unit quaternions $q_0$ and $q_1$ along the shorter great-circle arc on $S^3$.

### Algorithm

1. Compute dot product: $d = q_0 \cdot q_1 = x_0 x_1 + y_0 y_1 + z_0 z_1 + w_0 w_1$.
2. Shorter arc: if $d < 0$, negate $q_1$ and set $d \leftarrow -d$.
3. Near-identity fallback: if $d > 0.9995$, use normalised linear interpolation (avoids $\sin\theta \approx 0$ division).
4. Otherwise:

$$\theta = \arccos(d)$$

$$\text{slerp}(t) = \frac{\sin\bigl((1-t)\,\theta\bigr)}{\sin\theta}\,q_0 + \frac{\sin(t\,\theta)}{\sin\theta}\,q_1$$

**Properties:**
- Constant angular velocity: $\|\text{slerp}'(t)\|$ is constant
- Result is always a unit quaternion (up to floating-point precision)
- Commutative in arc selection: always takes the shorter path ($\le 180^\circ$)

**Why negate when $d < 0$:** Quaternions $q$ and $-q$ represent the same rotation. When $d < 0$ the naive slerp takes the long arc ($> 180^\circ$); negating ensures the short arc.

---

## 7. Arc-Length Parameterisation

Cubic Bezier curves are parameterised by $t$, not by arc length. A point at $t = 0.5$ is generally not at the geometric midpoint. Arc-length parameterisation provides uniform-speed traversal.

### Arc Length Estimation

Approximate by summing chord lengths over $N$ linear segments:

$$L \approx \sum_{i=1}^{N} \|B(t_i) - B(t_{i-1})\|, \quad t_i = \frac{i}{N}$$

In 1-D: $\|B(t_i) - B(t_{i-1})\| = |B(t_i) - B(t_{i-1})|$.

In 3-D: Euclidean distance $\sqrt{\Delta x^2 + \Delta y^2 + \Delta z^2}$.

Accuracy improves with $N$ (the `steps` parameter); $N = 100$ is typically sufficient for visual applications.

### Inverse: $t$ at Target Length (Binary Search)

Given a target arc length $\ell$, find the parameter $t^*$ such that:

$$L(t^*) = \ell, \quad \text{where } L(t) = \sum_{i=1}^{N} \|B(t \cdot i/N) - B(t \cdot (i-1)/N)\|$$

**Method:** Binary search on $t \in [0, 1]$:

1. Set $lo = 0$, $hi = 1$.
2. For each iteration: $\text{mid} = (lo + hi)/2$.
3. If $L(\text{mid}) < \ell$: $lo = \text{mid}$. Else: $hi = \text{mid}$.
4. After $k$ iterations, $t^* \approx (lo + hi)/2$ with precision $2^{-k}$.

With 20 bisection steps, precision is $\approx 10^{-6}$, more than sufficient for f32.
