# 0009 — Heavy separators: decisive NO. HC gate 60k → 165k: WIN.

**Result: dev 0.875036 → 0.874894** (hc-gate extension; the separator half is
a negative that closes a research line).

## Part 1 — separator family on the sparse heavy tier: dead

Hypothesis: the remaining sparse-heavy eval ties (unitcommit, optcdeg2,
dtoc5, powerflow2736spp, faclay60/70) are time-expanded structured patterns —
the textbook nested-dissection class — and METIS is gated to n<130k/nnz<320k.
`probe_heavy_separators` (isolated, min-of-2) on every sparse dev matrix
above the METIS gate:

| matrix | metis ratio (cost) | scotch ratio (cost) |
|---|---|---|
| unitcommit_200_100_1_mod_8 | 1.82 (0.18s) | 2.02 (0.23s) |
| transswitch2736spr | 2.56 (0.16s) | 2.82 (0.21s) |
| cont6-qq | 1.39 (0.30s) | 1.36 (0.42s) |
| gabriel10 | 4.49 (0.83s) | 3.30 (0.74s) |
| acopf_case9241pegase_qcqp | 2.93 (0.84s) | 4.08 (1.06s) |
| faclay75 | 2.23 (**5.3s**) | **9519** (3.1s) |

Uniformly 1.4-4.5× WORSE than AMD (Scotch catastrophically so on faclay75),
and expensive. The existing METIS gates were protecting real hazards. **Do
not build a heavy separator candidate for this corpus** — these KKT patterns
are minimum-degree-friendly at scale. If anything moves the structured heavy
ties it is NOT off-the-shelf ND.

## Part 2 — HC_MAX_NNZ was another unmeasured gate: 60k → 165k

`probe_hc_extend` on the (60k, 165k] window at the shipped budget (1-5
passes): phase cost ≤ 0.033 s per matrix, and crudeoil_pooling_dt3 falls
0.972 → **0.954** at ONE pass (plus nd_netgen-3000 0.9863→0.9858). Ties
nuclear10a / methanol400 / popdynm200 / faclay35 / gasprod_sarawak81 resist.
Shipped: `HC_MAX_NNZ = 165_000`. Dev 0.874894, worst order() 0.863 s,
tests green. Eval upside: the 60-165k window is well-populated there.

## Meta

Third and fourth data points for the gate-audit rule (0007, 0008): of four
pre-isolation gates audited, two hid wins (relabel budget, amdnd ceiling, now
hc gate) and one was correct for reasons the sizing never knew (METIS).
Remaining unaudited: AMF envelope, Scotch/KaHIP windows, METIS_TUNED/HITRIAL.
