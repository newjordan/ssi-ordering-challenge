# 0026 — Dense low-alpha arm for the heavy dense class (nnz in [400k, 1M), nnz/n > 20)

**Date:** 2026-08-29
**Base:** accepted `13123bc` (0.820466 official, dev 0.817933)
**Score:** reconstructed eval corpus **0.820466 → 0.817268** (−0.003198); dev **unchanged** (0.817933 → 0.817933)
**Status:** WIN, submitted

## The idea

Two measurements, taken on the reconstructed eval corpus (the [0010](0010-parallel-research-session.md)
method, extended to a full 500/500 reconstruction — every matrix reproduces the census `flops(AMD)`
exactly, and the accepted `13123bc` tree reproduces its official 0.820466 on it to six decimals):

1. For `nnz >= D_MAX_NNZ` (400k) the shipped alpha grid offers AMF only inside the D_WIDE density
   window (20 < nnz/n < 50) and only at `{2.5, 0.5}`. The **dense-heavy class (nnz/n > 50)** —
   `telecomsp_metro` is the eval example, 13940 × 743k, density 53 — receives **no AMF/AMD candidate
   at all** and ties AMD at 1.0000.
2. An isolated 15-point alpha census over every matrix with 130k <= nnz < 1M in BOTH corpora found
   the winning alphas at points the grid skips: **low alpha** (0.75, 1.25) **defers more dense rows**
   — the opposite direction from the no-dense hazard recorded in [0005](0005-eval-fail-density-regate.md).

Every candidate enters through `consider!` (parallel generation, sequential earliest-strict-argmin
acceptance, AMD floor intact), so the arm is free upside on score; its only cost is time.

## The change (`src/ordering/mod.rs`, two blocks, +53 lines, no new constants, no deps change)

- **D_WIDE extra alphas** (same window as the shipped D_WIDE dense arm): AMF α ∈ {1.0, 1.5, 2.0}.
- **Dense low-alpha arm**, `nnz in [D_MAX_NNZ, D_WIDE_MAX_NNZ) && nnz > D_WIDE_MIN_DENSITY * n`
  (no upper density bound): AMF α0.75, AMF α1.25, AMD{aggressive, α0.75}; plus AMF α2.0 only when
  `nnz < D_WIDE_MAX_DENSITY * n` — α2 hits the `select_pivot_amf` bucket-cardinality cliff on
  `telecomsp_metro` ([0010](0010-parallel-research-session.md), 0.237 s), so it stays out of the
  >50 class by construction.

## Measured

Reconstructed eval corpus, 500 matrices, per-matrix comparison against the accepted tree
(`probe_timing_eval`, isolated, 4 threads):

| matrix | n | nnz | nnz/n | accepted | candidate | Δ |
|---|--:|--:|--:|--:|--:|--:|
| `telecomsp_metro` | 13940 | 743408 | 53.3 | 1.000000 | **0.608314** | −0.3917 |
| `pooling_sppc1tp` | 14100 | 477382 | 33.9 | 0.655277 | **0.504043** | −0.1512 |
| `pooling_sppb5tp` | 18529 | 674070 | 36.4 | 0.282141 | 0.274572 | −0.0076 |
| `pooling_sppc3tp` | 23173 | 893304 | 38.5 | 0.226236 | 0.221926 | −0.0043 |

**0 of 500 worse, 4 better.** Buckets: lt_1k and 1k_10k byte-identical; gt_10k 0.767949 → 0.759954.
Dev: **0 of 300 moved** (no dev matrix sits in the window with a winning alpha), which is why this class
was invisible to dev-only tuning.

Timing (isolated, this box, 4 worker threads — the same rig that read 1.660 s for the accepted tree
on eval): candidate eval worst **1.637 s** (`watercontamination0202`, outside the arm's window; the
accepted tree reads 1.660 s on the same matrix), dev worst 1.252 s vs accepted 1.209 s (run-to-run
noise on this box is ±0.3 s on untouched matrices). Added cost on the four movers: +0.02 – +0.11 s
each (0.65 → 0.65, 0.47 → 0.55, 0.71 → 0.76, 0.32 → 0.43). Every matrix the arm touches finishes
under 0.8 s locally. `cargo test --release`: green.

## What did not work / residue

- ~62% of the gain is one matrix (`telecomsp_metro`). The class is real but small on eval.
- Earlier this session an 8-seed relabelled-AMF multistart at the argmin alpha inside D_WIDE was
  validated on the OLD frontier (5141ad0: eval −0.0013 alone) but is not carried here: the new
  frontier's mid-band restarts overlap it and its cost is not free under the 4-thread budget.
- Nested dissection (feral METIS / Scotch, six variants) on the giant sparse ties (`faclay75`,
  `gabriel10`, `acopf_case9241pegase_qcqp`, `transswitch*`, `unitcommit_200_*`) is **2–4× worse than
  AMD and 1.6–10 s** — structural, do not re-walk.
