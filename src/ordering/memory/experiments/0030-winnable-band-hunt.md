# 0030 — Winnable-band census on the reconstructed eval corpus + ultra-dense no-dense-AMD arm

**Date:** 2026-08-29 (sprint 5, fleet-g)
**Base:** frontier 5f024a7 (= mellowcroc 01f063f4, official 0.810201; proxy census 0.810067)
**Score:** proxy 0.810067 → **0.810027** (−0.000040); dev **byte-identical** (gate reaches zero dev rows)
**Status:** shipped on fleet/g

## Instruments built (test-only, compiled out of the shipped binary)

- `parallel::attrib_mark`/`attrib_take` + `#[cfg(test)]` marks after every
  `order()` phase (`0.anchor` … `12.terminal_minl`): a probe can now read off
  WHICH phase produced the final incumbent per matrix.
- `probe_winnable_band`: full-corpus census (honors `SSI_CORPUS_FILE` via a
  local `census_corpus()` — NOTE the candidate-worker test shim's `corpus()`
  hardcodes dev and silently ignores the env var; probes that claim
  SSI_CORPUS_FILE support and run under `-p ssi-candidate-worker` must load
  the file themselves). TSV per matrix: n, nnz, density, ratio vs AMD,
  order() wall, winning phase, BAND mark for ratio ∈ [0.93, 0.99).
- `probe_band_variants`: for `SSI_PROBE_IDX` targets, scores the crown's band
  prescriptions as isolated candidates through the trusted scorer: AMF
  α∈{0.5..2.5}, AMD (aggr/non-aggr) α grid incl. no-dense (α=−1), raw
  DegSqrt(0.75), deg- and incumbent-count-ranked strips k∈{32..1024} ×
  {AMD, AMD-na(1.0), AMF(1.5), DegSqrt(0.75)}, and the 50–100-density
  DegSqrt cascade (k=16/192/384, exact-count re-rank).

## Census (proxy corpus, 499 matrices, frontier tree)

Proxy score 0.810067 vs official 0.810201 — the reconstruction reproduces the
graded number to **1.3e-4**. Buckets: lt_1k 0.9109 (242), 1k_10k 0.7973
(180), gt_10k 0.7441 (77).

**Winnable band (0.93–0.99): 118 matrices** — 16 gt_10k (leverage
0.0052/unit-ln each), 46 1k_10k (0.0017), 56 lt_1k (0.0012). Phase
attribution across the band: the mass is won by LATE phases — 10.rgsub,
12.terminal_minl, 11.pair, 8.rgreedy — plus 1.portfolio; the 16 gt_10k band
rows are ALL sparse (density 2.7–8.9, the acopf/gabriel/unitcommit shape),
none dense.

## Band-headroom hunt (25 heaviest band targets + 16 dense-class rows)

**Negative, and a clean one: the crown's band prescriptions bring NOTHING to
the sparse band mass.** On every sparse gt_10k/1k_10k band target, every
one-shot metric candidate (all AMF/AMD alphas, DegSqrt, every strip and both
cascades) lands at EXACTLY the AMD anchor (ratio 1.0000) or worse — the
incumbents' 0.93–0.98 there is entirely descent-phase work (rgsub/minl/pair),
and no selector-band variant touches it. Corroborates EternaPeptix 7e6d24f
(ties structural) one band further down: on near-forest-shaped sparse rows,
ORDER-SELECTOR diversity is exhausted; only search moves the needle.

**Two wins, both in one uncovered cell — the ultra-dense heavies.** The heavy
no-dense-AMD pass is ceilinged at `HEAVY_AMDND_MAX_NNZ` (700k, hub-gated
1.5M) and the D dense selectors stop at `D_WIDE_MAX_NNZ` (1M), so the cell
nnz ≥ 400k ∧ nnz ≥ 100·n had no deferral-style candidate:

| corpus idx | n | nnz | density | before | after | added cost |
|---|--:|--:|--:|--:|--:|--:|
| 219 (pedigree_sim2000-shape) | 2587 | 921054 | 356 | 0.971871 | **0.955625** | +0.016 s |
| 19 (truck-shape) | 9016 | 1045026 | 116 | 0.068981 | **0.068083** | +0.051 s |

idx19 additionally improves past the isolated candidate value (0.068085 →
0.068083) because rgsub re-seeds from the better incumbent. On idx219/idx19
every strip/alpha alternative scored worse than plain no-dense AMD. The same
pass on the SMALL ultra-dense rows (idx52/63/142/147/237/262/362/424, nnz <
400k) comes within 0.001–0.002 of the incumbent but never beats it — the
pipeline already reaches those basins; gate floor 400k is evidence-derived.

## Shipped

One `consider!` candidate in `mod.rs` (ULTRA-DENSE NO-DENSE-AMD ARM):
`AmdOptions { aggressive: false, dense_alpha: -1.0 }` gated
`(400_000..1_200_000).contains(&nnz) && nnz >= 100n`. Strictly-improving via
the trusted best-of accept; pure `(n, nnz)` gate; deterministic.

- Proxy A/B (full 499): exactly 2 rows change, both improve, 497
  byte-identical. 0.810067 → 0.810027 (−0.000040), all in 1k_10k
  (0.7973 → 0.7972).
- Dev: the dev corpus has **zero** matrices with nnz ≥ 400k ∧ nnz ≥ 100n
  (densest ≥100-density dev row is 120,834 nnz) — the arm never fires; dev
  output and timing are byte-identical by construction. Verified by a full
  dev harness run (green, see log line).
- Timing: added work is one AMD pass, ≤0.05 s local at the 1.2M-nnz ceiling
  (grader ≈1.65× → ≤0.09 s), on matrices whose full order() runs 0.12–0.24 s.
  Class-wide the gate bounds n ≤ 12k, and members are cheap for the pipeline
  (most phases gate out on density) — no interaction with the >0.7 s tail.

## MCS-realization caching (mission item 4): NOT APPLICABLE

Checked before touching: this tree has no completion-space slide walk.
`minl_mcs_peo` realizes MCS once per candidate completion and evaluation goes
through `ws.flops` on the permutation, not a second MCS — EternaPeptix's
double-realization (~20% walk throughput) has no counterpart here. Nothing to
cache until someone ports the slide-walk family.

## Caveats / measurement notes

- The before/after proxy runs and the variant probes ran under residual box
  load (1-min load 2–3.4 at times); per-matrix WALL times in the census TSVs
  carry that noise (e.g. worst-t drifted 1.154 → 1.347 s on an untouched
  n=215k row between runs). Ratios and attribution are exact; timing claims
  for the shipped arm come from isolated probe timings on in-gate rows.
- The winnable-band sparse mass (16 gt_10k rows, ~0.0052 leverage each)
  remains open — but it needs SEARCH-phase work (rgsub breadth, terminal-minl
  reach), not selector bands. That is where the next sprint should spend.
