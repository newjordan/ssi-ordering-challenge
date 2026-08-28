<!-- 0023 -->
# 0023 — Low-nnz Search Depth Expansion & Strip-v2 Fine Grid

- **Date:** 2026-08-28
- **Score:** dev 0.842223 → **0.842123** (−0.000100), fill 0.948942 → **0.948935**
- **Buckets:** 1k_10k 0.875342 → **0.875032**, gt_10k **0.774477**
- **Worst-case timing:** **1.157 s** (safely bounded under the 2.0 s per-matrix SIGKILL, +0.84 s margin)
- **Status:** WIN — clean zero-allocation candidates.

## Mechanisms
1. **Low-nnz ($nnz \le 12\text{k}$) search depth**: At $nnz \le 12\text{k}$, execution cost is bounded by microseconds per restart. Raised low-nnz restart budget from 600k to 1.2M.
2. **Strip-v2 Fine-Grid**: Evaluates finer supernode elimination cuts on cheap subgraphs ($k \in \{4, 8, 16, 24, 48, 96\}$).
