# prime-voronoi — Math Reference

Voronoi diagrams, F1/F2 distance queries, and Lloyd relaxation.
All functions are pure. No mutation, no hidden state.

---

## Voronoi Nearest (F1)

Given a query point $\mathbf{q}$ and a set of seed points $\{s_0, s_1, \ldots, s_{n-1}\}$, find the nearest seed by Euclidean distance.

$$(\text{index}, F_1) = \underset{i}{\operatorname{argmin}}\; \|\mathbf{q} - \mathbf{s}_i\|$$

where $\|\cdot\|$ is the Euclidean norm:

$$\|\mathbf{q} - \mathbf{s}_i\| = \sqrt{(q_x - s_{i,x})^2 + (q_y - s_{i,y})^2}$$

**Implementation detail:** The scan compares squared distances to avoid $n$ square roots. Only the final winner distance is square-rooted once.

---

## F1 and F2 Distances

$F_1$ is the distance to the nearest seed. $F_2$ is the distance to the second-nearest seed. Together they characterize Voronoi cell structure and are the basis for cellular noise.

$$F_1 = \min_i \|\mathbf{q} - \mathbf{s}_i\|$$

$$F_2 = \min_{i \ne i^*} \|\mathbf{q} - \mathbf{s}_i\|, \quad i^* = \underset{j}{\operatorname{argmin}}\; \|\mathbf{q} - \mathbf{s}_j\|$$

The implementation maintains the two smallest squared distances in a single fold over all seeds. If a new squared distance $d^2$ is less than $F_1^2$, it becomes the new $F_1^2$ and the old $F_1^2$ becomes $F_2^2$. Otherwise if $d^2 < F_2^2$, it replaces $F_2^2$.

**Common noise patterns from F1/F2:**

| Expression | Pattern |
|---|---|
| $F_1$ | Standard Worley/cellular noise |
| $F_2 - F_1$ | Voronoi cell edges (thin lines at 0) |
| $F_2$ | Inverse cellular pattern |
| $F_2 / F_1$ | Ratio-based shading |

---

## Lloyd Relaxation

Lloyd relaxation iteratively moves seeds toward the centroids of their Voronoi cells, producing increasingly uniform point distributions.

### One Step

1. **Assign** each sample point $\mathbf{p}_j$ to its nearest seed:
$$\text{owner}(j) = \underset{i}{\operatorname{argmin}}\; \|\mathbf{p}_j - \mathbf{s}_i\|$$

2. **Compute centroids** for each seed's cell:
$$\mathbf{c}_i = \frac{1}{|S_i|} \sum_{j \in S_i} \mathbf{p}_j, \quad S_i = \{j : \text{owner}(j) = i\}$$

3. **Update** seeds:
$$\mathbf{s}_i' = \begin{cases} \mathbf{c}_i & \text{if } |S_i| > 0 \\ \mathbf{s}_i & \text{if } |S_i| = 0 \text{ (no samples assigned)} \end{cases}$$

### Convergence Properties

- Lloyd relaxation converges to a **centroidal Voronoi tessellation** (CVT) where each seed coincides with the centroid of its cell.
- Convergence is monotone: the total energy $E = \sum_i \sum_{j \in S_i} \|\mathbf{p}_j - \mathbf{s}_i\|^2$ is non-increasing at each step.
- The rate depends on sample density. More samples give a better centroid estimate per step.
- The fixed point is not unique; different initial seeds converge to different CVTs.

### Sample-Based Approximation

The implementation uses a discrete set of sample points rather than integrating over the continuous Voronoi cells. This is standard practice: a regular grid or stratified random sample set provides a good approximation. The centroid estimate improves with sample count.
