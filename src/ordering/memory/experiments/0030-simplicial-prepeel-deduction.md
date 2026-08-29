# 0030 — Exact Simplicial & Low-Degree Deduction Pre-Peeling

**Date:** 2026-08-29
**Base:** accepted `5f024a7` (dev 0.816337)
**Score:** dev 0.816337 → **0.816097** (−0.000240); fill 0.933260 → **0.933199**
**Status:** submitted

## The Idea

At a fundamental mathematical level (Dirac 1961, Rose 1972), if a vertex $v$ is **simplicial** in $G$ (its open neighborhood $N(v)$ induces a clique in the active elimination graph), eliminating $v$ first creates **zero fill-in** ($\operatorname{Def}(v) = \emptyset$) and leaves the fill and column counts of all other vertices unchanged or reduced ($F_{\pi'} \subseteq F_{\pi}$).

We implemented a deterministic $O(n + \text{nnz})$ deduction engine in `src/ordering/simplicial.rs` that iteratively peels:
1. Isolated vertices ($\operatorname{deg} = 0$, $c_v = 1$).
2. Pendant vertices / leaves ($\operatorname{deg} = 1$, $c_v = 2$).
3. Triangle vertices ($\operatorname{deg} = 2$ where $(u, w) \in E$, $c_v = 3$).
4. Low-degree simplicial cliques ($\operatorname{deg} \le 4$ where $N(v)$ is fully connected).

If a core remains, it extracts the induced subgraph on $V_{\text{core}} = V \setminus S$ and computes an ordering using AMD or AMF on the reduced core, then concatenates $[S, \pi_{\text{core}}]$.

## Standalone & Portfolio Results

Standalone (`probe_simplicial`), the pre-peel ordering alone beat feral's baseline AMD on **49 of 300 matrices** (e.g., `multiplants_stg5` 0.5527, `chp_shorttermplan2d` 0.7023, `crudeoil_lee4_10` 0.8128, `arki0013` 0.8542, `cont6-qq` 0.9174) with a worst single-matrix time of only **0.1219s** and a full 300-matrix runtime of **2.61s**.

When integrated into the portfolio via `consider!` (with 4-thread parallel evaluation and the baseline AMD anchor guaranteeing zero regression):

| Bucket | Count | Baseline Flops | New Flops | Delta |
|---|---|---|---|---|
| `lt_1k` | 147 | 0.892062 | **0.891259** | −0.000803 |
| `1k_10k` | 108 | 0.860482 | **0.860482** | 0.000000 |
| `gt_10k` | 45 | 0.726436 | **0.726436** | 0.000000 |
| **Overall** | **300** | **0.816337** | **0.816097** | **−0.000240** |

Fill ratio (tiebreak) improved from **0.933260** to **0.933199**.

## Timing & Safety

- Worst `order()` time remains strictly bounded and identical within noise.
- All 54 self-checks, scorer cross-checks, exact equivalence tests, and time cap tests passed.
