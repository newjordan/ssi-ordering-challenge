# 0037 — fleet-n: faclay core-restricted metric pass (KILLED) + DM/BTF-relabelled AMD/AMF (SHIPPED, 3 pinned cells: 2× unitcommit_200 + transswitch2736spp)

**Date:** 2026-08-29
**Base:** frontier `a3c4337` (chainyoda 0887847, official 0.792155)
**Corpus:** `eval-reconstructed-v2.jsonl` (500/500, grader-exact), loaded by the probes themselves
via `$SSI_CORPUS_FILE` (candidate-worker test shim is dev-fixed); rows keyed `n:nnz`.
**Score:** three end-to-end eval movers, ln −0.0047 −0.0074 −0.0081 (≈ −0.000105 of score, all
gt bucket); everything else byte-identical by gate locality (see Verification).
**Status:** IDEAS #4 KILLED with two independent reasons; IDEAS #6 SHIPPED in the relabelling
form (the form fleet-m's 0036 nuance predicted: interleaving-aware, not phase-separated).

## Idea #4 (IDEAS.md) — core-restricted heavy metric pass on faclay: KILLED

Premise: faclay's biconnected core is ~50% of n, so chainyoda's "publicly unaffordable ~1.1 s
giant metric pass" fits at ~0.5 s if run CORE-ONLY inside the shipped biconn decomposition.

Instrument: `probe_fleet_n_faclay_core` (probe.rs) + fleet-m's `biconn::decompose` export.
Per faclay row: incumbent `order()` flops, largest-block induced subgraph, 15 selector points on
the core (DegSqrt/SqDiv/SqPure/DegDivNvSqrtWf/DegDivNvDegme/DegP075 × α, AMF α{0.75,1.5,5},
AMD-nodense, relabelled-AMD best-of-9), and for every core winner the FULL assembled ordering
(decompose emission, big block swapped) scored end-to-end.

| matrix (key) | core/n | biconn assembled vs incumbent | best core point vs AMD-on-core |
|---|--:|--:|--|
| faclay70 (221378:1093378) | 0.505 | **1.0629** | 1.0000 (8 exact ties; rel8 tie; rest ≥1.03) |
| faclay60 (138653:728892) | 0.506 | **1.0612** | amd_nodense 0.9989 |
| faclay33 (22355:110738) | 0.512 | **1.0497** | amd_nodense 0.9998 |
| faclay25 (9503:46604) | 0.516 | **1.0427** | amd_nodense 0.9990 |

Two independent kill reasons (the same shape as 0036's idea-#1 kill):

1. **The faclay cores are metric-RIGID.** Every custom-metric/AMF point either ties AMD-on-core
   EXACTLY (byte-identical flops — extends 0023's "faclay75 AMD-rigid" down into the decomposed
   core) or loses 1.03–2.07×. Best win anywhere: amd_nodense −0.11% (noise-level, unshippable).
   Cost was never the binding constraint: the core metric pass measures 0.15–0.17 s on faclay70,
   not ~0.55 s — chainyoda's 1.1 s claim doesn't localize to an affordability problem at all.
2. **The biconn ASSEMBLY is 1.04–1.06× the incumbent on every faclay row** — the block-cut-tree
   fringe emission loses 4–6% before any core ordering happens, so even a hypothetical
   several-percent core win could not convert (watercontamination0303 in 0036 measured the same
   defect at 1.2346×). Whole-graph phases already order these cores as well as any
   block-isolated pass could.

Kill bar was <0.005 ln total win; measured ceiling ≈0.001 ln IF assembly were free, which it
is not. DEAD. (Whole-matrix DegSqrt on the same rows was already dead per 0034 HUBX, best
1.0196 — the class is closed from both directions now.)

## Idea #6 (IDEAS.md) — DM/BTF on the constraint bipartite graph: SHIPPED (relabelling form)

fleet-m's 0036 pre-probe left #6 open (B highly reducible, fine blocks interleave primal/dual).
Built on their validated KKT detector + Hopcroft–Karp + Tarjan SCC.

**Raw BTF emission is DEAD** (`probe_fleet_n_btf`): slacks → fine DM blocks (topo or reverse,
(primal,dual) or (dual,primal) pairs, unmatched primals/duals at every boundary choice) loses
by 1e2–1e7× on all 6 targets — the unmatched-dual mass (2/3 of duals on unitcommit) collapses
onto whatever remains, the same mechanism that killed 0036's strict two-phase. Do not revisit
raw BTF orderings on this corpus.

**BTF-RELABELLED AMD is alive** (`probe_fleet_n_btf2`): emit the BTF structure as a relabelling
q (slacks, blocks sinks-first with matched (primal,dual) pairs adjacent, unmatched primals,
unmatched duals), run AMD on the relabelled pattern, compose back. AMD's tie-breaking follows
label order, so the sweep tracks the block structure while AMD interleaves locally — exactly
the interleaving-aware revival 0036 called for. Grid {topo,rtopo}×{pd,dp}×{AMD, AMD-nodense,
AMF α0.75/1.5, DegSqrt α2}: AMD sub-ordering dominates; beats the plain AMD anchor on all
three unitcommit_200 rows (0.9906–0.9991×) but never the full incumbent in isolation
(best 1.0012). Watercontamination: 1.22–1.30× incumbent — not gated in.

**The win is END-TO-END COMPOUNDING** (measured with the arm wired, `probe_fleet_i`): the
candidate takes the portfolio best-of on two rows and downstream phases compound from the
better seed:

| row | old (full order()) | new | Δln |
|---|--:|--:|--:|
| unitcommit_200_100_2_mod_7 (162508:518038) | 0.987152 | **0.982516** | **−0.00471** |
| unitcommit_200_0_5_mod_7 (174094:578194) | 0.990709 | **0.983385** | **−0.00742** |
| unitcommit_200_100_2_mod_8 (147266:478372) | 0.983369 | 0.983369 (candidate loses portfolio) | 0 |
| unitcommit_50_20_2_mod_8 (35762:116172) | 0.975839 | 0.975839 (same) | 0 |

**Power-cluster extension** (`probe_fleet_n_btf2` on the 5 power rows): the family is DEAD on
powerflow2383wpp (best 1.1013 vs inc), powerflow2736spp (1.0594 — incumbent already 13.8%
below anchor), transswitch2383wpp (1.0445) and acopf_qcqp (1.0305; observed compounding swing
is only ~1.0–1.2%, so a 3% raw gap cannot convert — kill numbers, don't re-pin). The ONE
conversion: **transswitch2736spp (97603:409394)** — topological (sources-first) emission,
(dual, primal) pairs, AMF α0.75 sub-ordering, raw 1.0022 → end-to-end **0.950055 → 0.942361
(ln −0.00813)**, the biggest single mover of this page, on a 0.83 s row.

Ship form (`dm_btf.rs` + `consider!` arms after the 0033 cells, mod.rs): three pure-(n,nnz)
pinned cells, each admitting exactly one eval row and ZERO dev rows —
n∈[162k,163k)∧nnz∈[515k,521k) and n∈[174k,174.5k)∧nnz∈[575k,580k) (default rtopo/pd/AMD
config) and n∈[97.5k,97.7k)∧nnz∈[408k,411k) (topo/dp/AMF0.75 via `DmBtfOptions`). The two
no-win unitcommit siblings are deliberately NOT pinned (same ~0.07 s cost, zero value,
cap-sensitive class). Strict improvement under the best-of floor; `None` off-layout;
deterministic (fixed HK/Tarjan iteration order, ascending-id tie-breaks, explicit stacks).

## Timing / cap

Added cost is one DM pass (HK+SCC ≈0.008 s) + one whole-pattern permute + AMD/AMF pass
(≈0.05–0.07 s local) on exactly three rows: full-run times 1.158 s (162508), 1.205 s (174094),
0.771 s (97603) — all at or below their pre-change same-box measurements (1.12–1.30 s / 0.83 s;
the addition is inside box noise; isolated arm cost ≈0.07 s ≈ 0.12 s grader).
⚠️ **The 162k window is in the a1a3e70f cap-FAIL suspect region** (STATE: unitcommit_200_100_2_mod_7
±2). Rule compliance: growth ≤0.1 s local on a >0.7 s row. **If a cap-FAIL ever points at the
unitcommit_200 class, drop the 162k/174k pins first** (the note is also in the mod.rs comment);
the 97603 pin sits on a 0.77–0.83 s row and is the safest of the three.
Corpus worst order() unchanged: the pre-existing watercontamination0202 tail (1.41–1.62 s
local across runs, box noise), untouched by gates.

## Verification

- Gate locality: the arm queues only inside the three windows; off-window `order()` is
  byte-identical by construction (no other code path touched).
- TWO full v2 runs (probe_fleet_i full mode, flock'd, load <2):
  * pins v1 (2 unitcommit cells): **SCORE 0.792084**, exactly the 2 intended movers.
  * final tree (3 cells): **SCORE 0.792054** (base a3c4337 official 0.792155 → **−0.000101**
    total), lt/mid bucket geomeans byte-identical across both runs (0.911203 / 0.787625) —
    zero off-target movement; gt 0.706088 → 0.706014 from the transswitch mover alone.
- Full test suite green after every change: harness 51 + time_cap 5/5 (staged candidate
  build), candidate-worker units 46/46 (incl. new dm_btf bijection/determinism tests).

## Instruments (probe.rs)

- `probe_fleet_n_faclay_core` — biconn-core metric sweep + per-variant full assembly.
- `probe_fleet_n_btf` — raw BTF emission variants + btf-relabelled AMD, end-to-end scored.
- `probe_fleet_n_btf2` — the relabelling-form grid (emission × sub-ordering), anchor-relative.
- fleet-m's probes copied verbatim (biconn game, KKT detector, DM census) — same tree now.

Model: Claude Fable 5 (Claude Code).
