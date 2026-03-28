# prime-color — Mathematical Reference

Formulas and derivations for every color-space conversion and utility in `prime-color`.

---

## 1. sRGB Gamma Curve (IEC 61966-2-1)

The sRGB transfer function encodes linear light into a perceptually uniform gamma curve.

### Linear to sRGB (gamma encode)

$$\text{sRGB}(c) = \begin{cases} 12.92\,c & c \le 0.0031308 \\ 1.055\,c^{1/2.4} - 0.055 & c > 0.0031308 \end{cases}$$

### sRGB to Linear (gamma decode)

$$\text{linear}(c) = \begin{cases} c \;/\; 12.92 & c \le 0.04045 \\ \left(\frac{c + 0.055}{1.055}\right)^{2.4} & c > 0.04045 \end{cases}$$

**Continuity:** The threshold $0.04045 = 12.92 \times 0.0031308$ ensures both branches meet at the same point. The linear segment near zero avoids numerical instability in the power function.

**Fixed points:** $0 \to 0$, $1 \to 1$.

Applied per-channel to convert between sRGB and linear RGB representations.

---

## 2. Oklab Color Space

Perceptually uniform opponent color space by Bjorn Ottosson (2020). Perceptual uniformity means equal Euclidean distances in Lab space correspond to equal perceived color differences.

### sRGB to Oklab

**Step 1 — Gamma decode:** sRGB to linear RGB via the transfer function above.

**Step 2 — Linear RGB to LMS** (approximate cone responses):

$$\begin{pmatrix} l \\ m \\ s \end{pmatrix} = M_1 \begin{pmatrix} R \\ G \\ B \end{pmatrix}$$

$$M_1 = \begin{pmatrix} 0.4122 & 0.5363 & 0.0514 \\ 0.2119 & 0.6807 & 0.1074 \\ 0.0883 & 0.2817 & 0.6300 \end{pmatrix}$$

**Step 3 — Cube root nonlinearity** (perceptual compression):

$$l' = \sqrt[3]{l}, \quad m' = \sqrt[3]{m}, \quad s' = \sqrt[3]{s}$$

Signed cube root preserves sign: $\text{cbrt}(x) = \text{sgn}(x) \cdot |x|^{1/3}$.

**Step 4 — LMS' to Lab:**

$$\begin{pmatrix} L \\ a \\ b \end{pmatrix} = M_2 \begin{pmatrix} l' \\ m' \\ s' \end{pmatrix}$$

$$M_2 = \begin{pmatrix} 0.2105 & 0.7936 & -0.0041 \\ 1.9780 & -2.4286 & 0.4506 \\ 0.0259 & 0.7828 & -0.8087 \end{pmatrix}$$

**Result:** $L \in [0,1]$ (lightness), $a \approx [-0.5, 0.5]$ (green-red), $b \approx [-0.5, 0.5]$ (blue-yellow).

### Oklab to sRGB

Reverse the pipeline: $M_2^{-1}$ to get $(l', m', s')$, cube to get $(l, m, s)$, $M_1^{-1}$ to get linear RGB, gamma-encode, clamp to $[0, 1]$.

$$M_2^{-1} = \begin{pmatrix} 1 & 0.3963 & 0.2158 \\ 1 & -0.1056 & -0.0639 \\ 1 & -0.0895 & -1.2915 \end{pmatrix}$$

$$M_1^{-1} = \begin{pmatrix} 4.0767 & -3.3077 & 0.2310 \\ -1.2684 & 2.6098 & -0.3413 \\ -0.0042 & -0.7034 & 1.7076 \end{pmatrix}$$

---

## 3. HSL (Hue, Saturation, Lightness)

Cylindrical rearrangement of sRGB. Hue is angular, saturation and lightness are radial/vertical.

### sRGB to HSL

$$\text{max} = \max(R, G, B), \quad \text{min} = \min(R, G, B), \quad \delta = \text{max} - \text{min}$$

$$L = \frac{\text{max} + \text{min}}{2}$$

$$S = \begin{cases} 0 & \delta = 0 \\ \frac{\delta}{1 - |2L - 1|} & \text{otherwise} \end{cases}$$

$$H = \begin{cases} 0 & \delta = 0 \\ 60 \cdot \bigl(\frac{G - B}{\delta} \bmod 6\bigr) & \text{max} = R \\ 60 \cdot \bigl(\frac{B - R}{\delta} + 2\bigr) & \text{max} = G \\ 60 \cdot \bigl(\frac{R - G}{\delta} + 4\bigr) & \text{max} = B \end{cases}$$

Result: $H \in [0, 360)$ degrees, $S \in [0, 1]$, $L \in [0, 1]$.

### HSL to sRGB

$$q = \begin{cases} L(1 + S) & L < 0.5 \\ L + S - LS & L \ge 0.5 \end{cases}, \quad p = 2L - q$$

Each channel is computed from the sector function $f(p, q, t_k)$ where $t_R = H/360 + 1/3$, $t_G = H/360$, $t_B = H/360 - 1/3$:

$$f(p, q, t) = \begin{cases} p + (q-p) \cdot 6t & t < 1/6 \\ q & t < 1/2 \\ p + (q-p) \cdot (2/3 - t) \cdot 6 & t < 2/3 \\ p & \text{otherwise} \end{cases}$$

(with $t$ wrapped to $[0, 1]$).

---

## 4. HSV (Hue, Saturation, Value)

Similar to HSL but with Value = max channel instead of the HSL midpoint.

### sRGB to HSV

$$V = \text{max}, \qquad S = \begin{cases} 0 & \text{max} = 0 \\ \delta / \text{max} & \text{otherwise} \end{cases}$$

Hue uses the same sector logic as HSL.

### HSV to sRGB

$$C = V \cdot S, \quad H' = H / 60$$

$$X = C \cdot (1 - |H' \bmod 2 - 1|), \quad m = V - C$$

Select $(R_1, G_1, B_1)$ by sector of $H'$, then add $m$ to each channel:

| Sector | $R_1$ | $G_1$ | $B_1$ |
|--------|-------|-------|-------|
| $0 \le H' < 1$ | $C$ | $X$ | $0$ |
| $1 \le H' < 2$ | $X$ | $C$ | $0$ |
| $2 \le H' < 3$ | $0$ | $C$ | $X$ |
| $3 \le H' < 4$ | $0$ | $X$ | $C$ |
| $4 \le H' < 5$ | $X$ | $0$ | $C$ |
| $5 \le H' < 6$ | $C$ | $0$ | $X$ |

---

## 5. Luminance (BT.709)

Relative luminance per ITU-R BT.709. Input must be **linear** RGB (not gamma-encoded sRGB).

$$Y = 0.2126\,R + 0.7152\,G + 0.0722\,B$$

The coefficients reflect the human eye's spectral sensitivity: green contributes most to perceived brightness, blue the least.

---

## 6. Contrast Ratio (WCAG 2.x)

$$\text{ratio} = \frac{L_{\text{lighter}} + 0.05}{L_{\text{darker}} + 0.05}$$

where $L$ is relative luminance (computed via gamma decode then BT.709).

- Result $\ge 1$. Maximum is $21:1$ (white on black).
- WCAG AA requires $\ge 4.5:1$ for normal text, $\ge 3:1$ for large text.

---

## 7. Perceptual Mixing (`oklab_mix`)

Blends two sRGB colors through Oklab space for perceptually uniform interpolation.

1. Convert both colors to Oklab: $(L_0, a_0, b_0)$ and $(L_1, a_1, b_1)$.
2. Lerp each component:

$$L = L_0 + (L_1 - L_0) \cdot t, \quad a = a_0 + (a_1 - a_0) \cdot t, \quad b = b_0 + (b_1 - b_0) \cdot t$$

3. Convert blended Lab back to sRGB (clamped to $[0, 1]$).

**Why Oklab over sRGB lerp:** Linear interpolation in sRGB produces perceptually uneven transitions and unexpected dark bands (e.g. red-to-green through muddy brown). Oklab's perceptual uniformity ensures the midpoint looks "halfway" to a human observer.

---

## 8. Palette Generation (Hue Rotation)

All palette functions convert to HSL, rotate the hue angle, and convert back.

### Complementary

$$H' = (H + 180) \bmod 360$$

Opposite side of the color wheel. Maximum hue contrast.

### Triadic

$$H_1 = (H + 120) \bmod 360, \quad H_2 = (H + 240) \bmod 360$$

Three colors equally spaced at $120^\circ$.

### Analogous

$$H_1 = (H + 30) \bmod 360, \quad H_2 = (H - 30) \bmod 360$$

Adjacent hues. Low contrast, harmonious palette.

Saturation and lightness are preserved for all palette operations.
