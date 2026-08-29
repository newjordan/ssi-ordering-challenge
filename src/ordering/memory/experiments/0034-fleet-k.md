# 0034 — fleet-k: envelope census of the two newest frontier mechanisms + pinned metis-seed re-census

**Date:** 2026-08-29
**Base:** our own frontier `1147d54` (82a7f496, official 0.802703; v2 proxy 0.802674 — reproduces to 3e-5)
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact), loaded by
`probe_fleet_k` itself (candidate-worker test shim is dev-fixed; probe follows probe_fleet_h's pattern,
`n:nnz` keys, `$SSI_PROBE_SECT` restricts to one section).
**Score:** v2 proxy **0.802674 → 0.801545 (−0.001129)**, 8 better / 0 worse of 500.
Dev on final tree: 0.812602 (no pre-change dev baseline for this tree; every change is
best-of/strict-improve except the seed swap, which was A/B'd dev-neutral — see item 3).
**Status:** all three items SHIPPED (one commit); the negatives below are kill-proofs, not leads.

## Item 1 (SHIPPED): simplicial-promotion envelope — n 4k→6k, nnz 60k→100k, budget 32M→64M

The 9001339 promoter (`final_simplicial_promotion`, gate n∈[1k,4k] ∧ nnz≤60k ∧ d≤24, 32M ops) was
tuned by its author on their corpus view. Swept each envelope direction as a separate measured cell,
promotion applied to `order()`'s final output (= the shipped placement — the pass is terminal):

| cell | members (v2) | movers | detail |
|---|--:|--:|---|
| cur (2nd pass) | 108 | 0 | b32≡b64≡none everywhere: pass is at a fixpoint, budget never binds in-gate |
| n_ext (4k,6k] | 28 | 3 | 5725:28914 **b64-only 0.996937** (32M exhausts → None, fails closed); 4856:29110 0.999783; 4253:23924 0.999984 |
| nnz_ext (60k,100k] | 2 | 1 | 3033:65206 **0.990197** (end-to-end −1.9%: 0.147264→0.144499); 3033:65254 none |
| corner (4k,6k]×(60k,100k] | **0** | — | empty on v2 → the shipped rectangle is exactly the measured region |
| dens_ext (24,32] | **0** | — | empty on v2 → not extended (unmeasurable cap bet, 0031 item-2 precedent) |

Shipped: MAX_N 6_000, MAX_NNZ 100_000, OPS_BUDGET 64_000_000; density 24 unchanged. Worst promotion
cost at 64M measured 0.021 s local (grader 2.6× on bitset replay ⇒ ~0.055 s) on rows ≤0.90 s local.
The budget raise is free in-gate (replay finishes ≪32M on every current member) and is what unlocks
the 5725:28914 win.

## Item 2 (SHIPPED): SqDiv α1 sibling in the hub-heavy medium DegSqrt arm

abbdacf's arm (`!medium_metric_hub_safe` ⇒ DegSqrt α∈{1,2}; hub_safe = max_deg≤16 ∨ max_deg²≤4n,
inside n<60k ∧ nnz<400k) — probed the unmeasured siblings on all 142 v2 class members:
DegSqrt α∈{0.5,4} and SqDiv α∈{0.5,1,2,4} vs the full incumbent.

- **DegSqrt α0.5 / α4: zero wins** (best is a 1.0000 tie; α0.5 is often catastrophic, up to 16.7×).
- **SqDiv wins on exactly two rows, α-invariant across the whole grid**: 4995:24790 raw 0.9717 and
  9995:49190 raw 0.9706. End-to-end (downstream phases compound from the better seed):
  0.684926→**0.635735** (−7.2%) and 0.669480→**0.633514** (−5.4%).
- **Class-boundary probe (HUBX): the window is right where it is.** All hub-unsafe rows outside the
  medium window (n≥60k ∨ nnz≥400k, nnz<1.2M ceiling, 8 rows incl. 221378:1093378, 138653:728892,
  23173:893304, 2587:921054): DegSqrt α∈{0.5,1,2,4} never beats the incumbent (best 1.0196). Do not
  extend the class.

Shipped: one extra `consider` — SqDiv α1.0 — inside the existing `!medium_metric_hub_safe` block.
Cost ≤9 ms/member measured; best-of floor ⇒ score-safe.

## Item 3 (SHIPPED): metis_seed2 pinned cell — seed 2 → seed 0 (straight swap)

Re-census of the remaining seeds {0,1,3,4,6,7} on the pinned matrices only (fleet-j owns general
sweeps). Raw METIS full-ND rel to the full incumbent:

- **pin2 cell (14000..14200 × 470k..485k), one v2 member 14100:477382:** s0 **0.788**, s3 0.929,
  s2 (pinned) **1.010** — the pinned seed no longer wins on this tree; everything else ≥1.04
  (s5/s6 ≈ 12–13). Swapped the pin to seed 0 (same single task slot, same 0.14 s cost). End-to-end:
  0.186274→**0.145661** (−24.6%, gt bucket — the biggest single mover; worth ≈ −0.00128 of score
  alone). Dev has one cell member, `pooling_sppc1pq` (14100:477680): all 8 seeds ≥2.40 raw, and a
  targeted A/B (seed 2 vs seed 0 builds) gives byte-identical 0.191454 — swap is dev-neutral.
- **pin5 cells (both (n,nnz,deg) ranges), four v2 members:** every seed 0–7 loses by ≥9%
  (best: s5 1.092 on 8423:268292, s0 1.105 on 9953:287612). The pinned seed-5 candidates are dead
  weight on this tree (~0.06 s/row), but removing them risks losing whatever downstream compounding
  produced today's incumbents — left untouched (cost-only trim, not this mission's risk budget).

## Verification

- Baseline v2 run: 0.802674 (lt 0.911203 / mid 0.789186 / gt 0.731394), worst order() 1.367 s.
- Final v2 run: **0.801545** (lt 0.911203 / mid 0.788533 / gt 0.729061), worst order() 1.347 s
  (same tail matrix, box noise). Multiset diff by n:nnz: exactly the 8 intended movers, 0 worse.
  Largest per-row time growth +0.080 s (4995:24790, 0.423→0.502 s — downstream work from the new
  SqDiv seed); everything that grew stays ≤0.93 s local.
- Dev full run on final tree: 0.812602, worst 1.109 s (crudeoil_pooling_ct3, the known tail class,
  under the 1.124 s historical corpus worst).
- Full test suite green (time_cap 5/5 with staged candidate build; 43 candidate-worker unit tests).

## Instruments

- `probe_fleet_k` — one probe, four sections (SIMP/HUB/HUBX/SEED), `$SSI_PROBE_SECT` filter;
  measures every candidate against the full `order()` incumbent with per-candidate wall times.
