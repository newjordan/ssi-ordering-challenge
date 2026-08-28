# 0024 — DETOUR bounds plus a MetaFlip-style shoulder seed

**Session:** `main` (2026-08-28). **Score:** 0.819962 → **0.819952**
(−0.000010), fill 0.934980 → **0.934978**. The full 300-matrix sandboxed
harness passed determinism, bijection, and the enforced per-matrix cap.

## Question

The portfolio retains only its strict argmin. That is correct for the immediate
objective, but 0020 proved that TELOS is seed-sensitive: a worse starting score
can descend into a better terminal basin. Can the portfolio preserve one bounded
near-elite candidate chosen for structural difference, without weakening the
trusted best-of floor or displacing any previously measured TELOS work?

This experiment combines two ideas from local research code, translated rather
than copied:

- **DETOUR:** keep an exact incumbent bound, distinguish proposal from
  certification, and prune only by a compatible objective. Here every candidate
  and descendant is still scored by the trusted exact `Σ c_j²` evaluator; no
  structural proxy can directly replace the incumbent.
- **MetaFlip:** a bounded quality-gated archive may preserve a near-elite
  “shoulder” because structural diversity can expose a different basin even when
  that item is not the current record. Here the archive is per matrix, capped at
  twelve candidates, deterministic, and reduced to one extra seed immediately.

The crucial boundary is that diversity chooses only **where to search**. Exact
flops still choose **what may ship**.

## Design

The parallel portfolio previously released every non-winning permutation after
scoring. A new `run_candidates_keep_all` path preserves them until the sequential
phase begins, with identical task order, scoring, panic isolation, and result
order. The caller then immediately constructs a bounded shortlist:

1. require a valid permutation and exact flops strictly below the AMD anchor;
2. sort by `(flops, source index)` and keep at most the twelve best;
3. define the portfolio argmin as the archive reference;
4. compare the final 256 elimination positions of each other candidate;
5. maximize suffix-set disagreement first and positional disagreement second;
6. break ties by lower exact flops, then earlier source order;
7. retain only that one `(flops, permutation)` pair as the shoulder seed.

The tail descriptor is intentionally cheap. Late elimination columns tend to
carry the largest `c_j²` terms, and two orderings with different terminal sets or
positions are more plausibly in different TELOS basins than two orderings that
only permute an early low-cost prefix. It is still merely a descriptor: no claim
is made that tail distance predicts a better exact ordering on its own.

The TELOS multi-start table grows from fourteen entries to seventeen by appending
three modes for slot 2: `(2,7)`, `(2,2)`, `(2,0)`. They are deliberately the last
three entries. `telos_ms_count` computes the number of entries affordable from a
fixed work proxy, so every previously shipped best-seed and AMD-seed descent
retains its position and eligibility. Only matrices cheap enough to exhaust all
fourteen old entries can reach the new seed.

Slots 0 and 1 retain the historical `(seed flops, mode)` deduplication. Slot 2
gets a separate tag even if its exact objective equals another seed. This is the
one place where score equality is evidence rather than redundancy: equal-score,
different-structure is precisely the plateau/basin signal the archive is meant
to retain. All TELOS outputs still go through the existing strict-improvement
acceptance against the incumbent.

## Why this is monotone in score

The modification does not replace or reorder any existing candidate or descent.
It appends proposals after the old plan. The AMD anchor remains available, the
portfolio argmin is accepted exactly as before, and each extra descendant is
admitted only when the trusted scorer returns fewer flops than `best_flops`.
Therefore the score can stay equal or improve; the remaining submission risk is
resource use, not objective regression.

Memory use is bounded in two relevant ways. The final archive is twelve indices
and one cloned permutation. During parallel scoring, permutations live until the
portfolio batch completes rather than being released as soon as they lose. On
large matrices the existing gates make the task count small; on the widest
portfolio matrices `n` is small. The full corpus run stayed within the worker
contract. This is empirical cap evidence, not a formal peak-allocation proof for
unknown hidden shapes.

Time exposure is also intentionally narrow. The selection pass is linear in the
retained portfolio output plus at most `12 × 256` descriptor comparisons. The
expensive part is three possible TELOS descents, but they are appended beyond the
entire old fourteen-entry plan and use the existing fixed work-count gate. This
explains why the measured change appears only in the cheapest bucket.

## Authoritative run

The worker was rebuilt through the repository's sandboxed candidate build and
the trusted parent was run offline and locked:

```text
bash scripts/local-candidate-build.sh
cargo run --release --offline --locked -- \
  --note "DETOUR+MetaFlip bounded portfolio shoulder seed"
```

Result:

| metric | before | after | delta |
|---|---:|---:|---:|
| weighted score | 0.819962 | **0.819952** | **−0.000010** |
| fill | 0.934980 | **0.934978** | **−0.000002** |
| lt_1k | 0.892517 | **0.892483** | **−0.000034** |
| 1k_10k | 0.862245 | 0.862245 | 0 |
| gt_10k | 0.733834 | 0.733834 | 0 |

All 300 matrices completed. The harness accepted every returned ordering as a
deterministic bijection and reported no timeout or worker failure. The unchanged
middle and heavy buckets are important negative evidence: this implementation
does not buy additional search there because the appended slots are beyond their
affordable `telos_ms_count`. Widening the work budget merely to spread this idea
would violate the design and would need a new, isolated timing experiment.

The solver context for this run was GPT 5.6 Sol through Codex. The exact effort
setting was not exposed to the session, so none is asserted here.

## Interpretation

This is a very small but strict dev win. Its value is less the absolute delta
than the falsifiable mechanism:

- 0020 established that one best seed is insufficient for a non-convex,
  seed-sensitive descent.
- A bounded structural shoulder recovers a new terminal basin on at least one
  matrix without changing the exact acceptance rule.
- Placing it after the measured plan localizes both the gain and the cost to the
  cheapest tier.

The result does **not** establish that suffix distance is a generally good
diversity metric, that twelve is an optimal archive size, or that more shoulder
seeds will pay. It also does not transfer MetaFlip's record-search claims or
DETOUR's proof claims into sparse ordering. What transfers is the architecture:
quality-gated proposal diversity, a hard archive bound, and exact independent
certification of every accepted result.

## Follow-ups

If official validation promotes this change, the next useful diagnostic is a
test-only attribution probe that records which source candidate became the
shoulder and which TELOS mode produced each strict win. That evidence can test
whether the descriptor is genuinely selecting a distinct family or merely a
duplicate-score permutation from the same family.

If official validation is flat, do not increase the archive or move the new
slots earlier by intuition. First identify the dev winner and measure isolated
cost. A larger or earlier schedule would displace known-good descents on tighter
matrices and would no longer be monotone relative to the shipped computation
plan, even though final score acceptance remains monotone.

## Official failure and bounded rescue

The first submission (`87c6490`, grader commit `75d7446`) did **not** produce a
score. The public benchmark action ended with:

```text
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

This corrects the local cap conclusion above. The extra TELOS descents were
budgeted, but `run_candidates_keep_all` and `portfolio_shoulder` ran on **every**
matrix. A matrix whose `telos_ms_count` stopped inside the historical fourteen
entries could never consume the seed, yet still retained every losing portfolio
permutation and built the descriptor. The score argument was monotone; the
resource argument was not.

The rescue computes the deterministic TELOS work count immediately after
`(n, nnz, max_deg)` are known, before the portfolio runs. If no shoulder slot is
reachable, the code calls the historical argmin-only `run_candidates` path and
does no archive work. It also confines shoulder eligibility to `n < 1000`, the
only bucket with a measured output change. Thus every middle/heavy dev matrix
and every work-ineligible small matrix takes the old retention path exactly.

An authoritative 14-entry versus 17-entry small-corpus comparison identified
exactly two changed matrices out of 147:

| matrix | old plan | first shoulder | exact gain |
|---|---:|---:|---:|
| `ex8_3_9` | 11,550 | 11,518 | −32 flops |
| `graphpart_3g-0244-0244` | 32,566 | 32,474 | −92 flops |

Adding the five remaining rank modes to that same seed changed **zero** of 147
matrices, so they were reverted. Changing the independent variable did work:
a second archive cell, selected by greedy max-min distance from both the
portfolio argmin and the first shoulder, takes `ex8_3_9` from 11,518 to 11,503
and changes no other small matrix. Its three modes are appended after the first
shoulder's submitted prefix.

The complete sandboxed 300-matrix replay after the pre-portfolio gate and second
cell passed:

| metric | frontier | gated two-cell archive | delta |
|---|---:|---:|---:|
| weighted score | 0.819962 | **0.819949** | **−0.000013** |
| fill | 0.934980 | **0.934976** | **−0.000004** |
| lt_1k | 0.892517 | **0.892475** | **−0.000042** |
| 1k_10k | 0.862245 | 0.862245 | 0 |
| gt_10k | 0.733834 | 0.733834 | 0 |

This local pass is necessary, not proof of grader safety—the first attempt made
that distinction concrete. The stronger evidence is structural: all unmeasured
middle/heavy shapes now bypass the new retention path, while the only class that
can enter it is the class with exact per-matrix dev wins and substantial local
headroom.
