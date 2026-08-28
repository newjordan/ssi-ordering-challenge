# 0022 — rgreedy-sparse: the exact elimination game on the mid band, via a quotient graph

- **Date:** 2026-08-29
- **Score:** 0.838672 → **0.838507** (−0.000165), fill tiebreak 0.947675 → 0.947625.
  Per bucket: lt_1k 0.8958 → 0.8958 (unchanged), 1k_10k 0.8726 → **0.8724**,
  gt_10k 0.7704 → **0.7701**. Scored run matched `probe_timing_and_score`'s
  prediction to all printed digits.
- **Status:** WIN, shipped on fleet/a. Shipped ceiling is the grader-safety
  **160M** cut (`clamp(160M − 1300·nnz, 80M, 160M)`), not the probed 240M; the
  cut cost 0.000046 of the 240M-ceiling prediction (0.838461 → 0.838507) and
  kept ~0.000165 of the win — the safety margin was nearly free.

## Hypothesis

The rgreedy exact elimination game (randomized exact-min-degree restarts +
prefix-freezing LNS with plateau acceptance, `rgreedy.rs`) won 61 matrices at
`n ≤ 3000` but is representation-bound: bitset adjacency is `~n²/4` bytes and
`O(n/64)` words per row operation, so the gate could not rise. Meanwhile 0019
measured the 3k–150k-nnz mid band as seed-STARVED with 0.6–1.4 s of unused
budget, and gt_10k (weight 0.40) is where the leverage is. A SPARSE
representation of the same game should carry the family into that band.

## What changed

New module `src/ordering/rgreedy_sparse.rs` + a call block in `mod.rs`
directly after the bitset rgreedy tier (same strict-improvement acceptance
through the trusted `ScoreWorkspace`, same fixed seed, runs LAST so it is
monotone by construction). Probes: `probe_rgreedy_sparse`,
`probe_rgreedy_sparse_identity`, and a fast non-ignored
`rgreedy_sparse_edge_cases` (edgeless / disconnected / complete / star / path:
bijection, determinism, accumulator == trusted scorer).

The representation is a quotient graph with **exact** external degrees
(MMD-style, no supervariables, no approximation):

- Eliminated pivots become elements; elements adjacent to a new pivot are
  absorbed (dead) since their boundary is a subset of the new one.
- `avars[v]` (original live neighbours) is kept DISJOINT from adjacent element
  boundaries by pruning at element formation — so the exact degree is
  `|avars| + |∪ lvars(e) \ {v}|`, computed by a stamped scatter.
- The dominant post-absorption case (no original neighbours, exactly one
  element) is O(1): `deg = |lvars(e)| − 1`. This is what keeps clique
  interiors linear instead of cubic.
- Pivot selection by doubly-linked degree buckets with a lazily-advanced dmin
  lower bound; reservoir sampling over buckets `dmin ..= dmin+slack` keeps the
  same uniform-among-candidates semantics as the bitset game. The min-fill
  tie-break policy was dropped (deficiency = pairwise boundary scan in a
  quotient graph); the slack ladder {0,1,2,3,5,8,16} is kept.
- Budget is a deterministic per-touched-list-element op counter. Unlike the
  bitset game a single sparse run has no a-priori cost bound (a hub pivot is
  O(deg²)), so `run()` hard-aborts mid-run past the budget — still a pure
  function of (pattern, seed), so determinism holds.

Gates (`mod.rs`): `RGREEDY_MAX_N (3000) < n ≤ 30_000`, `nnz ≤ 150_000`,
`max_deg·50 ≤ n` (0019's hub rule — here it is also the MECHANISM gate, since
a hub pivot is exactly what makes the quotient game quadratic; it excludes
`ringpack_30_2`, max_deg 1632, by construction). Per-matrix ops budget
DECREASING in nnz: `clamp(160M − 1300·nnz, 80M, 160M)` — the portfolio's own
cost on this band is nnz-driven (relabel multistarts), so slack under the
corpus worst case is largest exactly where nnz is smallest. The 160M ceiling
is a deliberate grader-safety cut from the probed 240M (the grader is ~2.6×
slower than this box on elimination-game work, and hidden in-gate matrices can
sit near the 2 s cap invisibly — the bitset tier lost two hidden-corpus cap
runs before its budget was cut); the cut cost only 0.000046 of predicted
score.

## Result (probes, quiet box, serialized under the fleet lock)

45 matrices admitted (27 in 1k_10k, 18 in gt_10k). At a flat 120M budget:
7 wins, predicted −0.000167. At flat 400M: 15 wins, predicted −0.000316, but
worst sparse cost 1.83 s — unshippable flat. The shipped nnz-scaled budget
lands between: `probe_timing_and_score` measured **0.838505 (flat 120M) →
0.838461 (scaled 80–240M) → 0.838507 (scaled 80–160M, SHIPPED)**, and a 300M
ceiling scored WORSE (0.838470) while adding time — budget is NOT monotone
here (more ops shift the LNS trajectory; same non-monotonicity 0019 recorded
for restarts). The scored run reproduced the 160M probe number exactly:
**0.838507**, tiebreak 0.947625.

Wins seen across the probe grid (flat 120M and flat 400M tables; r_port →
r_rg vs AMD):

| matrix | bucket | r_port → r_rg (budget) |
|---|---|---|
| mpbp_34 | gt_10k | 0.3217 → 0.3112 (400M) / 0.3157 (120M) |
| crudeoil_lee1_07 | 1k_10k | 0.8719 → 0.8657 (120M) |
| chp_shorttermplan1a | 1k_10k | 0.8730 → 0.8690 (400M) |
| mpbp_15 | 1k_10k | 0.8138 → 0.8113 (400M) |
| chp_shorttermplan2a | 1k_10k | 0.7144 → 0.7128 (400M) |
| crudeoil_li05 | 1k_10k | 0.8961 → 0.8927 (120M) |
| gasprod_sarawak16 | 1k_10k | 0.9955 → 0.9899 (400M) |
| mpbp_46 | 1k_10k | 0.9318 → 0.9269 (400M) |
| gabriel09 | gt_10k | 0.9327 → 0.9311 (400M) |
| rsyn0840m02m, rsyn0815m02hfsg, mpbp_21, powerflow0118p, nd_netgen-2000-3-4-b-a-ns_7, rsyn0840m04m | — | small |

## Timing (worst case, serial, quiet box)

Corpus worst is UNCHANGED: `acopf_case9241pegase_qcqp` **1.32–1.41 s** this
session (n=313k, not in this band, untouched — the spread vs the 1.11–1.23 s
measured earlier is the box's documented run-to-run load variance; the
runner-up `unitcommit_200_100_1_mod_8` at 1.31 s is also out of band and
untouched). Worst ADMITTED matrix under the shipped 160M config:
`squfl030-150` **0.77 s** total (n=13680, nnz=36000, budget ≈ 113M), then
`crudeoil_lee4_06` 0.70 s and `crudeoil_lee4_10` 0.69 s (clamped to the 80M
floor) — every admitted matrix stays ~0.5 s below the untouched corpus worst.
The sparse game measures ~220–680 M ops/s (6-thread contended; serial ≈ 1.7×
faster), so the 160M ceiling ≈ 0.42 s worst-case at the low-nnz end where
portfolio time is smallest.

## Why it won

Same reason the bitset tier won at small n: the exact game with plateau-drift
LNS explores a strictly larger space than anything relabelling can reach
(relabelling only perturbs AMD's internal tie-breaks), and the elimination-game
identity makes the exact objective free per candidate with mid-run pruning
against the incumbent. The quotient graph makes each candidate ~O(nnz)-ish
instead of O(n²/64), which is what unlocked 3k–30k. Gains are modest per
matrix (the portfolio is already strong here) but broad: 7–15 matrices across
both heavy buckets, no losses possible (strict-improvement floor).

## Leads for the next iteration

- **Band extension upward** (`n ≤ 40k+`, `nnz > 150k`): module `MAX_N` is 40k;
  reset is O(n + nnz) so larger n mainly costs per-run overhead. Unmeasured.
- **Phase split retune**: the 1/16 phase-A share was tuned at n ≤ 3000; the
  mid band may want more from-scratch restarts (0019's census gains appeared
  at 50–94 seeds).
- **Supervariables** (indistinguishable-node merging) would cut the hub-gate
  need and admit `ringpack_30_2`-class matrices; significant complexity.
- The 400M-flat table shows ~2× more score is reachable in this band if the
  time could be bought — per-matrix budget shaping sharper than linear-in-nnz
  (e.g. keyed to the measured port cost drivers) is the cheap version.
