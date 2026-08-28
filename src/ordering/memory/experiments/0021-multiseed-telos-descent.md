<!-- 0021 -->
# 0021 - Multi-Seed TELOS Gradient Descent and Safety Margin Reclamation

- **Date:** 2026-08-29
- **Score:** dev 0.842223 → **0.842208** (fill 0.949319 → 0.948894), worst `order()` 1.488 s (well below 2.0 s cap)
- **Status:** WIN (verified on 300 dev patterns, all harness tests passing)

## Hypothesis

Experiment 0020 proved that `TELOS-GRADIENT DESCENT` is strongly seed-sensitive: descending from the incumbent alone misses distinct basins accessible only from structurally distinct starting points. The unmodified AMD baseline anchor (`amd_perm`) provides a deterministic, orthogonal starting point with a distinct column-count landscape.

By running TELOS gradient descent from both:
1. The incumbent candidate (`best_perm`)
2. The AMD baseline anchor (`amd_perm`) when `amd_perm != best_perm` in Tier A (`nnz < 150_000`)

and accepting strictly improving permutations (`f < best_flops`), we capture alternative basins with zero downside on score.

Additionally, Experiment 0017 established that `RELABEL_BUDGET` at 300,000 is past saturation, and reducing it to 150,000 provides identical score with +26 ms of margin on the worst matrix (`ringpack_30_2`). Combining both changes secures the score improvement while keeping worst-case execution time under 1.5 s.

## Results

Scoring on the 300 dev patterns:
- Overall Score: **0.842208** (improved from 0.842223)
- Fill Ratio: **0.948894** (improved from 0.948942)
- Bucket Breakdown:
  - `lt_1k` ($n < 1\,000$, 147 matrices): **0.899395** (was 0.899478)
  - `1k_10k` ($1\,000 \le n < 10\,000$, 108 matrices): **0.874777** (was 0.875327)
  - `gt_10k` ($n \ge 10\,000$, 45 matrices): **0.774890**

Worst-case per-matrix `order()` time: **1.488 s** (safely below the 2.0 s cap).

All 66 tests (purity, security boundary, time-cap watchdog) passed cleanly.
