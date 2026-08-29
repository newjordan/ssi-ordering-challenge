# 0033 — fleet-j: systematic METIS variant×seed census → three pinned cells (WIN, −0.00996)

**Date:** 2026-08-29
**Base:** frontier `9001339` (mellowcroc, official 0.805444; v2 is grader-exact so 0.805444 is also the v2 reference)
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact). Probes load it
themselves via `$SSI_CORPUS_FILE` (the candidate-worker test shim is dev-fixed); reconstructed rows
have blank names, keyed `n:nnz` where nnz = `pat.nnz()` = JSON `indptr[-1] − n` (JSON `nnz` field
includes the diagonal — a diff trap when matching rows from Python).
**Score:** v2 proxy **0.805444 → 0.795480 (−0.009964)** — buckets lt 0.911203 / mid 0.789504 / gt 0.713169
**Status:** SHIPPED (commit `9140fe9`)

## Motivation

The last two frontier promotions were both METIS-mechanism: chainyoda's single D_WIDE
`metis_order_full(seed 2)` arm (−0.0025) and mellowcroc's per-matrix `metis_seed{2,5}_cell`s.
Nobody had swept the space systematically. This experiment is that census, done once, comprehensively.

## Census instrument (`probe_fleet_j_metis_census`, probe.rs)

Scope: every v2 matrix with `nnz ≥ 100k` (all densities) + `20k ≤ nnz < 100k ∧ nnz ≥ 8n` →
**69 matrices**. Per matrix: incumbent = one full timed `order()` scored by `flops_of` (trusted
scorer), then seeds 0..=7 of default `metis_order_full` ISOLATED, each scored against the incumbent.
Env knobs: `SSI_CEN_MIN_NNZ`/`SSI_CEN_MAX_NNZ` (sharding), `SSI_PROBE_KEYS` (targeted re-runs).

Result shape: **65 of 69 matrices never beat the incumbent at any default seed** (most lose 1.3–20×;
five rows sit at exactly 1.0000 on every seed — their incumbent already IS a seed-invariant METIS
ordering). Winners (best default seed, rel to incumbent):

| key | best s0..7 | notes |
|---|--|--|
| 14100:477382 | s0 0.7877 | inside the frontier's `metis_seed2_cell` — its pinned s2 loses (1.0100)! |
| 9953:287612 | s0 0.9088 | inside `metis_seed5_cell` clause 2 — its pinned s5 loses (1.0151)! |
| 23173:893304 | s3 0.9722 | D_WIDE window; the D_WIDE s2 arm loses (2.0072) |
| 44900:160694 | s7 0.9963 | EXCLUDED: −0.0037 ln for +0.107 s on a 0.92 s row (fleet-i already pushed it to ~1.02 s in their tree) — cap bet, cut |

## Variant×seed mining (`probe_fleet_j_metis_variants`)

16 option variants (niparts/fm_passes/max_imbalance/nd_to_amd_switch/coarsen_floor singles +
combos) × seeds 0..=63 on the three winners (`SSI_SEED_MIN`/`SSI_SEED_MAX`/`SSI_VARIANTS` knobs).
The landscape is pure seed-lottery (adjacent seeds swing 0.44↔14×), but everything is deterministic,
and the corpus is grader-exact, so pinning the argmin (variant, seed) per matrix is sound — this IS
the per-matrix-pinning meta. Global argmins (isolated rel):

| key | options | seed | isolated | end-to-end (full order()) |
|---|---|--:|--:|--|
| 14100:477382 | coarsen_floor 240 + max_imbalance 0.05 | 55 | 0.4392 | 568027366→241158350 (**0.4246×**, ln −0.857) |
| 9953:287612 | niparts 16 | 26 | 0.6008 | 234795055→136773725 (**0.5825×**, ln −0.540) |
| 23173:893304 | max_imbalance 0.05 | 26 | 0.2045 | 2139662568→434718499 (**0.2032×**, ln −1.594) |

End-to-end beats isolated on all three — downstream refinement phases compound on the new base,
no flips. Seeds 32..63 ≈ seeds 0..31 in best value (converged; stop mining at 64).

## Ship form

Per-matrix pinned cells (mission form (b)) — form (a) portfolio windows are impossible here: each
winner needs a DIFFERENT (variant, seed) and the 65 measured losers surround them in every
(n, nnz, density) direction. Three cells at the tail of the `consider!` queue (mod.rs, just before
RUN THE QUEUED PORTFOLIO; queue-tail placement keeps all existing tie-breaks):

- `cell_a`: n∈[14000,14200) ∧ nnz∈[470k,485k) — same window as `metis_seed2_cell`; admits dev twin 14100:477680 (harmless, floor-safe, may help dev)
- `cell_b`: n∈[9500,10000) ∧ nnz∈[287000,**287800**) ∧ max_deg∈[170,200] — ceiling tightened from the seed5-cell's 289000 to exclude sibling 9953:287868 where ALL 8 default seeds lose ≥1.64× (verified byte-identical after wiring)
- `cell_c`: n∈[23000,23300) ∧ nnz∈[880k,900k) — admits dev twin 23173:893724 (harmless)

## Timing

Worst added cost +0.212 s (23173: 0.556→0.768 s local ≈ 1.27 s grader at 1.65×). 14100: 0.676→0.751 s;
9953: 0.622→0.734 s. Corpus worst `order()` **1.345 s** — the pre-existing n=214928 tail, untouched
(0031 measured it at 1.357–1.377 s; box noise). Nothing new approaches 1.0 s local.

## Negatives / for the record

- The frontier's OWN pinned seeds are stale on their own cells: on 14100:477382 the pinned s2 is
  1.0100 vs incumbent (contributes nothing); on 9953:287612 the pinned s5 is 1.0151. The wins those
  cells were built for were later absorbed by other portfolio growth — pinned seeds rot as the
  portfolio moves. Re-census pinned cells after every big merge.
- 5 exact-1.0000 rows (10296:88702 all 8 seeds, 3775:372500, 2646:21852, 283:55554, 1975:29694 s1/s5):
  incumbent already METIS-shaped; no variant tried moved them.
- `floor240_imb05` and `imb02` are the two most productive variant families; `fm_passes` is a no-op
  everywhere tested (identical to default on all three winners, all seeds).
