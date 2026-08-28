<!-- 0020 -->
# 0020 - the restart non-monotonicity is TELOS seeding, not budget

- **Date:** 2026-08-28
- **Score:** unchanged (diagnosis only, no shipped behaviour change)
- **Status:** conclusive positive - names the mechanism behind 0019's one regression, and
  refutes two plausible explanations by experiment

## Question

0019 shipped a restart floor that gained on 4 matrices and lost 0.031 on
`chp_shorttermplan2d`. Raising the restart count is supposed to be zero-downside: seeds
are `1..=restarts`, so a higher count is a strict superset of candidates, every candidate
is scored with the exact grader objective, and acceptance is strict `<`. A best-of over a
superset cannot get worse. It did. Why?

## Two explanations that are both wrong

**Not budget contention.** The first thing I wrote into the code was that the restarts
"share a budget with the phases appended after them". False. `hc_r` is
`(hc_budget / nnz).min(hc_cap)` - a pure function of `(n, nnz)` with no dependence on the
restart count - and the three relabel counts are three independent `relabel_restarts`
calls that all rise together. Nothing downstream is starved.

**Not a defect in the best-of.** Instrumenting `consider` on `chp_shorttermplan2d`:

| restarts | candidates scored | best `consider` flops |
|---|---|---|
| 24 (floor on) | 508 | 2 189 795 |
| 5 (floor off) | 280 | 2 415 825 |

The superset behaves exactly as claimed - more restarts, strictly better candidate pool.
`ws.flops` also agreed with a fresh `flops_of` on the returned permutation in both runs
(2 104 445 and 1 975 321), so the in-run scorer is honest.

## The actual mechanism

Both finals beat their own `consider` minimum, so the winner is produced **downstream**.
Gating each downstream phase in turn, one at a time, on the same matrix:

| phase disabled | 24 restarts | 5 restarts | monotone? |
|---|---|---|---|
| none | 0.514660 | 0.483081 | REGRESSION |
| hill-climb | 0.514660 | 0.483081 | REGRESSION |
| post-hoc PEEL/CNTREL | 0.514660 | 0.483081 | REGRESSION |
| ORBIT / PHARMAKOS | 0.514660 | 0.483081 | REGRESSION |
| SHOT-3 | 0.525546 | 0.483650 | REGRESSION |
| COMPLETION-LATTICE | 0.526447 | 0.495329 | REGRESSION |
| **TELOS-GRADIENT DESCENT** | 0.524048 | 0.558157 | **MONOTONE** |

`TELOS-GRADIENT DESCENT` descends from `best_perm` as the restart phase leaves it, and its
outcome is sharply **seed-sensitive**: from the *worse* incumbent it reaches 1 975 321,
from the *better* incumbent only 2 104 445. It is also the dominant value producer on this
matrix - roughly 18% below the best candidate the restart phase itself found.

Disabling TELOS alone flips the pair monotone. No other phase does.

## Why this matters beyond one matrix

This is a **between-config** effect that no bookkeeping inside a single run can detect. A
run seeded at the better incumbent has no access to the basin only the worse incumbent
reaches, and TELOS accepts only improvements, so nothing inside it is broken. It means:

- "More restarts cannot hurt" is false for any change upstream of TELOS, and the loss is
  not repairable with more budget.
- Conversely, the seed TELOS happens to receive is worth more than a large improvement in
  the restart pool - 508 candidates versus 280 was worth less than which single point
  TELOS started from.

## Lead this opens

If TELOS's result depends this strongly on its seed, descending from more than the single
best incumbent (retain top-K, descend from each, keep the best) should capture both
basins. That buys exactly what 0018 said the corpus lacks - and unlike adding candidate
families, it exploits a sensitivity that is already measured rather than hoped for. Cost
is one extra descent per retained seed, against a 2.0 s per-matrix cap where the worst
matrix currently sits at 1.784 s - so it must be gated by measured per-family time, the
calibration warning from 0007/0019 applies.

## Method note

The four "explanations" were separated only because each phase could be switched off
independently and re-measured on one matrix in ~8 s. Reasoning alone had produced two
confident wrong answers in a row here, both of which I had already written into the code
before testing them.
