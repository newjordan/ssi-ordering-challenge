# 0004 — The heavy tier: closing the blind spot the budget created

**Date:** 2026-07-26
**Score:** 0.876925 → **0.876329** (fill 0.962248 → 0.962012), 300-pattern dev corpus
**Status:** WIN on dev, **FAILED the grader** — superseded by [0005](0005-eval-fail-density-regate.md)

A modest gain, but it fixes a structural gap and it produced two methodological
findings that matter more than the score.

## The blind spot

[0003](0003-relabelled-amd-multistart.md) made the relabel budget its own gate:
`restarts = RELABEL_BUDGET / nnz`, so `nnz > 300_000` yields **zero** restarts.
Elegant for cost safety, and wrong in one specific way nobody checked at the
time: it excluded **10 of the 45 `gt_10k` matrices** — the bucket carrying weight
0.40 and ~4.4x the per-matrix leverage of `lt_1k`. **Six of those ten were still
tied at exactly 1.0000.** The highest-value candidate family was structurally
barred from the highest-value matrices.

| matrix | n | nnz | ratio before |
|---|---|---|---|
| `faclay75` | 272878 | 1379706 | 1.0000 |
| `acopf_case9241pegase_qcqp` | 313068 | 1292408 | 1.0000 |
| `gabriel10` | 244056 | 1148210 | 1.0000 |
| `unitcommit_200_100_1_mod_8` | 146830 | 476332 | 1.0000 |
| `kissing2` | 20772 | 401024 | 1.0000 |
| `transswitch2736spr` | 69651 | 331010 | 1.0000 |

## Finding 1 — `probe_large` was blind to its own subject

Its filter was `if n < 100_000 { continue }`. But five of the ten zero-restart
heavies have n between 14k and 70k (`pooling_sppc1pq` n=14100/nnz=477680 down to
`transswitch2736spr` n=69651/nnz=331010). Cost tracks nnz, and so does the budget
that excluded them, so selection must too. Now selects on `nnz >= 250_000`.

## Finding 2 — a probe can contaminate its own timings by 7x

`probe_large` measures cost and benefit in one pass. Its benefit column is exact
(flops are deterministic). Its **cost column is not usable at all**. Same code,
same inputs, minutes apart:

| matrix | `probe_timing_and_score` | `probe_large` | factor |
|---|---|---|---|
| `kissing2` | 0.222 s | 1.623 s | **7.3x** |
| `unitcommit_200_100_1_mod_8` | 0.296 s | 2.003 s | **6.8x** |
| `pooling_sppc3pq` | 0.330 s | 1.956 s | 5.9x |
| `nuclear104` | 0.839 s | 2.380 s | 2.8x |

Far outside the ~1.6x ambient variance from [0003](0003-relabelled-amd-multistart.md),
and self-inflicted: the probe runs AMF and METIS passes over 250k–1.4M-nnz
patterns for every matrix in sequence, and the `order()` calls timed alongside
them pay for it (thermal throttling or allocator pressure; not isolated further).

This nearly caused a wrong decision. Read from `probe_large`, `gabriel10` looked
like 1.869 s + 0.682 s ≈ 2.55 s and therefore unaffordable. Measured cleanly it
is 0.412 s + 0.232 s = 0.644 s, and it is one of the two best wins on the tier.

**Rule: never size a gate from a probe that does heavy work alongside the thing
it times.** `probe_heavy_cost` exists for this — one candidate at a time, no AMF,
no METIS, min-of-3 repetitions (min is the robust estimator, since contention can
only ever make a sample slower than the true cost).

## The clean measurements

| matrix | nnz | order_s | rl1_s | amdnd_s | cur_r | rl1_r | amdnd_r |
|---|---|---|---|---|---|---|---|
| `faclay75` | 1379706 | 0.543 | 0.358 | 0.771 | 1.0000 | 1.0000 | 1.0019 |
| `acopf_case9241pegase_qcqp` | 1292408 | 0.147 | 0.217 | 0.167 | 1.0000 | **0.9737** | 1.0608 |
| `gabriel10` | 1148210 | 0.412 | 0.200 | 0.232 | 1.0000 | 1.0530 | **0.9756** |
| `cont6-qq` | 557994 | 0.181 | 0.103 | 0.046 | 0.9145 | 1.5184 | **0.8899** |
| `unitcommit_200_100_1_mod_8` | 476332 | 0.292 | 0.143 | 0.109 | 1.0000 | **0.9978** | 1.0246 |
| `nuclear104` | 257806 | 0.833 | 0.088 | 0.068 | 1.0000 | 1.0238 | 1.0000 |

Nine of thirteen heavy matrices gained nothing — every candidate came back at or
above the current ratio.

## The change

Two gates, each sized from the clean costs:

- **`RELABEL_HEAVY_MAX_NNZ = 1_500_000`** — where `RELABEL_BUDGET / nnz` rounds to
  zero, take ONE restart anyway. Affordable because the heavy end has a far
  better cost constant than the corpus worst case: 0.358 s at nnz=1379706 is
  `k ~ 2.6e-7`, against the 9.4e-7 that 0003 sized the budget on. That outlier
  (`sfacloc2_3_80`, nnz=20600) is per-pass overhead, not per-nnz work. The
  ceiling sits just above the corpus maximum; past it there is no measurement.
- **`HEAVY_AMDND_MIN_NNZ = 250_000`, `HEAVY_AMDND_MAX_NNZ = 1_200_000`** — AMD
  with dense-column deferral suppressed (`aggressive: false, dense_alpha: -1.0`).
  The ROBUST block already runs this variant but only under `nnz < 130_000`, so
  no heavy matrix had ever seen it.

The upper bound on the second is a **behavioural boundary, not a tuned cutoff**,
which is what makes it defensible on a rebaselined corpus: above it the variant
loses on both matrices (`faclay75` 1.0019, `acopf` 1.0608); just below it
`gabriel10` wins (0.9756). That is the expected physics — the larger and denser
the pattern, the more genuine hub structure deferral was right to postpone.
Excluding the top also drops the most expensive pass on the tier (`faclay75`,
0.771 s) which returns nothing.

## Result

| matrix | before | after | via |
|---|---|---|---|
| `cont6-qq` | 0.9145 | 0.8899 | no-dense AMD |
| `acopf_case9241pegase_qcqp` | 1.0000 | 0.9737 | relabel x1 |
| `gabriel10` | 1.0000 | 0.9756 | no-dense AMD |
| `unitcommit_200_100_1_mod_8` | 1.0000 | 0.9978 | relabel x1 |

`gt_10k` 0.831659 → 0.830169. Predicted 0.876327 by hand from the sensitivity
`d(score) = w_b * geomean_b / count_b * Δln(ratio)`; harness measured
**0.876329**. Worst `order()` 0.963 s, under the 1.019 s comparative rule.

`acopf_case9241pegase_qcqp` is the satisfying one: until now it received *only*
the baseline AMD pass and nothing else in the entire portfolio.

## Calibration note

The plan that motivated this round estimated −0.002 to −0.005; actual was
−0.0006, ~5x optimistic. The error was assuming round-3-style depth. Round 3's
tie wins landed at 0.83–0.92; the heavy tier's land at 0.89–0.98. **Heavy tied
matrices are tied for a better reason than mid-size ones** — worth remembering
before pricing the next idea off a tie count alone. Use the runner-up margin, not
the tie count (see [open-questions](../open-questions.md)).

## Postscript: FAILED on eval

Submitted as `0cbb4def` and killed by the 2 s cap on `telecomsp_metro`
(n=13940, nnz=743408, nnz/n≈53). The nnz-window gates fired on an eval
dense-heavy class that dev lacks entirely (up to nnz/n=497). Root cause,
the eval-census discovery, and the density re-gate are in
[0005](0005-eval-fail-density-regate.md).
