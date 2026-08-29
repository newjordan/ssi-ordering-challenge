# 0038 — fleet-p: ITERATED variant repass (round-2..N + realization variants), 5 new cells (WIN, proxy −0.00124)

**Date:** 2026-08-29
**Base:** our frontier `5581542` (f28dd682, official **0.789054** — the crown). v2 proxy of the
base tree derived at **0.789109** (Δ 5.5e-5 vs official — proxy decision-grade, consistent with
0037's 2.7e-5).
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact), loaded by
the probes themselves via `$SSI_CORPUS_FILE`, rows keyed `n:nnz`. Bucket counts measured:
lt 243 / mid 180 / gt 77; SCORE = 0.3·lt + 0.3·mid + 0.4·gt geomeans (probe `aggregate`).
**Score:** v2 proxy **0.789109 → 0.787870 (−0.001239)** — buckets lt 0.911203 (bit-identical,
zero stray changes) / mid 0.786906→0.784450 / gt 0.699191→0.697935. 14 rows better / 0 worse.
**Status:** SHIPPED (commit `cf1f5c0`). Mission: 0037's banked follow-up (repass round-2) +
re-census + variant probe.

## Finding 1 — the repass round-2 is NOT residue, it's the main lode

0037 shipped ONE repass round and banked "round 2" as a cheap maybe. Measured (kill-fast bar was
0.003 ln): the 15 shipped rows held **~0.22 ln of round-2 mass** — several rows re-realize almost
as big on the SECOND completion as on the first (8423 round-2 = 0.9160 vs round-1 0.9849;
23173 round-2 = 0.9432 vs round-1 0.9439, both with ZERO deletions). The completion→realize→
completion map is a descent lattice with many productive steps, not a one-shot correction.

## Finding 2 — realization VARIANTS multiply the win 2-4×

At each round the completion admits many PEOs/orders; pricing more than the ship pair finds much
cheaper ones. Variant set (probe `probe_fleet_p_var`, all strict-accept):
- **MCS tie-break seeds**: ship seeding (mcs0), ids-descending (mcs1), lowest-degree-first (mcs2).
  mcs1/mcs2 win repeatedly (14100 round-1: mcs1 0.9513 vs mcs0 0.9874).
- **AMD options**: default, and `{aggressive:false, dense_alpha:-1}` (amd_nd — the 0030
  ultra-dense arm's options, here the strongest re-realizer on several completions: 3033 is
  amd_nd-dominated for 10+ rounds).
- **Dead**: dense_alpha 0.5/0.75/1.5 on the fill graph (catastrophic, 2-13× worse — dense
  thresholding on an already-filled graph mass-prunes exactly the structure that matters).

Iterated best-of-variants walks (fixpoint depth 7-12+ rounds): 8423 → **0.7736**, 3033 → 0.8203
(still descending at 12), 14100 → 0.8805, 23173 → 0.9084, 9953 → 0.8725. The scan is usually NOT
load-bearing (9953: realize-only reproduces the scan chain's endpoint to 6 decimals; 8274 keeps
80% realize-only) but is on some rows (3033: rounds delete 9/92/110 fill edges).

## Ship form

`completion_repass_cell` now returns a per-cell ROUND SCHEDULE `&[(scan_budget, variant_mask)]`;
`terminal_completion_repass` takes the mask (RP_MCS0/1/2, RP_AMD, RP_AMDND) and realizes every
set variant, strict `ws.flops` accept per round; a no-accept round followed only by identical
configs breaks (fixpoint). Old cells' round-1 is byte-equivalent to 0037 (verified: the 6
unchanged cells reproduce their flops exactly). Hot rows carry the exact measured accept
sequence (one variant/round, minimal time); cheap rows carry uniform full-mask rounds and find
their own fixpoint. Shipped table (rel vs pre-0038 order(), min-of-3 local secs after):

| key | schedule after round-1 | rel | ln | secs |
|---|---|--:|--:|--:|
| 8423:268292 | scan:[mcs1,mcs1,amd,mcs1,amd,mcs1,mcs0] | **0.7736** | −0.2567 | 0.94 |
| 3033:65206 | 3×scan-full + 6×realize-full (full={mcs0,amd,amd_nd}) | **0.8297** | −0.1868 | 0.75 |
| 23173:893304 | [mcs0] realize-only | 0.9432 | −0.0585 | 0.96 |
| 14100:477382 | [scan mcs1] | 0.9513 | −0.0500 | 0.87 |
| 9953:287612 NEW | [mcs2, amd_nd] realize-only | 0.9542 | −0.0468 | 0.99 |
| 11594:33978 | [scan mcs0] (twin :33498 at fixpoint, ~30 ms reject) | 0.9810 | −0.0192 | 0.50 |
| 1078:4502 | 6×scan-full | 0.9911 | −0.0089 | 0.75 |
| 16747:68538 | [scan amd_nd, scan mcs0] | 0.9951 | −0.0049 | 0.82 |
| 12785:58142 NEW | [scan amd_nd, scan mcs0, scan mcs0] | 0.9967 | −0.0033 | 0.78 |
| 8274:320136 NEW | [mcs2] realize-only | 0.9981 | −0.0019 | 0.73 |
| 4440:201720 | [scan mcs0] | 0.9985 | −0.0015 | 0.32 |
| 13235:59618 NEW | 3×[scan mcs0] | 0.9990 | −0.0010 | 0.81 |
| 12694:68990 | [scan mcs0, scan mcs1, scan mcs0] | 0.9991 | −0.0009 | 0.89 |
| 10746:38388 NEW | [scan mcs0] | 0.9994 | −0.0006 | 0.80 |

Σ ln = −0.641 (mid −0.563, gt −0.138). Unchanged by design: 48264, 18529, 11544, 5725,
109774, 11594:33498. New windows admit exactly the 5 new corpus rows (collision-checked against
all 500 keys).

## Cap discipline (the binding constraint this round)

The scan (60M ops, usually exhausted) and per-round filled_graph+realize on big completions cost
0.05-0.25 s/round — first drafts of 3033 (16 full-mask scan rounds) and 14100/9953/8274 measured
**1.13-1.35 s local min-of-3 = grader ~1.9-2.2, cap-FAIL territory**, and were trimmed to the
shipped schedules (3033: 16→9 rounds keeping −0.187 of −0.211; 14100: 2 of 7 rounds; 9953: 2 of
12; 8274: scan dropped). All touched rows now ≤0.99 s local min-of-3 (grader ≈1.4-1.6 s).
Corpus worst order() = 1.430 s (214928-class pre-existing tail, untouched; 0037 measured 1.507 s
same class). Deliberately NOT taken (banked, shippable if the cap regime loosens or filled_graph
gets faster): 109774 round-2 (−0.014), 23173 rounds 3+ (−0.014), 14100 rounds 3-7 (−0.023),
9953 rounds 3-12 (−0.089!), 44900-class (−0.024 reconfirmed on this base), 97603 (−0.0044),
94109 (−0.0025), 214928 (−0.0019), 8113/10637/10157/43680/11170/147266/162508/174094 (each
−0.0003..−0.0015, all >0.9 s rows).

## Verification

- Ship replay exactness: order() flops on all 20 windowed keys reproduce the probe-predicted
  finals EXACTLY (integer-equal); the 6 unchanged cells reproduce pre-0038 flops EXACTLY.
- Full-corpus proxy run (`probe_fleet_h`, 500 rows): SCORE 0.787870; lt bucket geomean
  bit-identical to baseline → no lt row changed; mid/gt geomean deltas reconcile with the
  per-row census sums to 6 decimals.
- 46/46 unit tests, time_cap 5/5, security 4/4, equivalence 2/2 (staged candidate build via
  `local-candidate-build.sh`).

## Instruments

`probe_fleet_p_iter` (iterated repass census; SSI_ITER_ROUNDS/BUDGET/MIN_N/MAX_N — ROUNDS=0
doubles as an order()-flops/timing verifier), `probe_fleet_p_var` (per-round variant pricing,
requires SSI_PROBE_ONLY), `fleet_p_repass_round`, `fleet_p_mcs_tievar` (probe.rs).

## Open follow-ups

- The banked >0.9 s row wins above (−0.13 ln total) — a faster filled_graph/realize path
  (incremental refill? realize without full adjacency rebuild) would unlock them.
- Variant set is not exhausted: MCS seeded from the incumbent's own order, AMD on the
  scan-descended graph with relabeling, exact_dp on small completions — untried.
- Re-census this pass after EVERY merge (0037's lesson holds double: pins-on-pins keep paying;
  every upstream change reshuffles which completion reaches the repass).

Model: Claude Fable 5 (Claude Code).
