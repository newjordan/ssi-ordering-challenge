# 0028 — Compose two disjoint same-round deltas: narrow PEP slack + RGSUB round 3, and the runner-up seed swap

**Date:** 2026-08-29
**Base:** accepted `f48a17f` (= submission 2b91c868, official 0.817268; dev 0.817933)
**Score:** dev 0.817933 → **0.817692** (−0.000241); official expectation below 0.8170
**Status:** submitted

## The idea

Two submissions graded in the same round on the same parent tree (`f48a17f`), each
improving the official score, and their diffs touch disjoint hunks of `order()`:

1. **19b52f11 (official 0.817018, −0.000250 vs parent)** — `mod.rs` + `rgreedy.rs`:
   - `RGREEDY_SLACK_MID_NARROW = 800M`: the deterministic rgreedy op floor is raised
     from 600M to 800M only inside the whole-matrix PEP envelope (`nnz <= 60k`) in the two
     density bands `nnz <= 2n` and `4n <= nnz <= 4.5n`. No phase gate widens.
   - RGSUB third outer round for `n >= 10k && nnz <= 100k` and for the hub-free sparse
     sub-10k class (`nnz <= 30k`, `max_deg*50 <= n`); the loop still breaks on
     non-improvement, so the round only runs when round 2 strictly improved.
   - Equal-work seed diversification in round 2: the D1 stream re-seeds, and stream 0
     re-seeds on alternating top-32 block ranks (`SubCfg.round`).
   (cc44570d, official 0.817021, is the same change without the stream-0 alternation.)
2. **bbebe613 (official 0.817113, −0.000155 vs parent)** — `mod.rs` only, "0022e rev13":
   the TELOS runner-up completion (`desc_alt`, kept alive past the descent phase) becomes
   a third multi-start seed slot, and its two descents (modes 7 and 2) REPLACE the last
   two planned descents instead of being appended — identical descent count, different
   seeds, cost-neutral by construction. Regime D and the dual-seed rgreedy (0022d) stay
   closed (both graded cap-FAILs in that lineage).

Composition hypothesis: (1) acts inside the rgreedy/RGSUB phases on the small and
sparse-mid classes; (2) acts inside the TELOS multi-start plan on the ≤55k-nnz extras
windows. Their wins should be close to additive: expected official ≈ 0.8166–0.8168.

## Measured (this box, 20-core aarch64, 4-thread carrier)

Full harness (`cargo run --release --offline --locked`), dev corpus, 300 matrices:

| tree | dev score | lt_1k | 1k_10k | gt_10k |
|---|--:|--:|--:|--:|
| base `f48a17f` | 0.817933 | 0.8923 | 0.8614 | 0.7296 |
| compose | 0.817692 | 0.8923 | 0.8610 | 0.7293 |

Per-matrix: 22 improved / 2 (lop97icx +0.0070 ln, popdynm200 +0.0021 ln) regressed vs base.

Timing (`probe_timing_and_score`, isolated, consecutive runs): base worst `order()`
1.066 s (crudeoil_lee4_10 0.924, chimera_selby-c16-01 0.949); compose worst 1.196 s under load, 0.782 s in a quiet back-to-back run (19b52f11 alone: 1.060 s in the same pair); per-matrix differences within box noise. Identity probe 300/300..

Grader tails of the two source runs (per-position wall time from the public Actions
log, includes ~0.15 s spawn/scoring overhead): 19b52f11 worst 1.96 s at hidden #149,
bbebe613 worst 2.00 s at #149, 1.97 s at #458. Both passed. The composition adds
bbebe613's cost-neutral swap to 19b52f11's tree, so the expected tail is 19b52f11's.

## Attribution / risk

- Every added unit of work in (1) is gated to classes that graded clean in the same round.
- (2) adds no descents. The only interaction is if the runner-up seed lands a different
  incumbent that changes which matrices the rgreedy slack floor reaches — measured on dev
  above, zero regressions expected.
- Cap: the corpus is saturated (grader p50 ≈ 1.0 s, p99 ≈ 1.8 s). Any further addition of
  work anywhere is a cap bet; this composition deliberately adds none beyond 19b52f11.
