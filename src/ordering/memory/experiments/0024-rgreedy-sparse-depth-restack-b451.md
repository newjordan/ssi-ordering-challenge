# 0024 — sparse RGREEDY depth survives the adjacent-pair frontier

- **Date:** 2026-08-29
- **Base:** `b45143594df66d9e78c19720c1912dc446552737` (official score 0.822229)
- **Public dev:** 0.820416 → **0.820113** (−0.000303; lower is better)
- **Status:** win; exact-live-source A/B, tests and production sandbox build pass

## 1. Where the structure was blocked

Whole-graph RGREEDY still used a wide/shallow allocation: `nnz <= 250_000` and
200M deterministic word-operations per stream. Since the preceding experiment,
the live frontier moved again. It now includes all of the following overlapping
small/mid search:

- adjacent-pair exact-degree descent for `1,000 <= n <= 4,000`, `nnz <= 60k`;
- small relabel multistart expanded from `600k/200` to `3M/800` restarts;
- an AMD-seeded one-round TELOS gate on a sparse low-hub band;
- the already-aggressive RGSUB refinement starting at `n >= 500`.

The question was therefore not whether the old result could be copied. It was
whether the new phases had absorbed the deeper whole-graph trajectory signal.

## 2. How the block opened

Only two production constants changed:

- `RGREEDY_MAX_NNZ`: 250,000 → **60,000**
- `RGREEDY_BUDGET`: 200M → **600M** operations per stream

This gives up low-yield dense reach and spends three times the deterministic
depth on sparse trajectories. The exact `b451435` full-300 A/B measured:

| bucket | live baseline | sparse-depth | delta |
|---|---:|---:|---:|
| `lt_1k` | 0.8934 | **0.8930** | −0.0004 |
| `1k_10k` | 0.8629 | **0.8623** | −0.0006 |
| `gt_10k` | 0.7338 | **0.7338** | 0 |
| **total** | **0.820416** | **0.820113** | **−0.000303** |

At the displayed four-decimal row precision, 26 matrices improve, 19 regress and
255 remain unchanged. Budget changes are not per-matrix monotone: the portfolio
decision is the full-corpus geomean, not a superset claim. Representative wins
include `pooling_adhya4pq` 0.8900→0.8647, `multiplants_stg1c`
0.9776→0.9568 and `ndcc13` 0.6591→0.6469. The unchanged heavy bucket is the
out-of-scope control.

The same restack improved the preceding exact `37131d5` source by −0.000504.
The new adjacent-pair/small-multi/TELOS structure absorbs about 40% of that local
gain, but a material −0.000303 remains. This is direct evidence that the phases
overlap without being equivalent: local pair orientation and relabel/TELOS
basins do not replace deeper whole-graph exact-elimination trajectories.

## 3. How the next rise can proceed

1. Preserve 60k/600M as the new controlled baseline on the latest source.
2. Measure one stream slot at a time; do not wholesale-swap streams 0–3 for
   4–7, which previously regressed 0.828055→0.828209 at equal 600M×4 work.
3. Test a deterministic cost-tier inside `nnz <= 60k`: allocate depth from the
   estimated cost of one run rather than using another global nnz threshold.
4. If testing the 40k/60k boundary, isolate the two public rows in that band
   (`qap`, `qspp_0_11_0_1_10_1`) before another full-corpus run. The 60k/80k
   band contains no publicly eligible row under the current `n` and degree
   gates, so 80k is a public-corpus no-op rather than evidence of hidden safety.

## 4. Where structural change can happen

- **Allocation gate:** `src/ordering/mod.rs`, at `RGREEDY_MAX_NNZ` and
  `RGREEDY_BUDGET`. This is the only production behavior changed here.
- **Run-cost tiering:** `src/ordering/rgreedy.rs`, using predicted or previous
  measured deterministic operations to decide whether another trajectory fits.
- **Overlap audits:** the adjacent-pair, RGSUB, TELOS and relabel-multistart
  blocks in `src/ordering/mod.rs`. Any one can absorb a result when the live
  frontier moves, so every candidate must be rebased and remeasured.
- **Orchestrator decision:** prefer latest-source full-corpus improvement with
  an unchanged control bucket; reject old-frontier deltas and wall-time-only
  claims.

## Evidence boundary

The baseline reported worst `order()` 2.006 s and the candidate 1.533 s, but
they were separate runs under changing shared-host load. The first baseline
attempt also left a timeout orphan; it was identified by exact worktree command
line and parent PID and terminated, after which the remaining baseline process
completed alone. The timings are non-causal and do **not** prove runtime
nonincrease.

Verified evidence: the deterministic score delta above, 38/38 nonignored tests,
and a network-denied production candidate build.

The hidden two-second watchdog remains an explicit promotion gate. The previous
P24 union failed that watchdog after combining RG600 with an extra adjacent-pair
cycle on the older `37131d5` source. That failure cannot identify which hidden
matrix or which member of the union consumed the final margin. The current
candidate removes the redundant extra pair cycle and inherits the single
hidden-proven pair phase from `b451435`, but it also inherits the stronger
low-nnz relabel floor. Therefore this note does not infer cap safety from the
separate-run worst time. The official runner must distinguish three outcomes:
watchdog failure retires or reduces this budget; a valid non-improving score
retains the mechanism as overlap evidence; only a valid score below the live
record authorizes promotion. A prepared 300M budget hedge is measured
separately rather than silently weakening this exact 600M experiment.

## Reproduction artifacts

- `target/probe-p26/p26-b451-baseline.txt`
- `target/probe-p26/p26-b451-rg600.txt`

These ignored outputs are local artifacts; the base hash, two constants and
aggregate measurements above are the durable record.
