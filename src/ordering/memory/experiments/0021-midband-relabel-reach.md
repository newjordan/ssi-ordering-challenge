# 0021 — the mid band is out of GENERATORS but not out of RELABEL REACH

- **Date:** 2026-08-28
- **Score:** 0.842223 → **0.841660** (−0.000563); lt_1k 0.899432 → 0.899432,
  1k_10k 0.875342 → 0.8747, gt_10k 0.774477 → 0.7735
- **Status:** win (after one grader FAIL and a re-costing — see "The FAIL")

## Hypothesis
The 1k_10k bucket (108 matrices, geomean 0.8753, 22 exact ties) plus the light
end of gt_10k (n < 100k, nnz < 250k) run at 0.07–0.85 s of the 2.0 s cap, so
there is budget to spend. Two things could use it: (a) generator OPTIONS the
gates shut off above n ≈ 5k–12k (`METIS_VAR_MAX_N` 5000, `KAHIP_MAX_N` 6000,
`SCOTCH_MAX_N` 12000, `METIS_TUNED_MAX_N` 21000), and (b) the reach of the
relabel multistarts.

## What changed
`src/ordering/mod.rs` only. Four new probes in `probe.rs`: `probe_mid_sweep`,
`probe_minfill_wall`, `probe_mid_census`, `probe_mid_obj`, `probe_midx_ab`.
`order_with_options` grew a `midx_enabled` flag (same isolation plumbing as
`orbit_enabled` / `pharmakos_enabled`) so `probe_midx_ab` can time the change
ON and OFF in ONE interleaved run — the only honest way to measure on a box
four other agents are using.

1. **MIDX** — two relabel-window extensions above the shipped ceilings:
   no-dense AMD for `RELABEL_AMDND_MAX_NNZ` (60k) < nnz ≤ 95k, and AMF α5 for
   `RELABEL_AMF_MAX_NNZ` (130k) < nnz ≤ 180k. Guards: `max_deg ≤ n/50` (the hub
   rule from 0019) and `nnz ≤ 8n`.
2. **`MINFILL_MAX_N` 3000 → 4000.**
3. **Relabelled AMF α=0.5, two seeds, nnz ≤ 40 000** — a fourth relabel
   objective.

## Result
Six matrices better, none worse.

| matrix | bucket | before → after | order() s |
|---|---|---|---|
| mpbp_48 | gt_10k | 0.5069 → 0.4938 | 0.816 → 0.840 |
| arki0013 | gt_10k | 0.6261 → 0.6159 | 0.612 → 0.699 |
| nd_netgen-3000-1-1-b-b-ns_7 | gt_10k | 0.9797 → 0.9676 | 0.564 → 0.607 |
| crudeoil_pooling_ct3 | 1k_10k | 0.7392 → 0.7056 | 0.451 → 0.459 |
| rsyn0815m02hfsg | 1k_10k | 0.9636 → 0.9381 | 0.267 → 0.294 |
| korcns | lt_1k | 0.9468 → 0.9439 | 0.038 → 0.038 |

Worst `order()` 1.235 s = `ringpack_30_2`, untouched (the hub guard excludes it
by construction; frontier measures 1.225 s in the same conditions). Largest
increase anywhere: arki0013 +0.088 s. Largest increase on a matrix that does
NOT win: gabriel09 +0.035 s, gasprod_sarawak81 +0.034 s.

## Why it won / lost
**The generators are finished; the relabel REACH was not.** `probe_mid_sweep`
ran 58 candidate generators — every option `feral_amd`, `feral_amf`,
`feral_metis`, `feral_scotch`, `feral_kahip` expose (AMD aggressive ×
dense_alpha, AMF dense_alpha 0.5–20 and −1, METIS seed/imbalance/
nd_to_amd_switch/niparts/coarsen_floor/two_hop_ratio/dense_quotient, Scotch
seed/n_sep_trials/amd_switch/coarsen_floor/max_imbalance/compress/FM caps,
KaHIP seed/Eco/Strong) plus MinFill — against the SHIPPED `order()` on all 71
in-scope matrices left at ratio ≥ 0.94. **It improved exactly one.** In geomean
against the incumbent every partitioner family is 1.7–2.8× WORSE on this class;
the AMD-family variants are the closest at 1.5–4.5% worse, which is what you
would expect of an AMD-anchored portfolio. This is a third independent
corroboration of 0016 and 0018.

What still had headroom is which OBJECTIVE each matrix is allowed to relabel.
`RELABEL_AMDND_MAX_NNZ` = 60 000 and `RELABEL_AMF_MAX_NNZ` = 130 000 were sized
against a whole-corpus cost argument, not against this band. `probe_mid_census`
(up to 300 seeds × 3 objectives, flops-only so it is deterministic and
load-independent) shows the excluded objective is the winning one on several
matrices — and crucially, **at seed 1 or 2**: `nd_netgen-3000` at seed 1,
`mpbp_48` and `arki0013` at seed 2. A win at seed 2 is a gate problem, not a
budget problem, and gate problems are nearly free to fix.

## The FAIL, which is the most useful thing here
The first version scored **0.841441** on dev and the grader **KILLED** it. Its
only material cost was `crudeoil_lee4_10`: 0.964 s → **1.24 s**, which is INSIDE
the frontier's own local worst case of 1.225 s (`ringpack_30_2`) and was still a
FAIL. That puts the grader at **≥ 1.6× this box** and gives a much harder
operating rule than "stay under the local worst":

> A matrix already above ~0.7 s locally may not grow by more than ~0.1 s, and
> nothing may approach 1.0 s.

Re-costed against that rule the same idea keeps three of its four wins at
0.016–0.088 s each, because they were all seed-1/2 wins. The one it loses is the
biggest (`crudeoil_lee4_10` 0.7271 → 0.6858, −0.00034 of score): its census
curve is 0.7833 / 0.7758 / 0.7628 / 0.7416 / **0.6858** at 1 / 2 / 4 / 8 / 16
seeds, i.e. nothing before seed 10, ~0.19 s of work on a matrix with no budget.
**Re-open `MIDX_AMDND_MAX_NNZ` to 130 000 (budget 1 600 000, cap 12) once the
portfolio runs candidates in parallel** — the relabel seeds are embarrassingly
parallel and each is an independent `consider` task.

A second instance of the same trap, caught before shipping: the α=0.5 block's
natural ceiling is the shipped relabelled-AMF block's 130 000 — and that window
CONTAINS `ringpack_30_2` (nnz 121 458). Any new block gated only by
`nnz ≤ 130_000` silently lands on the corpus worst case. Either carry the
`max_deg ≤ n/50` hub guard or drop the ceiling; here the ceiling went to 40 000
and the score was byte-identical.

## Negative results (do not repeat)
- **58 generator variants → 1 win.** See above. Widening `METIS_VAR_MAX_N`,
  `SCOTCH_MAX_N`, `KAHIP_MAX_N`, `METIS_TUNED_MAX_N` into this band is pure cost.
- **Post-hoc phase gate widening buys EXACTLY ZERO.** `TELOS_TIER_A_MAX_NNZ`
  150k → 180k, `ORBIT_MAX_N` 30k → 50k and `HC_MAX_NNZ` 165k → 220k applied
  together changed **not one ratio** on the 300-matrix corpus. Reverted.
- **MinFill does not scale past n ≈ 4000.** `probe_minfill_wall` measures it on
  every matrix with n ≤ 40 000: above the wall it is in budget-fallback almost
  everywhere (ratios 1.4–4131, never the argmin), and its `n*n` `Vec<bool>`
  membership matrix is 1.6 GB at n = 40 000 against the grader's 4 GiB
  RLIMIT_AS. It beat the portfolio on exactly ONE matrix corpus-wide
  (`rsyn0815m02hfsg`, n = 3054, 18 vertices past the gate).
- **Eight of nine extra relabel OBJECTIVES are worthless here.** `probe_mid_obj`
  ran AMF α∈{0.5, 2.5, 10, 20, −1} and AMD {no-aggressive, α−1, α2.5, α20}
  through the relabel trick at each matrix's shipped restart count across the
  band: 9 × 140 combinations, ONE win (α=0.5 on `crudeoil_pooling_ct3`).
- A **spurious cap breach** on `ringpack_30_2` occurred on a change that cannot
  touch it (all three gates already admitted it), while another agent held
  ~900% CPU. 0014's rule holds: never trust a lone cap-breach message.

## Follow-ups
- Parallel lane: re-open `MIDX_AMDND_MAX_NNZ` → 130 000 / 1 600 000 / 12 and
  re-measure `crudeoil_lee4_10` on 4 threads (−0.00034 waiting there).
- Census gains that need seeds 33–300 and are therefore parallel-lane only:
  `procurement1large` amf@64 (0.6343 → 0.6258), `crudeoil_pooling_dt3` amd@48
  (0.9541 → 0.9361, needs `MIDX_HUB_DIV` relaxed to ~25), `crudeoil_lee1_07`
  nd@48, `hydroenergy2` amf@196, `sporttournament48` amf@256,
  `edgecross10-040` amf@265.
- `probe_mid_obj` has only been run on the 1k_10k / light-gt_10k band. The same
  nine objectives at two seeds over lt_1k is a few minutes of probe time.

## Links
- Probes: `../../probe.rs` (`probe_mid_sweep`, `probe_minfill_wall`,
  `probe_mid_census`, `probe_mid_obj`, `probe_midx_ab`)
- Prior art: [0016](0016-spectral-fiedler-nd.md), [0018](0018-headroom-census.md),
  [0019](0019-mid-band-restart-floor.md), [0020](0020-telos-seed-sensitivity.md)
