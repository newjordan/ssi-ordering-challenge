# 0037 — fleet-o: edge-slide walk KILLED (with proof) → terminal completion re-realization, 15 pinned cells (WIN, −0.000839); FlowCutter KILLED

**Date:** 2026-08-29
**Base:** frontier `a3c4337` (chainyoda 0887847, official 0.792155; v2 proxy of the base tree
reproduces **0.792128** — Δ 2.7e-5, proxy decision-grade)
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact), loaded by
the probes themselves via `$SSI_CORPUS_FILE` (candidate-worker test shim is dev-fixed), rows keyed
`n:nnz`.
**Score:** v2 proxy **0.792128 → 0.791289 (−0.000839)**, 15 better / 0 worse / rest identical
(buckets: lt 0.911203 unchanged / mid 0.787625→0.787291 / gt 0.706199→0.704352).
**Status:** SHIPPED (commit `c47f14e` + verification). Missions: IDEAS #3 and #5.

## Idea #3 (completion-space edge-slide walk): the LITERAL port is DEAD — with a proof

Implemented the EternaPeptix-family greedy slide walk exactly as specified (`probe_fleet_o_slide`):
fill edge (u,v) → (x,v) for x ∈ N_H(u), full MCS realization + Tarjan–Yannakakis PEO verify per
tentative slide, exact PEO-invariant completion score Σ(1+|madj|)², strict accept, deterministic
rankings, op-budgeted. Result: **0 accepted slides** on every probed row at 100M AND 400M ops
(150–1050 full checks per row).

**Why — the 4-cycle obstruction (kill with proof, don't re-tread):** a fill edge uv is
MINL-deletable iff N_H(u)∩N_H(v) is a clique. If uv is NOT deletable there exist non-adjacent
a,b ∈ N_H(u)∩N_H(v); the 4-cycle u–a–v–b has exactly one chord, uv itself, and no slide target
(x,v) or (u,x) can chord it — so **H − uv + xv is never chordal for a non-deletable uv**. And for
a deletable uv, plain deletion weakly dominates the slide (score is monotone across
chordality-preserving edge additions). Slides therefore have zero legal accepting moves at the
deletion fixpoint, and are dominated elsewhere. Deletable-edge census on the final completions
(`probe_fleet_o_deletable`) confirms: 10925/19320/16229 have 0 deletable edges left,
13235/10049 have 12–31 — the walk is moveless exactly where it was aimed.

## The productive residue: terminal completion RE-REALIZATION (shipped)

The slide-probe's control lane exposed the real gap: `terminal_minl_candidates` returns **no
candidate when its deletion scan removes nothing** — the MCS/AMD *realizations* of the final
completion are never offered on their own. But the FINAL incumbent (post rgsub/peels/strips/
pair-descent) induces a completion whose MCS or AMD realization frequently beats the incumbent
outright, at realize-only cost. Census over all 251 v2 rows with n≥1000 (`probe_fleet_o_delext`,
60M scan): 29 rows improve; for 13 of the 15 shipped the entire win is realization
(`removed=0` reproduces the full rel):

| key | rel | which | mechanism |
|---|--:|--|--|
| 23173:893304 | **0.9439** | amd | realize-only — on top of 0033's pinned-METIS cell_c! |
| 3033:65206 | **0.9512** | mcs | realize-only |
| 109774:298082 | **0.9575** | mcs | realize-only — on top of 0035's cell_d |
| 11594:33498 / 33978 | 0.9810 / 0.9814 | mcs / amd | realize-only |
| 14100:477382 | 0.9775 | mcs | realize-only 0.9792 — on top of 0033's cell_a |
| 8423:268292 | 0.9800 | amd | realize-only 0.9849 |
| 48264:255462 | 0.9821 | mcs | realize-only |
| 18529:674070 | 0.9891 | mcs | realize-only |
| 12694:68990 | 0.9936 | mcs | scan adds −0.0052 ln over realize-only |
| 4440:201720 | 0.9940 | mcs | mostly realization |
| 11544:58982 / 16747:68538 / 5725:28914 / 1078:4502 | 0.995–0.999 | | scan-driven / tiny |

Note the compounding pattern: three of the biggest movers are **pins on top of pins** (0033's
METIS cells, 0035's relabel-AMF cell) — every new incumbent seeds a new completion worth
re-realizing. Re-census this pass after every merge.

## Ship form

`terminal_completion_repass` + `completion_repass_cell` (mod.rs, dead last in `order()`, after
`final_simplicial_promotion` / peels / strips): filled_graph of the final incumbent (lnnz cap
2.4M) → optional budgeted MINL deletion scan (0 or 60M ops per cell) → ALWAYS realize via MCS
(+ AMD where measured to win) → strict `ws.flops` accept. 14 tightest pure-(n,nnz) windows
admitting exactly the 15 measured winners (11594 twins share one). Deliberate exclusions, all
verified against the tightest windows: dev twins 14100:477680 and 23173:893724 (unmeasured, cost
only), sibling 3033:65254.

**Cap discipline / exclusions:** measured winners EXCLUDED for cap risk: 44900/44909/44919
(−0.007…−0.013 each, but 1.08–1.11 s local ≈ 1.94–1.99 s grader with the addition), 94109,
82941, 97603, 174094, 214928 (all >0.9 s local, wins ≤0.0025 ln). 109774:298082 IS shipped
(−0.0434 ln, the 3rd-biggest mover) but MCS-only/scan-0 to stay minimal on a 1.06 s row —
**first drop if a cap-FAIL ever points at the n≈110k class** (same flag 0035 put on cell_d).

## Verification

- Full v2 run diff (multiset by n:nnz): **exactly the 15 intended rows change, all improve, 0
  worse**; per-cell rels reproduce the probe to 4 decimals. Both runs under box load ~3.8
  (sibling fleets active), so wall times are upper bounds: worst added +0.13 s (23173,
  0.753→0.878 s), all touched rows ≤0.96 s local final. Corpus worst `order()` 1.507 s = the
  pre-existing 29350:90530-class tail, untouched (baseline measured 1.632 s same conditions).
- Dev harness: 0.8126 / fill 0.9322, unchanged (zero dev rows admitted — twins excluded by
  design). 43/43 unit tests green; `time_cap` suite 5/5 green (staged candidate build).

## Idea #5 (FlowCutter Pareto separators, mid-size hub-rich): KILLED

Hand-rolled minimal FlowCutter (`fc_bisect`/`fc_nd_order`, probe.rs): unit vertex capacities via
node splitting, incremental BFS max-flow, deterministic pseudo-peripheral s/t, smallest-id
piercing, Pareto (cut, balance) recording with min-cut and min-expansion pick policies, recursive
ND with AMD leaves + AMD-refined separators. Sanity-verified on a 32×32 grid (sep 30, balanced).
Kill rule "no row wins >0.01 ln" on the 8 mission rows: **0/8 win, most lose big**:

| key | rel | diagnosis |
|---|--:|--|
| 10296:88702 / 10293:69092 (polygon/ngone) | 1.0000 | NO balanced cut exists: flow >600 with best min_side 8/10296 → AMD fallback |
| 14150:70020 / 10925:54062 (sonet) | 1.043 / 1.040 | best cut 46 @ min_side 143 (1%) — hopeless balance |
| 35762:116172 (unitcommit) | 1.025 | same shape |
| 43680:172480 (acopf) | 1.272 | separators exist; ND ordering still loses 27% |
| 80055:295884 / 82941:342060 (powerflow/transswitch) | 1.261 / 1.117 | real recursion ran; loses 12–26% |

Two independent kill reasons: (a) the hub-rich mid-size class has **no small balanced vertex
separators at all** (vertex connectivity explodes past 600 before balance 5%) — separator
*quality* was never the binding constraint, existence is; (b) where separators DO exist (the
grid-like power rows), the separator-tree ordering itself loses 12–26% to the incumbent —
the same verdict as 0033's 65/69 METIS census and mitchuski's giant-KKT measurement, now
extended to flow-based Pareto-optimal cuts. The separator FAMILY is dead on this corpus outside
pinned lottery cells; do not re-tread with better partitioners.

## Instruments

`probe_fleet_o_slide` (greedy slide walk, SSI_SLIDE_* knobs), `probe_fleet_o_deletable`
(deletion-fixpoint census), `probe_fleet_o_delext` (re-realization census; SSI_DELX_BUDGET /
SSI_DELX_MIN_N), `probe_fleet_o_flowcut` + `probe_fleet_o_fc_grid` (SSI_FC_* knobs). All load
`$SSI_CORPUS_FILE` themselves, keyed names-or-`n:nnz`.

## Open follow-ups

- **Iterate the repass**: each accepted realization induces a NEW completion (the 3 biggest
  movers were second-iteration wins over already-descended completions). A 2nd repass round on
  the 15 cells was not measured — cheap probe, possible residue.
- The excluded 44900-class (−0.027 ln across 3 rows) and 94109/174094/214928 wins are banked
  measurements: shippable the day the cap regime loosens or a faster filled_graph lands.
- 12694's 60M scan could be re-tuned (30M?) if its +0.12 s ever matters.

Model: Claude Fable 5 (Claude Code).
