<!-- 0022 -->
# 0022 — TELOS top-K (K=2): the runner-up seed does NOT recover chp_shorttermplan2d (but finds a different, unclaimed basin on procurement1large)

- **Date:** 2026-08-29
- **Score:** unchanged 0.842223 (kill criterion fired at the probe stage; code REVERTED
  before any full run, per protocol)
- **Status:** negative result on the stated hypothesis / **reverted** — with one
  quantified positive lead worth its own candidate

## Hypothesis

0020 proved TELOS-GRADIENT DESCENT is ~18% seed-sensitive on `chp_shorttermplan2d`
(from the worse incumbent it reaches 1 975 321 flops, from the better only
2 104 445), and named the fix: descend from the top-K incumbents, keep the best.
This experiment implemented K=2: every accept site retains the **runner-up**
(best permutation distinct from the current best), and TELOS runs a second,
independent strict-improvement descent from it (gated `nnz <= 100_000`, keeping
`ringpack_30_2`/`crudeoil_lee4_09/10` at one descent per 0013), adopting the
second result only if it strictly beats the first — so no matrix can regress by
construction. Expected recovery: `chp_shorttermplan2d` 0.515 → ~0.484
(the −0.000138 class from 0019/0020).

**Pre-registered kill criterion (rank-1 protocol):** the descent from the
runner-up incumbent must reach ~0.484 on `chp_shorttermplan2d`; otherwise stop,
revert, record the negative.

## What changed (all REVERTED; diff preserved in this page's description)

`src/ordering/mod.rs`:
- `accept_top2(cand, f, &mut best_flops, &mut best_perm, &mut second_flops,
  &mut second_perm)` — the identical strict-`<` best-of update, extended to demote
  the old best into the runner-up slot and to accept `best < f < second` distinct
  candidates as the new runner-up. Wired into **every** accept site: `consider`,
  hill-climb, post-hoc PEEL, COUNT-RELABEL, and the SHOT-3 `try_cand` (per the
  rank-3 handoff warning that a consider-only runner-up would degenerate).
- TELOS phase restructured into a seed loop: descent 0 from `best_perm` exactly
  as shipped; descent 1 from `second_perm` when `nnz <= TELOS_TOPK_MAX_NNZ =
  100_000` and a runner-up exists. Each descent thresholds against **its own
  seed** (a worse seed must be free to move through territory above `best_flops`
  to reach its basin); the final incumbent is the strictly better of the two
  finals, ties favoring the best-seed descent.
- Test-only `TELOS_SEED_RESULTS` thread-local capturing `(entry, final)` flops
  per descent.

`src/ordering/probe.rs`: `probe_telos_topk` — runs the SHIPPED `order()` on a
named matrix list (default `chp_shorttermplan2d`), reports each seed's
entry/final ratio vs the AMD anchor plus final ratio and min-of-3 time.
~6 s per invocation, exactly as 0020's method note promised.

## Result — kill criterion FIRED

`chp_shorttermplan2d` (n=16 364, nnz=52 108), candidate build:

| seed | entry ratio | final ratio | final flops |
|---|---|---|---|
| best | 0.527855 | 0.526447 | 2 152 642 |
| runner-up | 0.535533 | **0.535533 (UNMOVED)** | 2 189 795 |
| final (after post-TELOS phases) | — | 0.514660 | = incumbent exactly |

The runner-up descent found **nothing at all** — not "less than 0.484", literally
zero accepted moves — and the final equals the shipped 0.514660. Target ~0.484
not reached. Reverted per protocol; 35/35 tests green on the reverted tree.

Diagnostic sweep on other TELOS-relevant matrices (same probe):

| matrix | best seed descent | runner-up descent | final (cand) | final (incumbent) |
|---|---|---|---|---|
| mpbp_48 | 0.509917 → unmoved | 0.509943 → unmoved | 0.506909 | (not measured) |
| chimera_lga-01 | 0.808284 → unmoved | 0.808507 → unmoved | 0.803396 | (not measured) |
| **procurement1large** | 0.634285 → unmoved | **0.636532 → 0.584669** | **0.581426** | **0.634285** |

## Why it lost

**Top-2-by-flops degenerates to a same-basin near-duplicate.** The runner-up on
`chp_shorttermplan2d` is 2 189 795 — *exactly* 0020's consider-pool best at 24
restarts. The chain is: consider's winner (2 189 795) got improved by the
post-hoc phases into the incumbent (≈2 158k at TELOS entry), demoting itself
into the runner-up slot. So the two TELOS seeds are consecutive points on ONE
improvement path — same basin by construction, and the second descent is a
strictly weaker rerun of the first. The same signature (runner-up within 0.05%
of best, both unmoved) appears on mpbp_48 and chimera_lga-01.

0020's second basin was seeded by the **best-of-5-restarts incumbent** — a
between-CONFIG seed from a truncated candidate pool. That seed is diverse
because it is the winner of a *different selection*, not because it is
second-best of this one. Flops-rank adjacency is the wrong diversity axis;
K=2-by-rank cannot capture what a K=1-at-lower-budget run reaches. The plan's
"non-negative transfer BY CONSTRUCTION" property held (nothing regressed), but
the construction selects the wrong second seed for the motivating matrix.

## The positive lead this opens (quantified, unclaimed)

On `procurement1large` the runner-up (0.636532, only 0.35% behind the best!)
descended **across a basin boundary** to 0.584669 while the best seed sat
unmoved at 0.634285 — final 0.581426 vs the incumbent's measured 0.634285, a
−0.0529 single-matrix ratio win in gt_10k. Back-of-envelope:
ln(0.5814/0.6343)/45 ⇒ gt_10k geomean ×0.99807 ⇒ weighted **≈ −0.0006** — larger
than this candidate's whole predicted range (−0.0001..−0.0003), from ONE matrix,
with zero possible score downside by the strict-keep construction. So the K=2
mechanism is NOT worthless — it is mis-targeted: it fails its named target and
wins somewhere nobody predicted. A future candidate could (a) rerun this exact
implementation as a "procurement1large-class win" with its own full-run +
timing protocol, or (b) pick the second seed by a real diversity axis (e.g.
the best candidate from a DIFFERENT family than the winner, or the
restart-prefix winner, per the 0020 mechanism) to also recover chp. Cost side,
measured min-of-3 with the double descent: chp 1.745 s (incumbent single-run
1.658 s same session), procurement1large 1.455 s (incumbent 1.217 s), mpbp_48
1.547 s — the second descent costs roughly one wasted TELOS round when the seed
is degenerate, ~0.1–0.25 s in the 40–90k nnz band; the >100k gate kept the cap
wall untouched.

## Follow-ups

- **Candidate for a future stage:** ship this exact K=2 implementation on the
  strength of procurement1large alone (full dev run + interleaved A/B timing
  required — this session never paid the full run per protocol), OR redesign the
  second-seed selection along a diversity axis before re-attempting the chp
  recovery. The full diff is small (one helper, five accept-site touches, TELOS
  seed loop, one gate constant) and reproducible from this page.
- The `TELOS_SEED_RESULTS`/`probe_telos_topk` instrumentation is cheap and
  worth resurrecting with any TELOS-seed experiment.

## Links

- Mechanism diagnosed: [0020](0020-telos-seed-sensitivity.md)
- Regression this tried to recover: [0019](0019-mid-band-restart-floor.md)
- Gate rationale (130k is not cheap): [0013](0013-gate-covariate-audit.md)
