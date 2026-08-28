# 0024 — MIDX second tier: the deferred 95k→130k no-dense-AMD window, re-opened for the parallel lane

- **Date:** 2026-08-29
- **Base:** 91f0b60 (promoted frontier, eval 0.827673), branch fleet/c
- **Score:** dev 0.834636 → **0.834239** (−0.000397); fill 0.945578 → 0.945302
  (tiebreak also improves). Buckets: lt_1k 0.893457 → 0.893457 (bit-identical),
  1k_10k 0.871580 → 0.871580 (bit-identical), gt_10k 0.762812 → **0.761820**.
- **Status:** WIN (verified full-corpus scored run, status OK, both harness runs)

## Hypothesis

0021-midband-relabel-reach measured the largest gain of its census on
`crudeoil_lee4_10` (0.7271 → 0.6858, ≈−0.00034 of score) and could not afford
it sequentially — the census curve (0.7833/0.7758/0.7628/0.7416/0.6858 at
1/2/4/8/16 seeds) shows nothing before seed 10, ~0.19 s of added sequential
work on a matrix then at 0.92–0.96 s. It deferred with an explicit re-open
condition: *"Re-open `MIDX_AMDND_MAX_NNZ` to 130 000 (budget 1 600 000,
cap 12) once the portfolio runs candidates in parallel."* At this base the
condition holds: every relabel seed is an independent `consider` task on the
4-thread output-identical carrier, so 12 seeds cost ~¼ of their sequential
time in wall-clock, and `crudeoil_lee4_10` itself now runs ~0.89 s (4-thread)
where the sequential lineage measured 0.96 s.

## What was already claimed at the base (verified in code, not from notes)

newjordan's 70a7701 note ("re-opened mid-band relabel windows, 60–95k no-dense
AMD, 130–180k AMF α5") describes the MIDX block that already shipped:
`MIDX_AMDND_MAX_NNZ` = **95_000** (budget 400k, cap 4), `MIDX_AMF_MAX_NNZ` =
180_000 (600k, cap 2), `MIDX_AMF05_MAX_NNZ` = 40_000. The (95k, 130k]
no-dense-AMD window was **unclaimed** — `RELABEL_AMF_MAX_NNZ` = 130k covers
AMF there, but the no-dense-AMD objective (the one that wins on
`crudeoil_lee4_10`) stopped at 95k.

## What changed

`src/ordering/mod.rs` only. A second no-dense-AMD tier as a separate block, so
the shipped 60–95k tier stays byte-identical:

- `MIDX2_AMDND_MAX_NNZ` = 130_000, `MIDX2_BUDGET` = 1_600_000,
  `MIDX2_MAX_RESTARTS` = 12 → 12 seeds across the whole window
  (1.6M/nnz ≥ 12.3 at the ceiling).
- Same guards, enclosing both tiers: `max_deg ≤ n/50` (`MIDX_HUB_DIV` —
  the only covariate separating `crudeoil_lee4_10`, max_deg 168, from its
  (n,nnz)-twin `ringpack_30_2`, max_deg 1632, the corpus worst case) and
  `nnz ≤ 8n` (`MIDX_MAX_AVG_DEG` — keeps the `telecomsp_metro` dense-KKT
  FAIL class out).
- Same closure body as the first tier (relabel → no-dense AMD
  (aggressive=false, dense_alpha=−1) → un-relabel), seeds 1..=12.

## Result

`probe_midx_ab` (interleaved OFF/ON, min of 2 reps, box quiet under the
scored-run lock) over the 60–180k guarded window, 7 matrices:

| matrix | nnz | off_r → on_r | off_s → on_s |
|---|---|---|---|
| crudeoil_lee4_10 | 120632 | 0.72713 → **0.68581** | 0.8949 → 0.8871 |
| crudeoil_lee4_09 | 101792 | 0.74480 → 0.74480 | 0.6936 → 0.7488 |
| (5 others, outside (95k,130k]) | | dr = 0.00000 | timing noise only |

- The full census prize arrives at 12 seeds — 0.68581 equals the 16-seed
  census value, so cap 12 loses nothing vs 16.
- Added time on `crudeoil_lee4_10` ≈ 0 in wall-clock (−0.008 s, noise): the
  12 extra tasks fill idle carrier slots. `crudeoil_lee4_09` pays the whole
  window's worst added cost, +0.055 s on a 0.69 s base → 0.749 s. Both far
  inside the "≤0.1 s growth above 0.7 s, nothing near 1.0 s" grader rule.
- Full-corpus scored run: **lt_1k and 1k_10k bucket geomeans bit-identical**
  to the baseline run — consistent with the gate proof that no matrix outside
  (95k, 130k]∩guards gets any new task, so no TELOS-seed side effects
  (0020's hazard) can occur outside the window. `crudeoil_lee4_09` is the
  only other admitted dev matrix and its ratio is unchanged (its no-dense-AMD
  seeds 1–12 never beat the incumbent).
- `ringpack_30_2` untouched by construction (hub gate), ratio 0.302 unchanged.

## Why it won

Pure gate re-open of a measured, reproducible gain whose only blocker was
sequential time. The parallel carrier changed the cost model (12 relabel
seeds ≈ 3 tasks/thread of ~0.012 s each, overlapped with the rest of the
portfolio queue), the hub+density guards were already proven load-bearing, and
the win itself was pre-measured by 0021's census. Lowest-risk item on the
frontier map for exactly this reason.

## Negative results / not done

- Did NOT fold the tiers together (one 60–130k window at 1.6M/12): that would
  raise seed counts 4→12 on the 60–95k tier — extra time on `mpbp_48`,
  `nd_netgen`, `gabriel09`, `gasprod_sarawak81` for gains 0021's census
  already shows do not exist there (wins were all at seed 1–2), plus a
  TELOS-seed-sensitivity exposure (0020) on matrices that currently win.
- 0021's deep-seed follow-ups remain unclaimed and need much bigger budgets:
  `procurement1large` amf@64 (0.6343→0.6258), `crudeoil_pooling_dt3` amd@48
  (needs `MIDX_HUB_DIV` relaxed to ~25 — separately risky),
  `crudeoil_lee1_07` nd@48, `hydroenergy2` amf@196, `sporttournament48`
  amf@256, `edgecross10-040` amf@265. amf@64 on `procurement1large` is the
  natural next probe: check its current 4-thread base time first.

## Links

- Plan source: [0021-midband-relabel-reach.md](0021-midband-relabel-reach.md)
  ("The FAIL" section + follow-ups); hub-gate derivation:
  [0019](0019-mid-band-restart-floor.md); TELOS-seed hazard:
  [0020](0020-telos-seed-sensitivity.md).
- Probe: `probe_midx_ab` (`SSI_AB_REPS=2`, under `/tmp/ssi-scored-run.lock`).
