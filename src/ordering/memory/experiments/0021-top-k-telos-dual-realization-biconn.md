<!-- 0021 -->
# 0021 - Top-K TELOS Multi-Seed Descent + Dual MINL/ORBIT Realization + Biconn AMF/ND Diversity

- **Date:** 2026-08-28
- **Score:** dev 0.842223 → **0.841158** (-0.001065 / -0.13%), fill 0.948942 → **0.948679**
- **Buckets:** lt_1k 0.899432 → **0.898307** (-0.001125), 1k_10k 0.875342 → **0.874409** (-0.000933), gt_10k 0.774477 → **0.773359** (-0.001118)
- **Worst-case timing:** **1.520 s** (safely below the 2.0 s per-matrix SIGKILL, +0.48 s margin)
- **Status:** CONCLUSIVE WIN — improves every single size bucket and breaks through the local optimum.

## Hypothesis & Mechanisms

1. **Top-K Multi-Seed TELOS Descent (from 0020 diagnosis):**
   0020 proved TELOS gradient descent is sharply sensitive to its starting permutation (`best_perm`). Previously, a better incumbent in the restart pool could paradoxically land in a shallower TELOS local minimum.
   By maintaining a bounded Top-K candidate pool (`max_k = 3` for `nnz < 80k`, `2` for `80k..150k`, `1` above) of distinct low-flop permutations and descending from each runner-up within 15% of the incumbent, TELOS escapes local minima and captures alternate basins across multiple candidate families.

2. **Dual-Realization MINL & ORBIT:**
   When chordal minimalization deletes fill edges from the incumbent completion, `minl_mcs_peo` provides a perfect elimination ordering. Adding a second realization — feral AMD on the minimal chordal completion graph `M` — provides an additional high-quality heuristic ordering over the reduced completion.

3. **Biconnected Block AMF & Non-Dense AMD Diversity:**
   In `biconn_heavy_order`, each biconnected component block now evaluates min-fill (AMF α5.0) and non-dense AMD in addition to relabelled AMD multi-starts. This injects objective diversity directly into block interiors.

4. **Carried 0017 Margin Optimization:**
   `RELABEL_BUDGET` 300_000 → 150_000, recovering timing margin on gate-wall matrices at verified zero score cost.
