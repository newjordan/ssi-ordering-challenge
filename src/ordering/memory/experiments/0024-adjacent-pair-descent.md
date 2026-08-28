# 0024 — exact adjacent-pair descent after final TELOS

- **Date:** 2026-08-29
- **Base:** `70a7701`
- **Candidate:** `865903f`
- **Funnel state:** **TERMINAL promoted — LEARNED**
- **Yukon UUID:** `a6439d59-5067-42cd-ad7c-5c1640414ba6`
- **Official score:** `0.827559`, fill `0.936716`, official diff `-0.000114`
- **Promoted source:** `33034d6`
- **Evidence state:** verified-correct, worker-verified, and official-terminal positive

## Mechanism

Replay the exact elimination game for the fully downstream final incumbent and
orient consecutive adjacent pivot pairs lower-current-degree first. For an
adjacent pair `a,b`, either orientation leaves the same residual graph after
both pivots and gives the second pivot the same live neighborhood
`(N(a) union N(b)) - {a,b}`. Only the first column-count term differs, so
`degree(b) < degree(a)` is an exact strict-improvement test for swapping them.
Non-adjacent eliminations commute and are left unchanged.

Two deterministic disjoint-pair sweeps use parity zero and one. The second
replays the permutation produced by the first, so every adjacent boundary is
examined under an updated exact state.

## Frontier response and placement

`70a7701` restores watcher MINL and composes all six portfolio arms, including
parallel RGREEDY and parallel final TELOS multi-start. Pair descent is a
COMPLEMENT placed after final TELOS and before bookkeeping/return. No watcher,
RGREEDY, TELOS, or other incumbent-driven phase consumes it as a seed.

This is not relabel-budget expansion, randomized exact-game search, completion
edge deletion, component fusion, articulation handling, or another TELOS move.
It is a deterministic lateral two-pivot exchange on the final ordering.

## Admission and fail-closed behavior

The production gate is:

```text
1,000 <= n <= 4,000
0 < off-diagonal nnz <= 60,000
two parity sweeps
cumulative deterministic operation budget = 32,000,000
```

Malformed input, a non-bijective seed, graph-build failure, operation-budget
exhaustion, or no strict swap returns `None`. A generated candidate is checked
again as a bijection and exact-scored through the shared `ScoreWorkspace`; it is
adopted only when `candidate_flops < existing_final_flops`.

## Verification

Focused release tests passed 4/4 on the exact `70a7701` integration base:

- a center-first star strict-improvement witness with deterministic bijection;
- 96 deterministic small graph/permutation cases asserting every returned
  candidate strictly improves the independent trusted score;
- singleton, duplicate seed, and budget-exhaustion fail-closed cases;
- public `order()` default-path reach exactly once with deterministic bijection.

The network-denied sandbox release worker build also passed. No dependency,
harness, scorer, corpus, or grader file changed.

## Complexity and cap argument

The bitset graph is about 4 MiB at the `n=4,000` ceiling; remaining state is
linear in `n`. The two replays share one cumulative deterministic operation
budget and budget exhaustion discards the candidate. A completed opportunity
pays one existing trusted exact score. There is no restart population,
completion watcher table, recursive search, or extra scoring workspace.

Numerical non-regression for completed calls is structural because the phase is
last and uses trusted strict adoption. Hidden activation frequency and benefit
remain a **HYPOTHESIS**; no public aggregate is claimed. Yukon terminal is the
official performance and hidden-cap authority.

## Official terminal result

Yukon promoted UUID `a6439d59-5067-42cd-ad7c-5c1640414ba6` at FLOPS score
`0.827559` and fill tiebreak `0.936716`, an official improvement of `0.000114`
over the immediately preceding live frontier. The exact adjacent-pair identity
therefore transferred to hidden matrices and completed inside the hidden cap.
Promoted source `33034d6` is now the campaign base.
