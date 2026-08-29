# 0032 — compose of 19b52f11 + bbebe613 + a32d51f5 + 72ec4464 on f48a17f, plus round-keyed RGSUB seeds (0030)

**Date:** 2026-08-29  **Base:** f48a17f  **Status:** submitted

## Components (all on `f48a17f`, all disjoint hunks, applied verbatim)

1. **19b52f11 (official 0.817018)** — `RGREEDY_SLACK_MID_NARROW = 800M` inside the PEP envelope
   (`nnz <= 60k`) in the density bands `nnz <= 2n` and `4n <= nnz <= 4.5n`; RGSUB third outer round
   for `n >= 10k && nnz <= 100k` and the hub-free sparse sub-10k class (loop still breaks on
   non-improvement); round-2 seed diversification (D1 stream re-seeded, stream 0 on alternating
   top-32 ranks). `SubCfg.round` added.
2. **bbebe613 (official 0.817113)** — the TELOS runner-up completion (`desc_alt`) becomes a third
   multi-start seed slot; its two descents (modes 7, 2) REPLACE the last two planned descents —
   identical descent count, cost-neutral by construction. Regime D and 0022d stay closed.
3. **a32d51f5 (official 0.817208)** — terminal block: in the dense low-alpha window
   (`400k <= nnz < 1M`, `nnz > 20n`) four deterministic relabelings of each AMF basin (α 0.75, 1.25),
   eight tasks in two four-worker waves, accepted through `parallel::accept` against the completed
   pipeline. Dev-neutral (no dev matrix in the window).
4. **72ec4464 (official 0.815285, −0.001983)** — in the accepted D_WIDE dense arm, above the
   D_WIDE density ceiling, the AMD α0.75 candidate is replaced one-for-one by a low-alpha DegSqrt
   quotient pass (`custom_metrics::order_variant(core, 0.75, true, DegSqrt)`): same candidate
   count, same timing envelope, materially more diverse. Dev-neutral (zero per-matrix changes on
   the 300 dev matrices), hidden −0.24 %.

5. **0030 (ours, equal work)** — in `rgreedy::subtree_refine`, outer RGSUB rounds that would
   otherwise re-run a byte-for-byte identical seed on an unchanged block get a round-keyed seed:
   below 10k every round >= 1 (both streams) XORs a constant × (2·round+1); at/above 10k the same
   for round >= 2 (round 1 already carries 19b52f11's diversification). No trajectories, ops or
   blocks are added; it is the same mechanism as 19b52f11's round-2 D1 re-seed, extended to the
   remaining repeated rounds. Dev: 16 matrices improve, 9 regress; 0.817692 → 0.817636
   (−0.000056; 1k_10k 0.8610 → 0.8608, lt_1k 0.8923 → 0.8922, gt_10k unchanged). Largest movers:
   chimera_selby-c16-01 0.591 → 0.582, rsyn0830m04m 0.896 → 0.891, rsyn0840m02m 0.963 → 0.959,
   crudeoil_pooling_ct3 0.636 → 0.642 (loss), crudeoil_lee1_07 0.776 → 0.780 (loss).

## Measured on this box (20-core aarch64, 4-thread carrier)

Full harness (`local-candidate-build.sh && cargo run --release --offline --locked`), 300 dev matrices:

| tree | dev | lt_1k | 1k_10k | gt_10k |
|---|--:|--:|--:|--:|
| base `f48a17f` | 0.817933 | 0.8923 | 0.8614 | 0.7296 |
| 1 + 2 (graded 0.816862) | 0.817692 | 0.8923 | 0.8610 | 0.7293 |
| 1 + 2 + 3 | 0.817692 | — | — | — |
| 1 + 2 + 3 + 4 | 0.817692 (zero per-matrix changes) | — | — | — |
| 1 + 2 + 3 + 0030 | 0.817636 | 0.8922 | 0.8608 | 0.7293 |

Components 3 and 4 act only inside dense heavy-tier windows that the dev corpus does not populate;
component 5 acts only in RGSUB. The three code paths are disjoint, so the submitted tree
(1+2+3+4+5) is expected to reproduce 0.817636 on dev and, on the hidden corpus,
0.817268 − 0.000250 − 0.000155 − 0.000060 − 0.001983 ≈ **0.81482** before 0030
(additivity has held to 1e-5 on every graded pair this round), with 0030's dev delta on top.

Determinism: `probe_parallel_identity` 300/300 identical on the 1+2 tree (corpus_hash
6af4a61b5cca81a2); 3, 4 and 5 are deterministic by construction (fixed seeds, fixed task order,
earliest-strict-argmin acceptance).

## Timing (the cap)

Grader tails read from the public Actions logs (gap between consecutive worker kills, includes
≈0.15 s spawn/scoring overhead):

| run | p50 | p99 | worst |
|---|--:|--:|--:|
| 19b52f11 | 0.97 s | 1.80 s | 1.96 s (#149) |
| bbebe613 | 1.02 s | 1.82 s | 2.00 s (#149) |
| a32d51f5 | — | — | 1.97 s (#149), 1.96 s (#458) |
| our 1e1acf12 (1+2) | 0.80 s | 1.42 s | 1.56 s (#268) — a faster runner instance |

Local `probe_timing_and_score`, quiet box: 1+2 worst 0.782 s; 1+2+3 worst 0.888 s (small-matrix
noise, ±0.3 s run-to-run on this host). Component 4 is a one-for-one candidate swap; component 5
adds no operations. The submitted tree therefore carries the same hidden envelope as 72ec4464
plus the measured-clean additions of 1–3.

## Risk

If this fails the cap, the attribution order is: 3 (adds eight AMF tasks on the dense heavy
class) → 1 (RGSUB round 3 / slack floor) → nothing else adds work. If the score lands above
the additive prediction, 0030's seed change is the only unmeasured-on-hidden component.
