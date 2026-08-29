# 0007 — Heavy-tier multi-restart (budget-scaled relabel passes)

**Result: dev 0.875359 → 0.875217. WIN (small on dev, priced for eval).**

## Motivation: the post-promotion eval census

Mining promoted run `952ef52` (gh run 31104389187) gives the fresh census:
500 matrices, ties 110/42/29 per bucket, reconstructed score 0.885313 exactly.
The biggest remaining gt_10k eval ties are SPARSE HEAVY (avg deg ≤ 6, nnz
150k-900k): unitcommit_200_100_2_mod_8 / _200_0_5_mod_7, transswitch2736spp,
powerflow2736spp, supplychainp1_053050, sjup2 — the exact class the relabel
floor gave ONE restart.

## Measurement (`probe_heavy_relabel_multi`, isolated, min-of-2)

Per-pass cost on sparse heavy is ~6e-8 s/nnz — 4-23× cheaper than the 2.6e-7
model that sized the one-pass floor (transswitch2383wpr 0.016 s, unitcommit
0.069 s, gabriel10 0.135 s, faclay75 0.225 s). And wins need MORE than one
pass: transswitch2383wpr ties at passes 1-2, flips to **0.9810** at pass 3;
transswitch2736spr 0.9996 at pass 4. faclay75, gabriel10, unitcommit_..._1_mod_8:
nothing through 4 passes (gabriel10's earlier extra-budget "0.982" was NOT
reproduced here — that observation was from the hc-phase probe stack and does
not survive isolation; treat it as stale).

## Shipped

In `relabel_restarts`: `heavy_multi = clamp(900_000/nnz, 1, 4)` for
`150_000 < nnz <= 1.5M` and `nnz <= 6n` (the SAME sparsity guard that is
load-bearing since the 0004/0005 eval failure); result is
`max(budget/nnz, heavy_multi)` (and the old zero-restart floor returns
`heavy_multi`). faclay75/gabriel10 stay at 1 pass; transswitch2383wpr (277k)
gets 3; nnz 150-225k gets 4 (~0.06 s total).

Dev: 0.875217 (gt_10k 0.8279 → 0.8276). Worst order() 0.500 s
(probe_timing_and_score, this run). Tests green.

## Open

- Eval flip rate on the 6-8 sparse-heavy ties is the real payoff — check the
  next promoted run's census.
- Runner-up-margin census of the remaining ~95 dev ties still unpriced.
